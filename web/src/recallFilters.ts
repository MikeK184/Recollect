import type { components } from "./api-schema";

export type RecallRequestBody = components["schemas"]["RecallRequest"];
export type Filters = {
  query: string;
  mode: string;
  exactKind: string;
  exactId: string;
  repositories: string[];
  areas: string[];
  environment: string | null;
  manifest: string | null;
  collection: string | null;
  knowledge: string;
  fact: string;
  channels: string[];
  strategy: "auto" | "manual";
  budget: number;
  limit: number;
  similarity: number;
  graphKind: string;
  graphDirection: string;
  graphHops: number;
  graphRelations: string;
  sourceDiversity: boolean;
};
export const label = (s: string) => s.replaceAll("_", " ");
export const initial = (): Filters => ({
  query: "",
  mode: "investigation",
  exactKind: "",
  exactId: "",
  repositories: [],
  areas: [],
  environment: null,
  manifest: null,
  collection: null,
  knowledge: "",
  fact: "",
  channels: ["exact", "lexical"],
  strategy: "auto",
  budget: 8,
  limit: 10,
  similarity: 0,
  graphKind: "knowledge",
  graphDirection: "both",
  graphHops: 2,
  graphRelations: "",
  sourceDiversity: true,
});
const instant = (s: string) => (s ? new Date(`${s}Z`).toISOString() : null);

export function buildRecallRequest(
  filters: Filters,
  queryText: string,
): RecallRequestBody {
  return {
    project_brief: false,
    continuation: null,
    query: queryText,
    exact: filters.exactKind
      ? { kind: filters.exactKind, id: filters.exactId }
      : null,
    operation_id: null,
    selection: {
      repository_ids: filters.repositories,
      area_ids: filters.areas,
      environment_id: filters.environment,
    },
    collection_id: filters.collection,
    manifest_revision_id: filters.manifest,
    knowledge_at: instant(filters.knowledge),
    fact_at: instant(filters.fact),
    mode: filters.mode,
    channels:
      filters.strategy === "auto" ? ["exact", "lexical"] : filters.channels,
    strategy: filters.strategy,
    semantic_request_id:
      filters.strategy === "auto" || filters.channels.includes("semantic")
        ? crypto.randomUUID()
        : null,
    semantic_min_similarity:
      filters.strategy === "manual" && filters.channels.includes("semantic")
        ? filters.similarity
        : null,
    semantic_exact: false,
    graph:
      filters.strategy === "manual" && filters.channels.includes("graph")
        ? {
            kind: filters.graphKind,
            direction: filters.graphDirection,
            max_hops: filters.graphHops,
            relations: filters.graphRelations
              .split(",")
              .map((s) => s.trim())
              .filter(Boolean),
          }
        : null,
    source_diversity: filters.sourceDiversity,
    limit: filters.limit,
    context_bytes: filters.budget * 1024,
  } satisfies RecallRequestBody;
}

export function recallSearchDisabled(filters: Filters, queryText: string) {
  return (
    (filters.strategy === "manual" &&
      filters.channels.includes("graph") &&
      (filters.channels.length === 1 ||
        filters.mode === "history" ||
        !!filters.knowledge ||
        (filters.graphKind === "repository" &&
          filters.repositories.length !== 1) ||
        (filters.graphKind === "combined" &&
          (!filters.environment || !filters.manifest)))) ||
    (filters.strategy === "manual" &&
      filters.channels.includes("semantic") &&
      !queryText.trim()) ||
    (!queryText.trim() && !filters.exactKind) ||
    (filters.strategy === "manual" && !filters.channels.length) ||
    (!!filters.exactKind &&
      !/^[0-9a-f]{8}-[0-9a-f-]{27}$/i.test(filters.exactId))
  );
}
