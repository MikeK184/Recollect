import { iconSize } from "./design/tokens";
import { useEffect, useMemo, useState, useRef } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Drawer,
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
import { Settings2, Network, TriangleAlert } from "lucide-react";
import { EmptyState } from "./components/AsyncState";
import "./features/feature-views.css";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { GraphChoice } from "./GraphCanvas";
import { useKnowledgeSelection } from "./features/knowledge/selection";
import { GraphRenderer } from "./GraphRenderer";
import { GraphSelectionPanel } from "./GraphSelectionPanel";
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
  const [choice, setGraphChoice] = useState<GraphChoice>(null);
  const [focusRequest, setFocusRequest] = useState(0);
  const setChoice = (next: GraphChoice) => {
    setGraphChoice(next);
  };
  const [showPath, setShowPath] = useState(false);
  const [expired, setExpired] = useState(false);
  const [coverageOpen, setCoverageOpen] = useState(false);
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
    setCoverageOpen(false);
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
  const { select, available } = useKnowledgeSelection();
  const ownsSelection = useRef(false);
  const focusInGraph = () => {
    ownsSelection.current = false;
    setFocusRequest((value) => value + 1);
    select(null);
  };
  const details = useRef<React.ReactNode>(null);
  function openEvidence(node: Node) {
    if (available && node.evidence.kind === "claim") {
      ownsSelection.current = true;
      select(
        {
          kind: "graph-node",
          key: node.key,
          claimId: node.evidence.id,
          revision: node.evidence.revision_id,
        },
        details.current,
      );
    } else {
      select(null);
      inspect(node);
    }
  }
  details.current = (
    <>
      {" "}
      {chosenNode && (
        <Card
          withBorder
          p="sm"

          style={{ overflowWrap: "anywhere" }}
        >
          <Stack gap="xs">
            {!available && (
              <>
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
              </>
            )}
            {witness && (
              <Text size="sm">
                Shortest witness: {witness.edges.length} hops ·{" "}
                {witness.nodes
                  .map((key) => nodeMap.get(key)?.evidence.label ?? key)
                  .join(" → ")}
              </Text>
            )}
            <Group>
              <Button
                size="xs"
                variant="default"
                disabled={!!pathNodes.length}
                onClick={focusInGraph}
              >
                Focus in graph
              </Button>
              <Button
                size="xs"
                onClick={() => {
                  openEvidence(chosenNode);
                }}
              >
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

          style={{ overflowWrap: "anywhere" }}
        >
          <Stack gap="xs">
            <Title order={5}>{label(chosenEdge.relation)} relationship</Title>
            <Text size="sm">
              Recorded direction: {nodeMap.get(chosenEdge.from)?.evidence.label}{" "}
              → {nodeMap.get(chosenEdge.to)?.evidence.label}
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
              <Button
                size="xs"
                variant="default"
                disabled={!!pathNodes.length}
                onClick={focusInGraph}
              >
                Focus in graph
              </Button>
              {edgeEvidence && (
                <Button
                  size="xs"
                  onClick={() => {
                    openEvidence(edgeEvidence);
                  }}
                >
                  Inspect relationship evidence
                </Button>
              )}
              <Button
                size="xs"
                variant="light"
                onClick={() => setChoice({ kind: "node", id: chosenEdge.from })}
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
    </>
  );
  useEffect(() => {
    if (!data && ownsSelection.current) {
      ownsSelection.current = false;
      select(null);
    }
  }, [data, select]);
  useEffect(() => {
    if (
      choice &&
      (!data ||
        (choice.kind === "node"
          ? !nodeMap.has(choice.id)
          : !edges.some((edge) => edge.id === choice.id)))
    )
      setChoice(null);
  }, [data, nodeMap, edges, choice]);
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
  const canvasTools = (
    <>
      <Button
        variant="default"
        leftSection={<Settings2 size={iconSize.small} />}
        onClick={() => setControlsOpened(true)}
      >
        Explore
      </Button>
      {path?.status === "path" && data && (
        <Button
          variant="light"
          onClick={() => {
            setShowPath((value) => !value);
            setChoice(null);
          }}
        >
          {inPath ? "Return to exploration" : "Show shortest path on canvas"}
        </Button>
      )}
    </>
  );
  return (
    <section
      className="graph-explorer"
      aria-label="Interactive graph exploration"
    >
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
          An input reached its retention deadline. Load the current graph again.
        </Alert>
      )}
      {data && nodes.length === 0 && (
        <EmptyState
          icon={Network}
          title="No eligible entities"
          description="This selection has no graph evidence to display. Adjust the filters or add permitted evidence to this Brain."
        />
      )}
      {/* When the bounded read has no data to display (an error such as an
          ineligible center, or a retention deadline), the canvas chrome is
          unmounted with it. Keep the exploration entry point reachable so a
          failed read can be corrected and reloaded from its new location. */}
      {!read.isFetching && !data && (
        <div
          className="graph-chrome graph-explorer-recover"
          data-graph-chrome=""
        >
          <Button
            variant="default"
            leftSection={<Settings2 size={iconSize.small} />}
            onClick={() => setControlsOpened(true)}
          >
            Explore
          </Button>
        </div>
      )}
      {data && nodes.length > 0 && (
        <>
          <div className="graph-stage-area">
            <GraphRenderer
              key={JSON.stringify([
                brain.id,
                view.scope,
                request.center,
                request.direction,
                request.max_hops,
                inPath,
              ])}
              tools={canvasTools}
              inspector={
                choice && (chosenNode || chosenEdge) ? (
                  <GraphSelectionPanel
                    nodes={nodes}
                    edges={edges}
                    choice={choice}
                    onChoose={setChoice}
                    onFocus={focusInGraph}
                    focusEnabled={!pathNodes.length}
                    description={
                      chosenNode ? (
                        <>
                          {chosenNode.evidence.claim && (
                            <Text size="xs" c="dimmed">
                              Review: {label(chosenNode.evidence.claim.review)}{" "}
                              · Freshness:{" "}
                              {label(chosenNode.evidence.claim.freshness)} ·
                              Operational:{" "}
                              {label(chosenNode.evidence.claim.operational)}
                            </Text>
                          )}
                          {chosenNode.evidence.kind !== "claim" && (
                            <Text size="xs" c="dimmed">
                              Recorded evidence in this qualified selection.
                            </Text>
                          )}
                          {!!(
                            chosenNode.evidence.qualifications.length ||
                            chosenNode.evidence.claim?.conflicting_claim_ids
                              .length
                          ) && (
                            <Text size="xs" c="orange.9">
                              {[
                                ...(chosenNode.evidence.claim
                                  ?.conflicting_claim_ids.length
                                  ? ["Unresolved conflict"]
                                  : []),
                                ...chosenNode.evidence.qualifications.map(
                                  label,
                                ),
                              ]
                                .filter(
                                  (value) =>
                                    value !==
                                    `review ${chosenNode.evidence.claim?.review}`,
                                )
                                .filter(
                                  (value, index, values) =>
                                    values.findIndex(
                                      (item) =>
                                        item.toLowerCase() ===
                                        value.toLowerCase(),
                                    ) === index,
                                )
                                .join(" · ")}
                            </Text>
                          )}
                        </>
                      ) : (
                        <Text size="xs" c="dimmed">
                          {chosenEdge!.family === "provenance"
                            ? "Recorded evidence contribution; acceptance is separate."
                            : "Recorded dependency or reference; runtime impact is not established."}
                        </Text>
                      )
                    }
                    actions={
                      <Button
                        size="xs"
                        disabled={
                          !!chosenNode &&
                          chosenNode.evidence.kind !== "claim" &&
                          !chosenNode.evidence.provenance.length
                        }
                        onClick={() => {
                          if (chosenNode) openEvidence(chosenNode);
                          else if (edgeEvidence) openEvidence(edgeEvidence);
                          else if (chosenEdge) {
                            ownsSelection.current = true;
                            select({
                              kind: "graph-edge",
                              relation: chosenEdge.relation,
                              evidenceKind: chosenEdge.evidence_kind,
                              evidenceId: chosenEdge.evidence_id,
                            });
                          }
                        }}
                      >
                        Open evidence
                      </Button>
                    }
                    details={details.current}
                  />
                ) : null
              }
              nodes={nodes}
              edges={edges}
              choice={choice}
              focusRequest={focusRequest}
              onChoose={setChoice}
              pathNodes={pathNodes}
              pathEdges={pathEdges}
            />
          </div>
          {data.center && nodes.length === 1 && (
            <Text size="sm">
              No other eligible entity is reachable within this direction and
              hop bound.
            </Text>
          )}
          <div
            className="graph-chrome graph-explorer-status"
            data-graph-chrome=""
          >
            <Text size="xs" c="dimmed" data-testid="exploration-counts">
              {nodes.length} loaded entities · {edges.length} directed
              relationships
            </Text>
            <Badge variant="light">
              {inPath
                ? "Shortest eligible path"
                : data.center
                  ? "Bounded reachability"
                  : "Eligible overview"}
            </Badge>
            {shownView!.coverage.partial && (
              <span className="graph-coverage">
                <button
                  type="button"
                  className="graph-coverage-toggle"
                  aria-expanded={coverageOpen}
                  onClick={() => setCoverageOpen((value) => !value)}
                >
                  <TriangleAlert size={iconSize.small} />
                  Coverage limits
                </button>
                {coverageOpen && (
                  <span className="graph-coverage-reasons">
                    {shownView!.coverage.reasons.map(label).join(" · ")}
                  </span>
                )}
              </span>
            )}
            <Text size="xs" c="dimmed" className="graph-display-note">
              Blue repository · amber memory · sage source; recorded edges
              describe this evidence only. Limit: 500 entities, 2,000
              relationships.
            </Text>
          </div>
        </>
      )}
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
    </section>
  );
}
