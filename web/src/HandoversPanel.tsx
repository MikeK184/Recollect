import { useState } from "react";
import { RotateCcw, ScanSearch, Sparkles, X } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { SkeletonRows } from "./components/Skeleton";
import { staggerStyle } from "./components/Motion";
import { SectionHeader } from "./components/PageHeader";
import { Alert, Button, Modal, StatusBadge, TextField } from "./components/ui";
import type { components } from "./api-schema";
import { ContributionPicker, combineContributions } from "./MemoryForms";
import { ClaimDialog } from "./ClaimsPanel";
import { useIdempotency } from "./useIdempotency";
import { usePollInterval } from "./usePollInterval";
import { useWorkspaceCatalogue } from "./catalogueQueries";
type View = components["schemas"]["ClaimView"];
function GenerateDialog({
  brain,
  onClose,
  onQueued,
}: {
  brain: Brain;
  onClose: () => void;
  onQueued: () => void;
}) {
  const [title, setTitle] = useState("");
  const [rows, setRows] = useState<View[]>([]);
  const command = useIdempotency();
  const send = useMutation({
    mutationFn: async () => {
      const body = {
        title,
        contributions: rows.map((row) => row.revision.id),
        operation_id: null,
      };
      return result(
        await client.POST("/api/brains/{brain}/handovers", {
          params: { path: { brain: brain.id } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: () => {
      command.reset();
      onQueued();
      onClose();
    },
  });
  const scope = combineContributions(rows).selection;
  return (
    <Modal
      opened
      onClose={onClose}
      size="xl"
      title="Generate handover"
      description="The selected text model will compose a handover from these records. It retains every contribution and follows this Brain's acceptance policy. Human review is optional."
      footer={
        <Button
          type="submit"
          form="generate-handover"
          variant="brand"
          disabled={!rows.length}
          loading={send.isPending}
        >
          <Sparkles aria-hidden="true" />
          Queue handover
        </Button>
      }
    >
      <form
        id="generate-handover"
        className="grid gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          send.mutate();
        }}
      >
        <TextField
          label="Handover title"
          required
          maxLength={256}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
        <ContributionPicker
          brain={brain}
          ids={rows.map((row) => row.revision.id)}
          onChange={setRows}
        />
        <p className="m-0 text-label text-foreground-secondary">
          <span className="mono">{rows.length}</span> contributions ·{" "}
          <span className="mono">{scope.repository_ids.length}</span> selected
          repositories · <span className="mono">{scope.area_ids.length}</span>{" "}
          selected areas
        </p>
        <p className="m-0 text-label text-muted-foreground">
          Requires the Brain model policy to allow synthesis, claim content and
          the requested title as query content.
        </p>
        {send.error && (
          <Alert variant="destructive" title="Handover could not be queued">
            {send.error.message}
          </Alert>
        )}
      </form>
    </Modal>
  );
}
export function HandoversPanel({
  brain,
  onInspect,
}: {
  brain: Brain;
  /** Open a generated handover in the page's own record inspector. */
  onInspect?: (claimId: string) => void;
}) {
  const pollNormal = usePollInterval("normal");
  const [offset, setOffset] = useState(0);
  const [creating, setCreating] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [queued, setQueued] = useState(false);
  const command = useIdempotency();
  const cache = useQueryClient();
  const runs = useQuery({
    queryKey: ["handovers", brain.id, offset],
    refetchInterval: pollNormal,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/handovers", {
          params: { path: { brain: brain.id }, query: { offset } },
        }),
      ),
  });
  const catalogue = useWorkspaceCatalogue(brain.id);
  const refresh = () => {
    for (const key of [
      "handovers",
      "claims",
      "claim",
      "models-usage",
      "jobs",
      "audit",
    ])
      void cache.invalidateQueries({ queryKey: [key, brain.id] });
  };
  const retry = useMutation({
    mutationFn: async (id: string) =>
      result(
        await client.POST("/api/brains/{brain}/handovers/{run}/retry", {
          params: { path: { brain: brain.id, run: id } },
          body: { operation_id: null },
          headers: { "Idempotency-Key": command.forInput({ id }) },
        }),
      ),
    onSuccess: () => {
      command.reset();
      refresh();
    },
  });
  const cancel = useMutation({
    mutationFn: async (id: string) =>
      result(
        await client.POST("/api/brains/{id}/jobs/{job}/cancel", {
          params: { path: { id: brain.id, job: id } },
        }),
      ),
    onSuccess: refresh,
  });
  const error = runs.error ?? retry.error ?? cancel.error;
  const writable = brain.role !== "reader" && !brain.archived;
  return (
    <div className="grid gap-4">
      <SectionHeader
        as="h3"
        title="Generated handovers"
        description="Bring selected claims, decisions and procedures together while preserving their evidence, applicability and review state."
        actions={
          writable && (
            <Button size="sm" onClick={() => setCreating(true)}>
              Generate handover
            </Button>
          )
        }
      />
      {queued && (
        <Alert
          variant="info"
          role="status"
          actions={
            <Button variant="ghost" size="sm" onClick={() => setQueued(false)}>
              <X aria-hidden="true" />
              Dismiss
            </Button>
          }
        >
          Handover queued. Its result will appear below.
        </Alert>
      )}
      {error && (
        <Alert variant="destructive" title="Handover request failed">
          {error.message}
        </Alert>
      )}
      {runs.isPending && <SkeletonRows label="Loading handovers…" rows={3} />}
      {runs.data?.total === 0 && (
        <p className="m-0 text-label text-muted-foreground">
          No generated handovers yet.
        </p>
      )}
      {!runs.error && !!runs.data?.items.length && (
        <ul className="m-0 grid list-none gap-2 p-0">
          {runs.data.items.map((run, index) => (
            <li
              key={run.id}
              className="grid animate-enter gap-2 rounded-lg border border-border bg-card p-3"
              style={staggerStyle(index)}
            >
              <div className="flex items-start justify-between gap-3">
                <span className="min-w-0 text-body font-medium text-foreground [overflow-wrap:anywhere]">
                  {run.title || "Handover attempt"}
                </span>
                <StatusBadge
                  state={
                    run.state === "succeeded"
                      ? "positive"
                      : ["failed", "removed"].includes(run.state)
                        ? "negative"
                        : "neutral"
                  }
                >
                  {run.state}
                </StatusBadge>
              </div>
              <p className="m-0 text-caption text-muted-foreground">
                <span className="mono">
                  {new Date(run.created_at).toLocaleString()}
                </span>{" "}
                · <span className="mono">{run.contributions.length}</span>{" "}
                contributions
              </p>
              {run.error_code && (
                <p className="m-0 text-label text-destructive-text">
                  {run.error_code.replaceAll("_", " ")}
                </p>
              )}
              <div className="flex flex-wrap gap-2 empty:hidden">
                {run.claim_id && run.state === "succeeded" && (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() =>
                      onInspect
                        ? onInspect(run.claim_id!)
                        : setSelected(run.claim_id!)
                    }
                  >
                    <ScanSearch aria-hidden="true" />
                    Inspect generated handover
                  </Button>
                )}
                {["failed", "cancelled"].includes(run.state) && writable && (
                  <Button
                    variant="secondary"
                    size="sm"
                    loading={retry.isPending && retry.variables === run.id}
                    disabled={retry.isPending && retry.variables !== run.id}
                    onClick={() => retry.mutate(run.id)}
                  >
                    <RotateCcw aria-hidden="true" />
                    Start new handover attempt
                  </Button>
                )}
                {["queued", "running"].includes(run.state) &&
                  brain.role === "admin" && (
                    <Button
                      variant="ghost"
                      size="sm"
                      loading={
                        cancel.isPending && cancel.variables === run.job_id
                      }
                      onClick={() => cancel.mutate(run.job_id)}
                    >
                      Cancel handover
                    </Button>
                  )}
              </div>
            </li>
          ))}
        </ul>
      )}
      {!!runs.data && runs.data.total > 20 && (
        <div className="flex gap-2">
          <Button
            variant="secondary"
            size="sm"
            disabled={!offset}
            onClick={() => setOffset(Math.max(0, offset - 20))}
          >
            Previous handovers
          </Button>
          <Button
            variant="secondary"
            size="sm"
            disabled={offset + 20 >= runs.data.total}
            onClick={() => setOffset(offset + 20)}
          >
            More handovers
          </Button>
        </div>
      )}
      {creating && (
        <GenerateDialog
          brain={brain}
          onClose={() => setCreating(false)}
          onQueued={() => {
            setQueued(true);
            setOffset(0);
            refresh();
          }}
        />
      )}
      {selected && (
        <ClaimDialog
          key={selected}
          brain={brain}
          id={selected}
          catalogue={catalogue.data}
          onClose={() => setSelected(null)}
          onSaved={refresh}
        />
      )}
    </div>
  );
}
