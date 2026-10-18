import type { components } from "./api-schema";

type Generation = components["schemas"]["GraphGeneration"];
type Status = components["schemas"]["GraphStatus"];
export type SceneGenerations = Pick<
  components["schemas"]["GraphView"],
  | "generation"
  | "inputs"
  | "memory_epoch"
  | "state"
  | "scope"
  | "total_nodes"
  | "total_edges"
  | "coverage"
  | "link_issues"
  | "link_issues_total"
>;

export function sceneGenerationIds(scene: SceneGenerations): string[] {
  return [
    ...new Set([scene.generation, ...scene.inputs].map((g) => g.id)),
  ].sort();
}

function newer(candidate: Generation, loaded: Generation) {
  const next = Date.parse(candidate.created_at),
    previous = Date.parse(loaded.created_at);
  if (!Number.isFinite(next) || !Number.isFinite(previous)) return false;
  if (next !== previous) return next > previous;
  // Keep PostgreSQL's sub-millisecond ordering before its UUID tie-breaker.
  const fraction = (value: string) =>
    (value.match(/\.(\d+)(?:Z|[+-])/)?.[1] ?? "").padEnd(9, "0");
  const a = fraction(candidate.created_at),
    b = fraction(loaded.created_at);
  return a !== b ? a > b : candidate.id > loaded.id;
}

function sameBinding(a: Generation, b: Generation) {
  return (
    a.brain_id === b.brain_id &&
    a.kind === b.kind &&
    (a.snapshot_id ?? null) === (b.snapshot_id ?? null) &&
    a.input_epoch === b.input_epoch &&
    a.adapter === b.adapter &&
    JSON.stringify(a.input_snapshot_ids ?? []) ===
      JSON.stringify(b.input_snapshot_ids ?? [])
  );
}

/** The exact tracking response must account for every requested immutable ID. */
export function verifySceneStatus(
  scene: SceneGenerations,
  status: Status,
): void {
  const loaded = new Map(
    [scene.generation, ...scene.inputs].map((g) => [g.id, g]),
  );
  const seen = new Set<string>();
  const invalid = () =>
    new Error("The graph inputs could not be verified. Load the graph again.");
  if (
    !Array.isArray(status.tracked_generations) ||
    !Array.isArray(status.missing_tracked_generation_ids)
  )
    throw invalid();
  const states = new Set([
    "queued",
    "running",
    "ready",
    "superseded",
    "removed",
    "failed",
    "cancelled",
  ]);
  for (const row of status.tracked_generations) {
    const expected = loaded.get(row?.id);
    if (
      !expected ||
      seen.has(row.id) ||
      !sameBinding(row, expected) ||
      !states.has(row.state)
    )
      throw invalid();
    seen.add(row.id);
  }
  for (const id of status.missing_tracked_generation_ids) {
    if (!loaded.has(id) || seen.has(id)) throw invalid();
    seen.add(id);
  }
  if (seen.size !== loaded.size) throw invalid();
}

export function invalidatesScene(
  scene: SceneGenerations,
  status: Status,
): boolean {
  if (
    status.memory_epoch > scene.memory_epoch ||
    (scene.generation.kind === "combined" &&
      status.link_epoch > scene.generation.input_epoch)
  )
    return true;
  const ids = new Set(sceneGenerationIds(scene));
  if (status.missing_tracked_generation_ids?.some((id) => ids.has(id)))
    return true;
  // Exact tracked rows are read after the inventory and take precedence over
  // its possibly older observation. Inventory absence never proves deletion.
  const observed = new Map(status.generations.map((g) => [g.id, g]));
  for (const generation of status.tracked_generations ?? [])
    observed.set(generation.id, generation);
  return [scene.generation, ...scene.inputs].some((loaded) => {
    const current = observed.get(loaded.id);
    if (
      current?.brain_id === loaded.brain_id &&
      ["superseded", "removed", "failed", "cancelled"].includes(current.state)
    )
      return true;
    return [...observed.values()].some(
      (candidate) =>
        candidate.id !== loaded.id &&
        candidate.state === "ready" &&
        sameBinding(candidate, loaded) &&
        newer(candidate, loaded),
    );
  });
}
