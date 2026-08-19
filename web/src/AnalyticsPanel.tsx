import { useEffect, useState } from "react";
import {
  Accordion,
  Alert,
  Badge,
  Button,
  Card,
  Code,
  Group,
  Loader,
  Progress,
  Select,
  SimpleGrid,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";

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
  const cache = useQueryClient();
  const idem = useIdempotency();
  const [algorithm, setAlgorithm] = useState("pagerank");
  const [direction, setDirection] = useState("outgoing");
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const list = useQuery({
    queryKey: ["analytics", brain.id, offset],
    refetchInterval: 3000,
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
    <Card
      withBorder
      component="section"
      aria-label="Graph analytics"
      data-testid="graph-analytics"
      style={{ minWidth: 0 }}
    >
      <Stack>
        <Title order={4}>Graph analytics</Title>
        <Text size="sm">
          Analyze the graph selection above. Choose its relation types
          explicitly; a score describes the selected topology, and does not
          establish acceptance or deployed behavior.
        </Text>
        <SimpleGrid cols={{ base: 1, sm: 2 }}>
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
        </SimpleGrid>
        <Text size="xs" c="dimmed">
          Up to 10,000 candidates and 50,000 relationships. One heavy job at a
          time; native memory estimates must fit 64 MiB. Each recorded
          relationship has unit weight, including parallel edges and self loops.
        </Text>
        {brain.role !== "reader" && (
          <Button
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
          <Text size="sm">
            Select at least one graph relation above to analyze.
          </Text>
        )}
        {queue.isSuccess && (
          <Alert color="teal">
            Analysis queued. Open the report after processing completes.
          </Alert>
        )}
        {error && <Alert color="red">{error.message}</Alert>}
        {list.isPending && <Loader size="sm" />}
        {list.data?.reports.length === 0 && (
          <Text size="sm">No analytical reports yet.</Text>
        )}
        {list.data?.reports.map((r) => {
          const job = list.data.jobs.find((j) => j.id === r.job_id);
          const pending = list.data.cleanup_pending.includes(r.id);
          return (
            <Card key={r.id} withBorder p="sm" data-testid="analytics-report">
              <Stack gap="xs">
                <Group justify="space-between">
                  <Text fw={600}>{names[r.algorithm]}</Text>
                  <Badge
                    color={
                      r.state === "ready"
                        ? "teal"
                        : r.state === "stale"
                          ? "yellow"
                          : "gray"
                    }
                  >
                    {label(r.state)}
                  </Badge>
                </Group>
                <Text size="xs" style={{ overflowWrap: "anywhere" }}>
                  {r.id} · {new Date(r.created_at).toLocaleString()}
                </Text>
                <Text size="sm">
                  {r.node_count} vertices · {r.edge_count} recorded
                  relationships · {r.direction}
                </Text>
                {job && ["queued", "running"].includes(job.state) && (
                  <>
                    <Progress
                      value={job.progress}
                      aria-label="Analysis progress"
                    />
                    <Text size="xs">
                      {label(job.state)} · {job.progress}%
                    </Text>
                  </>
                )}
                {pending && (
                  <Text size="sm">Temporary graph cleanup is pending.</Text>
                )}
                {r.error_code && (
                  <Text size="sm" c="orange">
                    {label(r.error_code)}
                  </Text>
                )}
                {r.state === "stale" && (
                  <Text size="sm">
                    Inputs changed. Queue a new analysis from the current
                    selection.
                  </Text>
                )}
                <Group>
                  <Button
                    size="xs"
                    variant="light"
                    onClick={() => {
                      setSelected(r.id);
                      setPage(0);
                    }}
                  >
                    Open analytical report
                  </Button>
                  {brain.role === "admin" &&
                    job &&
                    ["queued", "running"].includes(job.state) && (
                      <Button
                        size="xs"
                        variant="default"
                        loading={cancel.isPending}
                        onClick={() => cancel.mutate(job.id)}
                      >
                        Cancel analysis
                      </Button>
                    )}
                </Group>
              </Stack>
            </Card>
          );
        })}
        {list.data && list.data.total > 20 && (
          <Group>
            <Button
              size="xs"
              disabled={!offset}
              onClick={() => setOffset(offset - 20)}
            >
              Previous reports
            </Button>
            <Button
              size="xs"
              disabled={offset + 20 >= list.data.total}
              onClick={() => setOffset(offset + 20)}
            >
              More reports
            </Button>
          </Group>
        )}
        {selected && view.isFetching && <Loader size="sm" />}
        {selected && report && (
          <Stack data-testid="analytics-detail">
            <Group justify="space-between">
              <Title order={5}>{names[report.algorithm]} report</Title>
              <Button
                size="xs"
                variant="subtle"
                onClick={() => void view.refetch()}
                loading={view.isFetching}
              >
                Refresh report
              </Button>
            </Group>
            {!fresh && (
              <Alert color="yellow">
                {label(report.state)}. Current scores are unavailable; queue a
                new analysis when the selected inputs are ready.
              </Alert>
            )}
            {report.provenance?.coverage.partial && (
              <Alert color="yellow" title="Partial input coverage">
                {report.provenance.coverage.reasons.map(label).join(" · ")}
              </Alert>
            )}
            <Text size="sm">
              GDS {report.gds_version ?? "not run yet"} · {report.direction} ·
              unit weights · analytics epoch {report.analytics_epoch}
            </Text>
            {report.algorithm === "pagerank" && (
              <Text size="sm">
                50 iterations maximum; this stream result does not certify
                convergence.
              </Text>
            )}
            {report.algorithm !== "pagerank" && (
              <Text size="sm">
                Group numbers belong to this report and are not permanent entity
                identities.
              </Text>
            )}
            <Accordion>
              <Accordion.Item value="provenance">
                <Accordion.Control>
                  Exact analytical inputs and parameters
                </Accordion.Control>
                <Accordion.Panel>
                  <Stack gap="xs">
                    <Code
                      block
                      style={{
                        whiteSpace: "pre-wrap",
                        overflowWrap: "anywhere",
                      }}
                    >
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
                  </Stack>
                </Accordion.Panel>
              </Accordion.Item>
            </Accordion>
            {fresh &&
              view.data?.rows.map((r) => (
                <Card
                  key={r.node.key}
                  withBorder
                  p="sm"
                  data-testid="analytics-result"
                >
                  <Stack gap="xs">
                    <Group justify="space-between">
                      <Text
                        fw={600}
                        style={{ overflowWrap: "anywhere", minWidth: 0 }}
                      >
                        {r.node.evidence.label}
                      </Text>
                      <Badge>
                        {r.score != null
                          ? `Score ${r.score.toPrecision(6)}`
                          : `Group ${r.group}`}
                      </Badge>
                    </Group>
                    <Text size="xs" style={{ overflowWrap: "anywhere" }}>
                      {r.node.key}
                    </Text>
                    <Text size="sm">
                      {r.node.evidence.qualifications.map(label).join(" · ")}
                    </Text>
                    <Button
                      size="xs"
                      variant="light"
                      onClick={() => inspect(r.node)}
                    >
                      Inspect analytical evidence
                    </Button>
                  </Stack>
                </Card>
              ))}
            {fresh && view.data && view.data.total > 100 && (
              <Group>
                <Button disabled={!page} onClick={() => setPage(page - 100)}>
                  Previous results
                </Button>
                <Text size="sm">
                  {page + 1}–{Math.min(page + 100, view.data.total)} of{" "}
                  {view.data.total}
                </Text>
                <Button
                  disabled={page + 100 >= view.data.total}
                  onClick={() => setPage(page + 100)}
                >
                  More results
                </Button>
              </Group>
            )}
          </Stack>
        )}
      </Stack>
    </Card>
  );
}
