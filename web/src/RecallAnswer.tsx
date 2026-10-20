import { Check, Copy } from "lucide-react";
import type { components } from "./api-schema";
import type { Brain } from "./api";
import { EvidenceNotes } from "./components/EvidenceNotes";
import { Alert, Button } from "./components/ui";
import type { useEvidenceList } from "./catalogueQueries";
import {
  RecallDetails,
  RecallResults,
  RecallSelectionSummary,
} from "./RecallResults";
import { label } from "./recallFilters";

type Request = components["schemas"]["RecallRequest"];
type Item = components["schemas"]["RecallItem"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type Answer = components["schemas"]["RecallResponse"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];
type Groups = NonNullable<ReturnType<typeof useEvidenceList>["data"]>;

function RecallCoverageDetails({ answer }: { answer: Answer }) {
  return (
    <RecallDetails summary="Coverage and recorded methods">
      {answer.semantic && (
        <p className="m-0">
          Semantic coverage: {answer.semantic.scoped_entries.toLocaleString()}{" "}
          scoped representations ·{" "}
          {answer.semantic.profile?.model ?? "No index available"}.{" "}
          {answer.semantic.model_request_id
            ? "One query embedding request recorded."
            : "No query model call."}
        </p>
      )}
      <p className="m-0">
        Source coverage: {answer.context_selection.distinct_source_groups}{" "}
        resolved groups
        {answer.context_selection.unknown_lineage_items > 0
          ? ` · ${answer.context_selection.unknown_lineage_items} results with unknown ancestry`
          : ""}
        .
        {answer.context_selection.source_diversity
          ? " Coverage preference enabled."
          : " Rank-only selection."}{" "}
        Source groups do not establish independent corroboration.
      </p>
      {answer.graph && (
        <p className="m-0 text-label text-foreground" data-testid="recall-graph-status">
          Graph: {label(answer.graph.state)} · {answer.graph.anchors.length}{" "}
          anchors · {answer.graph.candidates} connected candidates
          {answer.graph.view.coverage.partial ? " · partial graph coverage" : ""}
          .
        </p>
      )}
    </RecallDetails>
  );
}

export function RecallAnswer({
  brain,
  answer,
  request,
  nonce,
  catalogue,
  groups,
  copied,
  copyError,
  setCopied,
  setCopyError,
  onClaim,
  onEvidence,
  onGraph,
}: {
  brain: Brain;
  answer: Answer;
  request: Request;
  nonce: string;
  catalogue?: Catalogue;
  groups?: Groups;
  copied: boolean;
  copyError: boolean;
  setCopied: (value: boolean) => void;
  setCopyError: (value: boolean) => void;
  onClaim: (id: string) => void;
  onEvidence: (evidence: Evidence) => void;
  onGraph: (item: Item) => void;
}) {
  return (
    <div role="group" aria-label="Recall results" className="grid gap-4 animate-enter">
      <div className="flex flex-wrap items-center gap-3">
        <p className="m-0 text-body text-foreground">
          <span className="mono">{answer.context.items.length}</span> result
          {answer.context.items.length === 1 ? "" : "s"}
          <span className="text-muted-foreground">
            {" · "}
            {Array.from(
              new Set(answer.context.items.flatMap((item) => item.channels)),
            )
              .map((channel) =>
                channel === "lexical"
                  ? "text"
                  : channel === "semantic"
                    ? "meaning"
                    : channel === "graph"
                      ? "relationships"
                      : channel,
              )
              .join(" + ") || "No matches"}
          </span>
        </p>
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto"
          onClick={() => {
            setCopyError(false);
            void Promise.resolve()
              .then(() =>
                navigator.clipboard.writeText(JSON.stringify(answer.context)),
              )
              .then(() => setCopied(true))
              .catch(() => {
                setCopied(false);
                setCopyError(true);
              });
          }}
        >
          {copied ? (
            <Check aria-hidden="true" className="text-brand-text" />
          ) : (
            <Copy aria-hidden="true" />
          )}
          {copied ? "Context copied" : "Copy attributed context"}
        </Button>
      </div>
      {copyError && (
        <Alert variant="destructive">
          Context could not be copied. Check clipboard access and try again.
        </Alert>
      )}
      <div className="grid gap-2">
        <RecallDetails summary="Scope and retrieval details">
          <RecallSelectionSummary
            answer={answer}
            request={request}
            brainName={brain.name}
            catalogue={catalogue}
            collectionName={
              groups?.groups.find((g) => g.id === request.collection_id)?.name
            }
          />
        </RecallDetails>
        <RecallCoverageDetails answer={answer} />
      </div>
      <EvidenceNotes
        notes={answer.coverage.reasons}
        partial={answer.coverage.partial}
      />
      {!answer.context.items.length && (
        <Alert
          role="status"
          title={
            answer.status === "no_match"
              ? "No matching evidence"
              : "Insufficient eligible evidence"
          }
        >
          {answer.status === "no_match"
            ? "Try a more specific subject, another phrase or an exact identifier."
            : "No result fits scope/trust/budget. Check qualifications, widen scope, or add evidence and let autonomous learning process it."}
        </Alert>
      )}
      <RecallResults
        key={nonce}
        answer={answer}
        request={request}
        catalogue={catalogue}
        onClaim={onClaim}
        onEvidence={onEvidence}
        onGraph={onGraph}
      />
    </div>
  );
}
