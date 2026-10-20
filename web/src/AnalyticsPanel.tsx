import { useEffect, useState } from "react";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
  Alert,
  Badge,
  Button,
  Card,
  Code,
  Progress,
  Select,
  Spinner,
  StatusBadge,
} from "./components/ui";
import { stateTone } from "./features/graph/GraphStatusDrawer";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import { usePollInterval } from "./usePollInterval";

type Scope = components["schemas"]["GraphSelection"];
type Node = components["schemas"]["GraphNode"];
const label = (value: string) => value.replaceAll("_", " ");
const names: Record<string, string> = {
  pagerank: "PageRank centrality",
  leiden: "Leiden communities",
  wcc: "Connected components",
};

export function AnalyticsPanel({
  brain,
  scope,
  inspect,
}: {
  brain: Brain;
  scope: Scope;
  inspect: (node: Node) => void;
}) {
  const pollNormal = usePollInterval("normal");
  const cache = useQueryClient();
  const idem = useIdempotency();
  const [algorithm, setAlgorithm] = useState("pagerank");
  const [direction, setDirection] = useState("outgoing");
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const list = useQuery({
    queryKey: ["analytics", brain.id, offset],
    refetchInterval: pollNormal,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/graph/analytics", {
          params: { path: { brain: brain.id }, query: { offset } },
        }),
      ),
  });
  const view = useQuery({
    queryKey: ["analytics-view", brain.id, selected, page],
    enabled: !!selected,
    retry: false,
    gcTime: 0,
    refetchOnWindowFocus: false,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/analytics/{id}/view", {
          params: { path: { brain: brain.id, id: selected! } },
          body: { operation_id: null, offset: page },
          signal,
        }),
      ),
  });
  useEffect(
    () =>
      cache.getQueryCache().subscribe((event) => {
        if (
          event.query.queryKey[1] === brain.id &&
          event.type === "updated" &&
          event.action.type === "invalidate" &&
          !String(event.query.queryKey[0]).startsWith("analytics")
        ) {
          setSelected(null);
          void cache.cancelQueries({ queryKey: ["analytics-view", brain.id] });
          void cache.invalidateQueries({ queryKey: ["analytics", brain.id] });
        }
      }),
    [cache, brain.id],
  );
  const queue = useMutation({
    mutationFn: async () => {
      const body = { scope, algorithm, direction };
      return result(
        await client.POST("/api/brains/{brain}/graph/analytics", {
          params: { path: { brain: brain.id } },
          body,
          headers: {
            "Idempotency-Key": idem.forInput({ brain: brain.id, ...body }),
          },
        }),
      );
    },
    onSuccess: () => {
      idem.reset();
      setSelected(null);
      setOffset(0);
      void list.refetch();
    },
  });
  const cancel = useMutation({
    mutationFn: async (job: string) =>
      result(
        await client.POST("/api/brains/{id}/jobs/{job}/cancel", {
          params: { path: { id: brain.id, job } },
        }),
      ),
    onSuccess: () => {
      void list.refetch();
      void view.refetch();
    },
  });
  const current = list.data?.reports.find((r) => r.id === selected);
  const fresh =
    view.data?.report.state === "ready" &&
    (!current || current.state === "ready");
  const report =
    current?.state !== "ready" && current ? current : view.data?.report;
  const error = list.error ?? view.error ?? queue.error ?? cancel.error;
  return (
    <section
      aria-label="Graph analytics"
      data-testid="graph-analytics"
      className="grid min-w-0 content-start gap-4"
    >
      <div className="grid gap-1">
        <h3 className="m-0 text-title font-semibold">Graph analytics</h3>
        <p className="m-0 text-body text-foreground-secondary">
          Analyze the graph selection above. Choose its relation types
          explicitly; a score describes the selected topology, and does not
          establish acceptance or deployed behavior.
        </p>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <Select
          label="Analysis"
          value={algorithm}
          data={Object.entries(names).map(([value, label]) => ({
            value,
            label,
          }))}
          onChange={(value) => {
            setAlgorithm(value ?? "pagerank");
            if (value !== "pagerank") setDirection("both");
          }}
        />
        <Select
          label="Analysis direction"
          value={direction}
          data={
            algorithm === "pagerank"
              ? ["outgoing", "incoming", "both"]
              : ["both"]
          }
          onChange={(value) => setDirection(value ?? "both")}
        />
      </div>
      <p className="m-0 text-caption text-muted-foreground">
        Up to 10,000 candidates and 50,000 relationships. One heavy job at a
        time; native memory estimates must fit 64 MiB. Each recorded
        relationship has unit weight, including parallel edges and self loops.
      </p>
      {brain.role !== "reader" && (
        <Button
          className="justify-self-start"
          onClick={() => queue.mutate()}
          loading={queue.isPending}
          disabled={
            brain.archived ||
            !scope.relations.length ||
            (scope.kind === "repository" &&
              !scope.snapshot_id &&
              !scope.selection.repository_ids?.length) ||
            (scope.kind === "combined" && !scope.manifest_revision_id)
          }
        >
          Queue analysis
        </Button>
      )}
      {!scope.relations.length && (
        <p className="m-0 text-body text-muted-foreground">
          Select at least one graph relation above to analyze.
        </p>
      )}
      {queue.isSuccess && (
        <Alert variant="brand" role="status">
          Analysis queued. Open the report after processing completes.
        </Alert>
      )}
      {error && <Alert variant="destructive">{error.message}</Alert>}
      {list.isPending && <Spinner label="Loading analytical reports" />}
      {list.data?.reports.length === 0 && (
        <p className="m-0 text-body text-muted-foreground">
          No analytical reports yet.
        </p>
      )}
      {list.data?.reports.map((r) => {
        const job = list.data.jobs.find((j) => j.id === r.job_id);
        const pending = list.data.cleanup_pending.includes(r.id);
        const running = !!job && ["queued", "running"].includes(job.state);
        return (
          <Card
            key={r.id}
            className="grid gap-1.5 p-3 [overflow-wrap:anywhere]"
            data-testid="analytics-report"
          >
            <div className="flex items-center justify-between gap-2">
              <p className="m-0 text-body font-semibold">
                {names[r.algorithm]}
              </p>
              <StatusBadge state={stateTone(r.state)}>
                {label(r.state)}
              </StatusBadge>
            </div>
            <p className="m-0 text-caption text-muted-foreground">
              <span className="mono">{r.id}</span> ·{" "}
              <span className="mono">
                {new Date(r.created_at).toLocaleString()}
              </span>
            </p>
            <p className="m-0 text-label text-foreground-secondary">
              <span className="mono">{r.node_count}</span> vertices ·{" "}
              <span className="mono">{r.edge_count}</span> recorded
              relationships · {r.direction}
            </p>
            {running && job && (
              <div className="grid gap-1">
                <Progress value={job.progress} label="Analysis progress" />
                <p className="m-0 text-caption text-muted-foreground">
                  {label(job.state)} ·{" "}
                  <span className="mono">{job.progress}%</span>
                </p>
              </div>
            )}
            {pending && (
              <p className="m-0 text-label">
                Temporary graph cleanup is pending.
              </p>
            )}
            {r.error_code && (
              <p className="m-0 text-label text-warning-text">
                {label(r.error_code)}
              </p>
            )}
            {r.state === "stale" && (
              <p className="m-0 text-label">
                Inputs changed. Queue a new analysis from the current
                selection.
              </p>
            )}
            <div className="flex flex-wrap gap-2 pt-1">
              <Button
                size="sm"
                variant="secondary"
                onClick={() => {
                  setSelected(r.id);
                  setPage(0);
                }}
              >
                Open analytical report
              </Button>
              {brain.role === "admin" && running && job && (
                <Button
                  size="sm"
                  variant="ghost"
                  loading={cancel.isPending}
                  onClick={() => cancel.mutate(job.id)}
                >
                  Cancel analysis
                </Button>
              )}
            </div>
          </Card>
        );
      })}
      {list.data && list.data.total > 20 && (
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="secondary"
            disabled={!offset}
            onClick={() => setOffset(offset - 20)}
          >
            Previous reports
          </Button>
          <Button
            size="sm"
            variant="secondary"
            disabled={offset + 20 >= list.data.total}
            onClick={() => setOffset(offset + 20)}
          >
            More reports
          </Button>
        </div>
      )}
      {selected && view.isFetching && <Spinner label="Reading report" />}
      {selected && report && (
        <section
          className="grid gap-3 border-t border-border pt-4"
          data-testid="analytics-detail"
        >
          <div className="flex items-center justify-between gap-2">
            <h4 className="m-0 text-title font-semibold">
              {names[report.algorithm]} report
            </h4>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => void view.refetch()}
              loading={view.isFetching}
            >
              Refresh report
            </Button>
          </div>
          {!fresh && (
            <Alert variant="warning" role="status">
              {label(report.state)}. Current scores are unavailable; queue a
              new analysis when the selected inputs are ready.
            </Alert>
          )}
          {report.provenance?.coverage.partial && (
            <Alert variant="warning" title="Partial input coverage" role="status">
              {report.provenance.coverage.reasons.map(label).join(" · ")}
            </Alert>
          )}
          <p className="m-0 text-label text-foreground-secondary">
            GDS <span className="mono">{report.gds_version ?? "not run yet"}</span>{" "}
            · {report.direction} · unit weights · analytics epoch{" "}
            <span className="mono">{report.analytics_epoch}</span>
          </p>
          {report.algorithm === "pagerank" && (
            <p className="m-0 text-label text-foreground-secondary">
              50 iterations maximum; this stream result does not certify
              convergence.
            </p>
          )}
          {report.algorithm !== "pagerank" && (
            <p className="m-0 text-label text-foreground-secondary">
              Group numbers belong to this report and are not permanent entity
              identities.
            </p>
          )}
          <Accordion type="single" collapsible>
            <AccordionItem value="provenance">
              <AccordionTrigger>
                Exact analytical inputs and parameters
              </AccordionTrigger>
              <AccordionContent>
                <Code block className="[overflow-wrap:anywhere]">
                  {JSON.stringify(
                    {
                      selection: report.selection,
                      provenance: report.provenance,
                      parameters: report.parameters,
                      estimated_bytes: report.estimated_bytes,
                      projected_relationships: report.projected_edge_count,
                    },
                    null,
                    2,
                  )}
                </Code>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
          {fresh &&
            view.data?.rows.map((r) => (
              <Card
                key={r.node.key}
                className="grid gap-1.5 p-3 [overflow-wrap:anywhere]"
                data-testid="analytics-result"
              >
                <div className="flex items-start justify-between gap-2">
                  <p className="m-0 min-w-0 text-body font-semibold">
                    {r.node.evidence.label}
                  </p>
                  <Badge className="mono">
                    {r.score != null
                      ? `Score ${r.score.toPrecision(6)}`
                      : `Group ${r.group}`}
                  </Badge>
                </div>
                <p className="mono m-0 text-caption text-muted-foreground">
                  {r.node.key}
                </p>
                <p className="m-0 text-label text-foreground-secondary">
                  {r.node.evidence.qualifications.map(label).join(" · ")}
                </p>
                <Button
                  size="sm"
                  variant="secondary"
                  className="justify-self-start"
                  onClick={() => inspect(r.node)}
                >
                  Inspect analytical evidence
                </Button>
              </Card>
            ))}
          {fresh && view.data && view.data.total > 100 && (
            <div className="flex flex-wrap items-center gap-2">
              <Button
                size="sm"
                variant="secondary"
                disabled={!page}
                onClick={() => setPage(page - 100)}
              >
                Previous results
              </Button>
              <p className="mono m-0 text-caption text-muted-foreground">
                {page + 1}–{Math.min(page + 100, view.data.total)} of{" "}
                {view.data.total}
              </p>
              <Button
                size="sm"
                variant="secondary"
                disabled={page + 100 >= view.data.total}
                onClick={() => setPage(page + 100)}
              >
                More results
              </Button>
            </div>
          )}
        </section>
      )}
    </section>
  );
}
