import { useState } from "react";
import { CodeSyntax } from "./components/CodeBlock";
import {
  Alert,
  Button,
  FileButton,
  Group,
  Modal,
  PasswordInput,
  SegmentedControl,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
} from "@mantine/core";
import { useMutation } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
  Globe,
  KeyRound,
  Link2,
  Grid2X2,
  Ban,
  CheckCircle2,
  Info,
  Server,
  FileCode,
  ShieldCheck,
} from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useWorkspace } from "./app/context";
import { useIdempotency } from "./useIdempotency";
import { ErrorState } from "./components/AsyncState";
import {
  parseConfigDocument,
  parseMcpConfig,
  safeConfigPreview,
  type ConfigFormat,
  type ServerDraft,
} from "./features/connections/mcpConfig";

type Manifest = components["schemas"]["McpDefinitionManifest"];
type Summary = components["schemas"]["McpDefinitionSummary"];
export function McpConnectorSetup({
  close,
  saved,
  brain,
  approvedConnectors = [],
  reuse,
  initialMethod,
}: {
  initialMethod?: string;
  close: () => void;
  saved: (connected?: boolean) => void;
  brain?: Brain;
  approvedConnectors?: Summary[];
  reuse?: (draft?: ServerDraft) => void;
}) {
  const session = useWorkspace();
  const owner = session.user.installation_owner;
  const [method, setMethod] = useState(
    owner ? (initialMethod ?? "url") : "approved",
  );
  const [format, setFormat] = useState<ConfigFormat>("auto");
  const [source, setSource] = useState("");
  const [drafts, setDrafts] = useState<ServerDraft[]>([]);
  const [selected, setSelected] = useState("0");
  const [name, setName] = useState("");
  const [editorScroll, setEditorScroll] = useState(0);
  const [url, setUrl] = useState("");
  const [auth, setAuth] = useState("none");
  const [token, setToken] = useState("");
  const [manifest, setManifest] = useState<Manifest | null>(null);
  const [issue, setIssue] = useState<Error | null>(null);
  const [phase, setPhase] = useState("edit");
  const [created, setCreated] = useState<
    components["schemas"]["McpConnectionSummary"] | null
  >(null);
  const command = useIdempotency();
  const draft: ServerDraft = drafts[Number(selected)] ?? {
    name,
    target: url,
    transport: "streamable_http",
    command: null,
    arguments: [],
    configuration: {},
    secrets:
      auth === "secret"
        ? [
            {
              kind: "header",
              destination: "Authorization",
              variable: "MCP_TOKEN",
              prefix: "Bearer ",
              value: token,
            },
          ]
        : [],
  };
  const headers = Object.fromEntries(
    draft.secrets
      .filter((s) => s.kind === "header")
      .map((s) => [s.destination, s.prefix + s.value]),
  );
  const clearReview = () => {
    setManifest(null);
    setPhase("edit");
    setIssue(null);
    inspection.reset();
    save.reset();
  };
  const inspection = useMutation({
    mutationFn: async () => {
      if (!owner)
        throw new Error(
          "The installation owner can inspect and approve connectors.",
        );
      if (draft.secrets.some((s) => s.value.length < 4))
        throw new Error(
          "Fill in each masked credential before inspecting this server.",
        );
      return result(
        await client.POST("/api/mcp/definitions/inspect-http", {
          body: { name: draft.name, url: draft.target, headers },
        }),
      );
    },
    onSuccess: (value) => {
      setManifest(value);
      setPhase("review");
    },
  });
  const save = useMutation({
    mutationFn: async () => {
      if (!manifest)
        throw new Error(
          "Review an inspected server or a connector manifest first.",
        );
      const definition = result(
        await client.POST("/api/mcp/definitions", { body: manifest }),
      );
      if (!brain) return false;
      let connection = created;
      if (!connection) {
        const body = {
          name: draft.name || manifest.name,
          description: manifest.description ?? "",
          definition_key: definition.key,
          target: draft.target || manifest.name,
          placement: "central",
          runner_reference: null,
          credential_alias: draft.secrets.length
            ? manifest.credential_aliases?.[0]
            : null,
          environment_id: null,
          configuration: draft.configuration,
          enabled: true,
          base_revision: null,
        };
        connection = result(
          await client.POST("/api/brains/{brain}/mcp/connections", {
            params: { path: { brain: brain.id } },
            body,
            headers: { "Idempotency-Key": command.forInput(body) },
          }),
        ).summary;
        setCreated(connection);
      }
      if (draft.secrets.length)
        result(
          await client.POST(
            "/api/brains/{brain}/mcp/connections/{id}/credentials",
            {
              params: { path: { brain: brain.id, id: connection.id } },
              body: {
                base_revision: connection.revision,
                headers: draft.secrets
                  .filter((s) => s.kind === "header")
                  .map((s) => ({
                    name: s.destination,
                    prefix: s.prefix,
                    value: s.value,
                  })),
                environment: Object.fromEntries(
                  draft.secrets
                    .filter((s) => s.kind === "environment")
                    .map((s) => [s.destination, s.value]),
                ),
              },
            },
          ),
        );
      return true;
    },
    onSuccess: (connected) => {
      setSource("");
      setDrafts([]);
      setToken("");
      saved(connected);
    },
  });
  const parse = () => {
    clearReview();
    try {
      const document = parseConfigDocument(source, format);
      if (typeof document.key === "string" && Array.isArray(document.tools)) {
        // Canonical manifests contain metadata only. The server validates the full schema.
        setManifest(document as Manifest);
        setSource(JSON.stringify(document, null, 2));
        setPhase("review");
      } else {
        const servers = parseMcpConfig(source, format);
        setDrafts(servers);
        setSelected("0");
        setSource(servers.map(safeConfigPreview).join("\n\n"));
      }
    } catch (error) {
      setIssue(error as Error);
    }
  };
  const busy = inspection.isPending || save.isPending;
  const credentialStorage = brain
    ? "Local development file"
    : "Inspection only";
  const setSecret = (i: number, value: string) => {
    setDrafts((ds) =>
      ds.map((d, n) =>
        n === Number(selected)
          ? {
              ...d,
              secrets: d.secrets.map((v, j) => (j === i ? { ...v, value } : v)),
            }
          : d,
      ),
    );
    clearReview();
  };
  const inspectDisabled =
    !draft.name.trim() ||
    !draft.target.trim() ||
    busy ||
    auth === "reference" ||
    (method === "config" && !drafts.length) ||
    draft.transport === "stdio";
  const displayedAuth =
    method === "config" ? (draft.secrets.length ? "secret" : "none") : auth;

  return (
    <Modal
      opened
      onClose={() => {
        if (!busy) close();
      }}
      title={brain ? "Add connection" : "Add a connector"}
      size={912}
      className="connection-setup-modal"
      closeOnClickOutside={!busy}
      closeOnEscape={!busy}
    >
      <Stack gap="md" className="connection-setup-body">
        <Text className="connection-setup-intro">
          {brain
            ? "Connect an MCP server to this Brain."
            : "Approve a reusable MCP definition for your installation."}
        </Text>
        <SegmentedControl
          fullWidth
          className="vision-segment connection-methods"
          value={method}
          disabled={busy || !!created}
          onChange={(v) => {
            clearReview();
            setMethod(v);
            setAuth("none");
            setDrafts([]);
            setSource("");
            setToken("");
          }}
          data={[
            {
              value: "url",
              label: (
                <span>
                  <Link2 size={20} />
                  Server URL
                </span>
              ),
              disabled: !owner,
            },
            {
              value: "config",
              label: (
                <span>
                  <FileCode size={20} />
                  Paste config
                </span>
              ),
              disabled: !owner,
            },
            ...(brain
              ? [
                  {
                    value: "approved",
                    label: (
                      <span>
                        <Grid2X2 size={20} />
                        Approved connector
                      </span>
                    ),
                  },
                ]
              : []),
          ]}
        />
        {method === "approved" ? (
          <div className="management-surface">
            <h3 className="management-title">Use an approved connector</h3>
            <p className="management-muted">
              {approvedConnectors.length} approved{" "}
              {approvedConnectors.length === 1 ? "connector" : "connectors"}{" "}
              available. Choose its target, credentials and execution location
              next.
            </p>
            <Button
              disabled={!approvedConnectors.length}
              onClick={() => reuse?.()}
            >
              Choose connector
            </Button>
            {owner && (
              <Text mt="md" size="sm">
                <Link to="/connectors">Manage the connector library →</Link>
              </Text>
            )}
          </div>
        ) : (
          <>
            {method === "url" ? (
              <Group grow align="start" className="connection-url-fields">
                <TextInput
                  label="Server name"
                  value={name}
                  onChange={(e) => {
                    setName(e.currentTarget.value);
                    clearReview();
                  }}
                  required
                  disabled={busy || !!created}
                />
                <TextInput
                  label="MCP server URL"
                  placeholder="https://example.com/mcp"
                  value={url}
                  onChange={(e) => {
                    setUrl(e.currentTarget.value);
                    clearReview();
                  }}
                  required
                  disabled={busy || !!created}
                />
              </Group>
            ) : (
              <>
                <Group
                  justify="space-between"
                  className="connection-config-heading"
                >
                  <h3 className="management-title">MCP configuration</h3>
                  <Group gap="sm">
                    <FileButton
                      accept=".json,.toml,.yaml,.yml,text/plain"
                      onChange={async (file) => {
                        if (!file) return;
                        clearReview();
                        setDrafts([]);
                        if (file.size > 128 * 1024) {
                          setIssue(
                            new Error(
                              "Choose a configuration file under 128 KiB.",
                            ),
                          );
                          return;
                        }
                        try {
                          setFormat("auto");
                          setSource(await file.text());
                        } catch {
                          setIssue(
                            new Error(
                              "This configuration file could not be read.",
                            ),
                          );
                        }
                      }}
                    >
                      {(props) => (
                        <Button
                          {...props}
                          variant="subtle"
                          size="compact-xs"
                          disabled={busy || !!created}
                          aria-label="Choose config file"
                        >
                          Import file
                        </Button>
                      )}
                    </FileButton>
                    <SegmentedControl
                      className="vision-segment config-format-tabs"
                      aria-label="Configuration format"
                      size="xs"
                      value={format}
                      onChange={(v) => {
                        setFormat(v as ConfigFormat);
                        clearReview();
                      }}
                      disabled={busy || !!created}
                      data={[
                        { value: "auto", label: "Auto-detect" },
                        { value: "json", label: "JSON" },
                        { value: "toml", label: "TOML" },
                        { value: "yaml", label: "YAML" },
                      ]}
                    />
                  </Group>
                </Group>
                <div className="numbered-config-editor">
                  <div
                    className="config-line-numbers"
                    aria-hidden="true"
                    style={{ transform: `translateY(-${editorScroll}px)` }}
                  >
                    {source.split("\n").map((_, i) => (
                      <div key={i}>{i + 1}</div>
                    ))}
                  </div>
                  <pre
                    aria-hidden="true"
                    className="config-highlight markdown-code-tokens"
                    style={{ transform: `translateY(-${editorScroll}px)` }}
                  >
                    <CodeSyntax
                      code={source}
                      language={
                        format === "auto"
                          ? /^\s*(?:\{|\[\s*[\{"\d-])/.test(source)
                            ? "json"
                            : /^\s*\[/.test(source) ||
                                /^\s*[\w.]+\s*=/.test(source)
                              ? "toml"
                              : "yaml"
                          : format
                      }
                    />
                  </pre>
                  <Textarea
                    aria-label="MCP configuration"
                    className="mcp-config-editor"
                    placeholder={
                      '{ "mcpServers": { "docs": { "url": "https://example.com/mcp" } } }'
                    }
                    rows={8}
                    value={source}
                    onScroll={(e) =>
                      setEditorScroll(
                        (e.target as HTMLTextAreaElement).scrollTop,
                      )
                    }
                    onChange={(e) => {
                      clearReview();
                      setSource(e.currentTarget.value);
                      setDrafts([]);
                    }}
                    disabled={busy || !!created}
                  />
                </div>
                {!drafts.length && phase === "edit" && (
                  <Button
                    variant="light"
                    size="compact-sm"
                    leftSection={<FileCode size={16} />}
                    onClick={parse}
                    disabled={!source.trim()}
                  >
                    Parse configuration
                  </Button>
                )}
                {drafts.length > 1 && (
                  <Select
                    label="Server to add"
                    value={selected}
                    data={drafts.map((d, i) => ({
                      value: String(i),
                      label: d.name,
                    }))}
                    onChange={(v) => {
                      setSelected(v ?? "0");
                      clearReview();
                    }}
                    disabled={busy || !!created}
                  />
                )}
              </>
            )}
            {drafts.length > 0 && (
              <div className="connection-parsed-strip">
                <CheckCircle2 size={17} />
                <span>
                  Parsed {drafts.length}{" "}
                  {drafts.length === 1 ? "server" : "servers"} ·{" "}
                  {draft.transport === "stdio" ? "Local command" : "HTTP"}.{" "}
                  Secret values are masked.
                </span>
              </div>
            )}
            {(method === "url" || drafts.length > 0) && (
              <>
                <div className="connection-setup-summary">
                  <div className="connection-summary-server">
                    <span className="management-icon">
                      <Server />
                    </span>
                    <div>
                      <strong>{draft.name || "New server"}</strong>
                      <small>
                        {draft.command || draft.target || "Enter a server URL"}
                      </small>
                    </div>
                  </div>
                  <div>
                    <small>Runs on</small>
                    <strong>
                      {draft.transport === "stdio"
                        ? "Approved runner required"
                        : "Recollect service"}{" "}
                      {reuse && (
                        <Button
                          variant="subtle"
                          size="compact-xs"
                          onClick={() => reuse(draft)}
                        >
                          Change
                        </Button>
                      )}
                    </strong>
                  </div>
                  <div>
                    <small>Environment</small>
                    <strong>Brain-wide</strong>
                  </div>
                </div>
                <div className="connection-authentication">
                  <Group justify="space-between" mb="md">
                    <h3 className="management-title">Authentication</h3>
                    <SegmentedControl
                      className="vision-segment connection-auth-tabs"
                      value={displayedAuth}
                      disabled={busy || !!created}
                      onChange={(v) => {
                        if (method === "config") {
                          if (v === "reference") reuse?.(draft);
                        } else {
                          setAuth(v);
                          setToken("");
                          clearReview();
                        }
                      }}
                      data={[
                        {
                          value: "secret",
                          label: (
                            <span>
                              <KeyRound size={18} />
                              Secret
                            </span>
                          ),
                          disabled:
                            method === "config" && !draft.secrets.length,
                        },
                        {
                          value: "reference",
                          label: (
                            <span>
                              <Link2 size={18} />
                              Credential reference
                            </span>
                          ),
                          disabled: !reuse,
                        },
                        {
                          value: "none",
                          label: (
                            <span>
                              <Ban size={18} />
                              None
                            </span>
                          ),
                          disabled:
                            method === "config" && !!draft.secrets.length,
                        },
                      ]}
                    />
                  </Group>
                  {method === "url" && auth === "reference" && (
                    <Text size="sm" c="dimmed">
                      Choose an approved connector to select an existing
                      credential alias.{" "}
                      <Button
                        variant="subtle"
                        size="compact-xs"
                        disabled={!reuse}
                        onClick={() => reuse?.()}
                      >
                        Use an approved connector
                      </Button>
                    </Text>
                  )}
                  {displayedAuth === "secret" &&
                    (method === "url"
                      ? [
                          {
                            variable: "MCP_TOKEN",
                            value: token,
                            destination: "Authorization",
                          },
                        ]
                      : draft.secrets
                    ).map((secret, i) => (
                      <div
                        className="connection-auth-fields"
                        key={secret.destination}
                      >
                        <TextInput
                          label="Secret variable name"
                          value={secret.variable}
                          readOnly
                          title={secret.destination}
                        />
                        <PasswordInput
                          label={
                            method === "url" ? "Bearer token" : "Secret value"
                          }
                          aria-label={
                            method === "url"
                              ? "Bearer token"
                              : secret.destination
                          }
                          value={secret.value}
                          onChange={(e) =>
                            method === "url"
                              ? (setToken(e.currentTarget.value), clearReview())
                              : setSecret(i, e.currentTarget.value)
                          }
                          disabled={busy || !!created}
                        />
                        <TextInput
                          label="Store with"
                          value={credentialStorage}
                          readOnly
                        />
                      </div>
                    ))}
                  {displayedAuth === "none" && (
                    <Text size="sm" c="dimmed">
                      No credential fields in this configuration.
                    </Text>
                  )}
                  {displayedAuth === "secret" && (
                    <Text
                      className="connection-credential-note"
                      size="xs"
                      c="dimmed"
                    >
                      {brain
                        ? "Stored separately from saved config in the installation-local development credential file. This is not a hardened production vault."
                        : "Used only for metadata inspection; approval saves references, never secret values."}
                    </Text>
                  )}
                </div>
                {method === "config" && (
                  <TextInput
                    label="Connection name (optional)"
                    value={draft.name}
                    onChange={(e) => {
                      const value = e.currentTarget.value;
                      setDrafts((ds) =>
                        ds.map((d, i) =>
                          i === Number(selected) ? { ...d, name: value } : d,
                        ),
                      );
                      clearReview();
                    }}
                    disabled={busy || !!created}
                  />
                )}
                {draft.transport === "stdio" && (
                  <Alert color="gray" title="Choose an approved command">
                    The import has not run or installed anything. Match this
                    command to an approved connector before saving.{" "}
                    {reuse && (
                      <Button size="xs" mt="sm" onClick={() => reuse(draft)}>
                        Choose approved connector
                      </Button>
                    )}
                  </Alert>
                )}
              </>
            )}
            {manifest && phase !== "edit" && (
              <div className="management-surface connection-review-card">
                <Group gap="sm">
                  <ShieldCheck size={20} />
                  <h3 className="management-title">Review connector</h3>
                </Group>
                <Text size="sm">
                  {manifest.name} · {manifest.transport} ·{" "}
                  {manifest.tools.length} tools
                </Text>
                <Text size="sm" c="dimmed" mt="sm">
                  {manifest.tools.map((t) => t.name).join(" · ") ||
                    "No tools declared"}
                </Text>
                <Text size="xs" c="dimmed" mt="sm">
                  {manifest.command
                    ? "Approved command: " + manifest.command
                    : "Metadata inspected. No tools have been called."}
                </Text>
              </div>
            )}
            {issue ? (
              <Alert color="red" title="Configuration could not be parsed">
                {issue.message}
              </Alert>
            ) : (
              <ErrorState error={inspection.error ?? save.error} />
            )}
            {created && save.error && (
              <Alert color="yellow">
                The connection was saved; credentials still need attention.
                Retry here to finish the same connection.
              </Alert>
            )}
            <div className="connection-setup-footer">
              <span>
                <Info size={17} />
                Imported settings are reviewed before use.
              </span>
              <Group gap="xs">
                <Button variant="default" disabled={busy} onClick={close}>
                  Cancel
                </Button>
                {phase !== "confirm" && (
                  <Button
                    variant="default"
                    loading={inspection.isPending}
                    disabled={inspectDisabled}
                    onClick={() => inspection.mutate()}
                  >
                    Inspect tools
                  </Button>
                )}
                <Button
                  loading={save.isPending}
                  disabled={!manifest || phase === "edit" || busy}
                  onClick={() =>
                    phase === "confirm" || !!created
                      ? save.mutate()
                      : setPhase("confirm")
                  }
                >
                  {phase === "confirm" || !!created
                    ? brain
                      ? "Add connection"
                      : "Approve connector"
                    : brain
                      ? "Review connection"
                      : "Review connector"}
                </Button>
              </Group>
            </div>
          </>
        )}
      </Stack>
    </Modal>
  );
}
