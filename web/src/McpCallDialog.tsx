import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Code,
  Group,
  Loader,
  Modal,
  Stack,
  Text,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpCatalogue } from "./McpPanel";
import { callLabel, activeCall } from "./McpRuntimePanel";
import { McpResolveForm } from "./McpResolveForm";
import { useContentDeadline } from "./useContentDeadline";
import { McpObservations } from "./McpObservations";
type Call = components["schemas"]["McpCall"];
type Resolution = components["schemas"]["McpResolution"];
function displayedResult(value: unknown): string {
  if (value && typeof value === "object") {
    const response = value as {
      structuredContent?: unknown;
      content?: { type: string; text?: string }[];
    };
    if (response.structuredContent != null)
      return JSON.stringify(response.structuredContent, null, 2);
    if (Array.isArray(response.content))
      return response.content.map((block) => block.text ?? "").join("\n");
  }
  return JSON.stringify(value, null, 2);
}

function ResolutionRow({ resolution: r }: { resolution: Resolution }) {
  const expired = useContentDeadline(r.explanation_expires_at);
  return (
    <Stack gap={2}>
      <Text size="sm">
        {r.kind.replaceAll("_", " ")} · {r.outcome} ·{" "}
        {new Date(r.created_at).toLocaleString()}
      </Text>
      {r.explanation && !expired && <Text size="sm">{r.explanation}</Text>}
      {r.source_version_id && (
        <Text size="xs" c="dimmed">
          Evidence version {r.source_version_id}
        </Text>
      )}
    </Stack>
  );
}
export function McpCallDialog({
  brain,
  actor,
  catalogue,
  id,
  close,
  select,
}: {
  brain: Brain;
  actor: string;
  catalogue: McpCatalogue;
  id: string;
  close: () => void;
  select: (id: string) => void;
}) {
  const cache = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [resolving, setResolving] = useState(false);
  const [viewingEvidence, setViewingEvidence] = useState(false);
  const [receiptRequest] = useState(() => ({
    request_id: crypto.randomUUID(),
    client_session_id: crypto.randomUUID(),
  }));
  const query = useQuery({
    queryKey: ["mcp", brain.id, "call", id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/calls/{id}", {
          params: { path: { brain: brain.id, id } },
          signal,
        }),
      ),
    refetchInterval: 2000,
    retry: false,
    gcTime: 0,
  });
  const call = query.error ? undefined : query.data;
  const profile = catalogue.profiles.find((p) => p.id === call?.profile_id);
  const canUse = !!profile?.rights.use_profile;
  const expired = useContentDeadline(call?.payload_expires_at);
  useEffect(() => {
    if (!canUse || expired || query.error) {
      cache.setQueryData<Call>(
        ["mcp", brain.id, "call", id],
        (old) =>
          old && {
            ...old,
            result: null,
            resolutions: canUse && !query.error ? old.resolutions : [],
          },
      );
      setResolving(false);
    }
  }, [cache, brain.id, id, canUse, expired, query.error]);
  const refresh = () => {
    void query.refetch();
    void cache.invalidateQueries({ queryKey: ["mcp", brain.id, "calls"] });
    void cache.invalidateQueries({ queryKey: ["audit", brain.id] });
  };
  const action = async (kind: "cancel" | "reconcile") => {
    setBusy(true);
    setError(null);
    try {
      if (kind === "cancel")
        result(
          await client.POST("/api/brains/{brain}/mcp/calls/{id}/cancel", {
            params: { path: { brain: brain.id, id } },
          }),
        );
      else {
        const receipt = result(
          await client.POST("/api/brains/{brain}/mcp/calls/{id}/reconcile", {
            params: { path: { brain: brain.id, id } },
            body: receiptRequest,
          }),
        );
        select(receipt.id);
      }
      refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not update this call.");
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      opened
      onClose={close}
      title="Tool call"
      size="xl"
      closeOnEscape={!viewingEvidence}
      closeOnClickOutside={!viewingEvidence}
      trapFocus={!viewingEvidence}
    >
      <Stack>
        {query.isPending && <Loader size="sm" />}
        {query.error && (
          <Alert color="red" title="Call unavailable">
            {query.error.message}
            <Button onClick={() => void query.refetch()} variant="light">
              Refresh call
            </Button>
          </Alert>
        )}
        {call && (
          <>
            <Group justify="space-between">
              <Text fw={600}>{call.tool_name}</Text>
              <Badge
                color={
                  call.state === "succeeded"
                    ? "teal"
                    : call.state === "unknown"
                      ? "yellow"
                      : "gray"
                }
              >
                {callLabel(call.state)}
              </Badge>
            </Group>
            <Text size="sm">Call {call.id}</Text>
            <Text size="sm">Runner: {call.runner_reference}</Text>
            <Text size="sm">
              Environment: {call.environment_id ?? "Brain-wide"} · Timeout{" "}
              {call.timeout_seconds}s
            </Text>
            {call.code && <Text size="sm">Reason: {call.code}</Text>}
            {call.scope && (
              <details>
                <summary>Recorded scope</summary>
                <Code block>{JSON.stringify(call.scope, null, 2)}</Code>
              </details>
            )}
            {activeCall(call.state) && (
              <Text size="sm">
                {call.state === "queued"
                  ? "Waiting for the selected runner. This request expires after ten minutes."
                  : "The runner owns this attempt. A missing response can leave completion unknown."}
              </Text>
            )}
            {call.cancel_requested && (
              <Alert color="yellow">
                Cancellation requested. This does not establish that an external
                effect was undone.
              </Alert>
            )}
            {activeCall(call.state) &&
              (actor === call.actor_id || brain.role === "admin") && (
                <Button
                  color="orange"
                  variant="light"
                  disabled={call.cancel_requested}
                  loading={busy}
                  onClick={() => void action("cancel")}
                >
                  Request cancellation
                </Button>
              )}
            {call.state === "unknown" && (
              <Alert color="yellow" title="Completion unknown">
                The tool may have performed its effect. A receipt lookup or
                retained evidence can record the observed outcome; this call
                will remain in the history.
              </Alert>
            )}
            {(!canUse || call.output_access === "permission_required") && (
              <Alert color="gray">
                Profile Use is required to view retained output.
              </Alert>
            )}
            {canUse && (expired || call.output_access === "expired") && (
              <Text>Retained output has expired.</Text>
            )}
            {canUse && !expired && call.result != null && (
              <>
                <Text fw={600}>Sanitized result</Text>
                <Code block style={{ maxHeight: 420, overflow: "auto" }}>
                  {displayedResult(call.result)}
                </Code>
                <details>
                  <summary>Full response</summary>
                  <Code block style={{ maxHeight: 300, overflow: "auto" }}>
                    {JSON.stringify(call.result, null, 2)}
                  </Code>
                </details>
              </>
            )}
            {call.reconciles_call_id && (
              <Button
                variant="light"
                onClick={() => select(call.reconciles_call_id!)}
              >
                Inspect original call
              </Button>
            )}
            <McpObservations
              key={call.id}
              brain={brain}
              call={call.id}
              disposition={call.capture_disposition}
              onInspectChange={setViewingEvidence}
            />
            {canUse &&
              call.resolutions.map((r) => (
                <ResolutionRow key={r.id} resolution={r} />
              ))}
            {call.state === "unknown" && canUse && !brain.archived && (
              <Group>
                {!call.reconciles_call_id && (
                  <Button
                    variant="light"
                    loading={busy}
                    onClick={() => void action("reconcile")}
                  >
                    Look up connector receipt
                  </Button>
                )}
                <Button variant="light" onClick={() => setResolving(true)}>
                  Record evidence
                </Button>
              </Group>
            )}
            {resolving && canUse && (
              <McpResolveForm
                brain={brain.id}
                call={call.id}
                saved={() => {
                  setResolving(false);
                  refresh();
                }}
              />
            )}
          </>
        )}
        {error && <Alert color="red">{error}</Alert>}
      </Stack>
    </Modal>
  );
}
