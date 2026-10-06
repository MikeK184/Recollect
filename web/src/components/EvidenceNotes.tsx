import { Info } from "lucide-react";
import "./evidence-notes.css";

const reasons: Record<string, string> = {
  content_excluded:
    "Some source content was excluded by the Brain’s current policy.",
  support_unavailable: "Supporting evidence for some records is unavailable.",
  source_lineage_bounded:
    "Some source ancestry falls outside this request’s lookup bounds.",
  automatic_model_policy_denied:
    "Meaning search is not enabled for this Brain. Exact and text search were used.",
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
}: {
  notes: readonly string[];
  partial?: boolean;
  important?: readonly string[];
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
    <aside className="evidence-notes" role="note" aria-label="Evidence notes">
      <div className="evidence-notes-heading">
        <Info size={15} />
        <strong>Evidence notes</strong>
        {partial && <span>Partial coverage</span>}
      </div>
      <p>{lead}</p>
      {material.map((note) => (
        <p key={note}>{note}</p>
      ))}
      {!!other.length && (
        <details>
          <summary>Details ({other.length})</summary>
          <ul>
            {other.map((note) => (
              <li key={note}>{note}</li>
            ))}
          </ul>
        </details>
      )}
    </aside>
  );
}
