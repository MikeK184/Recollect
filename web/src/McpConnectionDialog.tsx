import { useEffect, useState } from "react";
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
  Text,
  TextInput,
} from "@mantine/core";
import { useMutation, useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpCatalogue } from "./McpPanel";
import { useIdempotency } from "./useIdempotency";
import { usePrivateRunners } from "./McpPrivateRunners";

type Detail = components["schemas"]["McpConnectionDetail"];
type Input = components["schemas"]["McpConnectionInput"];
type Props = {
  brain: Brain;
  id: string;
  catalogue: McpCatalogue;
  environments: { value: string; label: string }[];
  close: () => void;
  saved: () => void;
  stale: (message: string) => void;
};

export function McpConnectionDialog(props: Props) {
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
  return (
    <Modal
      opened
      onClose={props.close}
      title={props.id === "new" ? "Add MCP connection" : "MCP connection"}
      size="lg"
    >
      {query.error ? (
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
          detail={query.data}
          currentRevision={summary?.revision}
        />
      )}
    </Modal>
  );
}
function ConnectionForm({
  detail,
  currentRevision,
  ...props
}: Props & { detail?: Detail; currentRevision?: string }) {
  const [input, setInput] = useState<Input>(() => ({
    name: detail?.summary.name ?? "",
    description: detail?.summary.description ?? "",
    definition_key: detail?.summary.definition_key ?? "",
    target: detail?.target ?? "",
    placement: detail?.summary.placement ?? "central",
    runner_reference: detail?.runner_reference ?? null,
    credential_alias: detail?.credential_alias ?? null,
    environment_id: detail?.summary.environment_id ?? null,
    enabled: detail?.summary.enabled ?? true,
    configuration: detail?.configuration ?? {},
    base_revision: detail?.summary.revision ?? null,
  }));
  const [settings, setSettings] = useState(
    JSON.stringify(detail?.configuration ?? {}, null, 2),
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
      props.stale(
        "This connection changed elsewhere. Reopen it before editing.",
      );
  }, [currentRevision, detail, input.base_revision, props.stale]);
  const save = useMutation({
    mutationFn: async () => {
      let configuration: unknown;
      try {
        configuration = JSON.parse(settings);
      } catch {
        throw new Error("Enter valid JSON for the non-secret settings.");
      }
      const body = { ...input, configuration };
      if (props.id === "new")
        return result(
          await client.POST("/api/brains/{brain}/mcp/connections", {
            params: { path: { brain: props.brain.id } },
            body,
            headers: { "Idempotency-Key": command.forInput(body) },
          }),
        );
      return result(
        await client.PUT("/api/brains/{brain}/mcp/connections/{id}", {
          params: { path: { brain: props.brain.id, id: props.id } },
          body,
        }),
      );
    },
    onSuccess: props.saved,
  });
  const patch = (value: Partial<Input>) => {
    setInput((v) => ({ ...v, ...value }));
    save.reset();
  };
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        save.mutate();
      }}
    >
      <Stack>
        <Text size="sm" c="dimmed">
          Select an approved connector and its intended target. Saving
          configures it; no credentials are loaded and no backend is started.
        </Text>
        {detail && detail.profile_ids.length > 0 && (
          <Alert color="yellow">
            This connection is used by {detail.profile_ids.length} profiles.
            Changes affect their subsequent discovery and calls.
          </Alert>
        )}
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
            <Select
              label="Connection environment"
              clearable
              placeholder="Brain-wide"
              value={input.environment_id}
              data={props.environments}
              onChange={(v) => patch({ environment_id: v })}
            />
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
            <Select
              label="Execution placement"
              required
              value={input.placement}
              data={definition.data.placements.map((p) => ({
                value: p,
                label: p,
              }))}
              onChange={(v) =>
                patch({
                  placement: v ?? "central",
                  runner_reference: null,
                })
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
                {!privateRunners.error && privateRunners.data?.length === 0 && (
                  <Alert color="yellow">
                    Register a private runner for this Brain before enabling
                    this connection.
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
                            label: "Previously selected runner · unavailable",
                          },
                        ]
                      : []),
                  ]}
                  onChange={(value) => patch({ runner_reference: value })}
                />
                <Text size="sm" c="dimmed">
                  Offline runners keep calls queued until the runner connects or
                  the queue deadline expires.
                </Text>
              </>
            )}
            <Select
              label="Credential alias"
              clearable
              placeholder="No credential selected"
              value={input.credential_alias}
              data={definition.data.credential_aliases}
              onChange={(v) => patch({ credential_alias: v })}
            />
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
              <Code block>
                {JSON.stringify(definition.data.configuration_schema, null, 2)}
              </Code>
            </details>
            <Checkbox
              label="Connection enabled"
              checked={input.enabled}
              onChange={(e) => patch({ enabled: e.currentTarget.checked })}
            />
          </>
        )}
        {save.error && (
          <Alert color="red" title="Connection not saved">
            {save.error.message}
          </Alert>
        )}
        <Group justify="end">
          <Button variant="default" onClick={props.close}>
            Close
          </Button>
          <Button
            type="submit"
            loading={save.isPending}
            disabled={
              props.brain.archived || !definition.data || !!definition.error
            }
          >
            Save connection
          </Button>
        </Group>
      </Stack>
    </form>
  );
}
