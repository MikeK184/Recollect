import type { Brain } from "./api";
import type { components } from "./api-schema";
import { ClaimDialog, EvidenceDialog } from "./ClaimsPanel";
import { RecallGraphDialog } from "./RecallGraphDialog";

type Request = components["schemas"]["RecallRequest"];
type Item = components["schemas"]["RecallItem"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type Answer = components["schemas"]["RecallResponse"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];

export type RecallDetail = {
  claim?: string;
  evidence?: Evidence;
  graph?: Item;
};

export function RecallDetailDialogs({
  brain,
  detail,
  answer,
  request,
  catalogue,
  setDetail,
  onInvalidate,
  onClaimSaved,
}: {
  brain: Brain;
  detail: RecallDetail | null;
  answer?: Answer;
  request?: Request;
  catalogue?: Catalogue;
  setDetail: (detail: RecallDetail | null) => void;
  onInvalidate: () => void;
  onClaimSaved: () => void;
}) {
  return (
    <>
      {detail?.claim && (
        <ClaimDialog
          key={`${detail.claim}:${answer?.knowledge_at}`}
          brain={brain}
          id={detail.claim}
          catalogue={catalogue}
          knowledgeAt={answer?.knowledge_at}
          factAt={request?.fact_at ?? undefined}
          onClose={() => setDetail(null)}
          onSaved={onClaimSaved}
        />
      )}
      {detail?.graph && answer && request && (
        <RecallGraphDialog
          key={detail.graph.revision_id}
          brain={brain}
          item={detail.graph}
          answer={answer}
          request={request}
          catalogue={catalogue}
          onClose={() => setDetail(null)}
          onInvalidated={onInvalidate}
          onClaim={(id) => setDetail({ claim: id })}
          onEvidence={(e) => setDetail({ evidence: e })}
        />
      )}
      {detail?.evidence && (
        <EvidenceDialog
          brain={brain}
          evidence={detail.evidence}
          returnLabel="Back to recall"
          eraseSource
          onClose={() => setDetail(null)}
        />
      )}
    </>
  );
}
