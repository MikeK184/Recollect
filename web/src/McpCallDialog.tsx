import { CodeBlock } from "./components/CodeBlock";
import { useEffect, useState, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpCatalogue } from "./McpPanel";
import { callLabel, activeCall, CallState } from "./McpRuntimePanel";
import { McpResolveForm } from "./McpResolveForm";
import { useContentDeadline } from "./useContentDeadline";
import { McpObservations } from "./McpObservations";
import {
  TerminalCard,
  type TerminalStatusTone,
} from "./components/TerminalCard";
import { DetailInspector } from "./components/DetailInspector";
import { Alert, Badge, Button, Modal, Spinner } from "./components/ui";
import { Disclosure, FactRows } from "./features/connections/ui";
import { usePollInterval } from "./usePollInterval";
type Call = components["schemas"]["McpCall"];
type Resolution = components["schemas"]["McpResolution"];
const callStateTone = (state: string): TerminalStatusTone =>
  state === "succeeded"
    ? "ok"
    : ["tool_error", "failed", "cancelled"].includes(state)
      ? "error"
      : "pending";
function runnerName(reference: string) {
  if (reference === "central") return "Recollect service";
  if (reference.startsWith("device:")) return "Paired device";
  if (reference.startsWith("private:")) return "Private runner";
  return reference;
}
const reasonName = (code: string) =>
  code === "connector_response"
    ? "Server responded"
    : code.replaceAll("_", " ");
function displayedResult(value: unknown): string {
  if (value && typeof value === "object") {
    const response = value as {
      structuredContent?: unknown;
      content?: { type: string; text?: string }[];
    };
    if (response.structuredContent != null)
      return JSON.stringify(response.structuredContent, null, 2);
    if (Array.isArray(response.content)) {
      const text = response.content
        .filter((block) => typeof block.text === "string" && block.text.trim())
        .map((block) => block.text);
      if (text.length) return text.join("\n");
    }
  }
  return JSON.stringify(value, null, 2);
}

function ResolutionRow({ resolution: r }: { resolution: Resolution }) {
  const expired = useContentDeadline(r.explanation_expires_at);
  return (
    <div className="grid gap-0.5 border-t border-border pt-2.5 first:border-t-0 first:pt-0">
      <p className="m-0 text-label text-foreground">
        {r.kind.replaceAll("_", " ")} · {r.outcome} ·{" "}
        <span className="mono text-caption text-muted-foreground">
          {new Date(r.created_at).toLocaleString()}
        </span>
      </p>
      {r.explanation && !expired && (
        <p className="m-0 text-label text-foreground-secondary">
          {r.explanation}
        </p>
      )}
      {r.source_version_id && (
        <p className="m-0 text-caption text-muted-foreground">
          Evidence version{" "}
          <span className="mono">{r.source_version_id}</span>
        </p>
      )}
    </div>
  );
}

/** A titled block inside the call record, separated by a hairline. */
function CallSection({
  label,
  children,
  aside,
  ...props
}: {
  label?: string;
  children: ReactNode;
  aside?: ReactNode;
  "aria-label"?: string;
}) {
  return (
    <section
      className="grid min-w-0 gap-2.5 border-t border-border pt-4"
      {...props}
    >
      {label && (
        <div className="flex items-center justify-between gap-2">
          <h3 className="section-label m-0">{label}</h3>
          {aside}
        </div>
      )}
      {children}
    </section>
  );
}

export function McpCallDialog({
  brain,
  actor,
  catalogue,
  id,
  close,
  select,
  overlay = false,
}: {
  brain: Brain;
  actor: string;
  catalogue: McpCatalogue;
  id: string;
  close: () => void;
  select: (id: string) => void;
  /** Above a modal drawer the record opens as a dialog instead of docking. */
  overlay?: boolean;
}) {
  const pollFast = usePollInterval("fast");
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
    refetchInterval: pollFast,
    retry: false,
    gcTime: 0,
  });
  const call = query.error ? undefined : query.data;
  const profile = catalogue.profiles.find((p) => p.id === call?.profile_id);
  const connection = catalogue.connections.find(
    (c) => c.id === call?.connection_id,
  );
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
  const body = (
    <div className="grid min-w-0 gap-4">
      {query.isPending && <Spinner label="Loading call" />}
      {query.error && (
        <Alert
          variant="destructive"
          title="Call unavailable"
          actions={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void query.refetch()}
            >
              Refresh call
            </Button>
          }
        >
          {query.error.message}
        </Alert>
      )}
      {call && (
        <>
          <section className="grid min-w-0 gap-3" aria-label="Call summary">
            <div className="flex min-w-0 items-start justify-between gap-3">
              <div className="min-w-0">
                <h2 className="mono m-0 text-title font-medium text-foreground [overflow-wrap:anywhere]">
                  {call.tool_name}
                </h2>
                {(connection || profile) && (
                  <p className="m-0 mt-0.5 text-label text-muted-foreground">
                    {[connection?.name, profile?.name]
                      .filter(Boolean)
                      .join(" · ")}
                  </p>
                )}
              </div>
              <CallState state={call.state} />
            </div>
            <FactRows
              columns={overlay ? 2 : 1}
              items={[
                { term: "Runs on", value: runnerName(call.runner_reference) },
                {
                  term: "Scope",
                  value:
                    call.scope?.environment?.name ??
                    call.environment_id ??
                    "Brain-wide",
                },
                {
                  term: "Requested",
                  value: new Date(call.created_at).toLocaleString(),
                  mono: true,
                },
                {
                  term: "Timeout",
                  value: `${call.timeout_seconds} seconds`,
                  mono: true,
                },
                ...(call.started_at
                  ? [
                      {
                        term: "Started",
                        value: new Date(call.started_at).toLocaleString(),
                        mono: true,
                      },
                    ]
                  : []),
                ...(call.completed_at
                  ? [
                      {
                        term: "Finished",
                        value: new Date(call.completed_at).toLocaleString(),
                        mono: true,
                      },
                    ]
                  : []),
                ...(call.code
                  ? [{ term: "Result", value: reasonName(call.code) }]
                  : []),
              ]}
            />
            <Disclosure summary="Call identifiers">
              <FactRows
                items={[
                  { term: "Call", value: call.id, mono: true },
                  { term: "Runner", value: call.runner_reference, mono: true },
                  ...(call.code
                    ? [{ term: "Code", value: call.code, mono: true }]
                    : []),
                ]}
              />
            </Disclosure>
          </section>
          {call.scope && (
            <Disclosure summary="Recorded scope">
              <CodeBlock
                language="json"
                code={JSON.stringify(call.scope, null, 2)}
              />
            </Disclosure>
          )}
          {activeCall(call.state) && (
            <p className="m-0 text-label text-foreground-secondary">
              {call.state === "queued"
                ? "Waiting for the selected runner. This request expires after ten minutes."
                : "The runner owns this attempt. A missing response can leave completion unknown."}
            </p>
          )}
          {call.cancel_requested && (
            <Alert variant="warning" role="status">
              Cancellation requested. This does not establish that an external
              effect was undone.
            </Alert>
          )}
          {activeCall(call.state) &&
            (actor === call.actor_id || brain.role === "admin") && (
              <Button
                variant="destructive-outline"
                className="justify-self-start"
                disabled={call.cancel_requested}
                loading={busy}
                onClick={() => void action("cancel")}
              >
                Request cancellation
              </Button>
            )}
          {call.state === "unknown" && (
            <Alert variant="warning" title="Completion unknown">
              The tool may have performed its effect. A receipt lookup or
              retained evidence can record the observed outcome; this call will
              remain in the history.
            </Alert>
          )}
          {(!canUse || call.output_access === "permission_required") && (
            <Alert role="status">
              Profile Use is required to view retained output.
            </Alert>
          )}
          {canUse && (expired || call.output_access === "expired") && (
            <CallSection
              label="Tool output"
              aria-label="Tool output"
              aside={<Badge>Expired</Badge>}
            >
              <p className="m-0 text-label text-muted-foreground">
                Retained output has expired.
              </p>
            </CallSection>
          )}
          {canUse && !expired && call.result != null && (
            <CallSection label="Tool output" aria-label="Tool output">
              <TerminalCard
                title={`mcp.call · ${call.tool_name}`}
                status={{
                  label: callLabel(call.state),
                  tone: callStateTone(call.state),
                }}
                text={displayedResult(call.result)}
              />
              <Disclosure summary="Full response">
                <CodeBlock
                  language="json"
                  code={JSON.stringify(call.result, null, 2)}
                />
              </Disclosure>
            </CallSection>
          )}
          {call.reconciles_call_id && (
            <Button
              variant="secondary"
              className="justify-self-start"
              onClick={() => select(call.reconciles_call_id!)}
            >
              Inspect original call
            </Button>
          )}
          <CallSection aria-label="Memory capture">
            <McpObservations
              key={call.id}
              brain={brain}
              call={call.id}
              disposition={call.capture_disposition}
              onInspectChange={setViewingEvidence}
            />
          </CallSection>
          {canUse && call.resolutions.length > 0 && (
            <CallSection label="Recorded outcomes">
              {call.resolutions.map((r) => (
                <ResolutionRow key={r.id} resolution={r} />
              ))}
            </CallSection>
          )}
          {call.state === "unknown" && canUse && !brain.archived && (
            <div className="flex flex-wrap gap-2">
              {!call.reconciles_call_id && (
                <Button
                  variant="secondary"
                  loading={busy}
                  onClick={() => void action("reconcile")}
                >
                  Look up connector receipt
                </Button>
              )}
              <Button variant="secondary" onClick={() => setResolving(true)}>
                Record evidence
              </Button>
            </div>
          )}
          {resolving && canUse && (
            <CallSection label="Reconciliation evidence">
              <McpResolveForm
                brain={brain.id}
                call={call.id}
                saved={() => {
                  setResolving(false);
                  refresh();
                }}
              />
            </CallSection>
          )}
        </>
      )}
      {error && <Alert variant="destructive">{error}</Alert>}
    </div>
  );
  if (overlay)
    return (
      <Modal
        opened
        onClose={close}
        title="Tool call"
        size="lg"
        closeOnClickOutside={!viewingEvidence}
        closeOnEscape={!viewingEvidence}
      >
        {body}
      </Modal>
    );
  return (
    <DetailInspector
      opened
      onClose={close}
      title="Tool call"
      closeOnEscape={!viewingEvidence}
    >
      {body}
    </DetailInspector>
  );
}
