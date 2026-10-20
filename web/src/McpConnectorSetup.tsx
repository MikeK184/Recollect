import { useState } from "react";
import { CodeSyntax } from "./components/CodeBlock";
import {
  Alert,
  Button,
  FileButton,
  Modal,
  PasswordField,
  SegmentedControl,
  Select,
  TextField,
  Tile,
} from "./components/ui";
import { cn } from "./lib/utils";
import { useMutation } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
  ArrowRight,
  KeyRound,
  Link2,
  Grid2X2,
  Ban,
  CheckCircle2,
  Info,
  Server,
  FileCode,
  ShieldCheck,
  Upload,
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

  const configLanguage =
    format === "auto"
      ? /^\s*(?:\{|\[\s*[\{"\d-])/.test(source)
        ? "json"
        : /^\s*\[/.test(source) || /^\s*[\w.]+\s*=/.test(source)
          ? "toml"
          : "yaml"
      : format;
  const footer =
    method === "approved" ? undefined : (
      <>
        <span className="mr-auto inline-flex items-center gap-1.5 text-caption text-muted-foreground">
          <Info aria-hidden="true" className="size-3.5" />
          Imported settings are reviewed before use.
        </span>
        <Button variant="secondary" disabled={busy} onClick={close}>
          Cancel
        </Button>
        {phase !== "confirm" && (
          <Button
            variant="secondary"
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
      </>
    );

  return (
    <Modal
      opened
      onClose={() => {
        if (!busy) close();
      }}
      title={brain ? "Add connection" : "Add a connector"}
      description={
        brain
          ? "Connect an MCP server to this Brain."
          : "Approve a reusable MCP definition for your installation."
      }
      size="xl"
      className="w-[912px]"
      bodyClassName="grid gap-4"
      closeOnClickOutside={!busy}
      closeOnEscape={!busy}
      footer={footer}
    >
      <SegmentedControl
        label="Connection method"
        fullWidth
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
              <>
                <Link2 aria-hidden="true" />
                Server URL
              </>
            ),
            disabled: !owner,
          },
          {
            value: "config",
            label: (
              <>
                <FileCode aria-hidden="true" />
                Paste config
              </>
            ),
            disabled: !owner,
          },
          ...(brain
            ? [
                {
                  value: "approved",
                  label: (
                    <>
                      <Grid2X2 aria-hidden="true" />
                      Approved connector
                    </>
                  ),
                },
              ]
            : []),
        ]}
      />
      {method === "approved" ? (
        <section className="grid animate-enter justify-items-start gap-3 rounded-lg border border-border bg-card p-4">
          <div className="flex items-start gap-3">
            <Tile>
              <ShieldCheck aria-hidden="true" />
            </Tile>
            <div className="min-w-0">
              <h3 className="m-0 text-title font-semibold text-foreground">
                Use an approved connector
              </h3>
              <p className="m-0 mt-0.5 text-body text-muted-foreground">
                <span className="mono">{approvedConnectors.length}</span>{" "}
                approved{" "}
                {approvedConnectors.length === 1 ? "connector" : "connectors"}{" "}
                available. Choose its target, credentials and execution
                location next.
              </p>
            </div>
          </div>
          <Button
            disabled={!approvedConnectors.length}
            onClick={() => reuse?.()}
          >
            Choose connector
          </Button>
          {owner && (
            <Link
              to="/connectors"
              className="inline-flex items-center gap-1 text-label font-medium text-brand-text underline-offset-[3px] hover:underline"
            >
              Manage the connector library
              <ArrowRight aria-hidden="true" className="size-3.5" />
            </Link>
          )}
        </section>
      ) : (
        <>
          {method === "url" ? (
            <div className="grid animate-enter grid-cols-2 items-start gap-3">
              <TextField
                label="Server name"
                value={name}
                onChange={(e) => {
                  setName(e.currentTarget.value);
                  clearReview();
                }}
                required
                disabled={busy || !!created}
              />
              <TextField
                label="MCP server URL"
                placeholder="https://example.com/mcp"
                inputClassName="mono text-label"
                value={url}
                onChange={(e) => {
                  setUrl(e.currentTarget.value);
                  clearReview();
                }}
                required
                disabled={busy || !!created}
              />
            </div>
          ) : (
            <div className="grid animate-enter gap-2">
              <div className="flex items-center justify-between gap-2">
                <h3 className="m-0 text-label font-medium text-foreground">
                  MCP configuration
                </h3>
                <div className="flex items-center gap-2">
                  <FileButton
                    accept=".json,.toml,.yaml,.yml,text/plain"
                    disabled={busy || !!created}
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
                        variant="ghost"
                        size="sm"
                        aria-label="Choose config file"
                      >
                        <Upload aria-hidden="true" />
                        Import file
                      </Button>
                    )}
                  </FileButton>
                  <SegmentedControl
                    label="Configuration format"
                    size="sm"
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
                </div>
              </div>
              <div className="relative h-[188px] overflow-hidden rounded-md border border-input bg-card shadow-xs transition-[border-color,box-shadow] duration-fast focus-within:border-ring focus-within:shadow-focus">
                <div
                  className="mono pointer-events-none absolute left-0 top-0 min-h-full w-12 border-r border-border bg-muted py-2 text-center text-caption leading-[22px] text-muted-foreground"
                  aria-hidden="true"
                  style={{ transform: `translateY(-${editorScroll}px)` }}
                >
                  {source.split("\n").map((_, i) => (
                    <div key={i}>{i + 1}</div>
                  ))}
                </div>
                <pre
                  aria-hidden="true"
                  className="markdown-code-tokens mono pointer-events-none absolute left-12 right-0 top-0 m-0 whitespace-pre px-3 py-2 text-label leading-[22px] text-foreground"
                  style={{ transform: `translateY(-${editorScroll}px)` }}
                >
                  <CodeSyntax code={source} language={configLanguage} />
                </pre>
                <textarea
                  aria-label="MCP configuration"
                  spellCheck={false}
                  wrap="off"
                  className="mono absolute inset-0 m-0 h-full w-full resize-none overflow-auto whitespace-pre border-0 bg-transparent py-2 pl-[60px] pr-3 text-label leading-[22px] text-transparent caret-foreground outline-none scrollbar-thin placeholder:text-muted-foreground disabled:cursor-not-allowed"
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
                  variant="secondary"
                  size="sm"
                  className="justify-self-start"
                  onClick={parse}
                  disabled={!source.trim()}
                >
                  <FileCode aria-hidden="true" />
                  Parse configuration
                </Button>
              )}
              {drafts.length > 1 && (
                <Select
                  label="Server to add"
                  className="max-w-[360px]"
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
            </div>
          )}
          {drafts.length > 0 && (
            <div
              className="flex animate-enter items-center gap-2.5 rounded-md bg-brand-soft px-3 py-2 text-label text-brand-text"
              role="status"
            >
              <CheckCircle2 aria-hidden="true" className="size-4 shrink-0" />
              <span>
                Parsed <span className="mono">{drafts.length}</span>{" "}
                {drafts.length === 1 ? "server" : "servers"} ·{" "}
                {draft.transport === "stdio" ? "Local command" : "HTTP"}.{" "}
                Secret values are masked.
              </span>
            </div>
          )}
          {(method === "url" || drafts.length > 0) && (
            <>
              <div className="grid grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)_minmax(0,0.7fr)] divide-x divide-border rounded-lg border border-border bg-muted/60">
                <div className="flex min-w-0 items-center gap-3 p-3">
                  <Tile>
                    <Server aria-hidden="true" />
                  </Tile>
                  <div className="min-w-0">
                    <strong className="block truncate text-label font-semibold text-foreground">
                      {draft.name || "New server"}
                    </strong>
                    <small className="mono block truncate text-caption text-muted-foreground">
                      {draft.command || draft.target || "Enter a server URL"}
                    </small>
                  </div>
                </div>
                <div className="grid min-w-0 content-center gap-0.5 p-3">
                  <small className="text-caption text-muted-foreground">
                    Runs on
                  </small>
                  <strong className="flex flex-wrap items-center gap-1 text-label font-medium text-foreground">
                    {draft.transport === "stdio"
                      ? "Approved runner required"
                      : "Recollect service"}{" "}
                    {reuse && (
                      <Button
                        variant="link"
                        size="sm"
                        onClick={() => reuse(draft)}
                      >
                        Change
                      </Button>
                    )}
                  </strong>
                </div>
                <div className="grid min-w-0 content-center gap-0.5 p-3">
                  <small className="text-caption text-muted-foreground">
                    Environment
                  </small>
                  <strong className="text-label font-medium text-foreground">
                    Brain-wide
                  </strong>
                </div>
              </div>
              <section className="grid gap-3 rounded-lg border border-border p-4">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <h3 className="m-0 text-title font-semibold text-foreground">
                    Authentication
                  </h3>
                  <SegmentedControl
                    label="Authentication"
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
                          <>
                            <KeyRound aria-hidden="true" />
                            Secret
                          </>
                        ),
                        disabled: method === "config" && !draft.secrets.length,
                      },
                      {
                        value: "reference",
                        label: (
                          <>
                            <Link2 aria-hidden="true" />
                            Credential reference
                          </>
                        ),
                        disabled: !reuse,
                      },
                      {
                        value: "none",
                        label: (
                          <>
                            <Ban aria-hidden="true" />
                            None
                          </>
                        ),
                        disabled: method === "config" && !!draft.secrets.length,
                      },
                    ]}
                  />
                </div>
                {method === "url" && auth === "reference" && (
                  <p className="m-0 text-label text-muted-foreground">
                    Choose an approved connector to select an existing
                    credential alias.{" "}
                    <Button
                      variant="link"
                      size="sm"
                      disabled={!reuse}
                      onClick={() => reuse?.()}
                    >
                      Use an approved connector
                    </Button>
                  </p>
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
                      className="grid animate-enter grid-cols-3 items-start gap-3"
                      key={secret.destination}
                    >
                      <TextField
                        label="Secret variable name"
                        inputClassName="mono text-label"
                        value={secret.variable}
                        readOnly
                        title={secret.destination}
                      />
                      <PasswordField
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
                      <TextField
                        label="Store with"
                        value={credentialStorage}
                        readOnly
                      />
                    </div>
                  ))}
                {displayedAuth === "none" && (
                  <p className="m-0 text-label text-muted-foreground">
                    No credential fields in this configuration.
                  </p>
                )}
                {displayedAuth === "secret" && (
                  <p className="m-0 text-caption text-muted-foreground">
                    {brain
                      ? "Stored separately from saved config in the installation-local development credential file. This is not a hardened production vault."
                      : "Used only for metadata inspection; approval saves references, never secret values."}
                  </p>
                )}
              </section>
              {method === "config" && (
                <TextField
                  label="Connection name (optional)"
                  className="max-w-[420px]"
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
                <Alert
                  role="status"
                  title="Choose an approved command"
                  actions={
                    reuse && (
                      <Button size="sm" onClick={() => reuse(draft)}>
                        Choose approved connector
                      </Button>
                    )
                  }
                >
                  The import has not run or installed anything. Match this
                  command to an approved connector before saving.
                </Alert>
              )}
            </>
          )}
          {manifest && phase !== "edit" && (
            <section
              className={cn(
                "grid animate-enter gap-2 rounded-lg border bg-card p-4",
                phase === "confirm" ? "border-brand/50" : "border-border",
              )}
            >
              <div className="flex items-center gap-2">
                <ShieldCheck
                  aria-hidden="true"
                  className="size-4 text-brand-text"
                />
                <h3 className="m-0 text-title font-semibold text-foreground">
                  Review connector
                </h3>
              </div>
              <p className="m-0 text-body text-foreground">
                {manifest.name} ·{" "}
                <span className="mono text-label">{manifest.transport}</span> ·{" "}
                <span className="mono text-label">{manifest.tools.length}</span>{" "}
                tools
              </p>
              <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
                {manifest.tools.map((t) => t.name).join(" · ") ||
                  "No tools declared"}
              </p>
              <p className="m-0 text-caption text-muted-foreground">
                {manifest.command
                  ? "Approved command: " + manifest.command
                  : "Metadata inspected. No tools have been called."}
              </p>
            </section>
          )}
          {issue ? (
            <Alert variant="destructive" title="Configuration could not be parsed">
              {issue.message}
            </Alert>
          ) : (
            <ErrorState error={inspection.error ?? save.error} />
          )}
          {created && save.error && (
            <Alert variant="warning">
              The connection was saved; credentials still need attention.
              Retry here to finish the same connection.
            </Alert>
          )}
        </>
      )}
    </Modal>
  );
}
