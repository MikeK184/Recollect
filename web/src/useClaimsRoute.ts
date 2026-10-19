import { useEffect, useState } from "react";
import { useDebouncedValue } from "@mantine/hooks";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { useBrainSearch } from "./app/useBrainSearch";
import { useStableRows } from "./components/useStableRows";
import { useWorkspaceCatalogue } from "./catalogueQueries";
import { utc } from "./claimsShared";
import { usePollInterval } from "./usePollInterval";

export type ClaimsPanelProps = {
  brain: Brain;
  kind?: string | null;
  onKindChange?: (value: string | null) => void;
  selectedId?: string | null;
  onSelectedIdChange?: (value: string | null) => void;
};

// The memory list's route-backed filters, search, paging, catalogue and claims
// queries, and the dialogs it can open. The panel calls this unconditionally.
export function useClaimsRoute({
  brain,
  kind,
  onKindChange,
  selectedId,
  onSelectedIdChange,
}: ClaimsPanelProps) {
  const pollNormal = usePollInterval("normal");
  const cache = useQueryClient();
  const [route, patchRoute] = useBrainSearch();
  const [offset, setOffset] = useState(0);
  const mode = route.mode ?? "investigation";
  const setMode = (mode: string | null) => patchRoute({ mode });
  const internalKind = ["claim", "decision", "procedure", "handover"].includes(
    route.kind ?? "",
  )
    ? route.kind!
    : null;
  const setInternalKind = (kind: string | null) => patchRoute({ kind });
  const memoryKind = kind === undefined ? internalKind : kind;
  const environment = route.environment ?? null;
  const setEnvironment = (environment: string | null) =>
    patchRoute({ environment });
  const repository = route.repository ?? null;
  const setRepository = (repository: string | null) =>
    patchRoute({ repository });
  const [search, setSearch] = useState("");
  const [query] = useDebouncedValue(search.trim(), 300);
  const [filtering, setFiltering] = useState(false);
  const [factDraft, setFactDraft] = useState(utc(route.fact));
  const [knowledgeDraft, setKnowledgeDraft] = useState(utc(route.knowledge));
  const times = { fact_at: route.fact, knowledge_at: route.knowledge };
  const setTimes = (times: { fact_at?: string; knowledge_at?: string }) =>
    patchRoute({
      fact: times.fact_at ?? null,
      knowledge: times.knowledge_at ?? null,
      revision: null,
    });
  useEffect(() => {
    setFactDraft(utc(route.fact));
    setKnowledgeDraft(utc(route.knowledge));
  }, [route.fact, route.knowledge]);
  useEffect(
    () => setOffset(0),
    [mode, memoryKind, environment, repository, route.fact, route.knowledge],
  );
  const [noting, setNoting] = useState(false);
  const [structuring, setStructuring] = useState(false);
  const selected =
    selectedId === undefined ? (route.claim ?? null) : selectedId;
  const setSelected = (value: string | null) => {
    patchRoute({ claim: value, revision: null, detail: null });
    onSelectedIdChange?.(value);
  };
  const catalogue = useWorkspaceCatalogue(brain.id, undefined, {
    enabled: filtering || noting || structuring || route.detail === "record",
    poll: "normal",
  });
  const claims = useQuery({
    queryKey: [
      "claims",
      brain.id,
      memoryKind,
      mode,
      environment,
      repository,
      times,
      query,
      offset,
    ],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/claims", {
          signal,
          params: {
            path: { brain: brain.id },
            query: {
              q: query || undefined,
              mode: mode ?? undefined,
              kind: memoryKind ?? undefined,
              environment_id: environment ?? undefined,
              repository_id: repository ?? undefined,
              ...times,
              offset,
            },
          },
        }),
      ),
    refetchInterval: pollNormal,
  });
  const refresh = () => {
    for (const key of ["claims", "claim", "audit", "jobs", "processing"])
      void cache.invalidateQueries({ queryKey: [key, brain.id] });
  };
  const clear = () => {
    patchRoute({
      mode: null,
      kind: null,
      environment: null,
      repository: null,
      fact: null,
      knowledge: null,
      revision: null,
    });
    onKindChange?.(null);
    setSearch("");
    setFactDraft("");
    setKnowledgeDraft("");
    setOffset(0);
  };
  const filtered =
    !!memoryKind ||
    mode !== "investigation" ||
    !!environment ||
    !!repository ||
    !!times.fact_at ||
    !!times.knowledge_at;
  const data = claims.error || catalogue.error ? undefined : claims.data;
  const stable = useStableRows(
    data?.items,
    JSON.stringify([
      brain.id,
      memoryKind,
      mode,
      environment,
      repository,
      times,
      query,
      offset,
    ]),
    !!selected,
    (row) => row.revision.claim_id,
  );
  return {
    route,
    patchRoute,
    offset,
    setOffset,
    mode,
    setMode,
    memoryKind,
    setInternalKind,
    environment,
    setEnvironment,
    repository,
    setRepository,
    search,
    setSearch,
    query,
    filtering,
    setFiltering,
    factDraft,
    setFactDraft,
    knowledgeDraft,
    setKnowledgeDraft,
    times,
    setTimes,
    noting,
    setNoting,
    structuring,
    setStructuring,
    selected,
    setSelected,
    catalogue,
    claims,
    refresh,
    clear,
    filtered,
    data,
    stable,
  };
}

export type ClaimsRouteState = ReturnType<typeof useClaimsRoute>;
