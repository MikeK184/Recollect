import { useEffect, useRef, useState } from "react";
import { useBrainSearch } from "./app/useBrainSearch";
import "./features/feature-views.css";
import { Drawer, Group, Loader, Text } from "@mantine/core";
import { useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { AnalyticsPanel } from "./AnalyticsPanel";
import { GraphExplorer } from "./GraphExplorer";
import { useContentDeadline } from "./useContentDeadline";
import { useWorkspaceCatalogue, useEvidenceList } from "./catalogueQueries";
import {
  detailForNode,
  initial,
  missingExactSelection,
  scopeFromRoute,
  type GraphDetail,
  type GraphDrawer,
  type GraphNodeView as Node,
  type Scope,
  type SubmittedPath,
  type SubmittedRead,
} from "./features/graph/graphScope";
import {
  Failure,
  GraphDetailDialogs,
  GraphEmptyState,
  GraphEntityCard,
  GraphScopeSummary,
  GraphToolbar,
} from "./features/graph/GraphPanelParts";
import { GraphFiltersDrawer } from "./features/graph/GraphFiltersDrawer";
import { GraphPathDrawer } from "./features/graph/GraphPathDrawer";
import { GraphEntitiesDrawer } from "./features/graph/GraphEntitiesDrawer";
import { GraphStatusDrawer } from "./features/graph/GraphStatusDrawer";
import {
  useGraphScene,
  useSceneVerification,
} from "./features/graph/useGraphScene";
import {
  useGraphEntities,
  useGraphPath,
  useGraphRead,
  useGraphRebuild,
  useGraphScopeOptions,
  useGraphStatus,
} from "./features/graph/useGraphScopeData";

export function GraphPanel({ brain }: { brain: Brain }) {
  const cache = useQueryClient();
  const [route, patchRoute] = useBrainSearch();
  const [scope, setScope] = useState(() => scopeFromRoute(route));
  const routeScopeKey = JSON.stringify(scopeFromRoute(route));
  const scopeKey = JSON.stringify(scope);
  const [drawer, setDrawer] = useState<GraphDrawer>(null);
  const initialRead = useRef<string | null>(null);
  const [entityOffset, setEntityOffset] = useState(0);
  const [offset, setOffset] = useState(0);
  const [start, setStart] = useState("");
  const [initialExploration, setInitialExploration] = useState<
    components["schemas"]["GraphExploration"] | undefined
  >();
  const [end, setEnd] = useState("");
  const [direction, setDirection] = useState("outgoing");
  const [hops, setHops] = useState(6);
  const [submitted, setSubmitted] = useState<SubmittedRead | null>(null);
  const [pathSubmitted, setPathSubmitted] = useState<SubmittedPath | null>(
    null,
  );
  const [detail, setDetail] = useState<GraphDetail | null>(null);
  const resets = {
    setInitialExploration,
    setSubmitted,
    setPathSubmitted,
    setDetail,
  };
  const sceneState = useGraphScene(brain.id, resets);
  const { scene, generationError, setGenerationError } = sceneState;
  const { sceneStarted, sceneAccepted, clear } = sceneState;
  function change(next: Scope) {
    clear();
    setScope(next);
    setStart("");
    setEnd("");
  }
  useEffect(() => {
    if (scopeKey !== routeScopeKey) {
      clear();
      setScope(scopeFromRoute(route));
      initialRead.current = null;
      setStart("");
      setEnd("");
    }
  }, [routeScopeKey]);
  function saveScope() {
    patchRoute({
      kind: scope.kind,
      repository: null,
      area: null,
      repositories: scope.selection.repository_ids ?? [],
      areas: scope.selection.area_ids ?? [],
      environment: scope.selection.environment_id ?? null,
      snapshot: scope.snapshot_id ?? null,
      manifest: scope.manifest_revision_id ?? null,
      collection: scope.collection_id ?? null,
      fact: scope.fact_at ?? null,
      mode: scope.mode,
      relations: scope.relations,
      center: null,
    });
  }
  useSceneVerification(brain.id, sceneState, resets);
  const status = useGraphStatus(brain.id, offset);
  const catalogue = useWorkspaceCatalogue(brain.id);
  const groups = useEvidenceList(brain.id);
  const scopeOptions = useGraphScopeOptions(brain.id, scope);
  const { snapshots, manifests } = scopeOptions;
  const read = useGraphRead(brain.id, submitted);
  const { entities, expired: entitiesExpired } = useGraphEntities(brain.id, {
    enabled:
      drawer === "entities" && !!submitted && !!read.data && !read.isError,
    submitted,
    entityOffset,
    memoryEpoch: status.data?.memory_epoch,
    linkEpoch: status.data?.link_epoch,
  });
  const readExpired = useContentDeadline(read.data?.expires_at);
  useEffect(() => setEntityOffset(0), [scopeKey]);
  const path = useGraphPath(brain.id, pathSubmitted);
  function load(page = 0, center = route.center ?? null) {
    if (missingExactSelection(scope)) return;
    initialRead.current = scopeKey;
    setGenerationError(null);
    sceneStarted();
    const next = {
      nonce: crypto.randomUUID(),
      scope: structuredClone(scope),
      offset: page,
    };
    setSubmitted(next);
    setPathSubmitted(null);
    setDetail(null);
    setInitialExploration(undefined);
    void cache
      .fetchQuery({
        queryKey: ["graph-read", brain.id, next],
        retry: false,
        gcTime: 0,
        queryFn: async ({ signal }) => {
          const exploration = result(
            await client.POST("/api/brains/{brain}/graph/explore", {
              params: { path: { brain: brain.id } },
              body: {
                scope: next.scope,
                center,
                direction: route.direction ?? "outgoing",
                max_hops: route.hops ?? 1,
                windowed: !center || (route.hops ?? 1) === 1,
              },
              signal,
            }),
          );
          if (!signal.aborted) setInitialExploration(exploration);
          return exploration.view;
        },
      })
      .catch(() => {});
  }
  useEffect(() => {
    if (
      initialRead.current !== scopeKey &&
      scopeKey === routeScopeKey &&
      status.data &&
      !status.error &&
      !missingExactSelection(scope)
    ) {
      load();
    }
  }, [status.data, status.error, scopeKey, routeScopeKey]);
  function findPath() {
    const next = {
      nonce: crypto.randomUUID(),
      request: {
        scope: read.data!.scope,
        start,
        end,
        direction,
        max_hops: hops,
      },
    };
    setPathSubmitted(next);
    void cache
      .fetchQuery({
        queryKey: ["graph-path", brain.id, next],
        retry: false,
        gcTime: 0,
        queryFn: async ({ signal }) =>
          result(
            await client.POST("/api/brains/{brain}/graph/path", {
              params: { path: { brain: brain.id } },
              body: next.request,
              signal,
            }),
          ),
      })
      .catch(() => {});
  }
  const rebuild = useGraphRebuild(brain, scope, clear);
  function inspect(n: Node) {
    const next = detailForNode(n);
    if (next) setDetail(next);
  }
  function editPath() {
    setPathSubmitted(null);
    setDetail(null);
  }
  function renderNode(n: Node, choose: boolean) {
    return (
      <GraphEntityCard
        key={n.key}
        n={n}
        choose={choose}
        repositories={catalogue.data?.repositories}
        onInspect={inspect}
        onStart={(key) => {
          setStart(key);
          editPath();
        }}
        onEnd={(key) => {
          setEnd(key);
          editPath();
        }}
      />
    );
  }
  const availableGroups = groups.data?.groups ?? [];
  const readable =
    submitted && read.data && !read.error && !readExpired
      ? read.data
      : undefined;
  const needsSelection = missingExactSelection(scope);
  return (
    <section
      className="feature-view graph-surface"
      aria-label="Evidence graphs"
    >
      <GraphToolbar
        scope={scope}
        readable={!!readable}
        onKindChange={(kind) => {
          change({ ...initial(), kind });
          setDrawer("filters");
        }}
        onOpen={setDrawer}
      />
      <GraphScopeSummary scope={scope} availableGroups={availableGroups} />
      <Failure
        error={
          read.error ?? generationError ?? (!readable ? status.error : null)
        }
      />
      {read.isFetching && (
        <Group role="status" my="lg">
          <Loader size="sm" />
          <Text>Reading eligible graph evidence…</Text>
        </Group>
      )}
      {!readable && !read.isFetching && (
        <GraphEmptyState
          needsSelection={needsSelection}
          unavailable={!!read.error}
          onChooseInputs={() => setDrawer("filters")}
          onLoad={() => load()}
        />
      )}
      {readable && submitted && (
        <>
          <GraphExplorer
            key={submitted.nonce}
            brain={brain}
            view={readable}
            path={pathSubmitted && !path.error ? (path.data ?? null) : null}
            inspect={inspect}
            onExpired={clear}
            compactControls
            initialCenter={route.center ?? ""}
            initialDirection={route.direction ?? "outgoing"}
            initialHops={route.hops ?? 1}
            initialExploration={initialExploration}
            onExplore={(center, direction, hops) =>
              patchRoute({ center, direction, hops })
            }
            onReadStarted={sceneStarted}
            onViewAccepted={sceneAccepted}
            onStart={(key) => {
              setStart(key);
              setPathSubmitted(null);
              setDrawer("path");
            }}
            onEnd={(key) => {
              setEnd(key);
              setPathSubmitted(null);
              setDrawer("path");
            }}
          />
        </>
      )}
      <GraphFiltersDrawer
        opened={drawer === "filters"}
        onClose={() => setDrawer(null)}
        scope={scope}
        change={change}
        repositories={catalogue.data?.repositories}
        availableGroups={availableGroups}
        snapshots={snapshots.data}
        snapshotOffset={scopeOptions.snapshotOffset}
        setSnapshotOffset={scopeOptions.setSnapshotOffset}
        manifests={manifests.data}
        manifestOffset={scopeOptions.manifestOffset}
        setManifestOffset={scopeOptions.setManifestOffset}
        error={
          catalogue.error ?? groups.error ?? snapshots.error ?? manifests.error
        }
        needsSelection={needsSelection}
        loading={read.isFetching}
        onApply={() => {
          saveScope();
          load(0, null);
          setDrawer(null);
        }}
      />
      <GraphPathDrawer
        opened={drawer === "path"}
        onClose={() => setDrawer(null)}
        readable={!!readable}
        start={start}
        end={end}
        direction={direction}
        hops={hops}
        onStartChange={(value) => {
          setStart(value);
          editPath();
        }}
        onEndChange={(value) => {
          setEnd(value);
          editPath();
        }}
        onDirectionChange={(value) => {
          setDirection(value);
          editPath();
        }}
        onHopsChange={(value) => {
          setHops(value);
          editPath();
        }}
        onFind={findPath}
        isFetching={path.isFetching}
        error={path.error}
        data={path.data}
        submitted={!!pathSubmitted}
        renderNode={renderNode}
      />
      <GraphEntitiesDrawer
        opened={drawer === "entities"}
        onClose={() => setDrawer(null)}
        readable={!!readable}
        expired={entitiesExpired}
        isError={entities.isError}
        isPending={entities.isPending}
        error={entities.error}
        data={entities.data}
        memoryEpoch={status.data?.memory_epoch}
        setEntityOffset={setEntityOffset}
        renderNode={renderNode}
      />
      <Drawer
        className="feature-drawer"
        opened={drawer === "insights"}
        onClose={() => setDrawer(null)}
        position="right"
        title="Graph insights"
        size="lg"
      >
        {drawer === "insights" && (
          <AnalyticsPanel brain={brain} scope={scope} inspect={inspect} />
        )}
      </Drawer>
      <GraphStatusDrawer
        opened={drawer === "status"}
        onClose={() => setDrawer(null)}
        brain={brain}
        scene={scene}
        scope={scope}
        status={status}
        rebuild={rebuild}
        offset={offset}
        setOffset={setOffset}
      />
      <GraphDetailDialogs
        brain={brain}
        detail={detail}
        onSaved={() => {
          clear();
          void status.refetch();
        }}
        onClose={() => setDetail(null)}
      />
    </section>
  );
}
