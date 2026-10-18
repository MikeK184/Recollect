import type { components } from "../../api-schema";
import type { BrainSearch } from "../../app/useBrainSearch";

export function organizationScope(
  route: BrainSearch,
): components["schemas"]["GraphSelection"] {
  return {
    kind: "knowledge",
    snapshot_id: null,
    operation_id: null,
    manifest_revision_id: route.manifest ?? null,
    collection_id: route.collection ?? null,
    fact_at: route.fact ?? null,
    mode: route.mode ?? "investigation",
    relations: [],
    selection: {
      repository_ids:
        route.repositories ?? (route.repository ? [route.repository] : []),
      area_ids: route.areas ?? (route.area ? [route.area] : []),
      environment_id: route.environment ?? null,
    },
  };
}
