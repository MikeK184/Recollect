import { useEffect, useRef } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Cog } from "lucide-react";
import { client, result } from "./api";
import { SkeletonRows } from "./components/Skeleton";
import { StatusDot } from "./components/StatusDot";
import { EmptyState } from "./components/AsyncState";
import {
  Alert,
  Badge,
  Button,
  Card,
  CardBar,
  Progress,
  StatusBadge,
} from "./components/ui";
import { cn } from "./lib/utils";
import { useFirstLoadStagger } from "./features/activity/firstLoad";
import { usePollInterval } from "./usePollInterval";

const stateLabels: Record<string, string> = {
  current: "Up to date",
  queued: "Queued",
  running: "Running",
  succeeded: "Completed",
  failed: "Needs attention",
  cancelled: "Cancelled",
  missing: "Not processed",
};
const errorLabels: Record<string, string> = {
  database_error:
    "A database call failed. Processing will retry within its attempt limit.",
  permission_revoked: "The submitting account no longer has access.",
  unsupported_kind: "This installation cannot process this job type.",
  input_missing: "The input is unavailable.",
  content_removed:
    "The input expired or was erased. This work cannot be retried.",
  artifact_unavailable:
    "Evidence storage is unavailable. Processing will retry within its attempt limit.",
  lease_expired: "The worker stopped before completing this job.",
  cancelled_by_admin: "An administrator cancelled this job.",
};
const kindLabels: Record<string, string> = {
  "brain.refresh": "Update Brain details",
  "source.learn": "Learn from source",
  "handover.generate": "Generate handover",
  "source.process": "Process source text",
  "repository.process": "Process repository snapshot",
};

/** Overall processing state: status colour always pairs with an icon and a word. */
function ProcessingState({ state }: { state: string }) {
  if (state === "current")
    return <StatusBadge state="positive">{stateLabels[state]}</StatusBadge>;
  if (state === "failed")
    return <StatusBadge state="negative">{stateLabels[state]}</StatusBadge>;
  if (state === "queued" || state === "running")
    return (
      <Badge variant="brand">
        <StatusDot tone="accent" live size={6} />
        {stateLabels[state]}
      </Badge>
    );
  return <StatusBadge state="neutral">{stateLabels[state] ?? state}</StatusBadge>;
}

export function JobsPanel({
  id,
  admin,
  selectedJob,
}: {
  id: string;
  admin: boolean;
  selectedJob?: string;
}) {
  const pollFast = usePollInterval("fast");
  const focused = useRef<HTMLDivElement>(null);
  const cache = useQueryClient();
  const processing = useQuery({
    queryKey: ["processing", id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{id}/processing", {
          params: { path: { id } },
        }),
      ),
    refetchInterval: pollFast,
  });
  const jobs = useQuery({
    queryKey: ["jobs", id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{id}/jobs", { params: { path: { id } } }),
      ),
    refetchInterval: pollFast,
  });
  const action = useMutation({
    mutationFn: async ({
      job,
      action,
    }: {
      job: string;
      action: "retry" | "cancel";
    }) =>
      result(
        await client.POST(
          action === "retry"
            ? "/api/brains/{id}/jobs/{job}/retry"
            : "/api/brains/{id}/jobs/{job}/cancel",
          { params: { path: { id, job } } },
        ),
      ),
    onSuccess: () => {
      void cache.invalidateQueries({ queryKey: ["jobs", id] });
      void cache.invalidateQueries({ queryKey: ["processing", id] });
      void cache.invalidateQueries({ queryKey: ["audit", id] });
    },
  });
  const error = processing.error ?? jobs.error ?? action.error;
  const stagger = useFirstLoadStagger(jobs.data?.map((job) => job.id));
  useEffect(() => {
    if (selectedJob && jobs.data)
      focused.current?.scrollIntoView({ block: "center" });
  }, [selectedJob, jobs.data]);
  return (
    <Card
      role="region"
      aria-label="Background processing"
      className="overflow-hidden p-0"
    >
      <CardBar>
        <h2>Processing</h2>
        <span className="truncate text-caption text-muted-foreground">
          Your changes are saved immediately. Background work updates the views
          that use them.
        </span>
        <span className="ml-auto">
          {!processing.error && processing.data && (
            <ProcessingState state={processing.data.state} />
          )}
        </span>
      </CardBar>
      {error && (
        <div className="px-4 pt-4">
          <Alert
            variant="destructive"
            actions={
              <Button
                size="sm"
                variant="destructive-ghost"
                onClick={() => {
                  void processing.refetch();
                  void jobs.refetch();
                }}
              >
                Refresh processing
              </Button>
            }
          >
            {error.message}
          </Alert>
        </div>
      )}
      {jobs.isPending ? (
        <div className="px-4 py-2">
          <SkeletonRows label="Loading processing…" rows={3} />
        </div>
      ) : jobs.error || processing.error ? null : !jobs.data?.length ? (
        <EmptyState
          icon={Cog}
          title="No background work yet"
          description="Background work appears here when a change needs processing."
          className="py-10"
        />
      ) : (
        <div className="grid">
          {jobs.data.map((job, index) => {
            const live = ["queued", "running"].includes(job.state);
            const entrance = stagger(job.id, index);
            const selected = job.id === selectedJob;
            return (
              <div
                className={cn(
                  // border-t-0/pt override the legacy .job-row rule (legacy layer).
                  "job-row grid min-h-11 grid-cols-[minmax(0,1fr)_220px_104px] items-center gap-x-4 border-b border-t-0 border-border px-4 py-2 last:border-b-0",
                  selected &&
                    "relative bg-brand-soft/60 before:absolute before:inset-y-0 before:left-0 before:w-0.5 before:bg-brand",
                  entrance.className,
                )}
                style={entrance.style}
                key={job.id}
                ref={selected ? focused : undefined}
                data-testid={selected ? "selected-job" : undefined}
              >
                <div className="grid min-w-0 gap-1">
                  <span className="truncate text-body font-medium text-foreground">
                    {kindLabels[job.kind] ?? (
                      <span className="mono text-label">{job.kind}</span>
                    )}
                  </span>
                  {job.state === "running" && (
                    <Progress
                      label="Job progress"
                      value={job.progress}
                      className="max-w-[420px]"
                    />
                  )}
                  {job.error_code && (
                    <span
                      className={cn(
                        "text-caption",
                        job.state === "failed"
                          ? "text-destructive-text"
                          : "text-muted-foreground",
                      )}
                    >
                      {errorLabels[job.error_code] ??
                        "Processing did not complete."}
                    </span>
                  )}
                </div>
                <span className="flex items-center gap-2 text-caption text-muted-foreground">
                  {live && (
                    <StatusDot tone="accent" live size={7} label="Processing" />
                  )}
                  {/* One text run, so the row never repeats the overall state badge. */}
                  <span
                    className={cn(
                      job.state === "failed" &&
                        "font-medium text-destructive-text",
                    )}
                  >
                    {stateLabels[job.state]}
                    {" · "}
                    <span className="mono">
                      Attempt {job.attempts} of {job.max_attempts}
                    </span>
                  </span>
                </span>
                <div className="flex justify-end">
                {admin &&
                  ["queued", "running", "failed", "cancelled"].includes(
                    job.state,
                  ) && (
                    <Button
                      size="sm"
                      variant="secondary"
                      loading={
                        action.isPending && action.variables?.job === job.id
                      }
                      disabled={
                        action.isPending ||
                        job.error_code === "content_removed" ||
                        (["failed", "cancelled"].includes(job.state) &&
                          [
                            "source.learn",
                            "handover.generate",
                            "semantic.generate",
                            "graph.analyze",
                          ].includes(job.kind))
                      }
                      onClick={() =>
                        action.mutate({
                          job: job.id,
                          action: live ? "cancel" : "retry",
                        })
                      }
                    >
                      {live ? "Cancel job" : "Retry job"}
                    </Button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}
