import { ChevronRight, Info } from "lucide-react";
import { Badge } from "./ui/badge";
import { cn } from "../lib/utils";

const reasons: Record<string, string> = {
  content_excluded:
    "Some source content was excluded by the Brain’s current policy.",
  support_unavailable: "Supporting evidence for some records is unavailable.",
  source_lineage_bounded:
    "Some source ancestry falls outside this request’s lookup bounds.",
  automatic_model_policy_denied:
    "Meaning search is not enabled for this Brain. Exact and text search were used.",
  automatic_model_not_selected:
    "Meaning search is not enabled for this Brain until a text model is chosen. Exact and text search were used.",
  automatic_model_credentials_missing:
    "Meaning search is unavailable. Exact and text search were used.",
  automatic_semantic_attempt_required:
    "No meaning-search attempt was submitted. Exact and text search were used.",
  automatic_semantic_profile_mismatch:
    "The meaning-search index uses a different model. Other search methods were used.",
  automatic_semantic_scope_too_large:
    "This scope is too broad for meaning search; other search methods were used.",
  automatic_semantic_scoped_coverage_missing:
    "No compatible meaning-search index is available for this scope. Other search methods were used.",
  automatic_graph_projection_missing:
    "Relationship search was unavailable for this selection; other search methods were used.",
  automatic_graph_input_unavailable:
    "Relationship search was unavailable for this selection; other search methods were used.",
  automatic_graph_scope_too_large:
    "This selection is too broad for relationship search; other search methods were used.",
  automatic_graph_read_input_too_large:
    "This selection is too broad for relationship search; other search methods were used.",
  automatic_graph_input_too_large:
    "This selection is too broad for relationship search; other search methods were used.",
  automatic_graph_input_changed:
    "Relationships changed during this request; other search methods were used.",
  automatic_graph_busy:
    "Relationship search was busy; other search methods were used.",
  semantic_scoped_coverage_missing:
    "No compatible meaning-search index is available for this scope.",
  semantic_scoped_coverage_partial:
    "Some evidence has not been indexed for meaning search yet.",
  semantic_representation_truncated:
    "Some indexed inputs were shortened; their original sources remain linked.",
  semantic_shortlist_approximate:
    "This selection is large, so meaning search ranked it with a fast approximation. Some close matches may be missing; narrow the selection for exact ranking.",
  semantic_shortlist_exhausted:
    "Fewer than 100 close meaning matches were found in this large selection. More may exist outside the approximate search.",
  semantic_candidate_limit:
    "More meaning matches may exist outside this bounded search.",
  graph_candidate_limit:
    "More connected evidence may exist outside this neighborhood.",
  source_lineage_unknown:
    "The ancestry of some evidence could not be resolved.",
  candidate_limit:
    "More matches may exist. Narrow the subject or scope to explore them.",
  result_limit: "Showing the selected results; more matches may exist.",
  context_budget: "Some matches did not fit this request’s evidence limit.",
  source_text_not_fully_indexed:
    "Some source text is still processing or is not retained.",
  source_text_unavailable:
    "Some source text is unavailable; stale text was withheld.",
  lexical_representation_limited:
    "Some large repository records are only partly indexed.",
  content_expired_during_recall:
    "Evidence that expired during this request was withheld.",
  known_ineligible_claims_excluded:
    "Records outside the selected lifecycle or eligibility were excluded.",
  raw_evidence_blocked_by_review_rule:
    "A correction or withdrawal excludes some raw evidence.",
  repository_manifest_required_for_environment:
    "Repository evidence needs an exact revision manifest for this environment.",
  manifest_snapshot_unavailable:
    "Some selected repository revisions have no available snapshot.",
  fragment_truncated:
    "Some source passages were shortened. Their original locations remain linked.",
};
export const evidenceReason = (value: string) =>
  reasons[value] ?? value.replaceAll("_", " ");

export function EvidenceNotes({
  notes,
  partial = false,
  important = [],
  className,
}: {
  notes: readonly string[];
  partial?: boolean;
  important?: readonly string[];
  className?: string;
}) {
  const importantNotes = important.filter(Boolean).map(evidenceReason);
  const unique = [
    ...new Set([
      ...importantNotes,
      ...notes.filter(Boolean).map(evidenceReason),
    ]),
  ];
  if (!unique.length && !partial) return null;
  const lead =
    unique[0] ?? "Some evidence was unavailable or did not fit this retrieval.";
  // Answer limitations are material in every language. Only recognized routine
  // retrieval metadata may be collapsed; unknown qualifications stay visible.
  const critical = new Set([
    "content_excluded",
    "support_unavailable",
    "content_expired_during_recall",
    "raw_evidence_blocked_by_review_rule",
    "source_text_unavailable",
    "manifest_snapshot_unavailable",
  ]);
  const material = unique
    .slice(1)
    .filter(
      (note) =>
        importantNotes.includes(note) ||
        notes.some(
          (raw) =>
            evidenceReason(raw) === note &&
            (!Object.hasOwn(reasons, raw) || critical.has(raw)),
        ),
    );
  const other = unique.slice(1).filter((note) => !material.includes(note));
  return (
    <aside
      className={cn(
        "grid gap-1.5 rounded-lg bg-muted px-3.5 py-3 text-label text-foreground-secondary",
        className,
      )}
      role="note"
      aria-label="Evidence notes"
    >
      <div className="flex items-center gap-2 text-caption font-medium text-foreground">
        <Info className="size-3.5 text-muted-foreground" aria-hidden="true" />
        <strong className="font-medium">Evidence notes</strong>
        {partial && (
          <Badge variant="warning" className="ml-auto">
            Partial coverage
          </Badge>
        )}
      </div>
      <p className="m-0">{lead}</p>
      {material.map((note) => (
        <p key={note} className="m-0">
          {note}
        </p>
      ))}
      {!!other.length && (
        <details className="group/notes">
          <summary className="flex w-fit cursor-pointer list-none items-center gap-1 text-caption font-medium text-brand-text [&::-webkit-details-marker]:hidden">
            <ChevronRight
              className="size-3.5 transition-transform duration-fast group-open/notes:rotate-90"
              aria-hidden="true"
            />
            Details ({other.length})
          </summary>
          <ul className="m-0 mt-1.5 grid gap-1 pl-5">
            {other.map((note) => (
              <li key={note}>{note}</li>
            ))}
          </ul>
        </details>
      )}
    </aside>
  );
}
