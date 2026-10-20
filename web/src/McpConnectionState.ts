import { useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpCatalogue } from "./McpPanel";
import { useWorkspace } from "./app/context";
import type {
  ServerDraft,
  DraftSecret,
} from "./features/connections/mcpConfig";
import { useIdempotency } from "./useIdempotency";
import { usePrivateRunners } from "./McpPrivateRunners";
import { usePollInterval } from "./usePollInterval";

export type Detail = components["schemas"]["McpConnectionDetail"];
export type Input = components["schemas"]["McpConnectionInput"];

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

export type Props = {
  brain: Brain;
  id: string;
  catalogue: McpCatalogue;
  environments: { value: string; label: string }[];
  close: () => void;
  saved: () => void;
  stale: (message: string) => void;
  manageRunners?: () => void;
  initialDraft?: ServerDraft;
  /** Approved connector preselected for a new connection. */
  initialDefinition?: string;
  inline?: boolean;
  nameMount?: HTMLElement | null;
};

// The draft, its validation and the save mutation for one connection form. The
// parent form calls this unconditionally, so secrets and settings survive any
// collapsed section for the lifetime of the form.
export function useConnectionState(
  props: Props,
  detail: Detail | undefined,
  currentRevision: string | undefined,
) {
  const pollNormal = usePollInterval("normal");
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
  const [input, setInput] = useState<Input>(() => ({
    name: detail?.summary.name ?? props.initialDraft?.name ?? "",
    description: detail?.summary.description ?? "",
    definition_key:
      detail?.summary.definition_key ?? props.initialDefinition ?? "",
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
    refetchInterval: pollNormal,
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
  return {
    owner,
    secrets,
    setSecrets,
    authMode,
    setAuthMode,
    created,
    stepError,
    setStepError,
    staleDraft,
    input,
    settings,
    setSettings,
    privateRunners,
    definition,
    save,
    patch,
    validateStep,
  };
}

export type ConnectionState = ReturnType<typeof useConnectionState>;
export type DefinitionDetail = NonNullable<
  ConnectionState["definition"]["data"]
>;
