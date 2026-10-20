import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, RequestError, type Brain } from "./api";
import { Alert } from "./components/ui";
import { RecallAnswer } from "./RecallAnswer";
import { RecallComposer } from "./RecallComposer";
import { RecallDetailDialogs, type RecallDetail } from "./RecallDetailDialogs";
import { RecallOptions } from "./RecallOptions";
import {
  buildRecallRequest,
  initial,
  recallSearchDisabled,
  type Filters,
  type RecallRequestBody as Request,
} from "./recallFilters";
import { useContentDeadline } from "./useContentDeadline";
import { usePollInterval } from "./usePollInterval";
import { useWorkspaceCatalogue, useEvidenceList } from "./catalogueQueries";

export function RecallPanel({
  brain,
  initialQuery = "",
  draft,
  setDraft,
}: {
  brain: Brain;
  initialQuery?: string;
  draft?: string;
  setDraft?: (value: string) => void;
}) {
  const pollNormal = usePollInterval("normal");
  const cache = useQueryClient();
  const [filters, setFilters] = useState(() => ({
    ...initial(),
    query: initialQuery,
  }));
  const [submitted, setSubmitted] = useState<{
    request: Request;
    nonce: string;
  } | null>(null);
  const [detail, setDetail] = useState<RecallDetail | null>(null);
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [manifestOffset, setManifestOffset] = useState(0);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const queryText = draft ?? filters.query;
  useEffect(
    () =>
      cache.getQueryCache().subscribe((event) => {
        if (event.query.queryKey[1] !== brain.id) return;
        if (
          event.type === "updated" &&
          event.action.type === "error" &&
          event.query.state.error instanceof RequestError &&
          [401, 403, 404].includes(event.query.state.error.status) &&
          [
            "claim",
            "claim-evidence-detail",
            "claim-review",
            "review-evidence",
            "investigation-graph",
            "graph-path",
            "graph-explore",
          ].includes(String(event.query.queryKey[0]))
        ) {
          invalidate();
          return;
        }
        if (
          event.query.queryKey[0] !== "recall" ||
          event.query.queryKey[1] !== brain.id
        )
          return;
        if (
          event.type === "updated" &&
          (event.action.type === "invalidate" ||
            (event.action.type === "setState" &&
              event.query.state.data === undefined))
        ) {
          // Canonical mutations clear displayed context. A cache refresh must never
          // turn a prior explicit search into another paid model request.
          setSubmitted(null);
          setDetail(null);
          setCopied(false);
          setCopyError(false);
          setNotice("Memory changed. Submit a fresh query to continue.");
          void cache.cancelQueries({
            queryKey: event.query.queryKey,
            exact: true,
          });
        }
      }),
    [cache, brain.id],
  );
  function change(patch: Partial<Filters>) {
    if (patch.query !== undefined) setDraft?.(patch.query);
    if (Object.hasOwn(patch, "environment")) setManifestOffset(0);
    setFilters((f) => ({ ...f, ...patch }));
    setSubmitted(null);
    setDetail(null);
    setCopied(false);
    setCopyError(false);
    setNotice(null);
    void cache.cancelQueries({ queryKey: ["recall", brain.id] });
  }
  const catalogue = useWorkspaceCatalogue(brain.id, undefined, {
    enabled: optionsOpen || !!submitted,
  });
  const groups = useEvidenceList(brain.id, undefined, {
    enabled: optionsOpen || !!submitted,
  });
  const manifests = useQuery({
    queryKey: [
      "recall-manifests",
      brain.id,
      filters.environment,
      manifestOffset,
    ],
    enabled: optionsOpen,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/revision-manifests", {
          params: {
            path: { brain: brain.id },
            query: {
              environment_id: filters.environment ?? undefined,
              offset: manifestOffset,
            },
          },
        }),
      ),
  });
  const query = useQuery({
    queryKey: ["recall", brain.id, submitted],
    enabled: false,
    gcTime: 0,
    staleTime: 0,
    retry: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchOnMount: false,
    networkMode: "always",
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/recall", {
          params: { path: { brain: brain.id } },
          body: submitted!.request,
          signal,
        }),
      ),
  });
  function search() {
    setCopied(false);
    setCopyError(false);
    setNotice(null);
    setDetail(null);
    const next = {
      nonce: crypto.randomUUID(),
      request: buildRecallRequest(filters, queryText),
    };
    setSubmitted(next);
    // Explicit submit is the only call site. The disabled observer above still
    // shows errors/loading, but reset/invalidation/focus cannot execute it.
    void cache
      .fetchQuery({
        queryKey: ["recall", brain.id, next],
        gcTime: 0,
        retry: false,
        networkMode: "always",
        queryFn: async ({ signal }) =>
          result(
            await client.POST("/api/brains/{brain}/recall", {
              params: { path: { brain: brain.id } },
              body: next.request,
              signal,
            }),
          ),
      })
      .catch(() => {
        /* The query observer renders the recorded failure. */
      });
  }
  const answer = submitted && !query.isError ? query.data : undefined;
  function invalidate() {
    setSubmitted(null);
    setDetail(null);
    setCopied(false);
    setCopyError(false);
    setNotice("Memory changed or expired. Submit a fresh query to continue.");
  }
  const graphStatus = useQuery({
    queryKey: ["graph", brain.id, 0],
    enabled: !!answer,
    refetchInterval: pollNormal,
    retry: false,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/graph", {
          params: { path: { brain: brain.id }, query: { offset: 0 } },
        }),
      ),
  });
  useEffect(() => {
    if (!answer) return;
    if (
      (graphStatus.errorUpdatedAt >= query.dataUpdatedAt &&
        graphStatus.isError) ||
      (graphStatus.dataUpdatedAt >= query.dataUpdatedAt &&
        graphStatus.data &&
        (graphStatus.data.memory_epoch !== answer.memory_epoch ||
          (answer.graph?.view.generation.kind === "combined" &&
            graphStatus.data.link_epoch !==
              answer.graph.view.generation.input_epoch)))
    ) {
      invalidate();
    }
  }, [
    answer,
    graphStatus.data,
    graphStatus.dataUpdatedAt,
    graphStatus.errorUpdatedAt,
    graphStatus.isError,
    query.dataUpdatedAt,
  ]);
  const expired = useContentDeadline(
    answer?.expires_at ?? answer?.graph?.expires_at,
  );
  useEffect(() => {
    if (expired) invalidate();
  }, [expired]);
  const disabled = recallSearchDisabled(filters, queryText);
  const selectedCount =
    filters.repositories.length +
    filters.areas.length +
    (filters.environment ? 1 : 0) +
    (filters.collection ? 1 : 0);
  return (
    <section
      className="mx-auto grid w-full max-w-[960px] content-start gap-4"
      aria-label="Recall memory"
    >
      <RecallComposer
        filters={filters}
        queryText={queryText}
        selectedCount={selectedCount}
        optionsOpen={optionsOpen}
        toggleOptions={() => setOptionsOpen((value) => !value)}
        hasResults={!!submitted}
        disabled={disabled}
        fetching={query.isFetching}
        onQueryChange={(value) => change({ query: value })}
        onSearch={search}
        onClear={() => {
          void cache.cancelQueries({
            queryKey: ["recall", brain.id],
          });
          setSubmitted(null);
          setDetail(null);
        }}
      >
        <RecallOptions
          filters={filters}
          change={change}
          open={optionsOpen}
          setOpen={setOptionsOpen}
          catalogue={catalogue.data}
          groups={groups.data}
          manifests={manifests.data}
          manifestOffset={manifestOffset}
          setManifestOffset={setManifestOffset}
        />
      </RecallComposer>
      {(catalogue.error || groups.error || manifests.error) && (
        <Alert variant="destructive">
          Scope choices could not be loaded. Reload the Brain and retry.
        </Alert>
      )}
      {notice && <Alert title="Investigation cleared">{notice}</Alert>}
      {submitted && query.error && (
        <Alert variant="destructive" title="Recall failed">
          {query.error.message}
        </Alert>
      )}
      {submitted && query.isLoading && (
        <p
          role="status"
          aria-label="Recalling memory"
          className="shimmer-text m-0 w-fit px-1 text-body"
        >
          Recalling memory from {brain.name}…
        </p>
      )}
      {answer && (
        <RecallAnswer
          brain={brain}
          answer={answer}
          request={submitted!.request}
          nonce={submitted!.nonce}
          catalogue={catalogue.data}
          groups={groups.data}
          copied={copied}
          copyError={copyError}
          setCopied={setCopied}
          setCopyError={setCopyError}
          onClaim={(id) => setDetail({ claim: id })}
          onEvidence={(e) => setDetail({ evidence: e })}
          onGraph={(item) => setDetail({ graph: item })}
        />
      )}
      <RecallDetailDialogs
        brain={brain}
        detail={detail}
        answer={answer}
        request={submitted?.request}
        catalogue={catalogue.data}
        setDetail={setDetail}
        onInvalidate={invalidate}
        onClaimSaved={() => {
          invalidate();
          void cache.invalidateQueries({
            predicate: (q) => q.queryKey.includes(brain.id),
          });
        }}
      />
    </section>
  );
}
