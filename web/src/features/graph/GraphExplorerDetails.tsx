import { Button, Card, Group, Stack, Text, Title } from "@mantine/core";
import { label } from "./graphExplorerShared";
import type { ExplorerSceneState } from "./useExplorerScene";
import type { GraphExplorationState } from "./useGraphExploration";

// The node and relationship cards shown in the knowledge inspector. The
// explorer stores this element in `details.current` so `select` still receives
// it as its second argument.
export function GraphExplorerDetails({
  exploration,
  scene,
  onStart,
  onEnd,
}: {
  exploration: GraphExplorationState;
  scene: ExplorerSceneState;
  onStart: (key: string) => void;
  onEnd: (key: string) => void;
}) {
  const { setCenter, load, setChoice } = exploration;
  const {
    available,
    chosenNode,
    chosenEdge,
    witness,
    nodeMap,
    pathNodes,
    edgeEvidence,
    focusInGraph,
    openEvidence,
  } = scene;
  return (
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
}
