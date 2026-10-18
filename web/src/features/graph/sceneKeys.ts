// Query roots whose invalidation can change what a loaded graph scene shows:
// the graph reads themselves and the memory, evidence and scope catalogues they
// are built from. Unrelated invalidations (audit, jobs, MCP, models, skills…)
// must not discard a scene the user is inspecting.
const sceneRoots = new Set([
  "graph",
  "graph-read",
  "graph-path",
  "graph-explore",
  "graph-entities",
  "graph-generation-status",
  "graph-snapshots",
  "graph-manifests",
  "claims",
  "claim",
  "evidence",
  "workspace",
  "revision-manifests",
  "revision-manifest",
  "repository-snapshots",
]);

export const feedsGraphScene = (queryKey: readonly unknown[]) =>
  sceneRoots.has(String(queryKey[0]));
