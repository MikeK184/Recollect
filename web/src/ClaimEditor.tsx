import { EvidenceDialog } from "./ClaimEvidenceDialog";
import {
  ClaimApplicabilitySection,
  ClaimEvidencePicker,
  ClaimFreshnessSection,
  ClaimStatementSection,
  ClaimSupportsSection,
  ClaimValiditySection,
} from "./ClaimEditorSections";
import { Alert, Button, Modal } from "./components/ui";
import { Failure } from "./claimsShared";
import {
  useClaimEditorState,
  type ClaimEditorProps,
} from "./useClaimEditorState";

export type { ClaimEditorProps };

export function ClaimEditor(props: ClaimEditorProps) {
  const { brain, catalogue, revision, onClose, heading, submitLabel, notice } =
    props;
  const state = useClaimEditorState(props);
  const { content, save, preview, setPreview } = state;
  if (preview)
    return (
      <EvidenceDialog
        brain={brain}
        evidence={preview}
        onClose={() => setPreview(null)}
      />
    );
  const formId = `claim-editor-${revision?.id ?? "new"}`;
  return (
    <Modal
      opened
      onClose={onClose}
      size="xl"
      title={heading ?? (revision ? "Revise proposal" : "New memory")}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            form={formId}
            loading={save.isPending}
            disabled={!catalogue || !content.supports.length}
          >
            {submitLabel ?? (revision ? "Save new proposal" : "Save proposal")}
          </Button>
        </>
      }
    >
      <form
        id={formId}
        className="grid gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate();
        }}
      >
        <Alert variant="info" role="status">
          {notice ??
            "Saved as a proposal. Authorship and operational assessment do not count as human review."}
        </Alert>
        <ClaimStatementSection brain={brain} state={state} />
        <ClaimApplicabilitySection catalogue={catalogue} state={state} />
        <ClaimValiditySection state={state} />
        <ClaimFreshnessSection state={state} />
        <ClaimSupportsSection state={state} />
        <ClaimEvidencePicker state={state} />
        <Failure error={save.error} />
      </form>
    </Modal>
  );
}
