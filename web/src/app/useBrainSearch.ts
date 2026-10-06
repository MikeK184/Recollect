import { useNavigate, useRouterState } from "@tanstack/react-router";

// Only opaque resource IDs and explicit, non-sensitive display criteria belong
// in shareable URLs. Questions, free-text searches and provider inputs do not.
export type BrainSearch = {
  tab?: string;
  view?: "memory" | "sources" | "repositories";
  perspective?: "provenance";
  detail?: "record";
  job?: string;
  call?: string;
  erasure?: string;
  device?: string;
  source?: string;
  version?: string;
  claim?: string;
  revision?: string;
  snapshot?: string;
  manifest?: string;
  repository?: string;
  repositories?: string[];
  collection?: string;
  area?: string;
  areas?: string[];
  environment?: string;
  kind?: string;
  mode?: string;
  knowledge?: string;
  fact?: string;
  center?: string;
  direction?: string;
  hops?: number;
  relations?: string[];
};
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const ids = [
  "job",
  "call",
  "erasure",
  "device",
  "source",
  "version",
  "claim",
  "revision",
  "snapshot",
  "manifest",
  "repository",
  "collection",
  "area",
  "environment",
] as const;
const kinds = [
  "all",
  "claim",
  "decision",
  "procedure",
  "handover",
  "knowledge",
  "repository",
  "combined",
];
const modes = [
  "investigation",
  "strict_accepted",
  "strict_operational",
  "history",
];

export function validateBrainSearch(raw: Record<string, unknown>): BrainSearch {
  const parsed: BrainSearch = {};
  if (raw.perspective === "provenance") parsed.perspective = "provenance";
  if (["memory", "sources", "repositories"].includes(String(raw.view)))
    parsed.view = raw.view as BrainSearch["view"];
  if (raw.detail === "record") parsed.detail = "record";
  for (const key of ids)
    if (typeof raw[key] === "string" && uuid.test(raw[key]))
      parsed[key] = raw[key];
  for (const key of ["repositories", "areas"] as const) {
    const values = raw[key];
    if (Array.isArray(values)) {
      const valid = [
        ...new Set(
          values.filter(
            (v): v is string => typeof v === "string" && uuid.test(v),
          ),
        ),
      ].slice(0, 50);
      if (valid.length) parsed[key] = valid;
    }
  }
  if (Array.isArray(raw.relations)) {
    const valid = [
      ...new Set(
        raw.relations.filter(
          (v): v is string => typeof v === "string" && /^[a-z_]{1,60}$/.test(v),
        ),
      ),
    ].slice(0, 30);
    if (valid.length) parsed.relations = valid;
  }
  if (typeof raw.tab === "string" && /^[a-z-]{1,24}$/.test(raw.tab))
    parsed.tab = raw.tab;
  if (typeof raw.kind === "string" && kinds.includes(raw.kind))
    parsed.kind = raw.kind;
  if (typeof raw.mode === "string" && modes.includes(raw.mode))
    parsed.mode = raw.mode;
  for (const key of ["knowledge", "fact"] as const) {
    const value = raw[key];
    if (
      typeof value === "string" &&
      /^\d{4}-\d{2}-\d{2}T[\d:.]+(?:Z|[+-]\d{2}:\d{2})$/.test(value) &&
      value.length <= 40 &&
      !Number.isNaN(Date.parse(value))
    )
      // Keep the server's microseconds: JS Date normalization would round the
      // cutoff down and could select the revision immediately before it.
      parsed[key] = value;
  }
  // Graph entity keys are server-generated evidence-kind:UUID identifiers.
  if (
    typeof raw.center === "string" &&
    /^[a-z_]+:[0-9a-f-]{36}$/i.test(raw.center) &&
    uuid.test(raw.center.slice(raw.center.indexOf(":") + 1))
  )
    parsed.center = raw.center;
  if (
    typeof raw.direction === "string" &&
    ["incoming", "outgoing", "both"].includes(raw.direction)
  )
    parsed.direction = raw.direction;
  const hops =
    typeof raw.hops === "number"
      ? raw.hops
      : typeof raw.hops === "string" && /^\d+$/.test(raw.hops)
        ? Number(raw.hops)
        : undefined;
  if (hops !== undefined && Number.isInteger(hops) && hops >= 1 && hops <= 8)
    parsed.hops = hops;
  return parsed;
}

export type BrainSearchPatch = {
  [K in keyof BrainSearch]?: BrainSearch[K] | null;
};
export function useBrainSearch() {
  const search = useRouterState({
    select: (state) =>
      validateBrainSearch(state.location.search as Record<string, unknown>),
  });
  const navigate = useNavigate();
  function patch(values: BrainSearchPatch) {
    void navigate({
      to: ".",
      search: (previous) => validateBrainSearch({ ...previous, ...values }),
      resetScroll: false,
    });
  }
  return [search, patch] as const;
}
