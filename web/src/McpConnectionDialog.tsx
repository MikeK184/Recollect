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
  Stepper,
  Title,
  Text,
  TextInput,
} from "@mantine/core";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpCatalogue } from "./McpPanel";
import { useIdempotency } from "./useIdempotency";
import "./features/feature-views.css";
import { usePrivateRunners } from "./McpPrivateRunners";

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
      size={props.id === "new" ? "xl" : "lg"}
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
  const [step, setStep] = useState(0);
  const [stepError, setStepError] = useState<string | null>(null);
  const isNew = props.id === "new";
  const show = (index: number) => !isNew || step === index;
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
  const advance = () => {
    const error = validateStep(step);
    setStepError(error);
    if (!error) setStep((value) => Math.min(4, value + 1));
  };
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        if (isNew && step < 4) {
          advance();
          return;
        }
        const invalid = [0, 1, 2, 3]
          .map((index) => ({ index, error: validateStep(index) }))
          .find((item) => item.error);
        if (isNew && invalid) {
          setStep(invalid.index);
          setStepError(invalid.error);
          return;
        }
        save.mutate();
      }}
    >
      <Stack>
        {isNew && (
          <Stepper
            active={step}
            size="sm"
            allowNextStepsSelect={false}
            onStepClick={(index) => {
              if (index < step) {
                setStep(index);
                setStepError(null);
              }
            }}
          >
            {["Connector", "Target", "Credentials", "Runner", "Review"].map(
              (title) => (
                <Stepper.Step key={title} label={title} />
              ),
            )}
          </Stepper>
        )}
        {stepError && (
          <Alert color="red" title="Complete this step">
            {stepError}
          </Alert>
        )}
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
                      rename it on the Runners tab, and manage the device itself
                      in{" "}
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
                    {JSON.stringify(
                      definition.data.configuration_schema,
                      null,
                      2,
                    )}
                  </Code>
                </details>
              </>
            )}
            {show(4) && (
              <>
                {isNew && (
                  <>
                    <Title order={3}>Review this connection</Title>
                    <dl className="connection-wizard-summary">
                      <dt>Connector</dt>
                      <dd>{definition.data.summary.name}</dd>
                      <dt>Connection</dt>
                      <dd>{input.name}</dd>
                      <dt>Target</dt>
                      <dd>{input.target}</dd>
                      <dt>Environment</dt>
                      <dd>
                        {props.environments.find(
                          (item) => item.value === input.environment_id,
                        )?.label ?? "Brain-wide"}
                      </dd>
                      <dt>Credentials</dt>
                      <dd>
                        {input.credential_alias ?? "No credential reference"}
                      </dd>
                      <dt>Execution</dt>
                      <dd>
                        {placementText(input.placement)}
                        {input.runner_reference
                          ? ` · ${input.runner_reference}`
                          : ""}
                      </dd>
                    </dl>
                  </>
                )}
                <Checkbox
                  label="Connection enabled"
                  checked={input.enabled}
                  onChange={(e) => patch({ enabled: e.currentTarget.checked })}
                />
                {isNew && (
                  <Alert title="Save first, then test explicitly">
                    Saving records configuration only. Add this connection to a
                    profile and grant Use independently before testing a tool. A
                    test may have real effects; cancelled or uncertain calls
                    must be inspected before any new attempt.
                  </Alert>
                )}
              </>
            )}
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
          {isNew && step > 0 && (
            <Button
              variant="default"
              onClick={() => {
                setStep((value) => value - 1);
                setStepError(null);
              }}
            >
              Back
            </Button>
          )}
          <Button
            type="submit"
            loading={save.isPending}
            disabled={
              props.brain.archived || !definition.data || !!definition.error
            }
          >
            {isNew && step < 4 ? "Continue" : "Save connection"}
          </Button>
        </Group>
      </Stack>
    </form>
  );
}
