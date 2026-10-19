import { Alert, Button, Divider } from "@mantine/core";
import type { Brain } from "./api";
import { label, type View } from "./claimsShared";
import type { ClaimDialogData } from "./useClaimDialogData";

// Exact supporting evidence plus the review and revise entry points.
export function ClaimEvidenceActions({
  brain,
  view,
  state,
}: {
  brain: Brain;
  view: View;
  state: ClaimDialogData;
}) {
  const { knowledge, setSelectedEvidence, setReviewing, setEditing } = state;
  const r = view.revision;
  return (
    <>
      <Divider label="Exact supporting evidence" />
      {view.evidence.map((e, index) => (
        <Button
          key={`${e.kind}:${e.id}`}
          variant="default"
          h="auto"
          py="sm"
          className="claim-evidence-button"
          onClick={() => setSelectedEvidence(e)}
        >
          {e.label} · {label(e.kind)} · {label(e.availability)}
          {r.content.supports[index]?.line_from
            ? ` · lines ${r.content.supports[index].line_from}–${r.content.supports[index].line_to}`
            : ""}
        </Button>
      ))}
      <Alert color="blue">
        New input is handled by autonomous learning. See Activity → Model usage
        and learning for recent learning.
      </Alert>
      <Button variant="light" onClick={() => setReviewing(view)}>
        Review and corrections
      </Button>
      {!knowledge &&
        !view.knowledge_until &&
        r.review === "proposed" &&
        brain.role !== "reader" &&
        !brain.archived && (
          <Button variant="default" onClick={() => setEditing(true)}>
            Revise proposal
          </Button>
        )}
    </>
  );
}
