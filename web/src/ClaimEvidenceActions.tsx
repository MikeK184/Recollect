import { PencilLine, ScanSearch } from "lucide-react";
import type { Brain } from "./api";
import { InspectorSection } from "./components/Inspector";
import { Alert, Button } from "./components/ui";
import { EvidenceButton, type View } from "./claimsShared";
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
      <InspectorSection
        label={`Exact supporting evidence · ${view.evidence.length}`}
      >
        {view.evidence.map((e, index) => (
          <EvidenceButton
            key={`${e.kind}:${e.id}`}
            className="claim-evidence-button"
            evidence={e}
            support={r.content.supports[index]}
            onOpen={() => setSelectedEvidence(e)}
          />
        ))}
        {!view.evidence.length && (
          <p className="m-0 text-label text-muted-foreground">
            No supporting evidence is recorded for this revision.
          </p>
        )}
      </InspectorSection>
      <InspectorSection label="Review">
        <Alert variant="info" role="status">
          New input is handled by autonomous learning. See Activity → Model
          usage and learning for recent learning.
        </Alert>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setReviewing(view)}
          >
            <ScanSearch aria-hidden="true" />
            Review and corrections
          </Button>
          {!knowledge &&
            !view.knowledge_until &&
            r.review === "proposed" &&
            brain.role !== "reader" &&
            !brain.archived && (
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setEditing(true)}
              >
                <PencilLine aria-hidden="true" />
                Revise proposal
              </Button>
            )}
        </div>
      </InspectorSection>
    </>
  );
}
