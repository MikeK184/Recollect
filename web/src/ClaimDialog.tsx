import { Button, Loader, Stack } from "@mantine/core";
import { ActionMenu } from "./components/ActionMenu";
import { DetailInspector } from "./components/DetailInspector";
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
        className="feature-drawer"
        position="right"
        opened
        onClose={onClose}
        size="min(44rem, 90vw)"
        title="Claim and knowledge history"
      >
        <Stack>
          <Failure error={query.error ?? exactError} />
          <Button variant="light" onClick={() => void query.refetch()}>
            Try again
          </Button>
        </Stack>
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
      className="feature-drawer"
      position="right"
      opened
      onClose={onClose}
      size="min(44rem, 90vw)"
      title="Claim and knowledge history"
    >
      <Stack className="claims-panel">
        {query.isPending && <Loader />}
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
          <>
            <ActionMenu label="More memory actions">
              <EraseAction
                brain={brain.id}
                permitted={brain.role === "admin"}
                target={{ kind: "claim", id }}
                name={view?.revision.content.subject ?? "Claim history"}
              />
            </ActionMenu>
          </>
        )}
      </Stack>
    </DetailInspector>
  );
}
