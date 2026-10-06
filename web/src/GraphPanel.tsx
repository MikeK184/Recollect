import { iconSize } from "./design/tokens";
import { useEffect, useRef, useState } from "react";
import { useBrainSearch, type BrainSearch } from "./app/useBrainSearch";
import {
  Network,
  SlidersHorizontal,
  Route,
  ChartNoAxesCombined,
  Info,
  List,
  MoreHorizontal,
} from "lucide-react";
import { EmptyState } from "./components/AsyncState";
import "./features/feature-views.css";
import {
  Accordion,
  Alert,
  Badge,
  Button,
  Card,
  Drawer,
  SegmentedControl,
  Group,
  Loader,
  MultiSelect,
  Menu,
  NumberInput,
  Select,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { ClaimDialog, EvidenceDialog } from "./ClaimsPanel";
import { useIdempotency } from "./useIdempotency";
import { AnalyticsPanel } from "./AnalyticsPanel";
import { GraphExplorer } from "./GraphExplorer";
import { useContentDeadline } from "./useContentDeadline";

type Scope = components["schemas"]["GraphSelection"];
type Node = components["schemas"]["GraphNode"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
const label = (s: string) => s.replaceAll("_", " ");
const evidenceModes = [
  { value: "investigation", label: "Investigation with qualifications" },
  { value: "strict_accepted", label: "Accepted claims only" },
  { value: "strict_operational", label: "Verified operational claims only" },
];
function localTime(value: string | null | undefined) {
  if (!value) return "";
  const time = new Date(value);
  time.setMinutes(time.getMinutes() - time.getTimezoneOffset());
  return time.toISOString().slice(0, 16);
}
const initial = (): Scope => ({
  kind: "knowledge",
  snapshot_id: null,
  selection: { repository_ids: [], area_ids: [], environment_id: null },
  operation_id: null,
  collection_id: null,
  manifest_revision_id: null,
  fact_at: null,
  mode: "investigation",
  relations: [],
});
function scopeFromRoute(route: BrainSearch): Scope {
  return {
    ...initial(),
    kind: ["knowledge", "repository", "combined"].includes(route.kind ?? "")
      ? route.kind!
      : "knowledge",
    snapshot_id: route.snapshot ?? null,
    manifest_revision_id: route.manifest ?? null,
    fact_at: route.fact ?? null,
    collection_id: route.collection ?? null,
    mode: route.mode ?? "investigation",
    selection: {
      repository_ids:
        route.repositories ?? (route.repository ? [route.repository] : []),
      area_ids: route.areas ?? (route.area ? [route.area] : []),
      environment_id: route.environment ?? null,
    },
    relations: route.relations ?? [],
  };
}
function missingExactSelection(scope: Scope) {
  return (
    (scope.kind === "repository" &&
      (!scope.selection.repository_ids?.length ||
        (!scope.snapshot_id && !scope.manifest_revision_id))) ||
    (scope.kind === "combined" &&
      (!scope.selection.environment_id || !scope.manifest_revision_id))
  );
}
const meanings: Record<string, string> = {
  supported_by:
    "The claim cites this evidence; citation does not establish acceptance.",
  contributed_to:
    "This claim revision is a required contribution to the handover.",
  terraform_module:
    "The parsed module source names this exact repository commit and directory. This is a declared dependency.",
  declares:
    "Static extraction records a declaration from this entity to the target.",
  calls: "Static extraction records a call from the source to the target.",
  names: "The source names the target; this is a reference, not a call.",
};
function Failure({ error }: { error: Error | null }) {
  return error ? <Alert color="red">{error.message}</Alert> : null;
}
export function GraphPanel({ brain }: { brain: Brain }) {
  const cache = useQueryClient();
  const [route, patchRoute] = useBrainSearch();
  const [scope, setScope] = useState(() => scopeFromRoute(route));
  const routeScopeKey = JSON.stringify(scopeFromRoute(route));
  const scopeKey = JSON.stringify(scope);
  const [drawer, setDrawer] = useState<
    "filters" | "path" | "insights" | "status" | "entities" | null
  >(null);
  const initialRead = useRef<string | null>(null);
  const [entityOffset, setEntityOffset] = useState(0);
  const [offset, setOffset] = useState(0);
  const [snapshotOffset, setSnapshotOffset] = useState(0);
  const [manifestOffset, setManifestOffset] = useState(0);
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [direction, setDirection] = useState("outgoing");
  const [hops, setHops] = useState(6);
  const [submitted, setSubmitted] = useState<{
    nonce: string;
    scope: Scope;
    offset: number;
  } | null>(null);
  const [pathSubmitted, setPathSubmitted] = useState<{
    nonce: string;
    request: components["schemas"]["GraphPathRequest"];
  } | null>(null);
  const [detail, setDetail] = useState<{
    claim?: string;
    evidence?: Evidence;
  } | null>(null);
  const idem = useIdempotency();
  function clear() {
    setSubmitted(null);
    setPathSubmitted(null);
    setDetail(null);
    void cache.cancelQueries({ queryKey: ["graph-read", brain.id] });
    void cache.cancelQueries({ queryKey: ["graph-path", brain.id] });
    void cache.cancelQueries({ queryKey: ["graph-explore", brain.id] });
    void cache.cancelQueries({ queryKey: ["graph-entities", brain.id] });
  }
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
  useEffect(
    () =>
      cache.getQueryCache().subscribe((event) => {
        if (
          event.query.queryKey[1] === brain.id &&
          event.type === "updated" &&
          (event.action.type === "invalidate" ||
            (event.action.type === "setState" &&
              event.query.state.data === undefined &&
              ["graph", "graph-read", "graph-path"].includes(
                String(event.query.queryKey[0]),
              )))
        ) {
          setSubmitted(null);
          setPathSubmitted(null);
          setDetail(null);
          void cache.cancelQueries({ queryKey: ["graph-read", brain.id] });
          void cache.cancelQueries({ queryKey: ["graph-path", brain.id] });
          void cache.cancelQueries({ queryKey: ["graph-explore", brain.id] });
          void cache.cancelQueries({ queryKey: ["graph-entities", brain.id] });
        }
      }),
    [brain.id, cache],
  );
  const status = useQuery({
    queryKey: ["graph", brain.id, offset],
    refetchInterval: 3000,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/graph", {
          params: { path: { brain: brain.id }, query: { offset } },
        }),
      ),
  });
  useEffect(() => {
    clear();
  }, [status.data?.memory_epoch, status.data?.link_epoch]);
  useEffect(() => {
    if (status.error) {
      clear();
    }
  }, [status.error]);
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id, "graph"],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
        }),
      ),
  });
  const groups = useQuery({
    queryKey: ["evidence", brain.id, "graph"],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          params: { path: { brain: brain.id } },
        }),
      ),
  });
  const repository = (scope.selection.repository_ids ?? [])[0] ?? "";
  const snapshots = useQuery({
    queryKey: ["graph-snapshots", brain.id, repository, snapshotOffset],
    enabled: scope.kind === "repository" && !!repository,
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/repositories/{repository}/snapshots",
          {
            params: {
              path: { brain: brain.id, repository },
              query: { offset: snapshotOffset },
            },
          },
        ),
      ),
  });
  const manifests = useQuery({
    queryKey: [
      "graph-manifests",
      brain.id,
      scope.selection.environment_id,
      manifestOffset,
    ],
    enabled: !!scope.selection.environment_id,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/revision-manifests", {
          params: {
            path: { brain: brain.id },
            query: {
              environment_id: scope.selection.environment_id ?? undefined,
              offset: manifestOffset,
            },
          },
        }),
      ),
  });
  const read = useQuery({
    queryKey: ["graph-read", brain.id, submitted],
    enabled: false,
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/view", {
          params: { path: { brain: brain.id } },
          body: { scope: submitted!.scope, offset: submitted!.offset },
          signal,
        }),
      ),
  });
  const entities = useQuery({
    queryKey: [
      "graph-entities",
      brain.id,
      submitted,
      entityOffset,
      status.data?.memory_epoch,
      status.data?.link_epoch,
    ],
    enabled:
      drawer === "entities" &&
      !!submitted &&
      !!read.data &&
      !read.isError &&
      !status.isError,
    gcTime: 0,
    retry: false,
    refetchInterval: drawer === "entities" ? 3000 : false,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/view", {
          params: { path: { brain: brain.id } },
          body: { scope: submitted!.scope, offset: entityOffset },
          signal,
        }),
      ),
  });
  const entitiesExpired = useContentDeadline(entities.data?.expires_at);
  const readExpired = useContentDeadline(read.data?.expires_at);
  useEffect(() => setEntityOffset(0), [scopeKey]);
  const path = useQuery({
    queryKey: ["graph-path", brain.id, pathSubmitted],
    enabled: false,
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/path", {
          params: { path: { brain: brain.id } },
          body: pathSubmitted!.request,
          signal,
        }),
      ),
  });
  function load(page = 0) {
    if (missingExactSelection(scope)) return;
    initialRead.current = scopeKey;
    const next = {
      nonce: crypto.randomUUID(),
      scope: structuredClone(scope),
      offset: page,
    };
    setSubmitted(next);
    setPathSubmitted(null);
    setDetail(null);
    void cache
      .fetchQuery({
        queryKey: ["graph-read", brain.id, next],
        retry: false,
        gcTime: 0,
        queryFn: async ({ signal }) =>
          result(
            await client.POST("/api/brains/{brain}/graph/view", {
              params: { path: { brain: brain.id } },
              body: { scope: next.scope, offset: page },
              signal,
            }),
          ),
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
  const rebuild = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/brains/{brain}/graph/rebuild", {
          params: { path: { brain: brain.id } },
          body: {
            kind: scope.kind,
            snapshot_id: scope.kind === "repository" ? scope.snapshot_id : null,
            manifest_revision_id:
              scope.kind === "combined" ? scope.manifest_revision_id : null,
          },
          headers: {
            "Idempotency-Key": idem.forInput({
              brain: brain.id,
              kind: scope.kind,
              snapshot: scope.snapshot_id,
              manifest:
                scope.kind === "combined" ? scope.manifest_revision_id : null,
            }),
          },
        }),
      ),
    onSuccess: () => {
      idem.reset();
      clear();
      void cache.invalidateQueries({ queryKey: ["graph", brain.id] });
      void cache.invalidateQueries({ queryKey: ["jobs", brain.id] });
    },
  });
  function inspect(n: Node) {
    if (n.evidence.kind === "claim") {
      setDetail({ claim: n.evidence.id });
      return;
    }
    const p = n.evidence.provenance[0];
    if (!p) return;
    setDetail({
      evidence: {
        kind: p.kind,
        id: p.id,
        label: n.evidence.label,
        source_id: p.source_id,
        repository_id: p.repository_id,
        snapshot_id: p.snapshot_id,
        revision: p.revision,
        reference: null,
        observed_at: null,
        created_at: n.evidence.recorded_at,
        availability: p.availability,
      },
    });
  }
  function node(n: Node, choose: boolean) {
    return (
      <Card
        key={n.key}
        withBorder
        p="sm"
        data-testid={choose ? "graph-entity" : "graph-path-entity"}
      >
        <Stack gap="xs">
          <Group justify="space-between">
            <Text fw={600} style={{ overflowWrap: "anywhere", minWidth: 0 }}>
              {n.evidence.label}
            </Text>
            <Badge>{label(n.evidence.kind)}</Badge>
          </Group>
          <Text size="xs" style={{ overflowWrap: "anywhere" }}>
            {n.key}
          </Text>
          {n.evidence.provenance[0]?.repository_id && (
            <Text size="sm" style={{ overflowWrap: "anywhere" }}>
              {catalogue.data?.repositories.find(
                (r) => r.id === n.evidence.provenance[0]?.repository_id,
              )?.canonical_origin ?? n.evidence.provenance[0].repository_id}
              {n.evidence.provenance[0].path
                ? ` · ${n.evidence.provenance[0].path}`
                : ""}
            </Text>
          )}
          {n.evidence.provenance[0]?.revision && (
            <Text size="xs" style={{ overflowWrap: "anywhere" }}>
              Commit: {n.evidence.provenance[0].revision}
            </Text>
          )}
          {n.evidence.claim && (
            <Text size="xs">
              {label(n.evidence.claim.review)} ·{" "}
              {label(n.evidence.claim.freshness)} ·{" "}
              {label(n.evidence.claim.operational)}
            </Text>
          )}
          <Text size="xs" c="dimmed">
            {n.evidence.qualifications.map(label).join(" · ")}
          </Text>
          <Group>
            <Button size="xs" variant="light" onClick={() => inspect(n)}>
              Inspect evidence
            </Button>
            {choose && (
              <>
                <Button
                  size="xs"
                  variant="subtle"
                  onClick={() => {
                    setStart(n.key);
                    setPathSubmitted(null);
                    setDetail(null);
                  }}
                >
                  Use as start
                </Button>
                <Button
                  size="xs"
                  variant="subtle"
                  onClick={() => {
                    setEnd(n.key);
                    setPathSubmitted(null);
                    setDetail(null);
                  }}
                >
                  Use as end
                </Button>
              </>
            )}
          </Group>
        </Stack>
      </Card>
    );
  }
  const availableGroups = groups.data?.groups ?? [];
  const readable =
    submitted && read.data && !read.error && !status.error && !readExpired
      ? read.data
      : undefined;
  const needsSelection = missingExactSelection(scope);
  return (
    <section
      className="feature-view graph-surface"
      aria-label="Evidence graphs"
    >
      {/* Canvas chrome: kind selection, filters, path, insights, status and
          entity pages live in one compact row above the canvas region. */}
      <div className="feature-toolbar graph-chrome" data-graph-chrome="">
        <SegmentedControl
          aria-label="Graph kind"
          value={scope.kind}
          data={[
            { value: "knowledge", label: "Knowledge" },
            { value: "repository", label: "Repository" },
            { value: "combined", label: "Combined" },
          ]}
          onChange={(kind) => {
            change({ ...initial(), kind });
            setDrawer("filters");
          }}
        />
        <Button
          variant="default"
          leftSection={<SlidersHorizontal size={iconSize.small} />}
          onClick={() => setDrawer("filters")}
        >
          Filters
        </Button>
        <Menu position="bottom-end" withinPortal>
          <Menu.Target>
            <Button
              variant="subtle"
              ml="auto"
              leftSection={<MoreHorizontal size={iconSize.small} />}
            >
              Graph tools
            </Button>
          </Menu.Target>
          <Menu.Dropdown>
            <Menu.Item
              leftSection={<Route size={iconSize.small} />}
              onClick={() => setDrawer("path")}
              disabled={!readable}
            >
              Find path
            </Menu.Item>
            <Menu.Item
              leftSection={<ChartNoAxesCombined size={iconSize.small} />}
              onClick={() => setDrawer("insights")}
            >
              Insights
            </Menu.Item>
            <Menu.Item
              leftSection={<Info size={iconSize.small} />}
              onClick={() => setDrawer("status")}
            >
              Graph status
            </Menu.Item>
            <Menu.Item
              leftSection={<List size={iconSize.small} />}
              onClick={() => setDrawer("entities")}
            >
              Entities
            </Menu.Item>
          </Menu.Dropdown>
        </Menu>
      </div>
      <div className="feature-scope graph-chrome" data-graph-chrome="">
        <Badge variant="light" color="gray">
          {scope.mode === "investigation"
            ? "Include uncertainties"
            : (evidenceModes.find((mode) => mode.value === scope.mode)?.label ??
              label(scope.mode))}
        </Badge>
        <Text size="xs" c="dimmed">
          {(scope.selection.repository_ids ?? []).length
            ? `${(scope.selection.repository_ids ?? []).length} selected repositories`
            : "All repositories in this Brain"}
        </Text>
        {scope.selection.environment_id && (
          <Badge variant="light">
            {availableGroups.find(
              (group) => group.id === scope.selection.environment_id,
            )?.name ?? "Selected environment"}
          </Badge>
        )}
        {scope.snapshot_id && (
          <Badge variant="outline">
            Snapshot {scope.snapshot_id.slice(0, 8)}
          </Badge>
        )}
        {scope.manifest_revision_id && (
          <Badge variant="outline">
            Manifest {scope.manifest_revision_id.slice(0, 8)}
          </Badge>
        )}
        {scope.fact_at && (
          <Badge variant="light">
            Fact time {new Date(scope.fact_at).toLocaleString()}
          </Badge>
        )}
        {!!(scope.selection.area_ids ?? []).length && (
          <Badge variant="light">
            {(scope.selection.area_ids ?? []).length} areas
          </Badge>
        )}
        {scope.collection_id && (
          <Badge variant="light">
            {availableGroups.find((group) => group.id === scope.collection_id)
              ?.name ?? "Selected collection"}
          </Badge>
        )}
        {!!scope.relations.length && (
          <Badge variant="light">{scope.relations.map(label).join(", ")}</Badge>
        )}
      </div>
      <Failure error={status.error ?? read.error} />
      {read.isFetching && (
        <Group role="status" my="lg">
          <Loader size="sm" />
          <Text>Reading eligible graph evidence…</Text>
        </Group>
      )}
      {!readable && !read.isFetching && (
        <EmptyState
          icon={Network}
          title={
            needsSelection
              ? "Choose an exact graph view"
              : read.error
                ? "This graph view is unavailable"
                : "Explore the evidence"
          }
          description={
            needsSelection
              ? "Choose a repository snapshot or exact environment manifest in Filters. Recollect will keep that revision selection explicit."
              : "Load a bounded view of eligible evidence. If a graph is too large, narrow its repositories, areas or relationships."
          }
          action={
            <Button
              onClick={() => (needsSelection ? setDrawer("filters") : load())}
            >
              {needsSelection ? "Choose inputs" : "Load graph view"}
            </Button>
          }
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
            initialHops={route.hops ?? 2}
            onExplore={(center, direction, hops) =>
              patchRoute({ center, direction, hops })
            }
            expectedMemoryEpoch={status.data?.memory_epoch}
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
      <Drawer
        className="feature-drawer"
        opened={drawer === "filters"}
        onClose={() => setDrawer(null)}
        position="right"
        title="Graph filters"
        size="lg"
      >
        <Stack>
          <SimpleGrid cols={{ base: 1, sm: 2 }}>
            <Select
              label="Graph kind"
              value={scope.kind}
              data={[
                {
                  value: "knowledge",
                  label: "Knowledge and supporting evidence",
                },
                { value: "repository", label: "Repository structure" },
                { value: "combined", label: "Combined repositories" },
              ]}
              onChange={(kind) =>
                change({ ...initial(), kind: kind ?? "knowledge" })
              }
            />
            <Select
              label="Graph evidence mode"
              value={scope.mode}
              data={evidenceModes}
              onChange={(mode) =>
                change({ ...scope, mode: mode ?? "investigation" })
              }
            />
            <MultiSelect
              label="Graph repositories"
              searchable
              value={scope.selection.repository_ids}
              maxValues={scope.kind === "repository" ? 1 : 100}
              data={(catalogue.data?.repositories ?? []).map((r) => ({
                value: r.id,
                label: r.canonical_origin,
              }))}
              onChange={(ids) => {
                setSnapshotOffset(0);
                change({
                  ...scope,
                  snapshot_id: null,
                  selection: { ...scope.selection, repository_ids: ids },
                });
              }}
            />
            <TextInput
              label="Graph fact time"
              description="Optional applicability time in your local timezone."
              type="datetime-local"
              value={localTime(scope.fact_at)}
              onChange={(event) =>
                change({
                  ...scope,
                  fact_at: event.currentTarget.value
                    ? new Date(event.currentTarget.value).toISOString()
                    : null,
                })
              }
            />
            {scope.kind === "repository" && (
              <Stack gap="xs">
                <Select
                  label="Graph snapshot"
                  searchable
                  clearable
                  value={scope.snapshot_id}
                  data={(snapshots.data?.items ?? []).map((s) => ({
                    value: s.id,
                    label: `${s.revision.slice(0, 12)} · ${new Date(s.created_at).toLocaleString()} · ${s.processing}`,
                  }))}
                  onChange={(id) => change({ ...scope, snapshot_id: id })}
                />
                {snapshots.data && snapshots.data.total > 20 && (
                  <Group>
                    <Button
                      size="xs"
                      disabled={!snapshotOffset}
                      onClick={() => setSnapshotOffset((n) => n - 20)}
                    >
                      Previous snapshots
                    </Button>
                    <Button
                      size="xs"
                      disabled={snapshotOffset + 20 >= snapshots.data.total}
                      onClick={() => setSnapshotOffset((n) => n + 20)}
                    >
                      More snapshots
                    </Button>
                  </Group>
                )}
              </Stack>
            )}
            <Select
              label="Graph environment"
              clearable
              value={scope.selection.environment_id}
              data={availableGroups
                .filter((g) => g.kind === "environment")
                .map((g) => ({ value: g.id, label: g.name }))}
              onChange={(id) => {
                setManifestOffset(0);
                change({
                  ...scope,
                  manifest_revision_id: null,
                  selection: { ...scope.selection, environment_id: id },
                });
              }}
            />
            {scope.selection.environment_id && (
              <Stack gap="xs">
                <Select
                  label="Graph manifest"
                  clearable
                  value={scope.manifest_revision_id}
                  data={(manifests.data?.items ?? []).map((m) => ({
                    value: m.id,
                    label: `${m.name} · ${label(m.kind)} · ${m.id.slice(0, 8)}`,
                  }))}
                  onChange={(id) =>
                    change({ ...scope, manifest_revision_id: id })
                  }
                />
                {manifests.data && manifests.data.total > 20 && (
                  <Group>
                    <Button
                      size="xs"
                      disabled={!manifestOffset}
                      onClick={() => setManifestOffset((n) => n - 20)}
                    >
                      Previous manifests
                    </Button>
                    <Button
                      size="xs"
                      disabled={manifestOffset + 20 >= manifests.data.total}
                      onClick={() => setManifestOffset((n) => n + 20)}
                    >
                      More manifests
                    </Button>
                  </Group>
                )}
              </Stack>
            )}
            <MultiSelect
              label="Graph areas"
              searchable
              value={scope.selection.area_ids}
              data={availableGroups
                .filter((g) => g.kind === "area")
                .map((g) => ({ value: g.id, label: g.name }))}
              onChange={(ids) =>
                change({
                  ...scope,
                  selection: { ...scope.selection, area_ids: ids },
                })
              }
            />
            {scope.kind === "knowledge" && (
              <Select
                label="Graph collection"
                clearable
                value={scope.collection_id}
                data={availableGroups
                  .filter((g) => g.kind === "collection")
                  .map((g) => ({ value: g.id, label: g.name }))}
                onChange={(id) => change({ ...scope, collection_id: id })}
              />
            )}
            <MultiSelect
              label="Graph relations"
              searchable
              value={scope.relations}
              data={(scope.kind === "knowledge"
                ? ["supported_by", "contributed_to"]
                : [
                    "declares",
                    "imports",
                    "calls",
                    "implements",
                    "depends_on",
                    "instantiates",
                    "injects",
                    "has_method",
                    "handled_by",
                    "implemented_by",
                    "names",
                    ...(scope.kind === "combined" ? ["terraform_module"] : []),
                  ]
              ).map((value) => ({ value, label: label(value) }))}
              onChange={(relations) => change({ ...scope, relations })}
            />
          </SimpleGrid>
          <Failure
            error={
              catalogue.error ??
              groups.error ??
              snapshots.error ??
              manifests.error
            }
          />

          <Text size="sm" c="dimmed">
            Selected revisions stay exact. A path follows only the selected
            evidence and relation types; committed structure does not prove
            deployed behavior.
          </Text>
          <Button
            onClick={() => {
              saveScope();
              load();
              setDrawer(null);
            }}
            loading={read.isFetching}
            disabled={needsSelection}
          >
            Apply graph filters
          </Button>
        </Stack>
      </Drawer>
      <Drawer
        className="feature-drawer"
        opened={drawer === "path"}
        onClose={() => setDrawer(null)}
        position="right"
        title="Find an evidence path"
        size="lg"
      >
        {readable ? (
          <Stack>
            <SimpleGrid cols={{ base: 1, sm: 2 }}>
              <TextInput
                label="Path start"
                value={start}
                onChange={(e) => {
                  setStart(e.currentTarget.value);
                  setPathSubmitted(null);
                  setDetail(null);
                }}
              />
              <TextInput
                label="Path end"
                value={end}
                onChange={(e) => {
                  setEnd(e.currentTarget.value);
                  setPathSubmitted(null);
                  setDetail(null);
                }}
              />
              <Select
                label="Path direction"
                value={direction}
                data={["outgoing", "incoming", "both"]}
                onChange={(value) => {
                  setDirection(value ?? "outgoing");
                  setPathSubmitted(null);
                  setDetail(null);
                }}
              />
              <NumberInput
                label="Maximum path hops"
                min={1}
                max={8}
                value={hops}
                onChange={(n) => {
                  setHops(Number(n) || 1);
                  setPathSubmitted(null);
                  setDetail(null);
                }}
              />
            </SimpleGrid>
            <Button
              onClick={findPath}
              loading={path.isFetching}
              disabled={!start || !end}
            >
              Find shortest eligible path
            </Button>
            <Failure error={path.error} />
            {pathSubmitted && path.data && !path.error && (
              <Card withBorder p="md">
                <Stack>
                  <Title order={4}>
                    {path.data.status === "path"
                      ? `${path.data.edges.length} hops in the eligible graph`
                      : "No path within the selected hop bound"}
                  </Title>
                  {path.data.nodes.map((n, i) => (
                    <Stack key={`${n.key}-${i}`} gap="xs">
                      {node(n, false)}
                      {path.data?.edges[i] && (
                        <Alert color="blue">
                          <Text fw={600}>
                            {label(path.data.edges[i].relation)} ·{" "}
                            {label(path.data.edges[i].family)}
                          </Text>
                          <Text size="sm" style={{ overflowWrap: "anywhere" }}>
                            Recorded direction:{" "}
                            {
                              path.data.nodes.find(
                                (node) => node.key === path.data!.edges[i].from,
                              )?.evidence.label
                            }
                            {" → "}
                            {
                              path.data.nodes.find(
                                (node) => node.key === path.data!.edges[i].to,
                              )?.evidence.label
                            }
                          </Text>
                          {path.data.edges[i].from !== n.key && (
                            <Text size="sm">
                              Followed in reverse for this path.
                            </Text>
                          )}
                          <Text size="sm">
                            {meanings[path.data.edges[i].relation] ??
                              `A static ${label(path.data.edges[i].relation)} relationship in the selected direction.`}
                          </Text>
                        </Alert>
                      )}
                    </Stack>
                  ))}
                </Stack>
              </Card>
            )}
          </Stack>
        ) : (
          <Text>Load a graph view first, then choose its path endpoints.</Text>
        )}
      </Drawer>
      <Drawer
        className="feature-drawer"
        opened={drawer === "entities"}
        onClose={() => setDrawer(null)}
        position="right"
        title="Entities"
        size="lg"
      >
        {entities.isError && <Failure error={entities.error} />}
        {readable && entities.isPending && <Loader size="sm" />}
        {(!readable || entitiesExpired) && (
          <Text size="sm" c="dimmed">
            Reload the graph to inspect current eligible entities.
          </Text>
        )}
        {readable &&
          !entitiesExpired &&
          !entities.isError &&
          entities.data &&
          entities.data.memory_epoch === status.data?.memory_epoch && (
            <Stack>
              <Text size="xs" c="dimmed">
                All eligible entities in the selected scope. Paging does not
                replace the displayed canvas.
              </Text>
              <SimpleGrid cols={{ base: 1, md: 2 }}>
                {entities.data.nodes.map((n) => node(n, true))}
              </SimpleGrid>
              {entities.data.total_nodes > 100 && (
                <Group>
                  <Button
                    disabled={!entities.data.offset}
                    onClick={() => setEntityOffset(entities.data!.offset - 100)}
                  >
                    Previous entities
                  </Button>
                  <Text size="sm">
                    {entities.data.offset + 1}–
                    {Math.min(
                      entities.data.offset + 100,
                      entities.data.total_nodes,
                    )}{" "}
                    of {entities.data.total_nodes}
                  </Text>
                  <Button
                    disabled={
                      entities.data.offset + 100 >= entities.data.total_nodes
                    }
                    onClick={() => setEntityOffset(entities.data!.offset + 100)}
                  >
                    More entities
                  </Button>
                </Group>
              )}
            </Stack>
          )}
      </Drawer>
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
      <Drawer
        className="feature-drawer"
        opened={drawer === "status"}
        onClose={() => setDrawer(null)}
        position="right"
        title="Graph status and maintenance"
        size="lg"
      >
        <Stack>
          {readable && (
            <>
              <Group>
                <Badge>{label(readable.state)}</Badge>
                <Text size="sm">
                  {readable.total_nodes} eligible entities ·{" "}
                  {readable.total_edges} eligible relationships
                </Text>
              </Group>
              <Text size="xs" style={{ overflowWrap: "anywhere" }}>
                Generation {readable.generation.id} · input epoch{" "}
                {readable.generation.input_epoch} ·{" "}
                {scope.kind === "combined" ? "origin epoch" : "memory epoch"}{" "}
                {scope.kind === "combined"
                  ? status.data?.link_epoch
                  : readable.memory_epoch}
              </Text>
              {scope.kind === "combined" && (
                <Stack gap="xs">
                  <Text fw={600}>Exact repository inputs</Text>
                  {readable.inputs.map((input) => (
                    <Text
                      key={input.id}
                      size="xs"
                      style={{ overflowWrap: "anywhere" }}
                    >
                      Snapshot {input.snapshot_id} · generation {input.id}
                    </Text>
                  ))}
                  {!!readable.link_issues_total && (
                    <Alert color="yellow" title="Unresolved repository links">
                      <Text size="sm">
                        Showing {readable.link_issues.length} of{" "}
                        {readable.link_issues_total} eligible source records.
                      </Text>
                      {readable.link_issues.map((issue) => (
                        <Text
                          key={issue.source}
                          size="xs"
                          style={{ overflowWrap: "anywhere" }}
                        >
                          {label(issue.code)} · {issue.source}
                        </Text>
                      ))}
                    </Alert>
                  )}
                </Stack>
              )}
              {readable.coverage.partial && (
                <Alert color="yellow">
                  {readable.coverage.reasons.map(label).join(" · ")}
                </Alert>
              )}
              {!readable.total_nodes && (
                <Text>No entities are eligible in this view.</Text>
              )}
            </>
          )}
          {brain.role !== "reader" && (
            <Button
              variant="default"
              loading={rebuild.isPending}
              disabled={
                brain.archived ||
                (scope.kind === "repository" && !scope.snapshot_id) ||
                (scope.kind === "combined" && !scope.manifest_revision_id)
              }
              onClick={() => rebuild.mutate()}
            >
              Rebuild graph
            </Button>
          )}
          <Failure error={rebuild.error} />
          {rebuild.isSuccess && (
            <Alert color="teal">
              Graph rebuild queued. Progress appears below.
            </Alert>
          )}
          <Title order={4}>Projection processing</Title>
          <Failure error={status.error} />
          {status.isPending && <Loader size="sm" />}
          {status.data?.generations.length === 0 && (
            <Text size="sm">
              No graph generation yet. The worker discovers materialized
              repositories and claim evidence automatically.
            </Text>
          )}
          {status.data?.generations.map((g) => (
            <Card withBorder p="sm" key={g.id}>
              <Stack gap="xs">
                <Group justify="space-between">
                  <Text fw={600}>{label(g.kind)} graph</Text>
                  <Badge>{label(g.state)}</Badge>
                </Group>
                <Text size="xs" style={{ overflowWrap: "anywhere" }}>
                  Generation {g.id}
                  {g.snapshot_id ? ` · snapshot ${g.snapshot_id}` : ""}
                  {g.kind === "combined"
                    ? ` · ${g.input_snapshot_ids?.length ?? 0} exact repository inputs · ${g.adapter}`
                    : ""}
                </Text>
                <Text size="sm">
                  {g.node_count} entities · {g.edge_count} relationships ·{" "}
                  {g.unresolved} unresolved · {g.ambiguous} ambiguous ·{" "}
                  {g.unsupported} unsupported
                </Text>
                <Text size="xs">
                  {new Date(g.created_at).toLocaleString()} ·{" "}
                  {g.kind === "knowledge" &&
                  g.input_epoch !== status.data?.memory_epoch
                    ? "Knowledge input changed; reads use current eligibility."
                    : g.kind === "combined" &&
                        g.input_epoch !== status.data?.link_epoch
                      ? "Origin bindings changed; current links are being rebuilt."
                      : "Exact input retained."}
                </Text>
                {g.error_code && (
                  <Alert color="red">{label(g.error_code)}</Alert>
                )}
                {status.data?.jobs.find((j) => j.id === g.job_id) && (
                  <Text size="xs">
                    Job:{" "}
                    {label(
                      status.data.jobs.find((j) => j.id === g.job_id)!.state,
                    )}{" "}
                    ·{" "}
                    {status.data.jobs.find((j) => j.id === g.job_id)!.progress}%
                  </Text>
                )}
              </Stack>
            </Card>
          ))}
          {status.data && status.data.total > 20 && (
            <Group>
              <Button
                disabled={!offset}
                onClick={() => setOffset((n) => n - 20)}
              >
                Previous generations
              </Button>
              <Button
                disabled={offset + 20 >= status.data.total}
                onClick={() => setOffset((n) => n + 20)}
              >
                More generations
              </Button>
            </Group>
          )}
        </Stack>
      </Drawer>
      {detail?.claim && (
        <ClaimDialog
          brain={brain}
          id={detail.claim}
          onSaved={() => {
            clear();
            void status.refetch();
          }}
          onClose={() => setDetail(null)}
        />
      )}
      {detail?.evidence && (
        <EvidenceDialog
          brain={brain}
          evidence={detail.evidence}
          returnLabel="Back to graph"
          onClose={() => setDetail(null)}
        />
      )}
    </section>
  );
}
