import { fromModal, modalOpen } from "../../lib/focus";
import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import {
  Check,
  Compass,
  Copy,
  FileText,
  FolderGit2,
  GitCommitHorizontal,
  History,
  Network,
  Waypoints,
} from "lucide-react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Link,
  Outlet,
  useNavigate,
  useRouterState,
} from "@tanstack/react-router";
import {
  defaultKnowledgeView,
  resolveKnowledgeView,
  type KnowledgeSection,
} from "../../app/navigation";
import { useBrain } from "../../app/context";
import { useBrainSearch, type BrainSearch } from "../../app/useBrainSearch";
import {
  DetailInspector,
  InspectorSection,
  Quote,
} from "../../components/Inspector";
import { Page } from "../../components/Page";
import { PageHeader } from "../../components/PageHeader";
import {
  Alert,
  Badge,
  Button,
  CopyButton,
  KindChip,
  StatusBadge,
} from "../../components/ui";
import { useContentDeadline } from "../../useContentDeadline";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import {
  EvidenceButton,
  freshnessTone,
  memoryKindChip,
  operationalTone,
  reviewTone,
} from "../../claimsShared";
import { useWideLayout } from "../../lib/hooks";
import { cn } from "../../lib/utils";
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
import {
  ExploreFrameContext,
  KnowledgeSwitcher,
  type ExploreView,
} from "./frame";
import { usePollInterval } from "../../usePollInterval";
import { useWorkspaceCatalogue } from "../../catalogueQueries";

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

/** A quiet one-line note inside the inspector. */
function Muted({ children }: { children: ReactNode }) {
  return (
    <p className="m-0 text-label text-muted-foreground [overflow-wrap:anywhere]">
      {children}
    </p>
  );
}

/** A short in-flight read: a spinner beside the words it is waiting for. */
function Pending({ label: text }: { label: string }) {
  return (
    <p
      role="status"
      className="m-0 flex items-center gap-2 text-label text-muted-foreground"
    >
      <span
        aria-hidden="true"
        className="size-3.5 animate-spin rounded-full border-2 border-border-strong border-t-transparent"
      />
      {text}
    </p>
  );
}

/**
 * One labelled fact line. The term and its value are separate visually but
 * read (and match) as "Term: value", so every dimension stays one sentence.
 */
function Fact({ term, children }: { term: string; children: ReactNode }) {
  return (
    <p className="m-0 grid grid-cols-[112px_minmax(0,1fr)] items-center gap-x-3 text-label">
      <span className="text-muted-foreground">
        {term}
        <span className="sr-only">:</span>
      </span>{" "}
      <span className="flex min-w-0 flex-wrap items-center gap-1.5 text-foreground [overflow-wrap:anywhere]">
        {children}
      </span>
    </p>
  );
}

/** Graph and evidence kinds as category chips where a category exists. */
function EvidenceKind({ kind }: { kind: string }) {
  if (kind === "claim") return <KindChip kind="claim">Claim</KindChip>;
  if (kind === "source_version")
    return <KindChip kind="source">Source</KindChip>;
  return <Badge variant="outline">{label(kind)}</Badge>;
}

/** One bounded read around a graph center; never an unbounded traversal. */
function useNeighbourhood(brain: Brain, center: string, enabled = true) {
  const pollNormal = usePollInterval("normal");
  const query = useQuery({
    queryKey: ["lineage-neighbourhood", brain.id, center],
    enabled: enabled && !!center,
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
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

const nodeButtonClass =
  "lineage-node-button -mx-2 flex min-w-0 flex-1 items-center gap-2 rounded-md px-2 py-1.5 text-left text-label text-foreground transition-colors duration-fast ease-standard hover:bg-accent focus-visible:outline-2 focus-visible:outline-ring";

function Neighbourhood({ brain, center }: { brain: Brain; center: string }) {
  const { select } = useKnowledgeSelection();
  const query = useNeighbourhood(brain, center);
  if (query.expired)
    return <Muted>Neighbourhood expired; waiting for a fresh read.</Muted>;
  if (query.isPending) return <Pending label="Loading neighbourhood…" />;
  if (query.isError)
    return (
      <Alert
        variant="destructive"
        actions={
          <Button
            variant="destructive-ghost"
            size="sm"
            onClick={() => void query.refetch()}
          >
            Retry neighbourhood
          </Button>
        }
      >
        {query.error.message}
      </Alert>
    );
  // A missing generation surfaces as a request error above; an existing
  // generation may be "ready" or "partial" (coverage limits) and both carry
  // eligible nodes, exactly like the graph canvas renders them.
  const data = query.data;
  if (!data) return <Pending label="Loading neighbourhood…" />;
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
    <div className="grid gap-2">
      {nodes.length === 0 && (
        <Muted>No eligible neighbours within one hop.</Muted>
      )}
      {nodes.length > 0 && (
        <ul className="m-0 grid list-none gap-0.5 p-0">
          {nodes.map((node) => (
            <li key={node.key} className="flex min-w-0 items-center gap-2">
              <button
                type="button"
                className={nodeButtonClass}
                onClick={() => openNode(node.key)}
              >
                <Waypoints
                  aria-hidden="true"
                  className="size-3.5 shrink-0 text-muted-foreground"
                />
                <span className="min-w-0 flex-1 [overflow-wrap:anywhere]">
                  {node.evidence.label}
                </span>
              </button>
              <EvidenceKind kind={node.evidence.kind} />
            </li>
          ))}
        </ul>
      )}
      {data.edges.length > 0 && (
        <p className="m-0 text-caption text-muted-foreground">
          <span className="mono">{data.edges.length}</span> recorded
          relationship{data.edges.length === 1 ? "" : "s"} in this bounded read.
        </p>
      )}
    </div>
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
      <div className="grid gap-2">
        {evidence.length === 0 && (
          <Muted>No supporting evidence is recorded for this revision.</Muted>
        )}
        {evidence.map((item, index) => (
          <EvidenceButton
            key={`${item.kind}:${item.id}`}
            className="lineage-evidence-button"
            evidence={item}
            support={supports?.[index]}
            onOpen={() => setSelected(item)}
          />
        ))}
      </div>
      {selected && (
        <Suspense fallback={null}>
          <EvidenceDialog
            brain={brain}
            evidence={selected}
            onClose={() => setSelected(null)}
            returnLabel="Back to lineage"
          />
        </Suspense>
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
  const pollNormal = usePollInterval("normal");
  const { select } = useKnowledgeSelection();
  const query = useQuery({
    queryKey: ["lineage-evidence", brain.id, kind, id],
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
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
  if (query.isPending) return <Pending label="Loading evidence…" />;
  if (query.isError || !query.data)
    return (
      <Alert
        variant="destructive"
        actions={
          <Button
            variant="destructive-ghost"
            size="sm"
            onClick={() => void query.refetch()}
          >
            Retry evidence read
          </Button>
        }
      >
        {query.error?.message ?? "This exact evidence is unavailable."}
      </Alert>
    );
  const item = query.data.evidence;
  return (
    <div className="grid gap-2.5">
      <h3 className="m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
        {item.label}
      </h3>
      <div className="flex flex-wrap gap-1.5">
        <EvidenceKind kind={item.kind} />
        <Badge variant="outline">{label(item.availability)}</Badge>
      </div>
      {item.reference && <Fact term="Reference">{item.reference}</Fact>}
      {item.revision && (
        <Fact term="Exact revision">
          <span className="mono text-caption">{item.revision}</span>
        </Fact>
      )}
      {kind === "source_version" && item.source_id && (
        <div>
          <Button
            variant="secondary"
            size="sm"
            onClick={() =>
              select({ kind: "source", id: item.source_id!, version: id })
            }
          >
            <FileText aria-hidden="true" />
            Open in Sources
          </Button>
        </div>
      )}
    </div>
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
  recordAction,
}: {
  brain: Brain;
  view: View;
  centerKey: string;
  /** Opens the full record (history and actions) for a memory selection. */
  recordAction?: ReactNode;
}) {
  const [evidenceItem, setEvidenceItem] = useState<Evidence | null>(null);
  const [reviewing, setReviewing] = useState<View | null>(null);
  const queries = useQueryClient();
  const catalogue = useWorkspaceCatalogue(brain.id, undefined, {
    enabled: !!reviewing,
  });
  useLineageOverlay(!!evidenceItem || !!reviewing);
  const r = view.revision;
  const flags: string[] = [];
  if (r.lifecycle === "withdrawn") flags.push("Withdrawn");
  if (view.eligibility.rule_ids?.length) flags.push("Blocked by review rule");
  if (view.eligibility.conflicting_claim_ids?.length)
    flags.push("Unresolved conflict");
  const chip = memoryKindChip(r.content.kind);
  const validity = r.content.validity;
  // Handover and procedure records repeat their kind as the predicate.
  const redundant =
    label(r.content.predicate).toLowerCase() === r.content.kind.toLowerCase();
  return (
    <>
      <InspectorSection>
        <div className="flex flex-wrap items-center gap-1.5">
          <KindChip kind={chip.kind}>{chip.label}</KindChip>
          {flags.map((flag) => (
            <StatusBadge key={flag} state="attention">
              {flag}
            </StatusBadge>
          ))}
        </div>
        <h3 className="m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
          {r.content.subject}
        </h3>
        <Quote className="whitespace-pre-wrap [overflow-wrap:anywhere]">
          {redundant ? null : `${r.content.predicate}: `}
          {r.content.value}
        </Quote>
        {recordAction && <div className="-ml-2.5">{recordAction}</div>}
      </InspectorSection>
      {/* The three dimensions stay separate lines; a compact view may
          summarize, never merge or reorder them. */}
      <InspectorSection label="Status">
        <Fact term="Review">
          <StatusBadge state={reviewTone(r.review)}>
            {label(r.review)}
          </StatusBadge>
        </Fact>
        <Fact term="Freshness">
          <StatusBadge
            state={freshnessTone(view.eligibility.effective_freshness)}
          >
            {label(view.eligibility.effective_freshness)}
          </StatusBadge>
        </Fact>
        <Fact term="Operational">
          <StatusBadge state={operationalTone(r.content.operational)}>
            {label(r.content.operational)}
          </StatusBadge>
        </Fact>
      </InspectorSection>
      <InspectorSection label="Time and provenance">
        <Fact term="Fact time">
          {validity.kind === "unknown" ? (
            "unknown; no interval is inferred"
          ) : validity.kind === "point" ? (
            <span>
              <span className="mono text-caption">
                {validity.from ? time(validity.from) : "unknown"}
              </span>{" "}
              ({validity.precision} precision)
            </span>
          ) : (
            <span>
              <span className="mono text-caption">
                {validity.from ? time(validity.from) : "unknown"} →{" "}
                {validity.to ? time(validity.to) : "unknown"}
              </span>{" "}
              (end excluded)
            </span>
          )}
        </Fact>
        <Fact term="Knowledge time">
          <span>
            learned{" "}
            <span className="mono text-caption">{time(r.recorded_at)}</span>
            {view.knowledge_until && (
              <>
                {" "}
                · replaced{" "}
                <span className="mono text-caption">
                  {time(view.knowledge_until)}
                </span>
              </>
            )}
          </span>
        </Fact>
        <Fact term="Provenance">
          {label(r.origin)} by {r.actor_name}
        </Fact>
        <div className="flex flex-wrap items-center gap-2 pt-1">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setReviewing(view)}
          >
            Review and corrections
          </Button>
          <Suspense fallback={null}>
            <EraseAction
              brain={brain.id}
              permitted={brain.role === "admin"}
              target={{ kind: "claim", id: r.claim_id }}
              name={r.content.subject}
            />
          </Suspense>
        </div>
        {reviewing && catalogue.isPending && (
          <Pending label="Loading correction scope…" />
        )}
        {reviewing && catalogue.isError && (
          <Alert
            variant="destructive"
            title="Correction scope unavailable"
            actions={
              <Button
                variant="destructive-ghost"
                size="sm"
                onClick={() => void catalogue.refetch()}
              >
                Retry correction scope
              </Button>
            }
          >
            {catalogue.error.message}
          </Alert>
        )}
      </InspectorSection>
      <InspectorSection label={`Supporting evidence · ${view.evidence.length}`}>
        <EvidenceList
          brain={brain}
          evidence={view.evidence}
          supports={r.content.supports}
        />
      </InspectorSection>
      {!view.knowledge_until && (
        <InspectorSection label="Current graph neighbourhood">
          <Neighbourhood brain={brain} center={centerKey} />
        </InspectorSection>
      )}
      {evidenceItem && (
        <Suspense fallback={null}>
          <EvidenceDialog
            brain={brain}
            evidence={evidenceItem}
            onClose={() => setEvidenceItem(null)}
            returnLabel="Back to lineage"
          />
        </Suspense>
      )}
      {reviewing && catalogue.data && !catalogue.isError && (
        // Lazy dialogs suspend locally, so loading one never hides the
        // inspector (and the control that keeps focus) behind a fallback.
        <Suspense fallback={null}>
          <ClaimReviewDialog
            brain={brain}
            catalogue={catalogue.data}
            view={reviewing}
            current={
              !view.knowledge_until &&
              reviewing.revision.id === view.revision.id
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
        </Suspense>
      )}
    </>
  );
}

/** Memory list selection: the id is a claim id. */
function MemoryLineage({
  brain,
  id,
  recordAction,
}: {
  brain: Brain;
  id: string;
  recordAction?: ReactNode;
}) {
  const pollNormal = usePollInterval("normal");
  const [search] = useBrainSearch();
  const query = useQuery({
    queryKey: ["lineage-claim", brain.id, id, search.knowledge, search.fact],
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
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
  if (query.isPending) return <Pending label="Loading memory…" />;
  if (query.isError || !query.data)
    return (
      <Alert
        variant="destructive"
        actions={
          <Button
            variant="destructive-ghost"
            size="sm"
            onClick={() => void query.refetch()}
          >
            Retry memory read
          </Button>
        }
      >
        {query.error?.message ?? "This memory is unavailable."}
      </Alert>
    );
  const view = query.data.selected;
  // Unavailable or erased content still keeps its history and remaining
  // actions reachable, so the record action shows here too.
  if (!view)
    return (
      <div className="grid gap-3">
        <Alert variant="warning" role="status">
          This memory’s content is unavailable. Its remaining history stays
          reachable from the Memory list; no substitute revision is shown.
        </Alert>
        {recordAction && <div className="-ml-2.5">{recordAction}</div>}
      </div>
    );
  return (
    <ClaimLineageView
      brain={brain}
      view={view}
      centerKey={`claim:${view.revision.id}`}
      recordAction={recordAction}
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
  const pollNormal = usePollInterval("normal");
  const revisionId = nodeKey.split(":")[1];
  const query = useQuery({
    queryKey: ["lineage-claim-revision", brain.id, revisionId],
    enabled: !!revisionId,
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/handover-inputs/{revision}", {
          signal,
          params: { path: { brain: brain.id, revision: revisionId! } },
        }),
      ),
  });
  if (!revisionId)
    return <Muted>This graph node has no resolvable claim identity.</Muted>;
  if (query.isPending) return <Pending label="Loading claim…" />;
  if (query.isError || !query.data)
    return (
      <Alert
        variant="warning"
        actions={
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void query.refetch()}
          >
            Retry claim read
          </Button>
        }
      >
        This graph node points at a claim revision that no longer resolves. Its
        remaining history stays reachable from the Memory list; no substitute
        revision is shown.
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
    return <Muted>Linked memory expired; waiting for a fresh read.</Muted>;
  if (query.isError)
    return (
      <Alert variant="default" role="status" title="Derived memory unavailable">
        {query.error.message}
      </Alert>
    );
  if (query.isPending) return <Pending label="Loading linked memory…" />;
  const nodes =
    query.data?.nodes.filter((node) => node.evidence.kind === "claim") ?? [];
  return (
    <div className="source-linked-memory grid gap-2">
      <p className="m-0 text-caption text-muted-foreground">
        Linked memory · <span className="mono">{nodes.length}</span> in this
        view
      </p>
      {nodes.length > 0 && (
        <ul className="m-0 grid list-none gap-0.5 p-0">
          {nodes.map((node) => {
            const dimensions = [
              node.evidence.claim?.review,
              node.evidence.claim?.freshness,
              node.evidence.claim?.operational,
              ...node.evidence.qualifications,
            ]
              .filter(Boolean)
              .map((value) => label(value!))
              .join(" · ");
            return (
              <li key={node.key} className="flex min-w-0">
                <button
                  type="button"
                  className={cn(
                    nodeButtonClass,
                    "derived-memory-row items-start",
                  )}
                  onClick={() => select({ kind: "graph-node", key: node.key })}
                >
                  <KindChip kind="claim" className="mt-px">
                    Claim
                  </KindChip>
                  <span className="grid min-w-0 flex-1 gap-0.5">
                    <span className="font-medium [overflow-wrap:anywhere]">
                      {node.evidence.label}
                    </span>
                    {dimensions && (
                      <span className="text-caption text-muted-foreground">
                        {dimensions}
                      </span>
                    )}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
      {!nodes.length && (
        <Muted>No linked memory in the current eligible graph.</Muted>
      )}
      <p className="m-0 text-caption text-muted-foreground">
        Derived records cite this exact version; shared evidence is not
        independent support.
      </p>
    </div>
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
      <InspectorSection>
        <CanonicalEvidenceRead
          brain={brain}
          kind={kind as (typeof canonicalEvidenceKinds)[number]}
          id={id}
        />
      </InspectorSection>
    );
  return (
    <InspectorSection>
      <Muted>
        Unknown graph entity kind “{kind}”. Its identity is {nodeKey}; no
        evidence or applicability is inferred for unsupported kinds.
      </Muted>
    </InspectorSection>
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
    <>
      <InspectorSection>
        <div className="flex items-center gap-1.5">
          <Badge variant="outline">
            <GitCommitHorizontal aria-hidden="true" />
            Relationship
          </Badge>
        </div>
        <h3 className="m-0 text-title font-semibold capitalize text-foreground">
          {label(relation)} relationship
        </h3>
        <Muted>
          A graph edge records a provenance or dependency relationship. It is
          never presented as independent support for a claim.
        </Muted>
      </InspectorSection>
      <InspectorSection label="Recorded evidence">
        {(canonicalEvidenceKinds as readonly string[]).includes(
          evidenceKind,
        ) ? (
          <CanonicalEvidenceRead
            brain={brain}
            kind={evidenceKind as (typeof canonicalEvidenceKinds)[number]}
            id={evidenceId}
          />
        ) : (
          <Muted>
            Recorded evidence: {label(evidenceKind)} ·{" "}
            <span className="mono text-caption">{evidenceId}</span>
          </Muted>
        )}
      </InspectorSection>
    </>
  );
}

function RepositoryLineage({ brain, id }: { brain: Brain; id: string }) {
  const pollNormal = usePollInterval("normal");
  const cache = useQueryClient();
  const [adding, setAdding] = useState(false);
  const [snapshots, setSnapshots] = useState(false);
  useLineageOverlay(adding || snapshots);
  const catalogue = useQuery({
    queryKey: ["repository-identity", brain.id, id],
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
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
  if (catalogue.isPending) return <Pending label="Loading repository…" />;
  if (catalogue.isError)
    return (
      <Alert
        variant="destructive"
        actions={
          <Button
            variant="destructive-ghost"
            size="sm"
            onClick={() => void catalogue.refetch()}
          >
            Retry repository
          </Button>
        }
      >
        The repository catalogue could not be checked. {catalogue.error.message}
      </Alert>
    );
  const repo = catalogue.data;
  if (!repo)
    return (
      <Alert variant="warning" role="status">
        This repository is no longer listed in the Brain catalogue. Its recorded
        scopes and history are preserved.
      </Alert>
    );
  const extra = repo.origins.filter(
    (origin) => origin !== repo.canonical_origin,
  );
  return (
    <>
      <InspectorSection>
        <div className="flex items-start gap-3">
          <span className="grid size-9 shrink-0 place-items-center rounded-md bg-muted text-foreground-secondary">
            <FolderGit2 aria-hidden="true" className="size-4" />
          </span>
          <div className="grid min-w-0 gap-1">
            <h3 className="m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
              {repo.canonical_origin.split("/").at(-1) || repo.canonical_origin}
            </h3>
            <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
              {repo.canonical_origin}
            </p>
          </div>
        </div>
        <Muted>Canonical normalized origin.</Muted>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setSnapshots(true)}
          >
            <GitCommitHorizontal aria-hidden="true" />
            Snapshots
          </Button>
          {brain.role === "admin" && !brain.archived && (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setAdding(true)}
            >
              Add origin
            </Button>
          )}
        </div>
      </InspectorSection>
      {extra.length > 0 && (
        <InspectorSection label="All recorded origins">
          <ul className="m-0 grid list-none gap-1 p-0">
            {repo.origins.map((origin) => (
              <li
                key={origin}
                className="mono text-caption text-foreground-secondary [overflow-wrap:anywhere]"
              >
                {origin}
              </li>
            ))}
          </ul>
        </InspectorSection>
      )}
      {/* Machine identifiers stay out of list rows; the inspector offers the
          copyable field instead. */}
      <InspectorSection>
        <details className="record-identity group/identity">
          <summary className="cursor-pointer text-label font-medium text-foreground-secondary hover:text-foreground">
            Repository identifier
          </summary>
          <div className="mt-2.5 grid gap-2">
            <code className="mono rounded-sm bg-muted px-2 py-1 text-caption text-foreground [overflow-wrap:anywhere]">
              {repo.id}
            </code>
            <CopyButton value={repo.id}>
              {({ copied, copy }) => (
                <Button
                  variant="secondary"
                  size="sm"
                  className="w-fit"
                  onClick={copy}
                >
                  {copied ? (
                    <Check aria-hidden="true" className="text-brand-text" />
                  ) : (
                    <Copy aria-hidden="true" />
                  )}
                  {copied ? "Copied" : "Copy repository ID"}
                </Button>
              )}
            </CopyButton>
          </div>
        </details>
      </InspectorSection>
      <Suspense fallback={null}>
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
      </Suspense>
    </>
  );
}

/**
 * The one canonical lineage inspector. Every Knowledge view renders this same
 * definition through the shared selection; no page forks a private copy.
 */
function LineageInspector({ recordAction }: { recordAction?: ReactNode }) {
  const brain = useBrain();
  const { selection } = useKnowledgeSelection();
  if (!selection) return null;
  switch (selection.kind) {
    case "memory":
      return (
        <MemoryLineage
          key={selection.id}
          brain={brain}
          id={selection.id}
          recordAction={recordAction}
        />
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

/**
 * The shared lineage inspector region; the surface renders it exactly once.
 *
 * The region (`aside`, "Lineage inspector") is one persistent element: it
 * lives in a detached container that moves into the docked inspector (from
 * 1440px) or the sheet (below) while a record is selected, and back to a
 * hidden parking spot when nothing is. Its React subtree never remounts, so
 * nested dialogs and their state survive while another inspector briefly owns
 * the dock, and every view switch keeps the same region.
 */
export function KnowledgeInspectorRegion({ view }: { view: KnowledgeView }) {
  const brain = useBrain();
  const navigate = useNavigate();
  const wide = useWideLayout();
  const { selection, select, actions, nestedOpen } = useKnowledgeSelection();
  const [search, patch] = useBrainSearch();
  const detailed =
    (selection?.kind !== "source" && search.detail === "record") ||
    !!search.knowledge ||
    !!search.revision;
  const opened = !!selection && !detailed;
  const [container] = useState(() => {
    const node = document.createElement("div");
    node.className = "contents";
    return node;
  });
  const parking = useRef<HTMLDivElement>(null);
  const region = useRef<HTMLElement>(null);
  const opener = useRef<HTMLElement | null>(null);
  useLayoutEffect(() => {
    if (!container.parentNode) parking.current?.appendChild(container);
    return () => container.remove();
  }, [container]);
  // The inspector body hands its slot over on mount and takes it back on
  // unmount; a stable callback is only called for those two events.
  const mount = useCallback(
    (slot: HTMLDivElement | null) => {
      if (slot) slot.appendChild(container);
      else if (parking.current && container.parentNode !== parking.current)
        parking.current.appendChild(container);
    },
    [container],
  );
  useEffect(() => {
    if (!selection) return;
    const active = document.activeElement as HTMLElement | null;
    if (active && active !== document.body) {
      // Remember the row, node or link that opened this record so closing
      // returns focus there, unless focus is already inside an inspector.
      if (
        !region.current?.contains(active) &&
        !active.closest("[data-inspector-dock], [role='dialog']")
      )
        opener.current = active;
      return;
    }
    // Following lineage replaces the activated link. Re-enter the region so
    // the next Tab stays inside the inspector instead of the page start.
    const frame = requestAnimationFrame(() => {
      const node = region.current;
      const now = document.activeElement;
      if (node?.isConnected && (!now || now === document.body))
        node.focus({ preventScroll: true });
    });
    return () => cancelAnimationFrame(frame);
  }, [selection]);
  const close = () => {
    const target = opener.current;
    opener.current = null;
    select(null);
    patch({
      claim: null,
      source: null,
      version: null,
      center: null,
      detail: null,
      ...(selection?.kind === "repository" ? { repository: null } : {}),
    });
    window.setTimeout(() => {
      if (target?.isConnected) target.focus({ preventScroll: true });
    }, 40);
  };
  // "Show in" links carry only identities the destination can resolve, and
  // never point at the route already shown.
  const onExplore =
    useRouterState({ select: (state) => state.location.pathname }).split(
      "/",
    )[3] === "explore";
  const viewLinks = knowledgeViews
    .filter(
      (entry) =>
        (entry.view === "explore" || entry.view === "graph") &&
        entry.view !== view &&
        !(entry.view === "explore" && onExplore),
    )
    .map((entry) => ({
      ...entry,
      search: selectionSearch(selection, entry.view, search),
    }))
    .filter((entry) => Object.keys(entry.search).length > 0);
  return (
    <>
      <div ref={parking} className="sr-only" />
      {createPortal(
        <aside
          ref={region}
          tabIndex={-1}
          className="knowledge-inspector-region flex min-h-full min-w-0 flex-1 flex-col outline-none"
          aria-label="Lineage inspector"
          data-knowledge-view={view}
          onKeyDown={(event) => {
            // In the docked inspector this region sits outside the panel's own
            // key handling; Escape closes it unless a nested dialog owns it.
            // Whether a nested dialog owns Escape is read from the DOM at the
            // keypress, so an Escape right after one closes is not lost.
            if (
              event.key !== "Escape" ||
              event.defaultPrevented ||
              !opened ||
              fromModal(event.target) ||
              modalOpen()
            )
              return;
            event.stopPropagation();
            close();
          }}
        >
          {opened && (
            <>
              <div
                className="grid min-w-0"
                data-testid={
                  selection?.kind === "graph-node"
                    ? "exploration-node"
                    : selection?.kind === "graph-edge"
                      ? "exploration-edge"
                      : undefined
                }
              >
                {actions && (
                  <InspectorSection className="empty:hidden">
                    {actions}
                  </InspectorSection>
                )}
                <Suspense fallback={<Pending label="Loading inspection…" />}>
                  <LineageInspector
                    recordAction={
                      selection?.kind === "memory" ? (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() =>
                            // Over a Memory list the record opens in place,
                            // keeping the list's route and filters (such as
                            // its mode) as the return location.
                            view === "memory"
                              ? patch({ claim: selection.id, detail: "record" })
                              : void navigate({
                                  to: "/brains/$brainId/memory",
                                  params: { brainId: brain.id },
                                  search: {
                                    ...selectionSearch(
                                      selection,
                                      "memory",
                                      search,
                                    ),
                                    detail: "record",
                                  },
                                })
                          }
                        >
                          <History aria-hidden="true" />
                          Memory history &amp; actions
                        </Button>
                      ) : undefined
                    }
                  />
                </Suspense>
              </div>
              {viewLinks.length > 0 && (
                <div
                  className={cn(
                    "sticky -bottom-4 z-[1] -mx-4 -mb-4 mt-auto flex flex-wrap items-center justify-end gap-2 border-t border-border px-4 py-3",
                    wide ? "bg-card" : "bg-popover",
                  )}
                >
                  {viewLinks.map((entry) => (
                    <Button
                      key={entry.view}
                      asChild
                      variant="secondary"
                      size="sm"
                    >
                      <Link
                        to={`/brains/$brainId/${entry.view}`}
                        params={{ brainId: brain.id }}
                        search={entry.search}
                      >
                        {entry.view === "graph" ? (
                          <Network aria-hidden="true" />
                        ) : (
                          <Compass aria-hidden="true" />
                        )}
                        Show in {entry.label}
                      </Link>
                    </Button>
                  ))}
                </div>
              )}
            </>
          )}
        </aside>,
        container,
      )}
      <DetailInspector
        opened={opened}
        onClose={close}
        title="Knowledge lineage"
        returnFocus={false}
      >
        <div ref={mount} className="flex min-h-full flex-col" />
      </DetailInspector>
    </>
  );
}

/**
 * Pathless layout route component wrapping the Knowledge routes. It stays
 * mounted across view switches, so the inspector region is a single instance
 * while each view keeps its own URL and lazy bundle. Memory, Sources,
 * Repositories and Explore share one Explore frame (compact header and view
 * switcher); Graph is full-bleed, with nothing stacked above its canvas.
 * Selections are carried through validated URL state by each page. Brain
 * changes never carry a selection.
 */
export function KnowledgeSurface() {
  const brain = useBrain();
  const view = useKnowledgeView();
  const path = useRouterState({ select: (state) => state.location.pathname });
  const explore = path.split("/")[3] === "explore";
  const framed = view !== "graph";
  const [selection, setSelection] = useState<KnowledgeSelection>(null);
  const [actions, setActions] = useState<ReactNode>(null);
  const [nestedOpen, setNestedOpen] = useState(false);
  const [actionSlot, setActionSlot] = useState<HTMLDivElement | null>(null);
  const [counts, setCounts] = useState<Partial<Record<ExploreView, number>>>(
    {},
  );
  const select = useCallback(
    (next: KnowledgeSelection, controls?: ReactNode) => {
      setSelection(next);
      setActions(controls ?? null);
    },
    [],
  );
  const setCount = useCallback(
    (target: ExploreView, count: number | null) =>
      setCounts((previous) =>
        (previous[target] ?? null) === count
          ? previous
          : { ...previous, [target]: count ?? undefined },
      ),
    [],
  );
  // Brain switches clear the selection: an identifier from one Brain is never
  // substituted into another. View switches are cleared by each page's own
  // unmount cleanup so a deep-linked selection for the arriving view (whose
  // effect runs first) survives the transition.
  // A Brain id (not a "first run" flag) decides, so development double
  // effects never clear a deep-linked selection on mount.
  const shownBrain = useRef(brain.id);
  useEffect(() => {
    if (shownBrain.current === brain.id) return;
    shownBrain.current = brain.id;
    select(null);
    setCounts({});
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
  const frame = useMemo(
    () => ({ actions: actionSlot, setCount }),
    [actionSlot, setCount],
  );
  const heading = explore
    ? "Explore"
    : (knowledgeViews.find((entry) => entry.view === view)?.label ?? "Explore");
  return (
    <KnowledgeSelectionContext.Provider value={value}>
      <ExploreFrameContext.Provider value={frame}>
        <div
          className="knowledge-surface flex min-h-0 min-w-0 flex-1 flex-col"
          data-knowledge-view={view}
        >
          <Page width={framed ? "wide" : "full"}>
            {framed ? (
              <PageHeader
                title={heading}
                description="Everything this Brain knows, with the evidence behind it."
                actions={
                  <div
                    ref={setActionSlot}
                    className="flex items-center gap-2"
                  />
                }
              />
            ) : null}
            <div
              className={cn(
                "flex min-w-0 flex-col",
                framed ? "gap-4" : "min-h-0 flex-1",
              )}
            >
              {framed ? (
                <KnowledgeSwitcher
                  view={view}
                  explore={explore}
                  counts={counts}
                />
              ) : null}
              <div
                className={cn(
                  "knowledge-view-region flex min-w-0 flex-col",
                  framed ? "gap-4" : "min-h-0 flex-1",
                )}
              >
                <Outlet />
              </div>
            </div>
          </Page>
        </div>
        <KnowledgeInspectorRegion view={view} />
      </ExploreFrameContext.Provider>
    </KnowledgeSelectionContext.Provider>
  );
}
