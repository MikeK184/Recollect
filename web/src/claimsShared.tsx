import { Alert, Badge, Button, Group, Text } from "@mantine/core";
import type { components } from "./api-schema";
import { StatusBadge } from "./components/StatusBadge";
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
    <Alert color="red" title="Request failed">
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
    <Group justify="space-between">
      <Button
        variant="subtle"
        disabled={!offset}
        onClick={() => onChange(Math.max(0, offset - 20))}
      >
        Previous page
      </Button>
      <Text size="xs">
        {offset + 1}–{Math.min(offset + 20, total)} of {total}
      </Text>
      <Button
        variant="subtle"
        disabled={
          nextOffset === null ||
          (nextOffset === undefined && offset + 20 >= total)
        }
        onClick={() => onChange(nextOffset ?? offset + 20)}
      >
        Next page
      </Button>
    </Group>
  ) : null;
}
export function States({ view }: { view: View }) {
  return (
    <Group gap="xs">
      <Badge color={view.revision.review === "accepted" ? "teal" : "gray"}>
        Review: {label(view.revision.review)}
      </Badge>
      <Badge
        color={
          view.eligibility.effective_freshness === "current" ? "blue" : "yellow"
        }
      >
        Freshness: {label(view.eligibility.effective_freshness)}
      </Badge>
      <Badge variant="outline">
        Assessment: {label(view.revision.content.operational)}
      </Badge>
      {view.revision.lifecycle === "withdrawn" && (
        <Badge color="orange">Withdrawn</Badge>
      )}
      {!!view.eligibility.rule_ids?.length && (
        <Badge color="red">Blocked by review rule</Badge>
      )}
      {!!view.eligibility.conflicting_claim_ids?.length && (
        <Badge color="yellow">Unresolved conflict</Badge>
      )}
    </Group>
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
  const text = [
    label(view.revision.review),
    ...(view.eligibility.effective_freshness === "current"
      ? []
      : [label(view.eligibility.effective_freshness)]),
    ...flags,
  ].join(" · ");
  const full = `Review: ${label(view.revision.review)} · Freshness: ${label(view.eligibility.effective_freshness)} · Operational: ${label(view.revision.content.operational)}`;
  const state = flags.length
    ? "attention"
    : view.revision.review === "accepted" &&
        view.eligibility.effective_freshness === "current"
      ? "positive"
      : "neutral";
  return (
    <div className="memory-record-status" aria-label={full} title={full}>
      <StatusBadge state={state}>{text}</StatusBadge>
    </div>
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
