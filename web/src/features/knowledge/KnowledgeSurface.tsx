import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  Alert,
  Badge,
  Button,
  CopyButton,
  Divider,
  Group,
  Loader,
  Portal,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { Check, Copy } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Link,
  Outlet,
  useNavigate,
  useRouterState,
} from "@tanstack/react-router";
import "../feature-views.css";
import "./knowledge-inspector.css";
import {
  defaultKnowledgeView,
  resolveKnowledgeView,
  type KnowledgeSection,
} from "../../app/navigation";
import { useBrain } from "../../app/context";
import { useBrainSearch, type BrainSearch } from "../../app/useBrainSearch";
import { DetailInspector } from "../../components/DetailInspector";
import { useContentDeadline } from "../../useContentDeadline";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
const SourceInspector = lazy(() =>
  import("../../EvidencePanel").then((m) => ({ default: m.SourceInspector })),
);
const AliasDialog = lazy(() =>
  import("../../WorkspacePanel").then((m) => ({ default: m.AliasDialog })),
);
const RepositoryDialog = lazy(() =>
  import("../../PublicationPanel").then((m) => ({
    default: m.RepositoryDialog,
  })),
);
const ClaimEditor = lazy(() =>
  import("../../ClaimsPanel").then((m) => ({ default: m.ClaimEditor })),
);
const EvidenceDialog = lazy(() =>
  import("../../ClaimsPanel").then((m) => ({ default: m.EvidenceDialog })),
);
const ClaimReviewDialog = lazy(() =>
  import("../../ClaimReviewDialog").then((m) => ({
    default: m.ClaimReviewDialog,
  })),
);
const EraseAction = lazy(() =>
  import("../../RetentionPanel").then((m) => ({ default: m.EraseAction })),
);
import {
  KnowledgeSelectionContext,
  useKnowledgeSelection,
  useLineageOverlay,
  type KnowledgeSelection,
} from "./selection";

// The surface's view value is the contracted route suffix, so `/memory`,
// `/sources`, `/graph` and `/repositories` stay deep-linkable, reloadable and
// Back/forward safe while one shell hosts the switcher and the inspector.
export type KnowledgeView = KnowledgeSection | "explore";

export { defaultKnowledgeView, resolveKnowledgeView };

/** Legacy view names remain addressable even when their sidebar links move. */
export const knowledgeViews: readonly { view: KnowledgeView; label: string }[] =
  [
    { view: "memory", label: "Memory" },
    { view: "sources", label: "Sources" },
    { view: "graph", label: "Graph" },
    { view: "repositories", label: "Repositories" },
    { view: "explore", label: "Explore" },
  ];

/** The view the current route selects, resolved against the four known values. */
export function useKnowledgeView(): KnowledgeView {
  const path = useRouterState({ select: (state) => state.location.pathname });
  const [search] = useBrainSearch();
  if (path.split("/")[3] === "explore")
    return resolveKnowledgeView(search.view);
  return resolveKnowledgeView(path.split("/")[3]);
}

// Carry only canonical identities a destination can resolve. A graph key is
// never substituted for a claim/source ID. All other view-specific state clears.
function selectionSearch(
  selection: KnowledgeSelection,
  view: KnowledgeView,
  current: BrainSearch = {},
): BrainSearch {
  if (view === "explore") {
    if (selection?.kind === "memory")
      return {
        view: "memory",
        claim: selection.id,
        knowledge: current.knowledge,
        fact: current.fact,
        revision: current.revision,
      };
    if (selection?.kind === "source")
      return {
        view: "sources",
        source: selection.id,
        version: selection.version,
      };
    if (selection?.kind === "repository")
      return { view: "repositories", repository: selection.id };
    if (selection?.kind === "graph-node") {
      if (selection.key.startsWith("claim:"))
        return { view: "memory", center: selection.key };
      if (selection.key.startsWith("source_version:"))
        return {
          view: "sources",
          version: selection.key.slice("source_version:".length),
        };
    }
    return {};
  }
  if (selection?.kind === "memory" && (view === "memory" || view === "graph"))
    return view === "memory"
      ? {
          claim: selection.id,
          knowledge: current.knowledge,
          fact: current.fact,
          revision: current.revision,
        }
      : current.knowledge || current.revision
        ? {}
        : { claim: selection.id };
  if (selection?.kind === "source" && (view === "sources" || view === "graph"))
    return { source: selection.id, version: selection.version };
  if (selection?.kind === "graph-node") {
    if (
      view === "graph" ||
      (view === "memory" && selection.key.startsWith("claim:"))
    )
      return { center: selection.key };
    if (view === "sources" && selection.key.startsWith("source_version:"))
      return { version: selection.key.slice("source_version:".length) };
  }
  return {};
}

type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type View = components["schemas"]["ClaimView"];
const label = (s: string) => s.replaceAll("_", " ");
const time = (s: string) => new Date(s).toLocaleString();
const canonicalEvidenceKinds = [
  "source_version",
  "repository_file",
  "repository_fact",
  "manifest_revision",
] as const;

/** Brain-wide knowledge scope for bounded lineage reads. */
function knowledgeScope() {
  return {
    kind: "knowledge",
    snapshot_id: null,
    selection: { repository_ids: [], area_ids: [], environment_id: null },
    operation_id: null,
    collection_id: null,
    manifest_revision_id: null,
    fact_at: null,
    mode: "investigation",
    relations: [],
  };
}

/** One bounded read around a graph center; never an unbounded traversal. */
function useNeighbourhood(brain: Brain, center: string, enabled = true) {
  const query = useQuery({
    queryKey: ["lineage-neighbourhood", brain.id, center],
    enabled: enabled && !!center,
    retry: false,
    gcTime: 0,
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/explore", {
          signal,
          params: { path: { brain: brain.id } },
          body: {
            scope: knowledgeScope(),
            center,
            direction: "both",
            max_hops: 1,
          },
        }),
      ),
  });
  const expired = useContentDeadline(query.data?.expires_at);
  return {
    ...query,
    data: expired || query.isError ? undefined : query.data,
    expired,
  };
}

function Neighbourhood({ brain, center }: { brain: Brain; center: string }) {
  const { select } = useKnowledgeSelection();
  const query = useNeighbourhood(brain, center);
  if (query.expired)
    return (
      <Text size="sm" c="dimmed">
        Neighbourhood expired; waiting for a fresh read.
      </Text>
    );
  if (query.isPending) return <Loader size="sm" />;
  if (query.isError)
    return (
      <Alert color="red">
        {query.error.message}
        <Button
          variant="subtle"
          size="xs"
          mt="xs"
          onClick={() => void query.refetch()}
        >
          Retry neighbourhood
        </Button>
      </Alert>
    );
  // A missing generation surfaces as a request error above; an existing
  // generation may be "ready" or "partial" (coverage limits) and both carry
  // eligible nodes, exactly like the graph canvas renders them.
  const data = query.data;
  if (!data) return <Loader size="sm" />;
  const nodes = data.nodes.filter((node) => node.key !== center);
  // Node keys carry revision ids for claims; the graph-node selection
  // resolves them through the canonical claim read.
  const openNode = (key: string) => {
    const node = nodes.find((node) => node.key === key);
    select({
      kind: "graph-node",
      key,
      claimId: node?.evidence.kind === "claim" ? node.evidence.id : undefined,
      revision:
        node?.evidence.kind === "claim" ? node.evidence.revision_id : undefined,
    });
  };
  return (
    <Stack gap="xs">
      {nodes.length === 0 && (
        <Text size="sm" c="dimmed">
          No eligible neighbours within one hop.
        </Text>
      )}
      {nodes.map((node) => (
        <Group key={node.key} justify="space-between" gap="xs" wrap="nowrap">
          <Button
            variant="subtle"
            size="compact-xs"
            className="lineage-node-button"
            onClick={() => openNode(node.key)}
          >
            {node.evidence.label}
          </Button>
          <Badge size="xs" variant="light" color="gray">
            {label(node.evidence.kind)}
          </Badge>
        </Group>
      ))}
      {data.edges.length > 0 && (
        <Text size="xs" c="dimmed">
          {data.edges.length} recorded relationship
          {data.edges.length === 1 ? "" : "s"} in this bounded read.
        </Text>
      )}
    </Stack>
  );
}

function EvidenceList({
  brain,
  evidence,
  supports,
}: {
  brain: Brain;
  evidence: Evidence[];
  supports?: View["revision"]["content"]["supports"];
}) {
  const [selected, setSelected] = useState<Evidence | null>(null);
  useLineageOverlay(!!selected);
  return (
    <>
      <Stack gap="xs">
        {evidence.length === 0 && (
          <Text size="sm" c="dimmed">
            No supporting evidence is recorded for this revision.
          </Text>
        )}
        {evidence.map((item, index) => (
          <Button
            key={`${item.kind}:${item.id}`}
            variant="default"
            size="compact-xs"
            className="lineage-evidence-button"
            onClick={() => setSelected(item)}
          >
            {item.label} · {label(item.kind)} · {label(item.availability)}
            {supports?.[index]?.line_from
              ? ` · lines ${supports[index].line_from}–${supports[index].line_to}`
              : ""}
          </Button>
        ))}
      </Stack>
      {selected && (
        <EvidenceDialog
          brain={brain}
          evidence={selected}
          onClose={() => setSelected(null)}
          returnLabel="Back to lineage"
        />
      )}
    </>
  );
}

/** The canonical exact-evidence read for non-claim graph entities. */
function CanonicalEvidenceRead({
  brain,
  kind,
  id,
}: {
  brain: Brain;
  kind: (typeof canonicalEvidenceKinds)[number];
  id: string;
}) {
  const { select } = useKnowledgeSelection();
  const query = useQuery({
    queryKey: ["lineage-evidence", brain.id, kind, id],
    retry: false,
    gcTime: 0,
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/claim-evidence/{kind}/{evidence}",
          {
            signal,
            params: { path: { brain: brain.id, kind, evidence: id } },
          },
        ),
      ),
  });
  if (query.isPending) return <Loader size="sm" />;
  if (query.isError || !query.data)
    return (
      <Alert color="red">
        {query.error?.message ?? "This exact evidence is unavailable."}
        <Button
          variant="subtle"
          size="xs"
          mt="xs"
          onClick={() => void query.refetch()}
        >
          Retry evidence read
        </Button>
      </Alert>
    );
  const item = query.data.evidence;
  return (
    <Stack gap="xs">
      <Text fw={600} size="sm" className="lineage-wrap">
        {item.label}
      </Text>
      <Group gap="xs">
        <Badge size="xs">{label(item.kind)}</Badge>
        <Badge size="xs" variant="light" color="gray">
          {label(item.availability)}
        </Badge>
      </Group>
      {item.reference && (
        <Text size="sm" className="lineage-wrap">
          Reference: {item.reference}
        </Text>
      )}
      {item.revision && (
        <Text size="xs" c="dimmed" className="lineage-wrap">
          Exact revision: {item.revision}
        </Text>
      )}
      {kind === "source_version" && item.source_id && (
        <Button
          variant="light"
          size="compact-xs"
          onClick={() =>
            select({ kind: "source", id: item.source_id!, version: id })
          }
        >
          Open in Sources
        </Button>
      )}
    </Stack>
  );
}

/**
 * The shared claim lineage body. Claims reached from the Memory list and
 * claims reached through a graph node key both render this one definition;
 * `centerKey` is the exact graph node key (claim:<revision id>) used for
 * the bounded neighbourhood read, because graph keys carry revision ids.
 */
function ClaimLineageView({
  brain,
  view,
  centerKey,
}: {
  brain: Brain;
  view: View;
  centerKey: string;
}) {
  const [evidenceItem, setEvidenceItem] = useState<Evidence | null>(null);
  const [reviewing, setReviewing] = useState<View | null>(null);
  const queries = useQueryClient();
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id],
    enabled: !!reviewing,
    retry: false,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          signal,
          params: { path: { brain: brain.id } },
        }),
      ),
  });
  useLineageOverlay(!!evidenceItem || !!reviewing);
  const r = view.revision;
  const flags: string[] = [];
  if (r.lifecycle === "withdrawn") flags.push("Withdrawn");
  if (view.eligibility.rule_ids?.length) flags.push("Blocked by review rule");
  if (view.eligibility.conflicting_claim_ids?.length)
    flags.push("Unresolved conflict");
  return (
    <Stack gap="sm">
      {evidenceItem && (
        <EvidenceDialog
          brain={brain}
          evidence={evidenceItem}
          onClose={() => setEvidenceItem(null)}
          returnLabel="Back to lineage"
        />
      )}
      {reviewing && catalogue.isPending && (
        <Text role="status" size="sm">
          Loading correction scope…
        </Text>
      )}
      {reviewing && catalogue.isError && (
        <Alert color="red" title="Correction scope unavailable">
          {catalogue.error.message}
          <Button variant="subtle" onClick={() => void catalogue.refetch()}>
            Retry correction scope
          </Button>
        </Alert>
      )}
      {reviewing && catalogue.data && !catalogue.isError && (
        <ClaimReviewDialog
          brain={brain}
          catalogue={catalogue.data}
          view={reviewing}
          current={
            !view.knowledge_until && reviewing.revision.id === view.revision.id
          }
          Editor={ClaimEditor}
          EvidenceViewer={EvidenceDialog}
          onClose={() => setReviewing(null)}
          onSaved={() =>
            void queries.invalidateQueries({
              predicate: (query) => query.queryKey.includes(brain.id),
            })
          }
        />
      )}
      <Title order={4}>{r.content.subject}</Title>
      <Text size="sm" className="lineage-wrap">
        {r.content.predicate}: {r.content.value}
      </Text>
      {/* The three dimensions stay separate lines; a compact view may
          summarize, never merge or reorder them. */}
      <Text size="sm">Review: {label(r.review)}</Text>
      <Text size="sm">
        Freshness: {label(view.eligibility.effective_freshness)}
      </Text>
      <Text size="sm">Operational: {label(r.content.operational)}</Text>
      {flags.length > 0 && (
        <Group gap="xs">
          {flags.map((flag) => (
            <Badge key={flag} size="xs" color="yellow">
              {flag}
            </Badge>
          ))}
        </Group>
      )}
      <Text size="xs" c="dimmed">
        Fact time:{" "}
        {r.content.validity.kind === "unknown"
          ? "unknown; no interval is inferred"
          : r.content.validity.kind === "point"
            ? `${r.content.validity.from ? time(r.content.validity.from) : "unknown"} (${r.content.validity.precision} precision)`
            : `${r.content.validity.from ? time(r.content.validity.from) : "unknown"} → ${r.content.validity.to ? time(r.content.validity.to) : "unknown"} (end excluded)`}
        . Knowledge time: learned {time(r.recorded_at)}
        {view.knowledge_until
          ? ` · replaced ${time(view.knowledge_until)}`
          : ""}
        .
      </Text>
      <Text size="xs" c="dimmed">
        Provenance: {label(r.origin)} by {r.actor_name}
      </Text>
      <Group gap="xs">
        <Button
          variant="default"
          size="compact-xs"
          onClick={() => setReviewing(view)}
        >
          Review and corrections
        </Button>
        <EraseAction
          brain={brain.id}
          permitted={brain.role === "admin"}
          target={{ kind: "claim", id: r.claim_id }}
          name={r.content.subject}
        />
      </Group>
      <Divider label="Supporting evidence" />
      <EvidenceList
        brain={brain}
        evidence={view.evidence}
        supports={r.content.supports}
      />
      {!view.knowledge_until && (
        <>
          <Divider label="Current graph neighbourhood" />
          <Neighbourhood brain={brain} center={centerKey} />
        </>
      )}
    </Stack>
  );
}

/** Memory list selection: the id is a claim id. */
function MemoryLineage({ brain, id }: { brain: Brain; id: string }) {
  const [search] = useBrainSearch();
  const query = useQuery({
    queryKey: ["lineage-claim", brain.id, id, search.knowledge, search.fact],
    retry: false,
    gcTime: 0,
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/claims/{claim}", {
          signal,
          params: {
            path: { brain: brain.id, claim: id },
            query: { knowledge_at: search.knowledge, fact_at: search.fact },
          },
        }),
      ),
  });
  if (query.isPending) return <Loader size="sm" />;
  if (query.isError || !query.data)
    return (
      <Alert color="red">
        {query.error?.message ?? "This memory is unavailable."}
        <Button
          variant="subtle"
          size="xs"
          mt="xs"
          onClick={() => void query.refetch()}
        >
          Retry memory read
        </Button>
      </Alert>
    );
  const view = query.data.selected;
  if (!view)
    return (
      <Alert color="yellow">
        This memory’s content is unavailable. Its remaining history stays
        reachable from the Memory list; no substitute revision is shown.
      </Alert>
    );
  return (
    <ClaimLineageView
      brain={brain}
      view={view}
      centerKey={`claim:${view.revision.id}`}
    />
  );
}

/**
 * Graph claim node keys carry the revision id, not the claim id. The
 * canonical handover-input read resolves a revision to its claim view; a
 * revision that no longer resolves is reported as unavailable, never
 * substituted.
 */
function GraphClaimLineage({
  brain,
  nodeKey,
}: {
  brain: Brain;
  nodeKey: string;
}) {
  const revisionId = nodeKey.split(":")[1];
  const query = useQuery({
    queryKey: ["lineage-claim-revision", brain.id, revisionId],
    enabled: !!revisionId,
    retry: false,
    gcTime: 0,
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/handover-inputs/{revision}", {
          signal,
          params: { path: { brain: brain.id, revision: revisionId! } },
        }),
      ),
  });
  if (!revisionId)
    return (
      <Text size="sm" c="dimmed">
        This graph node has no resolvable claim identity.
      </Text>
    );
  if (query.isPending) return <Loader size="sm" />;
  if (query.isError || !query.data)
    return (
      <Alert color="yellow">
        This graph node points at a claim revision that no longer resolves. Its
        remaining history stays reachable from the Memory list; no substitute
        revision is shown.
        <Button
          variant="subtle"
          size="xs"
          mt="xs"
          onClick={() => void query.refetch()}
        >
          Retry claim read
        </Button>
      </Alert>
    );
  return (
    <ClaimLineageView brain={brain} view={query.data} centerKey={nodeKey} />
  );
}

function SourceLineage({
  brain,
  id,
  version,
}: {
  brain: Brain;
  id: string;
  version?: string;
}) {
  return (
    <SourceInspector
      brain={brain}
      id={id}
      version={version}
      derived={(exact) => (
        <SourceDerivations key={exact} brain={brain} version={exact} />
      )}
    />
  );
}

function SourceDerivations({
  brain,
  version,
}: {
  brain: Brain;
  version: string;
}) {
  const { select } = useKnowledgeSelection();
  const query = useNeighbourhood(brain, `source_version:${version}`);
  if (query.expired)
    return (
      <Text size="sm" c="dimmed">
        Linked memory expired; waiting for a fresh read.
      </Text>
    );
  if (query.isError)
    return (
      <Alert color="gray" title="Derived memory unavailable">
        {query.error.message}
      </Alert>
    );
  if (query.isPending) return <Loader size="sm" />;
  const nodes =
    query.data?.nodes.filter((node) => node.evidence.kind === "claim") ?? [];
  return (
    <details className="feature-details" open>
      <summary>Linked memory · {nodes.length} in this view</summary>
      <Stack gap="xs" className="source-linked-memory">
        {nodes.map((node) => (
          <button
            type="button"
            className="derived-memory-row"
            key={node.key}
            onClick={() => select({ kind: "graph-node", key: node.key })}
          >
            <strong>{node.evidence.label}</strong>
            <span>
              {[
                node.evidence.claim?.review,
                node.evidence.claim?.freshness,
                node.evidence.claim?.operational,
                ...node.evidence.qualifications,
              ]
                .filter(Boolean)
                .map((value) => label(value!))
                .join(" · ")}
            </span>
          </button>
        ))}
        {!nodes.length && (
          <Text size="sm" c="dimmed">
            No linked memory in the current eligible graph.
          </Text>
        )}
        <Text size="xs" c="dimmed">
          Cites this exact version; shared evidence is not independent
          corroboration.
        </Text>
      </Stack>
    </details>
  );
}

function GraphNodeLineage({
  brain,
  nodeKey,
}: {
  brain: Brain;
  nodeKey: string;
}) {
  const [kind, id] = nodeKey.split(":");
  if (kind === "claim" && id)
    return <GraphClaimLineage brain={brain} nodeKey={nodeKey} />;
  if ((canonicalEvidenceKinds as readonly string[]).includes(kind) && id)
    return (
      <CanonicalEvidenceRead
        brain={brain}
        kind={kind as (typeof canonicalEvidenceKinds)[number]}
        id={id}
      />
    );
  return (
    <Text size="sm" c="dimmed">
      Unknown graph entity kind “{kind}”. Its identity is {nodeKey}; no evidence
      or applicability is inferred for unsupported kinds.
    </Text>
  );
}

function GraphEdgeLineage({
  brain,
  relation,
  evidenceKind,
  evidenceId,
}: {
  brain: Brain;
  relation: string;
  evidenceKind: string;
  evidenceId: string;
}) {
  return (
    <Stack gap="sm">
      <Title order={4}>{label(relation)} relationship</Title>
      <Text size="sm" c="dimmed">
        A graph edge records a provenance or dependency relationship. It is
        never presented as independent support for a claim.
      </Text>
      {(canonicalEvidenceKinds as readonly string[]).includes(evidenceKind) ? (
        <CanonicalEvidenceRead
          brain={brain}
          kind={evidenceKind as (typeof canonicalEvidenceKinds)[number]}
          id={evidenceId}
        />
      ) : (
        <Text size="sm" c="dimmed">
          Recorded evidence: {label(evidenceKind)} · {evidenceId}
        </Text>
      )}
    </Stack>
  );
}

function RepositoryLineage({ brain, id }: { brain: Brain; id: string }) {
  const cache = useQueryClient();
  const [adding, setAdding] = useState(false);
  const [snapshots, setSnapshots] = useState(false);
  useLineageOverlay(adding || snapshots);
  const catalogue = useQuery({
    queryKey: ["repository-identity", brain.id, id],
    retry: false,
    gcTime: 0,
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/workspace/repositories/{repository}",
          {
            signal,
            params: { path: { brain: brain.id, repository: id } },
          },
        ),
      ),
  });
  if (catalogue.isPending) return <Loader size="sm" />;
  if (catalogue.isError)
    return (
      <Alert color="red">
        The repository catalogue could not be checked. {catalogue.error.message}
        <Button
          variant="subtle"
          size="xs"
          onClick={() => void catalogue.refetch()}
        >
          Retry repository
        </Button>
      </Alert>
    );
  const repo = catalogue.data;
  if (!repo)
    return (
      <Alert color="yellow">
        This repository is no longer listed in the Brain catalogue. Its recorded
        scopes and history are preserved.
      </Alert>
    );
  const extra = repo.origins.filter(
    (origin) => origin !== repo.canonical_origin,
  );
  return (
    <Stack gap="sm">
      {snapshots && (
        <RepositoryDialog
          brain={brain}
          repository={repo}
          onClose={() => setSnapshots(false)}
        />
      )}
      {adding && (
        <AliasDialog
          brain={brain}
          repository={repo}
          onClose={() => setAdding(false)}
          onSaved={() => {
            setAdding(false);
            for (const key of [
              "repository-identity",
              "repository-catalogue",
              "workspace",
            ])
              void cache.invalidateQueries({ queryKey: [key, brain.id] });
          }}
        />
      )}
      <Title order={4}>{repo.canonical_origin}</Title>
      <Group>
        <Button variant="default" size="xs" onClick={() => setSnapshots(true)}>
          Snapshots
        </Button>
        {brain.role === "admin" && !brain.archived && (
          <Button variant="default" size="xs" onClick={() => setAdding(true)}>
            Add origin
          </Button>
        )}
      </Group>
      <Text size="xs" c="dimmed">
        Canonical normalized origin.
      </Text>
      {extra.length > 0 && (
        <Stack gap="4px">
          <Text size="sm" fw={600}>
            All recorded origins
          </Text>
          {repo.origins.map((origin) => (
            <Text key={origin} size="sm" className="lineage-wrap">
              {origin}
            </Text>
          ))}
        </Stack>
      )}
      {/* Machine identifiers stay out of list rows; the inspector offers the
          copyable field instead. */}
      <details className="record-identity">
        <summary>Repository identifier</summary>
        <CopyButton value={repo.id}>
          {({ copied, copy }) => (
            <Button
              size="xs"
              variant="default"
              onClick={copy}
              leftSection={copied ? <Check size={14} /> : <Copy size={14} />}
            >
              {copied ? "Copied" : "Copy repository ID"}
            </Button>
          )}
        </CopyButton>
      </details>
    </Stack>
  );
}

/**
 * The one canonical lineage inspector. Every Knowledge view renders this same
 * definition through the shared selection; no page forks a private copy.
 */
function LineageInspector() {
  const brain = useBrain();
  const { selection } = useKnowledgeSelection();
  if (!selection)
    return (
      <Text size="sm" c="dimmed">
        Select a memory, source, graph item, or repository to inspect its
        lineage here.
      </Text>
    );
  switch (selection.kind) {
    case "memory":
      return (
        <MemoryLineage key={selection.id} brain={brain} id={selection.id} />
      );
    case "source":
      return (
        <SourceLineage
          key={`${selection.id}-${selection.version ?? ""}`}
          brain={brain}
          id={selection.id}
          version={selection.version}
        />
      );
    case "graph-node":
      return (
        <GraphNodeLineage
          key={selection.key}
          brain={brain}
          nodeKey={selection.key}
        />
      );
    case "graph-edge":
      return (
        <GraphEdgeLineage
          key={`${selection.relation}-${selection.evidenceId}`}
          brain={brain}
          relation={selection.relation}
          evidenceKind={selection.evidenceKind}
          evidenceId={selection.evidenceId}
        />
      );
    case "repository":
      return (
        <RepositoryLineage key={selection.id} brain={brain} id={selection.id} />
      );
  }
}

/** The shared lineage inspector region; the surface renders it exactly once. */
export function KnowledgeInspectorRegion({ view }: { view: KnowledgeView }) {
  const brain = useBrain();
  const navigate = useNavigate();
  const { selection, select, actions, nestedOpen } = useKnowledgeSelection();
  const region = useRef<HTMLElement>(null);
  useEffect(() => {
    // Following lineage replaces the focused link without closing the drawer.
    // Re-enter the trap before the next Tab can reach the background page.
    if (selection && !region.current?.contains(document.activeElement))
      region.current?.querySelector<HTMLButtonElement>("button")?.focus();
  }, [selection]);
  const [search, patch] = useBrainSearch();
  const detailed =
    (selection?.kind !== "source" && search.detail === "record") ||
    !!search.knowledge ||
    !!search.revision;
  const close = () => {
    select(null);
    patch({
      claim: null,
      source: null,
      version: null,
      center: null,
      detail: null,
      ...(selection?.kind === "repository" ? { repository: null } : {}),
    });
  };
  return (
    <Portal>
      <aside
        ref={region}
        className="knowledge-inspector-region"
        aria-label="Lineage inspector"
        data-knowledge-view={view}
      >
        <DetailInspector
          opened={!!selection && !detailed}
          onClose={close}
          title="Knowledge lineage"
          withinPortal={false}
          closeOnEscape={!nestedOpen}
          trapFocus={!nestedOpen}
        >
          <Stack
            gap="md"
            data-testid={
              selection?.kind === "graph-node"
                ? "exploration-node"
                : selection?.kind === "graph-edge"
                  ? "exploration-edge"
                  : undefined
            }
          >
            <Group gap="xs" className="knowledge-view-links">
              {knowledgeViews
                .filter(
                  (entry) =>
                    (entry.view === "explore" || entry.view === "graph") &&
                    entry.view !== view,
                )
                .map((entry) => {
                  const destinationSearch = selectionSearch(
                    selection,
                    entry.view,
                    search,
                  );
                  return Object.keys(destinationSearch).length ? (
                    <Link
                      key={entry.view}
                      to={`/brains/$brainId/${entry.view}`}
                      params={{ brainId: brain.id }}
                      search={destinationSearch}
                    >
                      Show in {entry.label}
                    </Link>
                  ) : null;
                })}
            </Group>
            {selection?.kind === "memory" && (
              <Button
                onClick={() =>
                  void navigate({
                    to: `/brains/$brainId/${selection.kind === "memory" ? "memory" : "sources"}`,
                    params: { brainId: brain.id },
                    search: {
                      ...selectionSearch(
                        selection,
                        selection.kind === "memory" ? "memory" : "sources",
                        search,
                      ),
                      detail: "record",
                    },
                  })
                }
                variant="subtle"
                w="fit-content"
              >
                {selection.kind === "memory"
                  ? "Memory history & actions"
                  : "Source history & actions"}
              </Button>
            )}
            {actions}
            <Suspense
              fallback={
                <Text role="status" size="sm">
                  Loading inspection…
                </Text>
              }
            >
              <LineageInspector />
            </Suspense>
          </Stack>
        </DetailInspector>
      </aside>
    </Portal>
  );
}

/**
 * Pathless layout route component wrapping the four Knowledge routes. It stays
 * mounted across view switches, so the inspector region is a single instance
 * while each view keeps its own URL and lazy bundle. The contextual sidebar is
 * the sole navigation between the views; selections are carried through
 * validated URL state by each page. Brain changes never carry a selection.
 */
export function KnowledgeSurface() {
  const brain = useBrain();
  const view = useKnowledgeView();
  const [selection, setSelection] = useState<KnowledgeSelection>(null);
  const [actions, setActions] = useState<ReactNode>(null);
  const [nestedOpen, setNestedOpen] = useState(false);
  const select = useCallback(
    (next: KnowledgeSelection, controls?: ReactNode) => {
      setSelection(next);
      setActions(controls ?? null);
    },
    [],
  );
  // Brain switches clear the selection: an identifier from one Brain is never
  // substituted into another. View switches are cleared by each page's own
  // unmount cleanup so a deep-linked selection for the arriving view (whose
  // effect runs first) survives the transition.
  const firstBrain = useRef(true);
  useEffect(() => {
    if (firstBrain.current) {
      firstBrain.current = false;
      return;
    }
    select(null);
  }, [brain.id, select]);
  const value = useMemo(
    () => ({
      available: true,
      selection,
      select,
      actions,
      nestedOpen,
      setNestedOpen,
    }),
    [selection, select, actions, nestedOpen],
  );
  return (
    <KnowledgeSelectionContext.Provider value={value}>
      <div className="knowledge-surface" data-knowledge-view={view}>
        <div className="knowledge-view-region">
          <Outlet />
        </div>
        <KnowledgeInspectorRegion view={view} />
      </div>
    </KnowledgeSelectionContext.Provider>
  );
}
