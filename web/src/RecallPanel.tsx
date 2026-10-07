import { useEffect, useState } from "react";
import {
  Accordion,
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Group,
  Loader,
  MultiSelect,
  NumberInput,
  Select,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Textarea,
  Title,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ClaimDialog, EvidenceDialog } from "./ClaimsPanel";
import { client, result, RequestError, type Brain } from "./api";
import { RecallResults, RecallSelectionSummary } from "./RecallResults";
import { RecallGraphDialog } from "./RecallGraphDialog";
import { useContentDeadline } from "./useContentDeadline";
import type { components } from "./api-schema";
import { EvidenceNotes } from "./components/EvidenceNotes";
import { Search, SlidersHorizontal } from "lucide-react";

type Request = components["schemas"]["RecallRequest"];
type Item = components["schemas"]["RecallItem"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type Filters = {
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
const label = (s: string) => s.replaceAll("_", " ");
const initial = (): Filters => ({
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
  const cache = useQueryClient();
  const [filters, setFilters] = useState(() => ({
    ...initial(),
    query: initialQuery,
  }));
  const [submitted, setSubmitted] = useState<{
    request: Request;
    nonce: string;
  } | null>(null);
  const [detail, setDetail] = useState<{
    claim?: string;
    evidence?: Evidence;
    graph?: Item;
  } | null>(null);
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
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id, "recall"],
    enabled: optionsOpen || !!submitted,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
        }),
      ),
  });
  const groups = useQuery({
    queryKey: ["evidence", brain.id, "recall"],
    enabled: optionsOpen || !!submitted,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          params: { path: { brain: brain.id } },
        }),
      ),
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
      request: {
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
      } satisfies Request,
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
    refetchInterval: 3000,
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
  const manifestOptions =
    manifests.data?.items.map((m) => ({
      value: m.id,
      label: `${m.name} · ${label(m.kind)}`,
    })) ?? [];
  if (
    filters.manifest &&
    !manifestOptions.some((m) => m.value === filters.manifest)
  ) {
    manifestOptions.push({
      value: filters.manifest,
      label: "Selected exact manifest revision",
    });
  }
  const disabled =
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
      !/^[0-9a-f]{8}-[0-9a-f-]{27}$/i.test(filters.exactId));
  const selectedCount =
    filters.repositories.length +
    filters.areas.length +
    (filters.environment ? 1 : 0) +
    (filters.collection ? 1 : 0);
  return (
    <Card
      className="recall-view evidence-search"
      p={0}
      component="section"
      aria-label="Recall memory"
    >
      <Stack>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!disabled && !query.isFetching) search();
          }}
        >
          <Stack gap="sm">
            <Group gap="xs" className="evidence-search-toolbar">
              <Badge color="gray">
                {selectedCount
                  ? `${selectedCount} scope filters`
                  : "Entire Brain"}
              </Badge>
              {(filters.knowledge || filters.fact) && (
                <Badge color="yellow">Time filtered</Badge>
              )}
              {filters.mode !== "investigation" && (
                <Badge color="brand">{label(filters.mode)}</Badge>
              )}
              <Button
                variant="default"
                size="xs"
                leftSection={<SlidersHorizontal size={15} />}
                onClick={() => setOptionsOpen((value) => !value)}
                aria-expanded={optionsOpen}
              >
                Refine
              </Button>
              {!!submitted && (
                <Button
                  variant="subtle"
                  size="xs"
                  onClick={() => {
                    void cache.cancelQueries({
                      queryKey: ["recall", brain.id],
                    });
                    setSubmitted(null);
                    setDetail(null);
                  }}
                >
                  Clear results
                </Button>
              )}
            </Group>
            <div className="evidence-search-composer">
              <Textarea
                aria-label="Search evidence"
                placeholder="Find the evidence behind a decision, system or past work…"
                value={queryText}
                onChange={(e) => change({ query: e.currentTarget.value })}
                maxLength={512}
                autosize
                minRows={2}
                maxRows={6}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                    e.preventDefault();
                    if (!disabled && !query.isFetching) search();
                  }
                }}
              />
              <Button
                type="submit"
                disabled={disabled || query.isFetching}
                loading={query.isFetching && !!submitted}
                leftSection={<Search size={16} />}
              >
                Find evidence
              </Button>
            </div>
            <Text size="xs" c="dimmed">
              Finds stored evidence without generating an answer. Automatic
              retrieval may use the permitted embedding model.
            </Text>
            <details
              className="feature-advanced evidence-search-options"
              open={optionsOpen}
              onToggle={(e) => setOptionsOpen(e.currentTarget.open)}
            >
              <summary>Refine evidence search</summary>
              <Select
                label="Retrieval strategy"
                value={filters.strategy}
                allowDeselect={false}
                data={[
                  { value: "auto", label: "Automatic" },
                  { value: "manual", label: "Choose methods" },
                ]}
                onChange={(value) =>
                  change({ strategy: value === "manual" ? "manual" : "auto" })
                }
                mb="md"
              />
              <SimpleGrid cols={{ base: 1, sm: 2 }}>
                <Select
                  label="Recall mode"
                  value={filters.mode}
                  allowDeselect={false}
                  data={[
                    { value: "investigation", label: "Investigation" },
                    { value: "strict_accepted", label: "Accepted and current" },
                    {
                      value: "strict_operational",
                      label: "Accepted and operationally verified",
                    },
                    { value: "history", label: "Qualified history" },
                  ]}
                  onChange={(v) => change({ mode: v ?? "investigation" })}
                />
                <Select
                  label="Recall collection"
                  clearable
                  placeholder="All collections"
                  value={filters.collection}
                  data={
                    groups.data?.groups
                      .filter((g) => g.kind === "collection")
                      .map((g) => ({ value: g.id, label: g.name })) ?? []
                  }
                  onChange={(v) => change({ collection: v })}
                />
              </SimpleGrid>
              <Accordion>
                <Accordion.Item value="filters">
                  <Accordion.Control>
                    Scope, time and exact lookup
                  </Accordion.Control>
                  <Accordion.Panel>
                    <Stack>
                      <SimpleGrid cols={{ base: 1, sm: 2 }}>
                        <MultiSelect
                          label="Recall repositories"
                          placeholder="All repositories"
                          searchable
                          clearable
                          value={filters.repositories}
                          data={
                            catalogue.data?.repositories.map((r) => ({
                              value: r.id,
                              label: r.canonical_origin,
                            })) ?? []
                          }
                          onChange={(v) => change({ repositories: v })}
                          maxValues={100}
                        />
                        <MultiSelect
                          label="Recall areas"
                          placeholder="All areas"
                          searchable
                          clearable
                          value={filters.areas}
                          data={
                            catalogue.data?.areas.map((a) => ({
                              value: a.id,
                              label: a.name,
                            })) ?? []
                          }
                          onChange={(v) => change({ areas: v })}
                          maxValues={100}
                        />
                        <Select
                          label="Recall environment"
                          clearable
                          placeholder="All environments"
                          value={filters.environment}
                          data={
                            catalogue.data?.environments.map((a) => ({
                              value: a.id,
                              label: a.name,
                            })) ?? []
                          }
                          onChange={(v) =>
                            change({ environment: v, manifest: null })
                          }
                        />
                        <Select
                          label="Recall revision manifest"
                          clearable
                          placeholder="Select an exact manifest"
                          value={filters.manifest}
                          data={manifestOptions}
                          onChange={(v) =>
                            change({
                              manifest: v,
                              environment:
                                manifests.data?.items.find((m) => m.id === v)
                                  ?.environment_id ?? filters.environment,
                            })
                          }
                        />
                        {(manifests.data?.total ?? 0) > 20 && (
                          <Group>
                            <Button
                              variant="subtle"
                              size="xs"
                              disabled={!manifestOffset}
                              onClick={() =>
                                setManifestOffset(
                                  Math.max(0, manifestOffset - 20),
                                )
                              }
                            >
                              Previous manifests
                            </Button>
                            <Text size="xs">
                              {manifestOffset + 1}–
                              {Math.min(
                                manifestOffset + 20,
                                manifests.data!.total,
                              )}{" "}
                              of {manifests.data!.total}
                            </Text>
                            <Button
                              variant="subtle"
                              size="xs"
                              disabled={
                                manifestOffset + 20 >= manifests.data!.total
                              }
                              onClick={() =>
                                setManifestOffset(manifestOffset + 20)
                              }
                            >
                              Next manifests
                            </Button>
                          </Group>
                        )}
                        <TextInput
                          type="datetime-local"
                          label="Recall knowledge time (UTC)"
                          step={1}
                          value={filters.knowledge}
                          onChange={(e) =>
                            change({ knowledge: e.currentTarget.value })
                          }
                        />
                        <TextInput
                          type="datetime-local"
                          label="Recall fact time (UTC)"
                          step={1}
                          value={filters.fact}
                          onChange={(e) =>
                            change({ fact: e.currentTarget.value })
                          }
                        />
                        <Select
                          label="Exact record kind"
                          clearable
                          placeholder="Optional exact lookup"
                          value={filters.exactKind || null}
                          data={[
                            "claim",
                            "source_version",
                            "repository_fact",
                            "manifest_revision",
                          ].map((value) => ({ value, label: label(value) }))}
                          onChange={(v) =>
                            change({
                              exactKind: v ?? "",
                              channels: v
                                ? Array.from(
                                    new Set([...filters.channels, "exact"]),
                                  )
                                : filters.channels,
                            })
                          }
                        />
                        <TextInput
                          label="Exact record UUID"
                          disabled={!filters.exactKind}
                          value={filters.exactId}
                          onChange={(e) =>
                            change({ exactId: e.currentTarget.value })
                          }
                        />
                      </SimpleGrid>
                      {filters.strategy === "manual" ? (
                        <Checkbox.Group
                          label="Search channels"
                          value={filters.channels}
                          onChange={(v) =>
                            change({ channels: v, strategy: "manual" })
                          }
                        >
                          <Group mt="xs">
                            <Checkbox
                              value="exact"
                              label="Exact identities and literals"
                            />
                            <Checkbox value="lexical" label="Text search" />
                            <Checkbox
                              value="graph"
                              label="Graph relationships"
                            />
                            <Checkbox
                              value="semantic"
                              label="Semantic similarity"
                            />
                          </Group>
                        </Checkbox.Group>
                      ) : (
                        <Text size="sm" c="dimmed">
                          Automatic retrieval chooses eligible methods for this
                          query. Scope, time and evidence eligibility stay
                          fixed.
                        </Text>
                      )}
                      <Checkbox
                        label="Prefer source coverage before adding depth"
                        checked={filters.sourceDiversity}
                        onChange={(e) =>
                          change({ sourceDiversity: e.currentTarget.checked })
                        }
                      />
                      {filters.strategy === "manual" &&
                        filters.channels.includes("graph") && (
                          <Stack gap="xs">
                            <Text size="sm">
                              Graph search expands up to three eligible matches
                              from the other selected channels. Recorded
                              relationships do not establish truth or deployment
                              behavior. Current knowledge only.
                            </Text>
                            {(filters.mode === "history" ||
                              !!filters.knowledge ||
                              filters.channels.length === 1) && (
                              <Alert
                                color="yellow"
                                title="Graph search needs current query anchors"
                              >
                                Select another search channel and current
                                knowledge, or disable graph search for history.
                              </Alert>
                            )}
                            <SimpleGrid cols={3}>
                              <Select
                                label="Recall graph"
                                value={filters.graphKind}
                                data={[
                                  "knowledge",
                                  "repository",
                                  "combined",
                                ].map((value) => ({
                                  value,
                                  label: label(value),
                                }))}
                                onChange={(value) =>
                                  change({ graphKind: value ?? "knowledge" })
                                }
                              />
                              <Select
                                label="Recall graph direction"
                                value={filters.graphDirection}
                                data={["both", "outgoing", "incoming"].map(
                                  (value) => ({ value, label: label(value) }),
                                )}
                                onChange={(value) =>
                                  change({ graphDirection: value ?? "both" })
                                }
                              />
                              <NumberInput
                                label="Recall graph hops"
                                min={1}
                                max={3}
                                allowDecimal={false}
                                value={filters.graphHops}
                                onChange={(value) =>
                                  change({
                                    graphHops:
                                      typeof value === "number" ? value : 2,
                                  })
                                }
                              />
                            </SimpleGrid>
                            <TextInput
                              label="Recall relationships"
                              description="Optional comma-separated relationship names; empty includes all eligible relationships."
                              value={filters.graphRelations}
                              onChange={(e) =>
                                change({
                                  graphRelations: e.currentTarget.value,
                                })
                              }
                            />
                            {filters.graphKind === "repository" &&
                              filters.repositories.length !== 1 && (
                                <Text c="orange">
                                  Select one repository above for its structural
                                  graph.
                                </Text>
                              )}
                            {filters.graphKind === "combined" &&
                              (!filters.environment || !filters.manifest) && (
                                <Text c="orange">
                                  Select an environment and exact manifest above
                                  for combined relationships.
                                </Text>
                              )}
                          </Stack>
                        )}
                      {filters.strategy === "manual" &&
                        filters.channels.includes("semantic") && (
                          <>
                            <Text size="sm">
                              Each Recall action may send this query to the
                              approved embedding model. Failed queries are not
                              automatically resent. Similarity does not
                              establish truth or acceptance.
                            </Text>
                            <NumberInput
                              label="Minimum semantic similarity"
                              min={0}
                              max={1}
                              step={0.05}
                              decimalScale={2}
                              value={filters.similarity}
                              onChange={(v) =>
                                change({
                                  similarity: typeof v === "number" ? v : 0,
                                })
                              }
                            />
                            <Text size="xs" c="dimmed">
                              Zero applies no positive similarity threshold. A
                              higher value can exclude useful evidence; it is
                              not a confidence score.
                            </Text>
                          </>
                        )}
                    </Stack>
                  </Accordion.Panel>
                </Accordion.Item>
              </Accordion>
            </details>
          </Stack>
        </form>
        {(catalogue.error || groups.error || manifests.error) && (
          <Alert color="red">
            Scope choices could not be loaded. Reload the Brain and retry.
          </Alert>
        )}
        {notice && <Alert title="Investigation cleared">{notice}</Alert>}
        {submitted && query.error && (
          <Alert color="red" title="Recall failed">
            {query.error.message}
          </Alert>
        )}
        {submitted && query.isLoading && (
          <Loader aria-label="Recalling memory" />
        )}
        {answer && (
          <Stack aria-label="Recall results">
            <details className="evidence-result-details">
              <summary>Scope and retrieval details</summary>
              <RecallSelectionSummary
                answer={answer}
                request={submitted!.request}
                brainName={brain.name}
                catalogue={catalogue.data}
                collectionName={
                  groups.data?.groups.find(
                    (g) => g.id === submitted!.request.collection_id,
                  )?.name
                }
              />
            </details>
            <Group justify="space-between">
              <Text size="sm">
                {answer.context.items.length} result
                {answer.context.items.length === 1 ? "" : "s"} ·{" "}
                {Array.from(
                  new Set(
                    answer.context.items.flatMap((item) => item.channels),
                  ),
                )
                  .map((channel) =>
                    channel === "lexical"
                      ? "text"
                      : channel === "semantic"
                        ? "meaning"
                        : channel === "graph"
                          ? "relationships"
                          : channel,
                  )
                  .join(" + ") || "No matches"}
              </Text>
              <Button
                variant="subtle"
                size="xs"
                onClick={() => {
                  setCopyError(false);
                  void Promise.resolve()
                    .then(() =>
                      navigator.clipboard.writeText(
                        JSON.stringify(answer.context),
                      ),
                    )
                    .then(() => setCopied(true))
                    .catch(() => {
                      setCopied(false);
                      setCopyError(true);
                    });
                }}
              >
                {copied ? "Context copied" : "Copy attributed context"}
              </Button>
            </Group>
            {copyError && (
              <Alert color="red">
                Context could not be copied. Check clipboard access and try
                again.
              </Alert>
            )}
            <EvidenceNotes
              notes={answer.coverage.reasons}
              partial={answer.coverage.partial}
            />
            <details className="evidence-result-details">
              <summary>Coverage and recorded methods</summary>
              {answer.semantic && (
                <Text size="xs" c="dimmed">
                  Semantic coverage:{" "}
                  {answer.semantic.scoped_entries.toLocaleString()} scoped
                  representations ·{" "}
                  {answer.semantic.profile?.model ?? "No index available"}.{" "}
                  {answer.semantic.model_request_id
                    ? "One query embedding request recorded."
                    : "No query model call."}
                </Text>
              )}
              <Text size="xs" c="dimmed">
                Source coverage:{" "}
                {answer.context_selection.distinct_source_groups} resolved
                groups
                {answer.context_selection.unknown_lineage_items > 0
                  ? ` · ${answer.context_selection.unknown_lineage_items} results with unknown ancestry`
                  : ""}
                .
                {answer.context_selection.source_diversity
                  ? " Coverage preference enabled."
                  : " Rank-only selection."}{" "}
                Source groups do not establish independent corroboration.
              </Text>
              {answer.graph && (
                <Text size="sm" data-testid="recall-graph-status">
                  Graph: {label(answer.graph.state)} ·{" "}
                  {answer.graph.anchors.length} anchors ·{" "}
                  {answer.graph.candidates} connected candidates
                  {answer.graph.view.coverage.partial
                    ? " · partial graph coverage"
                    : ""}
                  .
                </Text>
              )}
            </details>
            {!answer.context.items.length && (
              <Alert
                title={
                  answer.status === "no_match"
                    ? "No matching evidence"
                    : "Insufficient eligible evidence"
                }
              >
                {answer.status === "no_match"
                  ? "Try a more specific subject, another phrase or an exact identifier."
                  : "No result fits scope/trust/budget. Check qualifications, widen scope, or add evidence and let autonomous learning process it."}
              </Alert>
            )}
            <RecallResults
              key={submitted!.nonce}
              answer={answer}
              request={submitted!.request}
              catalogue={catalogue.data}
              onClaim={(id) => setDetail({ claim: id })}
              onEvidence={(e) => setDetail({ evidence: e })}
              onGraph={(item) => setDetail({ graph: item })}
            />
          </Stack>
        )}
      </Stack>
      {detail?.claim && (
        <ClaimDialog
          key={`${detail.claim}:${answer?.knowledge_at}`}
          brain={brain}
          id={detail.claim}
          catalogue={catalogue.data}
          knowledgeAt={answer?.knowledge_at}
          factAt={submitted?.request.fact_at ?? undefined}
          onClose={() => setDetail(null)}
          onSaved={() => {
            invalidate();
            void cache.invalidateQueries({
              predicate: (q) => q.queryKey.includes(brain.id),
            });
          }}
        />
      )}
      {detail?.graph && answer && submitted && (
        <RecallGraphDialog
          key={detail.graph.revision_id}
          brain={brain}
          item={detail.graph}
          answer={answer}
          request={submitted.request}
          catalogue={catalogue.data}
          onClose={() => setDetail(null)}
          onInvalidated={invalidate}
          onClaim={(id) => setDetail({ claim: id })}
          onEvidence={(e) => setDetail({ evidence: e })}
        />
      )}
      {detail?.evidence && (
        <EvidenceDialog
          brain={brain}
          evidence={detail.evidence}
          returnLabel="Back to recall"
          eraseSource
          onClose={() => setDetail(null)}
        />
      )}
    </Card>
  );
}
