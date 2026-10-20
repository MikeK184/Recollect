import { Button } from "../../components/ui";
import { label } from "./graphExplorerShared";
import type { ExplorerSceneState } from "./useExplorerScene";
import type { GraphExplorationState } from "./useGraphExploration";

// The node and relationship details shown under "Details & actions" in the
// selection inspector, and in the knowledge inspector. The explorer stores
// this element in `details.current` so `select` still receives it as its
// second argument.
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
      {chosenNode && (
        <div className="grid gap-2.5 [overflow-wrap:anywhere]">
          {!available && (
            <>
              <h4 className="m-0 text-title font-semibold">
                {chosenNode.evidence.label}
              </h4>
              <p className="m-0 text-body">
                {label(chosenNode.evidence.kind)} ·{" "}
                <span className="mono text-caption">{chosenNode.key}</span>
              </p>
              {chosenNode.evidence.claim && (
                <p className="m-0 text-body">
                  {label(chosenNode.evidence.claim.review)} ·{" "}
                  {label(chosenNode.evidence.claim.freshness)} ·{" "}
                  {label(chosenNode.evidence.claim.operational)}
                </p>
              )}
            </>
          )}
          {witness && (
            <p className="m-0 text-body text-foreground-secondary">
              Shortest witness:{" "}
              <span className="mono">{witness.edges.length} hops</span> ·{" "}
              {witness.nodes
                .map((key) => nodeMap.get(key)?.evidence.label ?? key)
                .join(" → ")}
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="secondary"
              disabled={!!pathNodes.length}
              onClick={focusInGraph}
            >
              Focus in graph
            </Button>
            <Button size="sm" onClick={() => openEvidence(chosenNode)}>
              Inspect selected evidence
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => {
                setCenter(chosenNode.key);
                load(chosenNode.key);
              }}
            >
              Explore connections
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                onStart(chosenNode.key);
                setChoice(null);
              }}
            >
              Set path start
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                onEnd(chosenNode.key);
                setChoice(null);
              }}
            >
              Set path end
            </Button>
          </div>
        </div>
      )}
      {chosenEdge && (
        <div className="grid gap-2.5 [overflow-wrap:anywhere]">
          <h4 className="m-0 text-title font-semibold">
            {label(chosenEdge.relation)} relationship
          </h4>
          <p className="m-0 text-body">
            Recorded direction: {nodeMap.get(chosenEdge.from)?.evidence.label}{" "}
            → {nodeMap.get(chosenEdge.to)?.evidence.label}
          </p>
          <p className="m-0 text-body text-foreground-secondary">
            {label(chosenEdge.family)} ·{" "}
            {chosenEdge.family === "provenance"
              ? "This relationship records an evidence contribution; it does not establish acceptance."
              : "This relationship records a static dependency or reference; runtime impact is not established."}
          </p>
          <p className="m-0 text-caption text-muted-foreground">
            Evidence: {label(chosenEdge.evidence_kind)} ·{" "}
            <span className="mono">{chosenEdge.evidence_id}</span> ·
            relationship ordinal{" "}
            <span className="mono">{chosenEdge.evidence_ordinal}</span>
          </p>
          <p className="m-0 text-caption text-muted-foreground">
            Relationship <span className="mono">{chosenEdge.id}</span>
          </p>
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="secondary"
              disabled={!!pathNodes.length}
              onClick={focusInGraph}
            >
              Focus in graph
            </Button>
            {edgeEvidence && (
              <Button size="sm" onClick={() => openEvidence(edgeEvidence)}>
                Inspect relationship evidence
              </Button>
            )}
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setChoice({ kind: "node", id: chosenEdge.from })}
            >
              Inspect source entity
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setChoice({ kind: "node", id: chosenEdge.to })}
            >
              Inspect target entity
            </Button>
          </div>
        </div>
      )}
    </>
  );
}
