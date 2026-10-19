import type { BrainSearch } from "../../app/useBrainSearch";
import type { components } from "../../api-schema";

export type Scope = components["schemas"]["GraphSelection"];
export type GraphNodeView = components["schemas"]["GraphNode"];
export type GraphEvidence = components["schemas"]["ClaimEvidenceChoice"];
export type SubmittedRead = {
  nonce: string;
  scope: Scope;
  offset: number;
};
export type SubmittedPath = {
  nonce: string;
  request: components["schemas"]["GraphPathRequest"];
};
export type GraphDetail = {
  claim?: string;
  evidence?: GraphEvidence;
};
export type GraphDrawer =
  "filters" | "path" | "insights" | "status" | "entities" | null;

export const label = (s: string) => s.replaceAll("_", " ");
export const evidenceModes = [
  { value: "investigation", label: "Investigation with qualifications" },
  { value: "strict_accepted", label: "Accepted claims only" },
  { value: "strict_operational", label: "Verified operational claims only" },
];
export function localTime(value: string | null | undefined) {
  if (!value) return "";
  const time = new Date(value);
  time.setMinutes(time.getMinutes() - time.getTimezoneOffset());
  return time.toISOString().slice(0, 16);
}
export const initial = (): Scope => ({
  kind: "knowledge",
  snapshot_id: null,
  selection: { repository_ids: [], area_ids: [], environment_id: null },
  operation_id: null,
  collection_id: null,
  manifest_revision_id: null,
  fact_at: null,
  mode: "investigation",
  relations: [],
});
export function scopeFromRoute(route: BrainSearch): Scope {
  return {
    ...initial(),
    kind: ["knowledge", "repository", "combined"].includes(route.kind ?? "")
      ? route.kind!
      : "knowledge",
    snapshot_id: route.snapshot ?? null,
    manifest_revision_id: route.manifest ?? null,
    fact_at: route.fact ?? null,
    collection_id: route.collection ?? null,
    mode: route.mode ?? "investigation",
    selection: {
      repository_ids:
        route.repositories ?? (route.repository ? [route.repository] : []),
      area_ids: route.areas ?? (route.area ? [route.area] : []),
      environment_id: route.environment ?? null,
    },
    relations: route.relations ?? [],
  };
}
export function missingExactSelection(scope: Scope) {
  return (
    (scope.kind === "repository" &&
      (!scope.selection.repository_ids?.length ||
        (!scope.snapshot_id && !scope.manifest_revision_id))) ||
    (scope.kind === "combined" &&
      (!scope.selection.environment_id || !scope.manifest_revision_id))
  );
}
export const meanings: Record<string, string> = {
  supported_by:
    "The claim cites this evidence; citation does not establish acceptance.",
  contributed_to:
    "This claim revision is a required contribution to the handover.",
  terraform_module:
    "The parsed module source names this exact repository commit and directory. This is a declared dependency.",
  declares:
    "Static extraction records a declaration from this entity to the target.",
  calls: "Static extraction records a call from the source to the target.",
  names: "The source names the target; this is a reference, not a call.",
};
// The detail dialog target for an inspected node, or null when the node has
// no claim and no provenance to inspect.
export function detailForNode(n: GraphNodeView): GraphDetail | null {
  if (n.evidence.kind === "claim") return { claim: n.evidence.id };
  const p = n.evidence.provenance[0];
  if (!p) return null;
  return {
    evidence: {
      kind: p.kind,
      id: p.id,
      label: n.evidence.label,
      source_id: p.source_id,
      repository_id: p.repository_id,
      snapshot_id: p.snapshot_id,
      revision: p.revision,
      reference: null,
      observed_at: null,
      created_at: n.evidence.recorded_at,
      availability: p.availability,
    },
  };
}
