import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useReducedMotion } from "motion/react";
import { Drawer, Modal } from "../../components/ui";
import { client, result, type Brain } from "../../api";
import { EvidenceDialog } from "../../ClaimsPanel";
import type { GraphChoice } from "../../GraphCanvas";
import { useContentDeadline } from "../../useContentDeadline";
import { useIdempotency } from "../../useIdempotency";
import { AliasReviewPanel } from "./AliasReviewPanel";
import { FieldOrganizationPanel } from "./FieldOrganizationPanel";
import { useOrganizationSources } from "./useOrganizationSources";
import { usePollInterval } from "../../usePollInterval";
import { feedsGraphScene } from "./sceneKeys";
import { useQualifiedRead } from "./useQualifiedRead";
import {
  sourceChoice,
  useOrganizationMaps,
  type Center,
  type Cursor,
  type EvidenceChoice,
  type EvidenceNode,
  type Scope,
  type SemanticCard,
  type TopicChange,
  type View,
} from "./organizationModel";
import { OrganizationOverviewSection } from "./OrganizationOverview";
import { OrganizationScopeDrawer } from "./OrganizationScopeDrawer";
import { OrganizationSourcesSection } from "./OrganizationSources";
import {
  OrganizationStatus,
  OrganizationToolbar,
  ScopeBadges,
} from "./OrganizationToolbar";
import { TopicEditor } from "./OrganizationTopicEditor";

export function OrganizationPanel({
  brain,
  scope,
  onScopeChange,
}: {
  brain: Brain;
  scope: Scope;
  onScopeChange: (scope: Scope) => void;
}) {
  const pollNormal = usePollInterval("normal");
  const cache = useQueryClient();
  const scopeKey = JSON.stringify(scope);
  const [center, setCenter] = useState<Center | null>(null),
    [cursor, setCursor] = useState<Cursor | null>(null),
    [history, setHistory] = useState<(Cursor | null)[]>([]);
  const [nonce, setNonce] = useState(0),
    [filters, setFilters] = useState(false),
    [aliases, setAliases] = useState(false),
    [draft, setDraft] = useState(scope);
  const [choice, setChoice] = useState<{
    key: string;
    value: GraphChoice;
  } | null>(null);
  const [openedSource, setOpenedSource] = useState<{
    key: string;
    evidence: EvidenceChoice;
  } | null>(null);
  const [editing, setEditing] = useState<{ key: string; id: string } | null>(
    null,
  );
  const [commandError, setCommandError] = useState<Error | null>(null),
    [commandBusy, setCommandBusy] = useState(false),
    [saved, setSaved] = useState(false);
  const [sourceLimit, setSourceLimit] = useState(20),
    [connectionLimit, setConnectionLimit] = useState(20);
  const idem = useIdempotency();
  const status = useQuery({
    queryKey: ["graph", brain.id, 0],
    retry: false,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/graph", {
          params: { path: { brain: brain.id }, query: { offset: 0 } },
          signal,
        }),
      ),
  });
  const requestKey = JSON.stringify([
    brain.id,
    scope,
    center,
    cursor,
    nonce,
    status.data?.memory_epoch,
    status.data?.link_epoch,
    status.data?.organization_epoch,
  ]);
  const activeRequest = useRef(requestKey);
  activeRequest.current = requestKey;
  const [read] = useQualifiedRead<View>({
    key: requestKey,
    enabled: !!status.data && !status.isError,
    initialPending: true,
    fallback: "Organization could not be read.",
    read: async (signal) =>
      result(
        await client.POST("/api/brains/{brain}/organization/view", {
          params: { path: { brain: brain.id } },
          body: { scope, center, cursor, limit: 25 },
          signal,
        }),
      ),
  });
  const expires = useContentDeadline(
    read.key === requestKey ? read.view?.expires_at : null,
  );
  const view =
    read.key === requestKey &&
    !read.pending &&
    !read.error &&
    !status.isError &&
    !expires &&
    !commandError &&
    read.view?.brain_id === brain.id
      ? read.view
      : undefined;
  const receipt = view ? `${requestKey}:${view.epoch}` : null;
  const sourceStream = useOrganizationSources({
    brain: brain.id,
    center,
    view,
    qualificationKey: receipt,
    onClear: () => {
      setChoice(null);
      setOpenedSource(null);
      setEditing(null);
    },
    onFailure: failQualification,
  });
  const sourcePage = view ? sourceStream.page : undefined;

  useEffect(() => {
    setCenter(null);
    setCursor(null);
    setHistory([]);
    setChoice(null);
    setOpenedSource(null);
    setEditing(null);
    setAliases(false);
    setCommandError(null);
    setSaved(false);
    setDraft(scope);
  }, [brain.id, scopeKey]);
  useEffect(
    () =>
      cache.getQueryCache().subscribe((event) => {
        if (
          event.query.queryKey[1] === brain.id &&
          event.type === "updated" &&
          event.action.type === "invalidate" &&
          feedsGraphScene(event.query.queryKey)
        ) {
          setNonce((value) => value + 1);
          setChoice(null);
          setOpenedSource(null);
          setEditing(null);
        }
      }),
    [brain.id, cache],
  );
  useEffect(() => {
    setSourceLimit(20);
    setConnectionLimit(20);
  }, [requestKey]);
  const maps = useOrganizationMaps(view, sourcePage);
  const { sourceMap, topicMap, evidenceNodes } = maps;
  const selected = receipt && choice?.key === receipt ? choice.value : null;
  const selectedSource =
    selected?.kind === "node" ? sourceMap.get(selected.id) : undefined;
  const editTopic =
    receipt && editing?.key === receipt ? topicMap.get(editing.id) : undefined;
  const writable = ["admin", "writer"].includes(brain.role);
  function refresh() {
    setCommandError(null);
    setSaved(false);
    setChoice(null);
    setOpenedSource(null);
    setEditing(null);
    setNonce((value) => value + 1);
    void status.refetch();
  }
  function failQualification(error: Error) {
    setChoice(null);
    setOpenedSource(null);
    setEditing(null);
    setAliases(false);
    setCommandError(error);
  }
  // Exploring a topic or entity brings its evidence graph into view, so what
  // was expanded is visible (and its renderer, paused off-screen, settles).
  const sourcesRegion = useRef<HTMLDivElement>(null);
  const reducedMotion = useReducedMotion();
  const reveal = useRef(false);
  useEffect(() => {
    if (!reveal.current || !center) return;
    reveal.current = false;
    const frame = requestAnimationFrame(() =>
      sourcesRegion.current?.scrollIntoView({
        block: "start",
        behavior: reducedMotion ? "auto" : "smooth",
      }),
    );
    return () => cancelAnimationFrame(frame);
  }, [center, reducedMotion]);
  function expand(next: Center | null) {
    reveal.current = !!next;
    setCenter(next);
    setCursor(null);
    setHistory([]);
    setChoice(null);
    setOpenedSource(null);
    setEditing(null);
    setSaved(false);
  }
  function inspect(node: EvidenceNode) {
    const evidence = sourceChoice(node);
    if (evidence && receipt) setOpenedSource({ key: receipt, evidence });
  }
  async function changeTopic(topic: SemanticCard, change: TopicChange) {
    if (!topic.change_id || !receipt) return;
    const key = receipt;
    const body = { base_change_id: topic.change_id, change };
    setCommandBusy(true);
    setCommandError(null);
    setSaved(false);
    try {
      const response = result(
        await client.PUT("/api/brains/{brain}/organization/topics/{id}", {
          params: { path: { brain: brain.id, id: topic.id } },
          body,
          headers: {
            "Idempotency-Key": idem.forInput({
              brain: brain.id,
              id: topic.id,
              body,
            }),
          },
        }),
      );
      if (activeRequest.current !== requestKey) return;
      idem.reset();
      setEditing(null);
      setChoice(null);
      setOpenedSource(null);
      setSaved(true);
      if (response.split_topic_id)
        setCenter({ kind: "topic", id: response.split_topic_id });
      setCursor(null);
      setHistory([]);
      setNonce((value) => value + 1);
      void cache.invalidateQueries({ queryKey: ["graph", brain.id] });
    } catch (error) {
      if (activeRequest.current === requestKey)
        setCommandError(
          error instanceof Error ? error : new Error("Topic change failed."),
        );
    } finally {
      if (activeRequest.current === requestKey || key === receipt)
        setCommandBusy(false);
    }
  }
  function organize(id: string) {
    if (receipt) setEditing({ key: receipt, id });
  }
  function selectSource(node: EvidenceNode) {
    if (receipt)
      setChoice({ key: receipt, value: { kind: "node", id: node.key } });
  }
  function applyScope() {
    setChoice(null);
    setOpenedSource(null);
    setEditing(null);
    setCenter(null);
    setCursor(null);
    setHistory([]);
    setNonce((value) => value + 1);
    setFilters(false);
    onScopeChange(draft);
  }
  const failure =
    commandError ??
    (read.key === requestKey ? read.error : undefined) ??
    status.error;
  const lower = view?.coverage.counts_are_lower_bounds ?? false;
  const fieldPanel =
    view && receipt ? (
      <FieldOrganizationPanel
        key={receipt}
        brain={brain}
        scope={scope}
        epoch={view.epoch}
        qualificationKey={receipt}
        onFailure={failQualification}
      />
    ) : null;
  return (
    <section
      className="organization-panel grid min-w-0 gap-6"
      aria-label="Automatic knowledge organization"
    >
      <OrganizationToolbar
        center={center}
        hasView={!!view}
        onAllTopics={() => expand(null)}
        onAliases={() => setAliases(true)}
        onScope={() => {
          setDraft(scope);
          setFilters(true);
        }}
        onRefresh={refresh}
      />
      <ScopeBadges scope={scope} />
      <p className="m-0 max-w-[80ch] text-body text-muted-foreground">
        Topics organize eligible sources and source-backed entities across
        sessions. A source can belong to several topics; mentions and
        source-described links keep their evidence.
      </p>
      <OrganizationStatus
        failure={failure}
        expires={expires}
        saved={saved}
        hasView={!!view}
        onRefresh={refresh}
      />
      {view && (
        <>
          <OrganizationOverviewSection
            view={view}
            center={center}
            lower={lower}
            writable={writable}
            maps={maps}
            fieldPanel={fieldPanel}
            onExpand={expand}
            onOrganize={organize}
          />
          <div ref={sourcesRegion} className="scroll-mt-4" />
          <OrganizationSourcesSection
            view={view}
            center={center}
            receipt={receipt}
            maps={maps}
            history={history.length}
            sourceStream={sourceStream}
            sourcePage={sourcePage}
            selected={selected}
            selectedSource={selectedSource}
            sourceLimit={sourceLimit}
            connectionLimit={connectionLimit}
            onPrevious={() => {
              setCursor(history[history.length - 1]);
              setHistory((old) => old.slice(0, -1));
            }}
            onNext={() => {
              setHistory((old) => [...old, cursor]);
              setCursor(view.next_cursor ?? null);
            }}
            onChoose={(value) => receipt && setChoice({ key: receipt, value })}
            onSelect={selectSource}
            onInspect={inspect}
            onMoreSources={() => setSourceLimit((value) => value + 20)}
            onMoreConnections={() => setConnectionLimit((value) => value + 20)}
          />
        </>
      )}
      <Drawer
        opened={aliases && !!view}
        onClose={() => setAliases(false)}
        title="Review alias evidence"
        size={680}
      >
        {aliases && view && receipt && (
          <AliasReviewPanel
            key={receipt}
            brain={brain}
            scope={scope}
            epoch={view.epoch}
            qualification={receipt}
            onInspect={inspect}
            onChanged={() =>
              void cache.invalidateQueries({ queryKey: ["graph", brain.id] })
            }
            onFailure={failQualification}
          />
        )}
      </Drawer>
      <OrganizationScopeDrawer
        brainId={brain.id}
        opened={filters}
        scope={scope}
        draft={draft}
        setDraft={setDraft}
        onClose={() => setFilters(false)}
        onApply={applyScope}
      />
      {view && editTopic && (
        <Modal
          opened
          onClose={() => {
            if (!commandBusy) setEditing(null);
          }}
          title={
            editTopic.topic_kind === "manual_overlay"
              ? "Organize manual group"
              : "Organize automatic topic"
          }
          size="lg"
        >
          <TopicEditor
            key={editTopic.change_id}
            topic={editTopic}
            view={{
              ...view,
              evidence: evidenceNodes,
              source_memberships: maps.sourceMemberships,
              representative_sources: maps.representativeSources,
            }}
            busy={commandBusy}
            onChange={(change) => void changeTopic(editTopic, change)}
          />
        </Modal>
      )}
      {view && receipt && openedSource?.key === receipt && (
        <EvidenceDialog
          brain={brain}
          evidence={openedSource.evidence}
          onClose={() => setOpenedSource(null)}
          returnLabel="Back to organization"
        />
      )}
    </section>
  );
}
