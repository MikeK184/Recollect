import { iconSize } from "./design/tokens";
import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import {
  Accordion,
  Alert,
  Badge,
  Button,
  Card,
  Drawer,
  SegmentedControl,
  Group,
  Loader,
  NumberInput,
  Select,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { Search, Settings2, Network } from "lucide-react";
import { EmptyState } from "./components/AsyncState";
import "./features/feature-views.css";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { GraphChoice } from "./GraphCanvas";
const GraphCanvas = lazy(() => import("./GraphCanvas"));
type Node = components["schemas"]["GraphNode"];
type View = components["schemas"]["GraphView"];
type Path = components["schemas"]["GraphPath"];
const label = (s: string) => s.replaceAll("_", " ");

export function GraphExplorer({
  brain,
  view,
  path,
  inspect,
  onStart,
  onEnd,
  onExpired,
  initialCenter = "",
  initialDirection = "outgoing",
  initialHops = 2,
  compactControls = false,
  expectedMemoryEpoch,
  onExplore,
}: {
  brain: Brain;
  view: View;
  path: Path | null;
  inspect: (node: Node) => void;
  onStart: (key: string) => void;
  onEnd: (key: string) => void;
  onExpired: () => void;
  initialCenter?: string;
  initialDirection?: string;
  initialHops?: number;
  compactControls?: boolean;
  expectedMemoryEpoch?: number;
  onExplore?: (center: string | null, direction: string, hops: number) => void;
}) {
  const [center, setCenter] = useState(initialCenter);
  const [controlsOpened, setControlsOpened] = useState(false);
  const [representation, setRepresentation] = useState("canvas");
  const [entitySearch, setEntitySearch] = useState("");
  const [direction, setDirection] = useState(initialDirection);
  const [hops, setHops] = useState(initialHops);
  const [request, setRequest] = useState({
    nonce: crypto.randomUUID(),
    center: initialCenter || null,
    direction: initialDirection,
    max_hops: initialHops,
  });
  useEffect(() => {
    if (
      request.center === (initialCenter || null) &&
      request.direction === initialDirection &&
      request.max_hops === initialHops
    )
      return;
    setCenter(initialCenter);
    setDirection(initialDirection);
    setHops(initialHops);
    setExpired(false);
    setRequest({
      nonce: crypto.randomUUID(),
      center: initialCenter || null,
      direction: initialDirection,
      max_hops: initialHops,
    });
  }, [initialCenter, initialDirection, initialHops]);
  const [choice, setChoice] = useState<GraphChoice>(null);
  const [showPath, setShowPath] = useState(false);
  const [expired, setExpired] = useState(false);
  const read = useQuery({
    queryKey: [
      "graph-explore",
      brain.id,
      view.scope,
      view.generation.id,
      request,
    ],
    retry: false,
    gcTime: 0,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/explore", {
          params: { path: { brain: brain.id } },
          body: {
            scope: view.scope,
            center: request.center,
            direction: request.direction,
            max_hops: request.max_hops,
          },
          signal,
        }),
      ),
  });
  useEffect(() => {
    setChoice(null);
    setShowPath(false);
  }, [request, path]);
  useEffect(() => {
    const deadline = read.data?.expires_at;
    if (!deadline) return;
    let timer: ReturnType<typeof setTimeout>;
    function check() {
      const remaining = new Date(deadline!).getTime() - Date.now();
      if (remaining <= 0) {
        setExpired(true);
        setChoice(null);
        onExpired();
      } else timer = setTimeout(check, Math.min(remaining + 1, 2_147_483_647));
    }
    check();
    return () => clearTimeout(timer);
  }, [read.data, onExpired]);
  function load(from: string | null) {
    onExplore?.(from, direction, hops);
    setChoice(null);
    setShowPath(false);
    setExpired(false);
    setRequest({
      nonce: crypto.randomUUID(),
      center: from,
      direction,
      max_hops: hops,
    });
  }
  const changed =
    expectedMemoryEpoch !== undefined &&
    read.data !== undefined &&
    read.data.view.memory_epoch !== expectedMemoryEpoch;
  useEffect(() => {
    if (changed) onExpired();
  }, [changed, onExpired]);
  const data = !read.isError && !expired && !changed ? read.data : undefined;
  const inPath = showPath && path?.status === "path";
  const shownView = inPath ? path.view : data?.view;
  const matchingPath =
    path?.view.generation.id === data?.view.generation.id &&
    path?.view.memory_epoch === data?.view.memory_epoch;
  const nodes = (inPath ? path.nodes : data?.nodes) ?? [];
  const edges = (inPath ? path.edges : data?.edges) ?? [];
  const nodeMap = useMemo(() => new Map(nodes.map((n) => [n.key, n])), [nodes]);
  const pathNodes = useMemo(
    () => (inPath || matchingPath ? (path?.nodes.map((n) => n.key) ?? []) : []),
    [path, inPath, matchingPath],
  );
  const pathEdges = useMemo(
    () => (inPath || matchingPath ? (path?.edges.map((e) => e.id) ?? []) : []),
    [path, inPath, matchingPath],
  );
  const chosenNode =
    choice?.kind === "node" ? nodeMap.get(choice.id) : undefined;
  const chosenEdge =
    choice?.kind === "edge" ? edges.find((e) => e.id === choice.id) : undefined;
  const witness =
    !inPath && chosenNode && data?.reach.find((r) => r.key === chosenNode.key);
  const edgeEvidence =
    chosenEdge &&
    nodes.find(
      (n) =>
        n.evidence.id === chosenEdge.evidence_id ||
        n.evidence.revision_id === chosenEdge.evidence_id,
    );
  const settings = (
    <SimpleGrid cols={{ base: 1, sm: 2 }}>
      <TextInput
        label="Exploration center"
        description="An exact entity key; select a node below to fill it."
        value={center}
        onChange={(e) => setCenter(e.currentTarget.value)}
      />
      <Select
        label="Exploration direction"
        value={direction}
        data={["outgoing", "incoming", "both"]}
        onChange={(value) => setDirection(value ?? "outgoing")}
      />
      <NumberInput
        label="Exploration hops"
        min={1}
        max={8}
        value={hops}
        onChange={(value) => setHops(Number(value) || 1)}
      />
      <Select
        label="Center from current entity page"
        placeholder="Choose an eligible entity"
        searchable
        clearable
        value={view.nodes.some((n) => n.key === center) ? center : null}
        data={view.nodes.map((n) => ({
          value: n.key,
          label: `${n.evidence.label} · ${n.evidence.kind}`,
        }))}
        onChange={(value) => setCenter(value ?? "")}
      />
    </SimpleGrid>
  );
  const listedNodes = nodes.filter((node) =>
    `${node.evidence.label} ${node.evidence.kind} ${node.key}`
      .toLocaleLowerCase()
      .includes(entitySearch.toLocaleLowerCase()),
  );
  return (
    <section
      className="graph-explorer"
      aria-label="Interactive graph exploration"
    >
      <Stack gap="md">
        <div className="graph-explorer-toolbar">
          <Select
            className="graph-entity-search"
            label="Inspect graph entity"
            placeholder="Find an entity…"
            leftSection={<Search size={iconSize.small} />}
            searchable
            clearable
            limit={50}
            value={choice?.kind === "node" ? choice.id : null}
            data={nodes.map((node) => ({
              value: node.key,
              label: `${node.evidence.label} · ${label(node.evidence.kind)}`,
            }))}
            onChange={(id) => setChoice(id ? { kind: "node", id } : null)}
          />
          <SegmentedControl
            aria-label="Graph presentation"
            value={representation}
            onChange={setRepresentation}
            data={[
              { value: "canvas", label: "Canvas" },
              { value: "list", label: "List" },
            ]}
          />
          <Button
            variant="default"
            leftSection={<Settings2 size={iconSize.small} />}
            onClick={() => setControlsOpened(true)}
          >
            Explore from an entity
          </Button>
          {path?.status === "path" && data && (
            <Button
              variant="light"
              onClick={() => {
                setShowPath((value) => !value);
                setChoice(null);
              }}
            >
              {inPath
                ? "Return to exploration"
                : "Show shortest path on canvas"}
            </Button>
          )}
        </div>
        {read.isFetching && (
          <Group role="status">
            <Loader size="sm" />
            <Text>Reading the qualified graph…</Text>
          </Group>
        )}
        {read.error && (
          <Alert color="red" title="Graph could not be displayed">
            {read.error.message}
          </Alert>
        )}
        {expired && (
          <Alert color="yellow">
            An input reached its retention deadline. Load the current graph
            again.
          </Alert>
        )}
        {data && (
          <>
            <Group justify="space-between">
              <Text size="xs" c="dimmed" data-testid="exploration-counts">
                {nodes.length} displayed entities · {edges.length} directed
                relationships
              </Text>
              <Badge variant="light">
                {inPath
                  ? "Shortest eligible path"
                  : data.center
                    ? "Bounded reachability"
                    : "Eligible overview"}
              </Badge>
            </Group>
            {shownView!.coverage.partial && (
              <Alert color="yellow" title="Coverage limits">
                {shownView!.coverage.reasons.map(label).join(" · ")}
              </Alert>
            )}
            {nodes.length === 0 ? (
              <EmptyState
                icon={Network}
                title="No eligible entities"
                description="This selection has no graph evidence to display. Adjust the filters or add permitted evidence to this Brain."
              />
            ) : (
              <>
                {data.center && nodes.length === 1 && (
                  <Text size="sm">
                    No other eligible entity is reachable within this direction
                    and hop bound.
                  </Text>
                )}
                {representation === "canvas" ? (
                  <Suspense
                    fallback={
                      <Group role="status">
                        <Loader size="sm" />
                        <Text>Loading graph renderer…</Text>
                      </Group>
                    }
                  >
                    <GraphCanvas
                      nodes={nodes}
                      edges={edges}
                      choice={choice}
                      onChoose={setChoice}
                      pathNodes={pathNodes}
                      pathEdges={pathEdges}
                    />
                  </Suspense>
                ) : (
                  <>
                    <TextInput
                      label="Filter displayed entities"
                      placeholder="Search the displayed graph…"
                      value={entitySearch}
                      onChange={(event) =>
                        setEntitySearch(event.currentTarget.value)
                      }
                      leftSection={<Search size={iconSize.small} />}
                    />
                    <Text size="xs" c="dimmed">
                      {listedNodes.length} matches in the {nodes.length}{" "}
                      displayed entities. Select an item to inspect its evidence
                      and relationships.
                    </Text>
                    <div className="graph-node-list">
                      {listedNodes.map((node) => (
                        <button
                          className="graph-node-button"
                          key={node.key}
                          onClick={() =>
                            setChoice({ kind: "node", id: node.key })
                          }
                        >
                          <strong>{node.evidence.label}</strong>
                          <span className="feature-meta">
                            {label(node.evidence.kind)}
                          </span>
                          <span className="feature-meta">
                            {node.evidence.qualifications
                              .map(label)
                              .join(" · ")}
                          </span>
                        </button>
                      ))}
                    </div>
                  </>
                )}
                <Select
                  label="Inspect graph relationship"
                  placeholder="Search directed relationships"
                  searchable
                  clearable
                  limit={50}
                  value={choice?.kind === "edge" ? choice.id : null}
                  data={edges.map((edge) => ({
                    value: edge.id,
                    label: `${nodeMap.get(edge.from)?.evidence.label} → ${nodeMap.get(edge.to)?.evidence.label} · ${label(edge.relation)}`,
                  }))}
                  onChange={(id) => setChoice(id ? { kind: "edge", id } : null)}
                />
              </>
            )}
            <Text size="xs" c="dimmed">
              Recorded relationships describe this evidence; they do not
              establish runtime impact or confirm a claim. Display limit: 500
              entities and 2,000 relationships.
            </Text>
          </>
        )}
      </Stack>
      <Drawer
        className="feature-drawer"
        opened={controlsOpened}
        onClose={() => setControlsOpened(false)}
        position="right"
        title="Explore relationships"
        size="lg"
      >
        <Stack>
          <Text size="sm" c="dimmed">
            Choose an exact entity and a bounded traversal. Scope and
            eligibility stay the same as the current graph.
          </Text>
          {settings}
          <Button
            onClick={() => {
              load(center);
              setControlsOpened(false);
            }}
            disabled={!center || read.isFetching}
          >
            Explore from center
          </Button>
          <Button
            variant="default"
            onClick={() => {
              load(null);
              setControlsOpened(false);
            }}
            disabled={read.isFetching}
          >
            Show eligible overview
          </Button>
          {data && (
            <Text size="xs" className="feature-meta">
              Generation {shownView!.generation.id} · {label(shownView!.state)}{" "}
              · {shownView!.total_nodes} eligible entities in the full selection
            </Text>
          )}
        </Stack>
      </Drawer>
      <Drawer
        className="feature-drawer"
        opened={!!choice && !!data}
        onClose={() => setChoice(null)}
        position="right"
        title={chosenNode ? "Graph entity" : "Graph relationship"}
        size="md"
      >
        {chosenNode && (
          <Card
            withBorder
            p="sm"
            data-testid="exploration-node"
            style={{ overflowWrap: "anywhere" }}
          >
            <Stack gap="xs">
              <Title order={5}>{chosenNode.evidence.label}</Title>
              <Text size="sm">
                {label(chosenNode.evidence.kind)} · {chosenNode.key}
              </Text>
              {chosenNode.evidence.claim && (
                <Text size="sm">
                  {label(chosenNode.evidence.claim.review)} ·{" "}
                  {label(chosenNode.evidence.claim.freshness)} ·{" "}
                  {label(chosenNode.evidence.claim.operational)}
                </Text>
              )}
              {chosenNode.evidence.provenance.map((p, i) => (
                <Text key={i} size="xs">
                  {label(p.kind)} · {p.id}
                  {p.path ? ` · ${p.path}` : ""}
                  {p.revision ? ` · commit ${p.revision}` : ""}
                  {p.snapshot_id ? ` · snapshot ${p.snapshot_id}` : ""}
                </Text>
              ))}
              <Text size="xs">
                {chosenNode.evidence.qualifications.map(label).join(" · ")}
              </Text>
              {witness && (
                <Text size="sm">
                  Shortest witness: {witness.edges.length} hops ·{" "}
                  {witness.nodes
                    .map((key) => nodeMap.get(key)?.evidence.label ?? key)
                    .join(" → ")}
                </Text>
              )}
              <Group>
                <Button size="xs" onClick={() => inspect(chosenNode)}>
                  Inspect selected evidence
                </Button>
                <Button
                  size="xs"
                  variant="light"
                  onClick={() => {
                    setCenter(chosenNode.key);
                    load(chosenNode.key);
                  }}
                >
                  Explore connections
                </Button>
                <Button
                  size="xs"
                  variant="subtle"
                  onClick={() => {
                    onStart(chosenNode.key);
                    setChoice(null);
                  }}
                >
                  Set path start
                </Button>
                <Button
                  size="xs"
                  variant="subtle"
                  onClick={() => {
                    onEnd(chosenNode.key);
                    setChoice(null);
                  }}
                >
                  Set path end
                </Button>
              </Group>
            </Stack>
          </Card>
        )}
        {chosenEdge && (
          <Card
            withBorder
            p="sm"
            data-testid="exploration-edge"
            style={{ overflowWrap: "anywhere" }}
          >
            <Stack gap="xs">
              <Title order={5}>{label(chosenEdge.relation)} relationship</Title>
              <Text size="sm">
                Recorded direction:{" "}
                {nodeMap.get(chosenEdge.from)?.evidence.label} →{" "}
                {nodeMap.get(chosenEdge.to)?.evidence.label}
              </Text>
              <Text size="sm">
                {label(chosenEdge.family)} ·{" "}
                {chosenEdge.family === "provenance"
                  ? "This relationship records an evidence contribution; it does not establish acceptance."
                  : "This relationship records a static dependency or reference; runtime impact is not established."}
              </Text>
              <Text size="xs">
                Evidence: {label(chosenEdge.evidence_kind)} ·{" "}
                {chosenEdge.evidence_id} · relationship ordinal{" "}
                {chosenEdge.evidence_ordinal}
              </Text>
              <Text size="xs">Relationship {chosenEdge.id}</Text>
              <Group>
                {edgeEvidence && (
                  <Button size="xs" onClick={() => inspect(edgeEvidence)}>
                    Inspect relationship evidence
                  </Button>
                )}
                <Button
                  size="xs"
                  variant="light"
                  onClick={() =>
                    setChoice({ kind: "node", id: chosenEdge.from })
                  }
                >
                  Inspect source entity
                </Button>
                <Button
                  size="xs"
                  variant="light"
                  onClick={() => setChoice({ kind: "node", id: chosenEdge.to })}
                >
                  Inspect target entity
                </Button>
              </Group>
            </Stack>
          </Card>
        )}
      </Drawer>
    </section>
  );
}
