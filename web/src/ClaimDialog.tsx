import { ActionMenu } from "./components/ActionMenu";
import { DetailInspector } from "./components/DetailInspector";
import { InspectorSection } from "./components/Inspector";
import { Button } from "./components/ui";
import { ClaimEditor } from "./ClaimEditor";
import { ClaimDetailBody, ClaimHistorySection } from "./ClaimDetailBody";
import { EvidenceDialog } from "./ClaimEvidenceDialog";
import { ClaimEvidenceActions } from "./ClaimEvidenceActions";
import { ClaimReviewDialog } from "./ClaimReviewDialog";
import { Failure } from "./claimsShared";
import { EraseAction } from "./RetentionPanel";
import {
  useClaimDialogData,
  type ClaimDialogProps,
} from "./useClaimDialogData";

export type { ClaimDialogProps };

function Loading() {
  return (
    <p
      role="status"
      className="m-0 flex items-center gap-2 text-label text-muted-foreground"
    >
      <span
        aria-hidden="true"
        className="size-3.5 animate-spin rounded-full border-2 border-border-strong border-t-transparent"
      />
      Loading memory…
    </p>
  );
}

/**
 * A memory's full record and knowledge history in the shared inspector. Its
 * drill-ins (editor, review, exact evidence, a contributing memory) replace
 * it while open, so only one record surface is ever shown.
 */
export function ClaimDialog(props: ClaimDialogProps) {
  const { brain, id, catalogue, onKnowledgeChange, onClose, onSaved } = props;
  const state = useClaimDialogData(props);
  const {
    query,
    exactError,
    view,
    selectedEvidence,
    setSelectedEvidence,
    reviewing,
    setReviewing,
    editing,
    setEditing,
    contribution,
    setContribution,
  } = state;
  if (query.isError || exactError)
    return (
      <DetailInspector
        opened
        onClose={onClose}
        title="Claim and knowledge history"
      >
        <div className="grid gap-3">
          <Failure error={query.error ?? exactError} />
          <Button
            variant="secondary"
            className="w-fit"
            onClick={() => void query.refetch()}
          >
            Try again
          </Button>
        </div>
      </DetailInspector>
    );
  if (view && contribution)
    return (
      <ClaimDialog
        key={contribution.id + contribution.at}
        brain={brain}
        id={contribution.id}
        knowledgeAt={contribution.at}
        catalogue={catalogue}
        onClose={() => setContribution(null)}
        onSaved={onSaved}
      />
    );
  if (view && editing)
    return (
      <ClaimEditor
        brain={brain}
        catalogue={catalogue}
        revision={view.revision}
        evidence={view.evidence}
        onClose={() => setEditing(false)}
        onSaved={() => {
          setEditing(false);
          void query.refetch();
          onSaved();
        }}
      />
    );
  if (view && selectedEvidence)
    return (
      <EvidenceDialog
        brain={brain}
        evidence={selectedEvidence}
        onClose={() => setSelectedEvidence(null)}
      />
    );
  if (view && reviewing)
    return (
      <ClaimReviewDialog
        brain={brain}
        view={reviewing}
        catalogue={catalogue}
        current={reviewing.revision.id === query.data?.current_revision}
        Editor={ClaimEditor}
        EvidenceViewer={EvidenceDialog}
        onClose={() => setReviewing(null)}
        onSaved={() => {
          void query.refetch();
          onSaved();
        }}
      />
    );
  return (
    <DetailInspector
      opened
      onClose={onClose}
      title="Claim and knowledge history"
    >
      <div className="grid min-w-0">
        {query.isPending && (
          <InspectorSection>
            <Loading />
          </InspectorSection>
        )}
        {query.data && (
          <ClaimHistorySection
            data={query.data}
            view={view}
            state={state}
            onKnowledgeChange={onKnowledgeChange}
          />
        )}
        {view && (
          <>
            <ClaimDetailBody view={view} state={state} catalogue={catalogue} />
            <ClaimEvidenceActions brain={brain} view={view} state={state} />
          </>
        )}
        {query.data && (
          <InspectorSection>
            <ActionMenu label="More memory actions">
              <EraseAction
                brain={brain.id}
                permitted={brain.role === "admin"}
                target={{ kind: "claim", id }}
                name={view?.revision.content.subject ?? "Claim history"}
              />
            </ActionMenu>
          </InspectorSection>
        )}
      </div>
    </DetailInspector>
  );
}
