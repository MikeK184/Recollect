import type { ReactNode } from "react";
import { Button } from "../../components/ui";
import { GraphSelectionPanel } from "../../GraphSelectionPanel";
import { label } from "./graphExplorerShared";
import type { ExplorerSceneState } from "./useExplorerScene";
import type { GraphExplorationState } from "./useGraphExploration";

// The selection inspector beside the canvas: what the chosen node or
// relationship records and the hand-off to its evidence.
export function GraphExplorerSelection({
  exploration,
  scene,
  details,
}: {
  exploration: GraphExplorationState;
  scene: ExplorerSceneState;
  details: ReactNode;
}) {
  const { choice, setChoice } = exploration;
  const {
    nodes,
    edges,
    pathNodes,
    chosenNode,
    chosenEdge,
    edgeEvidence,
    focusInGraph,
    openEvidence,
    ownsSelection,
    select,
  } = scene;
  return (
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
              <p className="m-0 text-caption text-muted-foreground">
                Review: {label(chosenNode.evidence.claim.review)} · Freshness:{" "}
                {label(chosenNode.evidence.claim.freshness)} · Operational:{" "}
                {label(chosenNode.evidence.claim.operational)}
              </p>
            )}
            {chosenNode.evidence.kind !== "claim" && (
              <p className="m-0 text-caption text-muted-foreground">
                Recorded evidence in this qualified selection.
              </p>
            )}
            {!!(
              chosenNode.evidence.qualifications.length ||
              chosenNode.evidence.claim?.conflicting_claim_ids.length
            ) && (
              <p className="m-0 text-caption text-warning-text">
                {[
                  ...(chosenNode.evidence.claim?.conflicting_claim_ids.length
                    ? ["Unresolved conflict"]
                    : []),
                  ...chosenNode.evidence.qualifications.map(label),
                ]
                  .filter(
                    (value) =>
                      value !== `review ${chosenNode.evidence.claim?.review}`,
                  )
                  .filter(
                    (value, index, values) =>
                      values.findIndex(
                        (item) => item.toLowerCase() === value.toLowerCase(),
                      ) === index,
                  )
                  .join(" · ")}
              </p>
            )}
          </>
        ) : (
          <p className="m-0 text-caption text-muted-foreground">
            {chosenEdge?.family === "provenance"
              ? "Recorded evidence contribution; acceptance is separate."
              : "Recorded dependency or reference; runtime impact is not established."}
          </p>
        )
      }
      actions={
        <Button
          size="sm"
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
      details={details}
    />
  );
}
