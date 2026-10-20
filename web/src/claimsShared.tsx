import { FileText } from "lucide-react";
import type { components } from "./api-schema";
import {
  Alert,
  Badge,
  Button,
  StatusBadge,
  type StatusState,
} from "./components/ui";
import type { Kind } from "./design/tokens";
export type Content = components["schemas"]["ClaimContent"];
export type Revision = components["schemas"]["ClaimRevision"];
export type View = components["schemas"]["ClaimView"];
export type Evidence = components["schemas"]["ClaimEvidenceChoice"];
export type Catalogue = components["schemas"]["WorkspaceCatalogue"];
export const label = (s: string) => s.replaceAll("_", " ");
export const time = (s: string) => new Date(s).toLocaleString();
export const utc = (s?: string | null) =>
  s ? s.replace(/Z$/, "").slice(0, 19) : "";
export const instant = (s: string) =>
  s ? new Date(`${s}Z`).toISOString() : null;
// Memory kinds carry their category accent; the badge text keeps the meaning.
export const kindBadgeColor: Record<string, string> = {
  claim: "blue",
  decision: "yellow",
  procedure: "violet",
  handover: "cyan",
};
const memoryKindChips: Record<string, { kind: Kind; label: string }> = {
  claim: { kind: "claim", label: "Claim" },
  decision: { kind: "decision", label: "Decision" },
  procedure: { kind: "procedure", label: "Procedure" },
  handover: { kind: "handover", label: "Handover" },
};
/** The category chip for a memory kind; unknown kinds read as claims. */
export const memoryKindChip = (kind: string) =>
  memoryKindChips[kind] ?? { kind: "claim" as Kind, label: label(kind) };
/** Status tone for each separate dimension; the word always stays visible. */
export const reviewTone = (review: string): StatusState =>
  review === "accepted"
    ? "positive"
    : review === "rejected"
      ? "negative"
      : "neutral";
export const freshnessTone = (freshness: string): StatusState =>
  freshness === "current"
    ? "positive"
    : freshness === "needs_verification"
      ? "attention"
      : "neutral";
export const operationalTone = (operational: string): StatusState =>
  operational === "verified" ? "positive" : "neutral";
export const freshOptions = ["current", "needs_verification", "superseded"].map(
  (value) => ({ value, label: label(value) }),
);
export const opOptions = [
  "declared",
  "implemented",
  "deployed",
  "verified",
].map((value) => ({ value, label: label(value) }));
export function Failure({ error }: { error: Error | null }) {
  return error ? (
    <Alert variant="destructive" title="Request failed">
      {error.message}
    </Alert>
  ) : null;
}
export function Pages({
  offset,
  total,
  nextOffset,
  onChange,
}: {
  offset: number;
  total: number;
  nextOffset?: number | null;
  onChange: (value: number) => void;
}) {
  return total > 20 ? (
    <div className="flex items-center justify-between gap-3">
      <Button
        variant="secondary"
        size="sm"
        disabled={!offset}
        onClick={() => onChange(Math.max(0, offset - 20))}
      >
        Previous page
      </Button>
      <span className="mono text-caption text-muted-foreground">
        {offset + 1}–{Math.min(offset + 20, total)} of {total}
      </span>
      <Button
        variant="secondary"
        size="sm"
        disabled={
          nextOffset === null ||
          (nextOffset === undefined && offset + 20 >= total)
        }
        onClick={() => onChange(nextOffset ?? offset + 20)}
      >
        Next page
      </Button>
    </div>
  ) : null;
}
export function States({ view }: { view: View }) {
  return (
    <div className="flex flex-wrap gap-1.5">
      <StatusBadge state={reviewTone(view.revision.review)}>
        Review: {label(view.revision.review)}
      </StatusBadge>
      <StatusBadge state={freshnessTone(view.eligibility.effective_freshness)}>
        Freshness: {label(view.eligibility.effective_freshness)}
      </StatusBadge>
      <Badge variant="outline">
        Assessment: {label(view.revision.content.operational)}
      </Badge>
      {view.revision.lifecycle === "withdrawn" && (
        <StatusBadge state="attention">Withdrawn</StatusBadge>
      )}
      {!!view.eligibility.rule_ids?.length && (
        <StatusBadge state="negative">Blocked by review rule</StatusBadge>
      )}
      {!!view.eligibility.conflicting_claim_ids?.length && (
        <StatusBadge state="attention">Unresolved conflict</StatusBadge>
      )}
    </div>
  );
}
/** One composite status mark for dense list rows. The three dimensions keep
 * their separate order and meaning inside the text, so no distinction relies
 * on color alone; the full dimension set stays in the inspector. */
export function CompositeStatus({ view }: { view: View }) {
  const flags: string[] = [];
  if (view.revision.lifecycle === "withdrawn") flags.push("Withdrawn");
  if (view.eligibility.rule_ids?.length) flags.push("Blocked by review rule");
  if (view.eligibility.conflicting_claim_ids?.length)
    flags.push("Unresolved conflict");
  const words = [
    label(view.revision.review),
    ...(view.eligibility.effective_freshness === "current"
      ? []
      : [label(view.eligibility.effective_freshness)]),
    ...flags,
  ].join(" · ");
  const text = words.charAt(0).toUpperCase() + words.slice(1);
  const full = `Review: ${label(view.revision.review)} · Freshness: ${label(view.eligibility.effective_freshness)} · Operational: ${label(view.revision.content.operational)}`;
  const state: StatusState = flags.length
    ? "attention"
    : view.eligibility.effective_freshness === "needs_verification"
      ? "attention"
      : view.revision.review === "rejected"
        ? "negative"
        : view.revision.review === "accepted" &&
            view.eligibility.effective_freshness === "current"
          ? "positive"
          : "neutral";
  return (
    <div
      className="memory-record-status relative flex min-w-0 items-center"
      title={full}
    >
      <span aria-hidden="true" className="flex min-w-0">
        <StatusBadge state={state} className="max-w-full">
          <span className="truncate">{text}</span>
        </StatusBadge>
      </span>
      {/* Visually hidden (clipped, not overflow-hidden) full dimension text. */}
      <span className="absolute -m-px size-px overflow-clip whitespace-nowrap border-0 p-0 [clip-path:inset(50%)]">
        {full}
      </span>
    </div>
  );
}
/**
 * One exact supporting evidence item as a small card. Its accessible name is
 * the full "label · kind · availability · lines" line the card displays.
 */
export function EvidenceButton({
  evidence,
  support,
  onOpen,
  className,
}: {
  evidence: Evidence;
  support?: Content["supports"][number];
  onOpen: () => void;
  className?: string;
}) {
  const lines = support?.line_from
    ? ` · lines ${support.line_from}–${support.line_to}`
    : "";
  return (
    <button
      type="button"
      aria-label={`${evidence.label} · ${label(evidence.kind)} · ${label(evidence.availability)}${lines}`}
      className={
        "grid w-full gap-1 rounded-md border border-border bg-card px-3 py-2.5 text-left transition-[border-color,box-shadow,transform] duration-fast ease-standard hover:-translate-y-px hover:border-border-strong hover:shadow-sm focus-visible:outline-2 focus-visible:outline-ring " +
        (className ?? "")
      }
      onClick={onOpen}
    >
      <span className="flex min-w-0 items-center gap-1.5 text-label font-medium text-foreground">
        <FileText
          aria-hidden="true"
          className="size-3.5 shrink-0 text-muted-foreground"
        />
        <span className="min-w-0 [overflow-wrap:anywhere]">
          {evidence.label}
        </span>
      </span>
      <span className="mono text-caption text-muted-foreground">
        {label(evidence.kind)} · {label(evidence.availability)}
        {lines}
      </span>
    </button>
  );
}
export const emptyContent = (): Content => ({
  kind: "claim",
  subject: "",
  predicate: "",
  value: "",
  rationale: "",
  selection: { repository_ids: [], area_ids: [], environment_id: null },
  manifest_revision_id: null,
  validity: { kind: "unknown", from: null, to: null, precision: "unknown" },
  freshness: "current",
  operational: "declared",
  observed_at: null,
  observation: "",
  supports: [],
});
