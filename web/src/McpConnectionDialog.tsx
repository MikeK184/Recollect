import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import {
  Alert,
  Button,
  Checkbox,
  Code,
  Group,
  JsonInput,
  Loader,
  Modal,
  Select,
  Stack,
  PasswordInput,
  SegmentedControl,
  Text,
  TextInput,
} from "@mantine/core";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpCatalogue } from "./McpPanel";
import { useWorkspace } from "./app/context";
import type {
  ServerDraft,
  DraftSecret,
} from "./features/connections/mcpConfig";
import { useIdempotency } from "./useIdempotency";
import "./features/feature-views.css";
import { usePrivateRunners } from "./McpPrivateRunners";
import { CodeBlock } from "./components/CodeBlock";
import "./features/connections/inline-management.css";

type Detail = components["schemas"]["McpConnectionDetail"];
type Input = components["schemas"]["McpConnectionInput"];

// The catalogue stores placement as `central`, `local` or `private`
// (mcp-catalogue-and-profiles). Render the plain-language concept instead of the
// stored noun, and say what each one decides: which network can reach the target.
const placementLabel: Record<string, string> = {
  central: "Central service · the Recollect service reaches the target",
  local: "Paired device · your device's plugin runner reaches the target",
  private: "Private-network runner · a registered device on a private network",
};
export const placementText = (value: string) =>
  placementLabel[value] ?? `Runs on: ${value}`;
type Props = {
  brain: Brain;
  id: string;
  catalogue: McpCatalogue;
  environments: { value: string; label: string }[];
  close: () => void;
  saved: () => void;
  stale: (message: string) => void;
  manageRunners?: () => void;
  initialDraft?: ServerDraft;
  inline?: boolean;
  nameMount?: HTMLElement | null;
};

export function McpConnectionDialog(props: Props) {
  const [formRevision, setFormRevision] = useState(0);
  const query = useQuery({
    queryKey: ["mcp", props.brain.id, "connection", props.id],
    enabled: props.id !== "new",
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/connections/{id}", {
          params: { path: { brain: props.brain.id, id: props.id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  const summary = props.catalogue.connections.find((c) => c.id === props.id);
  const contents = query.error ? (
    <Alert color="red">
      {query.error.message}
      <Button size="xs" onClick={() => void query.refetch()}>
        Reload connection
      </Button>
    </Alert>
  ) : props.id !== "new" && !query.data ? (
    <Loader />
  ) : (
    <ConnectionForm
      {...props}
      key={formRevision}
      detail={query.data}
      currentRevision={summary?.revision}
      reload={() =>
        void query.refetch().then((response) => {
          if (!response.error) setFormRevision((v) => v + 1);
        })
      }
    />
  );
  return props.inline ? (
    contents
  ) : (
    <Modal
      opened
      onClose={props.close}
      title={props.id === "new" ? "Add connection" : "Edit connection"}
      size={800}
      className="connection-setup-modal"
    >
      {contents}
    </Modal>
  );
}
function ConnectionForm({
  detail,
  currentRevision,
  reload,
  ...props
}: Props & { detail?: Detail; currentRevision?: string; reload: () => void }) {
  const owner = useWorkspace().user.installation_owner;
  const [secrets, setSecrets] = useState<DraftSecret[]>(
    props.initialDraft?.secrets ?? [],
  );
  const [authMode, setAuthMode] = useState(
    props.initialDraft?.secrets.length ? "secret" : "reference",
  );
  const [created, setCreated] = useState<
    components["schemas"]["McpConnectionSummary"] | null
  >(null);
  const [createdInput, setCreatedInput] = useState<string | null>(null);
  const [stepError, setStepError] = useState<string | null>(null);
  const [staleDraft, setStaleDraft] = useState(false);
  const isNew = props.id === "new";
  const show = (_index: number) => true;
  const [input, setInput] = useState<Input>(() => ({
    name: detail?.summary.name ?? props.initialDraft?.name ?? "",
    description: detail?.summary.description ?? "",
    definition_key: detail?.summary.definition_key ?? "",
    target:
      detail?.target ??
      (props.initialDraft?.target || props.initialDraft?.name || ""),
    placement: detail?.summary.placement ?? "central",
    runner_reference: detail?.runner_reference ?? null,
    credential_alias: detail?.credential_alias ?? null,
    environment_id: detail?.summary.environment_id ?? null,
    enabled: detail?.summary.enabled ?? true,
    configuration:
      detail?.configuration ?? props.initialDraft?.configuration ?? {},
    base_revision: detail?.summary.revision ?? null,
  }));
  const [settings, setSettings] = useState(
    JSON.stringify(
      detail?.configuration ?? props.initialDraft?.configuration ?? {},
      null,
      2,
    ),
  );
  const command = useIdempotency();
  const privateRunners = usePrivateRunners(props.brain.id);
  const definition = useQuery({
    queryKey: ["mcp", props.brain.id, "definition", input.definition_key],
    enabled: !!input.definition_key,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/definitions/{key}", {
          params: {
            path: { brain: props.brain.id, key: input.definition_key },
          },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  useEffect(() => {
    if (
      input.base_revision &&
      ((currentRevision && input.base_revision !== currentRevision) ||
        (detail && input.base_revision !== detail.summary.revision))
    )
      if (!created) {
        if (props.inline) setStaleDraft(true);
        else
          props.stale(
            "This connection changed elsewhere. Reopen it before editing.",
          );
      }
  }, [
    currentRevision,
    detail,
    input.base_revision,
    props.stale,
    props.inline,
    created,
  ]);
  const save = useMutation({
    mutationFn: async () => {
      if (staleDraft && !created)
        throw new Error(
          "Reload this connection before saving the changed revision.",
        );
      let configuration: unknown;
      try {
        configuration = JSON.parse(settings);
      } catch {
        throw new Error("Enter valid JSON for the non-secret settings.");
      }
      if (props.initialDraft?.command) {
        const global = result(
          await client.GET("/api/mcp/definitions/{key}", {
            params: { path: { key: input.definition_key } },
          }),
        );
        if (
          global.manifest.command !== props.initialDraft.command ||
          JSON.stringify(global.manifest.arguments ?? []) !==
            JSON.stringify(props.initialDraft.arguments)
        )
          throw new Error(
            "Choose the approved connector with this exact command and arguments. Imports never execute an unapproved command.",
          );
      }
      if (
        authMode === "secret" &&
        (!input.credential_alias ||
          input.placement !== "central" ||
          secrets.length === 0 ||
          secrets.some((s) => s.value.length < 4))
      )
        throw new Error(
          "Choose an approved credential alias and fill in each secret for central execution.",
        );
      const body = { ...input, configuration };
      if (
        created &&
        (JSON.stringify(body) !== createdInput || authMode !== "secret")
      )
        throw new Error(
          "This connection is already saved. Restore its original details to retry credentials, or close and edit the saved connection.",
        );
      const connection =
        created ??
        (props.id === "new"
          ? result(
              await client.POST("/api/brains/{brain}/mcp/connections", {
                params: { path: { brain: props.brain.id } },
                body,
                headers: { "Idempotency-Key": command.forInput(body) },
              }),
            ).summary
          : result(
              await client.PUT("/api/brains/{brain}/mcp/connections/{id}", {
                params: { path: { brain: props.brain.id, id: props.id } },
                body,
              }),
            ).summary);
      setCreated(connection);
      setCreatedInput(JSON.stringify(body));
      if (authMode === "secret")
        result(
          await client.POST(
            "/api/brains/{brain}/mcp/connections/{id}/credentials",
            {
              params: { path: { brain: props.brain.id, id: connection.id } },
              body: {
                base_revision: connection.revision,
                headers: secrets
                  .filter((s) => s.kind === "header")
                  .map((s) => ({
                    name: s.destination,
                    value: s.value,
                    prefix: s.prefix,
                  })),
                environment: Object.fromEntries(
                  secrets
                    .filter((s) => s.kind === "environment")
                    .map((s) => [s.destination, s.value]),
                ),
              },
            },
          ),
        );
      setSecrets([]);
      return connection;
    },
    onSuccess: props.saved,
  });
  const patch = (value: Partial<Input>) => {
    setInput((v) => ({ ...v, ...value }));
    save.reset();
  };
  const validateStep = (index: number): string | null => {
    if (
      index === 0 &&
      (!input.name.trim() ||
        !input.definition_key ||
        !definition.data ||
        definition.error)
    )
      return "Choose an approved connector and give this connection a name.";
    if (index === 1 && !input.target.trim())
      return "Enter the exact target for this connection.";
    if (index === 2) {
      try {
        const value = JSON.parse(settings);
        if (!value || Array.isArray(value) || typeof value !== "object")
          return "Non-secret settings must be a JSON object.";
      } catch {
        return "Enter valid JSON for the non-secret settings.";
      }
    }
    if (
      index === 3 &&
      (!definition.data?.placements.includes(input.placement) ||
        (input.placement !== "central" && !input.runner_reference))
    )
      return "Choose where this connection runs and the runner that serves it.";
    return null;
  };
  if (props.inline)
    return (
      <form
        className="connection-inline-editor"
        onSubmit={(e) => {
          e.preventDefault();
          const invalid = [0, 1, 2, 3].map(validateStep).find(Boolean);
          if (invalid) {
            setStepError(invalid);
            return;
          }
          save.mutate();
        }}
      >
        <fieldset disabled={save.isPending} className="mcp-inline-fields">
          {props.nameMount &&
            createPortal(
              <TextInput
                aria-label="Connection name"
                required
                className="mcp-inline-title"
                maxLength={120}
                disabled={save.isPending}
                value={input.name}
                onChange={(e) => patch({ name: e.currentTarget.value })}
              />,
              props.nameMount,
            )}
          <Stack gap="sm">
            <h3 className="management-title inspector-section-title">
              Configuration
            </h3>
            {!props.nameMount && (
              <TextInput
                label="Connection name"
                required
                maxLength={120}
                value={input.name}
                onChange={(e) => patch({ name: e.currentTarget.value })}
              />
            )}
            <TextInput
              label={
                definition.data?.summary.transport === "stdio"
                  ? "Target label"
                  : "Target"
              }
              required
              maxLength={2048}
              value={input.target}
              onChange={(e) => patch({ target: e.currentTarget.value })}
            />
            <Select
              label="Scope"
              clearable
              placeholder="Brain-wide"
              data={props.environments}
              value={input.environment_id}
              onChange={(value) => patch({ environment_id: value })}
            />
            <Select
              label="Runs on"
              required
              value={input.placement}
              data={(definition.data?.placements ?? [input.placement]).map(
                (value) => ({ value, label: placementText(value) }),
              )}
              onChange={(value) =>
                patch({ placement: value ?? "central", runner_reference: null })
              }
            />
            {input.placement === "local" && (
              <TextInput
                label="Runner reference"
                required
                maxLength={200}
                value={input.runner_reference ?? ""}
                onChange={(e) =>
                  patch({ runner_reference: e.currentTarget.value })
                }
              />
            )}
            {input.placement === "private" && (
              <Select
                label="Private runner"
                required
                value={input.runner_reference}
                data={(privateRunners.data ?? []).map((runner) => ({
                  value: `private:${runner.id}`,
                  label: `${runner.name} · ${runner.available ? "Connected" : "Offline"}`,
                  disabled: !runner.eligible,
                }))}
                onChange={(value) => patch({ runner_reference: value })}
              />
            )}
            {privateRunners.error && input.placement === "private" && (
              <Alert color="red">{privateRunners.error.message}</Alert>
            )}
            <details className="inspector-more-configuration">
              <summary>More configuration</summary>
              <Stack mt="sm" gap="sm">
                <TextInput
                  label="Description"
                  maxLength={2000}
                  value={input.description}
                  onChange={(e) =>
                    patch({ description: e.currentTarget.value })
                  }
                />
                <Select
                  label="Approved connector"
                  required
                  value={input.definition_key || null}
                  data={props.catalogue.definitions.map((d) => ({
                    value: d.key,
                    label: d.name,
                    disabled: !d.enabled,
                  }))}
                  onChange={(value) =>
                    patch({
                      definition_key: value ?? "",
                      credential_alias: null,
                    })
                  }
                />
                <Select
                  label="Credential alias"
                  clearable
                  placeholder="None selected"
                  value={input.credential_alias}
                  data={definition.data?.credential_aliases ?? []}
                  onChange={(value) => patch({ credential_alias: value })}
                />
                {owner &&
                  input.placement === "central" &&
                  !!definition.data?.credential_aliases.length && (
                    <SegmentedControl
                      value={authMode}
                      onChange={(value) => {
                        setAuthMode(value);
                        setSecrets([]);
                      }}
                      data={[
                        { value: "reference", label: "Credential reference" },
                        { value: "secret", label: "Secret" },
                      ]}
                    />
                  )}
                {authMode === "secret" && (
                  <>
                    {!secrets.length && (
                      <TextInput
                        label={
                          definition.data?.summary.transport === "stdio"
                            ? "Environment variable"
                            : "Header name"
                        }
                        placeholder={
                          definition.data?.summary.transport === "stdio"
                            ? "MCP_TOKEN"
                            : "Authorization"
                        }
                        onChange={(e) =>
                          setSecrets([
                            {
                              kind:
                                definition.data?.summary.transport === "stdio"
                                  ? "environment"
                                  : "header",
                              destination: e.currentTarget.value,
                              variable: "MCP_TOKEN",
                              prefix:
                                definition.data?.summary.transport === "stdio"
                                  ? ""
                                  : "Bearer ",
                              value: "",
                            },
                          ])
                        }
                      />
                    )}
                    {secrets.map((secret, index) => (
                      <Group key={index} grow align="start">
                        <TextInput
                          label={
                            secret.kind === "header"
                              ? "Header name"
                              : "Environment variable"
                          }
                          value={secret.destination}
                          onChange={(e) => {
                            const destination = e.currentTarget.value;
                            setSecrets((values) =>
                              values.map((v, i) =>
                                i === index ? { ...v, destination } : v,
                              ),
                            );
                          }}
                        />
                        <PasswordInput
                          label="Secret value"
                          value={secret.value}
                          onChange={(e) => {
                            const value = e.currentTarget.value;
                            setSecrets((values) =>
                              values.map((v, i) =>
                                i === index ? { ...v, value } : v,
                              ),
                            );
                          }}
                        />
                      </Group>
                    ))}
                    <Text size="xs" c="dimmed">
                      Saved separately in the installation-local credential
                      file.
                    </Text>
                  </>
                )}
                <JsonInput
                  label="Non-secret settings"
                  value={settings}
                  onChange={(value) => {
                    setSettings(value);
                    save.reset();
                  }}
                  validationError="Enter valid JSON"
                  formatOnBlur
                  autosize
                  minRows={3}
                />
                {definition.data && (
                  <details>
                    <summary>Approved settings schema</summary>
                    <CodeBlock
                      language="json"
                      code={JSON.stringify(
                        definition.data.configuration_schema,
                        null,
                        2,
                      )}
                    />
                  </details>
                )}
                <Checkbox
                  label="Connection enabled"
                  checked={input.enabled}
                  onChange={(e) => patch({ enabled: e.currentTarget.checked })}
                />
              </Stack>
            </details>
            {definition.error && (
              <Alert color="red">{definition.error.message}</Alert>
            )}
            {stepError && <Alert color="red">{stepError}</Alert>}
            {staleDraft && !created && (
              <Alert color="yellow" title="Connection changed">
                Your draft is retained. Reload replaces it with current
                settings.
                <Button variant="subtle" size="xs" onClick={reload}>
                  Reload settings
                </Button>
              </Alert>
            )}
            {save.error && (
              <Alert
                color="red"
                title={
                  created
                    ? "Connection saved · credentials not saved"
                    : "Connection not saved"
                }
              >
                {save.error.message}
              </Alert>
            )}
            <Group justify="end" className="connection-setup-footer">
              <Button
                variant="default"
                disabled={save.isPending}
                onClick={props.close}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                loading={save.isPending}
                disabled={
                  props.brain.archived ||
                  (staleDraft && !created) ||
                  [0, 1, 2, 3].some((index) => !!validateStep(index))
                }
              >
                {created ? "Retry credentials" : "Save connection"}
              </Button>
            </Group>
          </Stack>
        </fieldset>
      </form>
    );
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        const invalid = [0, 1, 2, 3]
          .map((index) => ({ index, error: validateStep(index) }))
          .find((item) => item.error);
        if (invalid) {
          setStepError(invalid.error);
          return;
        }
        save.mutate();
      }}
    >
      <Stack className="connection-edit-form">
        {stepError && (
          <Alert color="red" title="Complete this connection">
            {stepError}
          </Alert>
        )}
        <Text size="sm" c="dimmed">
          Choose an approved connector, its target and execution location.
          Saving does not call a tool.
        </Text>
        {detail && detail.profile_ids.length > 0 && (
          <Alert color="yellow">
            This connection is used by {detail.profile_ids.length}{" "}
            {detail.profile_ids.length === 1 ? "profile" : "profiles"}. Changes
            affect their subsequent discovery and calls.
          </Alert>
        )}
        {show(0) && (
          <>
            <TextInput
              label="Connection name"
              required
              maxLength={120}
              value={input.name}
              onChange={(e) => patch({ name: e.currentTarget.value })}
            />
            <TextInput
              label="Connection description"
              maxLength={2000}
              value={input.description}
              onChange={(e) => patch({ description: e.currentTarget.value })}
            />
            <Select
              label="Approved connector"
              required
              value={input.definition_key || null}
              data={props.catalogue.definitions.map((d) => ({
                value: d.key,
                label: `${d.name}${d.enabled ? "" : " · disabled"}`,
              }))}
              onChange={(v) =>
                patch({ definition_key: v ?? "", credential_alias: null })
              }
            />
          </>
        )}
        {definition.isFetching && !definition.data && <Loader size="xs" />}
        {definition.error && (
          <Alert color="red">{definition.error.message}</Alert>
        )}
        {definition.data && !definition.error && (
          <>
            <Text size="sm">
              {definition.data.summary.description} ·{" "}
              {definition.data.summary.transport}
            </Text>
            {!definition.data.summary.enabled && (
              <Alert color="yellow">
                This definition is disabled. Enablement cannot be saved until an
                approved definition is selected.
              </Alert>
            )}
            {show(1) && (
              <>
                <Select
                  label="Connection environment"
                  clearable
                  placeholder="Brain-wide"
                  value={input.environment_id}
                  data={props.environments}
                  onChange={(v) => patch({ environment_id: v })}
                />
                <Button
                  variant="subtle"
                  size="xs"
                  component="a"
                  href={`/brains/${props.brain.id}/settings?tab=general#environments`}
                >
                  Manage optional environments
                </Button>
                <TextInput
                  label={
                    definition.data.summary.transport === "streamable_http"
                      ? "MCP target URL"
                      : "Target label"
                  }
                  required
                  maxLength={2048}
                  value={input.target}
                  onChange={(e) => patch({ target: e.currentTarget.value })}
                />
              </>
            )}
            {show(3) && (
              <>
                <Select
                  label="Where this connection runs"
                  description="This decides which network can reach the target. The chosen place never falls back to another."
                  required
                  value={input.placement}
                  data={definition.data.placements.map((p) => ({
                    value: p,
                    label: placementText(p),
                  }))}
                  onChange={(v) =>
                    patch({
                      placement: v ?? "central",
                      runner_reference: null,
                    })
                  }
                />
                {props.manageRunners && input.placement === "private" && (
                  <Button variant="subtle" onClick={props.manageRunners}>
                    Manage private-network execution
                  </Button>
                )}
                {input.placement === "local" && (
                  <TextInput
                    label="Runner reference"
                    description="Name the runner on your paired device. Offline devices apply nothing."
                    required
                    maxLength={200}
                    value={input.runner_reference ?? ""}
                    onChange={(e) =>
                      patch({ runner_reference: e.currentTarget.value })
                    }
                  />
                )}
                {input.placement === "private" && (
                  <>
                    {privateRunners.error && (
                      <Alert color="red">
                        {privateRunners.error.message}
                        <Button
                          size="xs"
                          onClick={() => void privateRunners.refetch()}
                        >
                          Refresh private runners
                        </Button>
                      </Alert>
                    )}
                    {!privateRunners.error &&
                      privateRunners.data?.length === 0 && (
                        <Alert color="yellow">
                          This Brain has no private runner. Register one for a
                          paired device on the Runners tab first; until then
                          this target has nothing that can reach it.
                        </Alert>
                      )}
                    <Select
                      label="Private runner"
                      required
                      placeholder="Select this Brain's runner"
                      value={input.runner_reference}
                      data={[
                        ...(privateRunners.error
                          ? []
                          : (privateRunners.data ?? [])
                        ).map((r) => ({
                          value: `private:${r.id}`,
                          label: `${r.name} · ${!r.eligible ? "Unavailable" : r.available ? "Connected" : "Offline"}`,
                          disabled:
                            !r.eligible &&
                            input.runner_reference !== `private:${r.id}`,
                        })),
                        ...(input.runner_reference &&
                        !(privateRunners.data ?? []).some(
                          (r) => input.runner_reference === `private:${r.id}`,
                        )
                          ? [
                              {
                                value: input.runner_reference,
                                label:
                                  "Previously selected runner · unavailable",
                              },
                            ]
                          : []),
                      ]}
                      onChange={(value) => patch({ runner_reference: value })}
                    />
                    <Text size="sm" c="dimmed">
                      Offline runners keep calls queued until the runner
                      connects or the queue deadline expires. A registration is
                      metadata: it proves nothing until a call succeeds there.
                    </Text>
                    <Text size="sm" c="dimmed">
                      Each runner is bound to one paired device. Register or
                      rename it in execution administration, and manage the
                      device itself in{" "}
                      <Link
                        to="/brains/$brainId/agents"
                        params={{ brainId: props.brain.id }}
                      >
                        this Brain&apos;s Agents list
                      </Link>
                      .
                    </Text>
                  </>
                )}
              </>
            )}
            {show(2) && (
              <>
                <Text fw={600} size="sm">
                  Authentication
                </Text>
                {owner &&
                  input.placement === "central" &&
                  definition.data.credential_aliases.length > 0 && (
                    <SegmentedControl
                      value={authMode}
                      onChange={(v) => {
                        setAuthMode(v);
                        setSecrets([]);
                      }}
                      data={[
                        { value: "reference", label: "Credential reference" },
                        { value: "secret", label: "Secret" },
                      ]}
                    />
                  )}

                <Select
                  label="Credential alias"
                  clearable
                  placeholder="No credential selected"
                  value={input.credential_alias}
                  data={definition.data.credential_aliases}
                  onChange={(v) => patch({ credential_alias: v })}
                />
                {authMode === "secret" && (
                  <>
                    {!secrets.length && (
                      <Group grow>
                        <TextInput
                          label={
                            definition.data.summary.transport === "stdio"
                              ? "Environment variable"
                              : "Header name"
                          }
                          placeholder={
                            definition.data.summary.transport === "stdio"
                              ? "MCP_TOKEN"
                              : "Authorization"
                          }
                          onChange={(e) =>
                            setSecrets([
                              {
                                kind:
                                  definition.data!.summary.transport === "stdio"
                                    ? "environment"
                                    : "header",
                                destination: e.currentTarget.value,
                                variable: "MCP_TOKEN",
                                prefix:
                                  definition.data!.summary.transport === "stdio"
                                    ? ""
                                    : "Bearer ",
                                value: "",
                              },
                            ])
                          }
                        />
                      </Group>
                    )}
                    {secrets.map((secret, i) => (
                      <Group key={i} grow align="start">
                        <TextInput
                          label={
                            secret.kind === "header"
                              ? "Header name"
                              : "Environment variable"
                          }
                          value={secret.destination}
                          onChange={(e) => {
                            const value = e.currentTarget.value;
                            setSecrets((values) =>
                              values.map((v, j) =>
                                j === i ? { ...v, destination: value } : v,
                              ),
                            );
                          }}
                        />
                        <PasswordInput
                          label={
                            secret.prefix ? "Bearer token" : "Secret value"
                          }
                          value={secret.value}
                          onChange={(e) => {
                            const value = e.currentTarget.value;
                            setSecrets((values) =>
                              values.map((v, j) =>
                                j === i ? { ...v, value } : v,
                              ),
                            );
                          }}
                        />
                      </Group>
                    ))}
                    <Text size="xs" c="dimmed">
                      Values are saved separately in the installation-local
                      development credential file. They are never stored in this
                      connection’s settings.
                    </Text>
                  </>
                )}
                <JsonInput
                  label="Non-secret settings"
                  description="Use only settings permitted by the approved schema. Credential values belong in the credential provider."
                  value={settings}
                  onChange={(v) => {
                    setSettings(v);
                    save.reset();
                  }}
                  validationError="Enter valid JSON"
                  formatOnBlur
                  autosize
                  minRows={4}
                />
                <details>
                  <summary>Approved settings schema</summary>
                  <CodeBlock
                    language="json"
                    code={JSON.stringify(
                      definition.data.configuration_schema,
                      null,
                      2,
                    )}
                  />
                </details>
              </>
            )}
            {show(4) && (
              <>
                <Checkbox
                  label="Connection enabled"
                  description="Allow future tool calls. Disable to pause use; dispatched calls may finish."
                  checked={input.enabled}
                  onChange={(e) => patch({ enabled: e.currentTarget.checked })}
                />
              </>
            )}
          </>
        )}
        {save.error && (
          <Alert color="red" title="Connection not saved">
            {save.error.message}
          </Alert>
        )}
        <Group justify="end" className="connection-setup-footer">
          <Button variant="default" onClick={props.close}>
            Close
          </Button>
          <Button
            type="submit"
            loading={save.isPending}
            disabled={
              props.brain.archived ||
              [0, 1, 2, 3].some((index) => !!validateStep(index)) ||
              (authMode === "secret" &&
                (!input.credential_alias ||
                  input.placement !== "central" ||
                  !secrets.length ||
                  secrets.some((s) => s.value.length < 4)))
            }
          >
            {"Save connection"}
          </Button>
        </Group>
      </Stack>
    </form>
  );
}
