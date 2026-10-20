import type { Dispatch, SetStateAction } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Drawer,
  Spinner,
  StatusBadge,
  type StatusState,
} from "../../components/ui";
import type { UseQueryResult } from "@tanstack/react-query";
import type { Brain } from "../../api";
import type { components } from "../../api-schema";
import type { SceneGenerations } from "../../graph-generation-status";
import { label, type Scope } from "./graphScope";
import { Failure } from "./GraphPanelParts";
import type { useGraphRebuild } from "./useGraphScopeData";

export function GraphStatusDrawer({
  opened,
  onClose,
  brain,
  scene,
  scope,
  status,
  rebuild,
  offset,
  setOffset,
}: {
  opened: boolean;
  onClose: () => void;
  brain: Brain;
  scene: SceneGenerations | null;
  scope: Scope;
  status: UseQueryResult<components["schemas"]["GraphStatus"], Error>;
  rebuild: ReturnType<typeof useGraphRebuild>;
  offset: number;
  setOffset: Dispatch<SetStateAction<number>>;
}) {
  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title="Graph status and maintenance"
      size={560}
      bodyClassName="grid content-start gap-4"
    >
      {scene && (
        <section className="grid gap-2 [overflow-wrap:anywhere]">
          <div className="flex flex-wrap items-center gap-2">
            <StatusBadge state={stateTone(scene.state)}>
              {label(scene.state)}
            </StatusBadge>
            <p className="mono m-0 text-caption text-foreground-secondary">
              {scene.total_nodes} eligible entities · {scene.total_edges}{" "}
              eligible relationships
            </p>
          </div>
          <p className="m-0 text-caption text-muted-foreground">
            Generation <span className="mono">{scene.generation.id}</span> ·
            input epoch{" "}
            <span className="mono">{scene.generation.input_epoch}</span> ·{" "}
            {scene.scope.kind === "combined" ? "origin epoch" : "memory epoch"}{" "}
            <span className="mono">
              {scene.scope.kind === "combined"
                ? scene.generation.input_epoch
                : scene.memory_epoch}
            </span>
          </p>
          {scene.scope.kind === "combined" && (
            <div className="grid gap-1.5">
              <p className="m-0 text-label font-semibold">
                Exact repository inputs
              </p>
              {scene.inputs.map((input) => (
                <p
                  key={input.id}
                  className="m-0 text-caption text-muted-foreground"
                >
                  Snapshot <span className="mono">{input.snapshot_id}</span> ·
                  generation <span className="mono">{input.id}</span>
                </p>
              ))}
              {!!scene.link_issues_total && (
                <Alert variant="warning" title="Unresolved repository links">
                  <p>
                    Showing {scene.link_issues.length} of{" "}
                    {scene.link_issues_total} eligible source records.
                  </p>
                  {scene.link_issues.map((issue) => (
                    <p key={issue.source} className="text-caption">
                      {label(issue.code)} ·{" "}
                      <span className="mono">{issue.source}</span>
                    </p>
                  ))}
                </Alert>
              )}
            </div>
          )}
          {scene.coverage.partial && (
            <Alert variant="warning" role="status">
              {scene.coverage.reasons.map(label).join(" · ")}
            </Alert>
          )}
          {!scene.total_nodes && (
            <p className="m-0 text-body">
              No entities are eligible in this view.
            </p>
          )}
        </section>
      )}
      {brain.role !== "reader" && (
        <Button
          variant="secondary"
          className="justify-self-start"
          loading={rebuild.isPending}
          disabled={
            brain.archived ||
            (scope.kind === "repository" && !scope.snapshot_id) ||
            (scope.kind === "combined" && !scope.manifest_revision_id)
          }
          onClick={() => rebuild.mutate()}
        >
          Rebuild graph
        </Button>
      )}
      <Failure error={rebuild.error} />
      {rebuild.isSuccess && (
        <Alert variant="brand" role="status">
          Graph rebuild queued. Progress appears below.
        </Alert>
      )}
      <h3 className="m-0 border-t border-border pt-4 text-title font-semibold">
        Projection processing
      </h3>
      <Failure error={status.error} />
      {status.isPending && <Spinner label="Loading graph status" />}
      {status.data?.generations.length === 0 && (
        <p className="m-0 text-body text-muted-foreground">
          No graph generation yet. The worker discovers materialized
          repositories and claim evidence automatically.
        </p>
      )}
      {status.data?.generations.map((g) => {
        const job = status.data?.jobs.find((j) => j.id === g.job_id);
        return (
          <Card key={g.id} className="grid gap-1.5 p-3 [overflow-wrap:anywhere]">
            <div className="flex items-center justify-between gap-2">
              <p className="m-0 text-body font-semibold">
                {label(g.kind)} graph
              </p>
              <StatusBadge state={stateTone(g.state)}>{label(g.state)}</StatusBadge>
            </div>
            <p className="m-0 text-caption text-muted-foreground">
              Generation <span className="mono">{g.id}</span>
              {g.snapshot_id ? (
                <>
                  {" "}
                  · snapshot <span className="mono">{g.snapshot_id}</span>
                </>
              ) : (
                ""
              )}
              {g.kind === "combined"
                ? ` · ${g.input_snapshot_ids?.length ?? 0} exact repository inputs · ${g.adapter}`
                : ""}
            </p>
            <p className="mono m-0 text-caption text-foreground-secondary">
              {g.node_count} entities · {g.edge_count} relationships ·{" "}
              {g.unresolved} unresolved · {g.ambiguous} ambiguous ·{" "}
              {g.unsupported} unsupported
            </p>
            <p className="m-0 text-caption text-muted-foreground">
              <span className="mono">
                {new Date(g.created_at).toLocaleString()}
              </span>{" "}
              ·{" "}
              {g.kind === "knowledge" &&
              g.input_epoch !== status.data?.memory_epoch
                ? "Knowledge input changed; reads use current eligibility."
                : g.kind === "combined" &&
                    g.input_epoch !== status.data?.link_epoch
                  ? "Origin bindings changed; current links are being rebuilt."
                  : "Exact input retained."}
            </p>
            {g.error_code && (
              <Alert variant="destructive">{label(g.error_code)}</Alert>
            )}
            {job && (
              <p className="m-0 text-caption text-muted-foreground">
                Job: {label(job.state)} ·{" "}
                <span className="mono">{job.progress}%</span>
              </p>
            )}
          </Card>
        );
      })}
      {status.data && status.data.total > 20 && (
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            size="sm"
            disabled={!offset}
            onClick={() => setOffset((n) => n - 20)}
          >
            Previous generations
          </Button>
          <Button
            variant="secondary"
            size="sm"
            disabled={offset + 20 >= status.data.total}
            onClick={() => setOffset((n) => n + 20)}
          >
            More generations
          </Button>
        </div>
      )}
    </Drawer>
  );
}

/** Status pairs colour with an icon and the recorded word. */
export function stateTone(state: string): StatusState {
  if (["ready", "succeeded", "completed", "current"].includes(state))
    return "positive";
  if (["failed", "error", "unavailable", "cancelled", "removed"].includes(state))
    return "negative";
  if (["queued", "running", "building", "stale", "partial"].includes(state))
    return "attention";
  return "neutral";
}
