import {
  ChartNoAxesCombined,
  ChevronDown,
  Info,
  List,
  Network,
  Route,
  SlidersHorizontal,
} from "lucide-react";
import {
  Alert,
  Badge,
  Button,
  Card,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  SegmentedControl,
} from "../../components/ui";
import type { Brain } from "../../api";
import { ClaimDialog, EvidenceDialog } from "../../ClaimsPanel";
import { EmptyState } from "../../components/AsyncState";
import type { useEvidenceList } from "../../catalogueQueries";
import {
  evidenceModes,
  label,
  type GraphDetail,
  type GraphDrawer,
  type GraphNodeView,
  type Scope,
} from "./graphScope";

export type GraphGroups = NonNullable<
  ReturnType<typeof useEvidenceList>["data"]
>["groups"];

export function Failure({ error }: { error: Error | null }) {
  return error ? <Alert variant="destructive">{error.message}</Alert> : null;
}

export function GraphEntityCard({
  n,
  choose,
  repositories,
  onInspect,
  onStart,
  onEnd,
}: {
  n: GraphNodeView;
  choose: boolean;
  repositories: { id: string; canonical_origin: string }[] | undefined;
  onInspect: (n: GraphNodeView) => void;
  onStart: (key: string) => void;
  onEnd: (key: string) => void;
}) {
  return (
    <Card
      className="grid min-w-0 gap-2 p-3 [overflow-wrap:anywhere]"
      data-testid={choose ? "graph-entity" : "graph-path-entity"}
    >
      <div className="flex min-w-0 items-start justify-between gap-2">
        <p className="m-0 min-w-0 text-body font-semibold text-foreground">
          {n.evidence.label}
        </p>
        <Badge>{label(n.evidence.kind)}</Badge>
      </div>
      <p className="mono m-0 text-caption text-muted-foreground">{n.key}</p>
      {n.evidence.provenance[0]?.repository_id && (
        <p className="m-0 text-label text-foreground-secondary">
          {repositories?.find(
            (r) => r.id === n.evidence.provenance[0]?.repository_id,
          )?.canonical_origin ?? n.evidence.provenance[0].repository_id}
          {n.evidence.provenance[0].path
            ? ` · ${n.evidence.provenance[0].path}`
            : ""}
        </p>
      )}
      {n.evidence.provenance[0]?.revision && (
        <p className="m-0 text-caption text-muted-foreground">
          Commit:{" "}
          <span className="mono">{n.evidence.provenance[0].revision}</span>
        </p>
      )}
      {n.evidence.claim && (
        <p className="m-0 text-caption text-foreground-secondary">
          {label(n.evidence.claim.review)} ·{" "}
          {label(n.evidence.claim.freshness)} ·{" "}
          {label(n.evidence.claim.operational)}
        </p>
      )}
      {!!n.evidence.qualifications.length && (
        <p className="m-0 text-caption text-muted-foreground">
          {n.evidence.qualifications.map(label).join(" · ")}
        </p>
      )}
      <div className="flex flex-wrap gap-1.5 pt-1">
        <Button size="sm" variant="secondary" onClick={() => onInspect(n)}>
          Inspect evidence
        </Button>
        {choose && (
          <>
            <Button size="sm" variant="ghost" onClick={() => onStart(n.key)}>
              Use as start
            </Button>
            <Button size="sm" variant="ghost" onClick={() => onEnd(n.key)}>
              Use as end
            </Button>
          </>
        )}
      </div>
    </Card>
  );
}

// Evidence perspective tools in the header card: graph kind and the
// secondary tools menu (path, insights, status, entities). They float on the
// canvas with the page header instead of stacking above it.
export function GraphToolbar({
  scope,
  readable,
  onKindChange,
  onOpen,
}: {
  scope: Scope;
  readable: boolean;
  onKindChange: (kind: string) => void;
  onOpen: (drawer: GraphDrawer) => void;
}) {
  return (
    <div
      className="feature-toolbar flex flex-wrap items-center gap-1.5"
      data-graph-chrome=""
    >
      <SegmentedControl
        size="sm"
        label="Graph kind"
        value={scope.kind}
        data={[
          { value: "knowledge", label: "Knowledge" },
          { value: "repository", label: "Repository" },
          { value: "combined", label: "Combined" },
        ]}
        onChange={onKindChange}
      />
      <DropdownMenu modal={false}>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="sm">
            Graph tools
            <ChevronDown aria-hidden="true" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem
            disabled={!readable}
            onSelect={() => onOpen("path")}
          >
            <Route aria-hidden="true" />
            Find path
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onOpen("insights")}>
            <ChartNoAxesCombined aria-hidden="true" />
            Insights
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onOpen("status")}>
            <Info aria-hidden="true" />
            Graph status
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onOpen("entities")}>
            <List aria-hidden="true" />
            Entities
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

/** Opens the scope editor; sits beside the applied scope it changes. */
export function GraphFiltersButton({ onOpen }: { onOpen: () => void }) {
  return (
    <Button variant="secondary" size="sm" onClick={onOpen}>
      <SlidersHorizontal aria-hidden="true" />
      Filters
    </Button>
  );
}

export function GraphScopeSummary({
  scope,
  availableGroups,
}: {
  scope: Scope;
  availableGroups: GraphGroups;
}) {
  return (
    <div
      className="feature-scope flex flex-wrap items-center gap-1.5"
      data-graph-chrome=""
      aria-label="Applied graph scope"
    >
      <Badge variant="outline">
        {scope.mode === "investigation"
          ? "Include uncertainties"
          : (evidenceModes.find((mode) => mode.value === scope.mode)?.label ??
            label(scope.mode))}
      </Badge>
      <Badge variant="outline">
        {(scope.selection.repository_ids ?? []).length
          ? `${(scope.selection.repository_ids ?? []).length} selected repositories`
          : "All repositories in this Brain"}
      </Badge>
      {scope.selection.environment_id && (
        <Badge>
          {availableGroups.find(
            (group) => group.id === scope.selection.environment_id,
          )?.name ?? "Selected environment"}
        </Badge>
      )}
      {scope.snapshot_id && (
        <Badge variant="outline" className="mono">
          Snapshot {scope.snapshot_id.slice(0, 8)}
        </Badge>
      )}
      {scope.manifest_revision_id && (
        <Badge variant="outline" className="mono">
          Manifest {scope.manifest_revision_id.slice(0, 8)}
        </Badge>
      )}
      {scope.fact_at && (
        <Badge>Fact time {new Date(scope.fact_at).toLocaleString()}</Badge>
      )}
      {!!(scope.selection.area_ids ?? []).length && (
        <Badge>{(scope.selection.area_ids ?? []).length} areas</Badge>
      )}
      {scope.collection_id && (
        <Badge>
          {availableGroups.find((group) => group.id === scope.collection_id)
            ?.name ?? "Selected collection"}
        </Badge>
      )}
      {!!scope.relations.length && (
        <Badge>{scope.relations.map(label).join(", ")}</Badge>
      )}
    </div>
  );
}

export function GraphEmptyState({
  needsSelection,
  unavailable,
  onChooseInputs,
  onLoad,
}: {
  needsSelection: boolean;
  unavailable: boolean;
  onChooseInputs: () => void;
  onLoad: () => void;
}) {
  return (
    <EmptyState
      icon={Network}
      title={
        needsSelection
          ? "Choose an exact graph view"
          : unavailable
            ? "This graph view is unavailable"
            : "Explore the evidence"
      }
      description={
        needsSelection
          ? "Choose a repository snapshot or exact environment manifest in Filters. Recollect will keep that revision selection explicit."
          : "Load a bounded view of eligible evidence. If a graph is too large, narrow its repositories, areas or relationships."
      }
      action={
        <Button onClick={() => (needsSelection ? onChooseInputs() : onLoad())}>
          {needsSelection ? "Choose inputs" : "Load graph view"}
        </Button>
      }
    />
  );
}

export function GraphDetailDialogs({
  brain,
  detail,
  onSaved,
  onClose,
}: {
  brain: Brain;
  detail: GraphDetail | null;
  onSaved: () => void;
  onClose: () => void;
}) {
  return (
    <>
      {detail?.claim && (
        <ClaimDialog
          brain={brain}
          id={detail.claim}
          onSaved={onSaved}
          onClose={onClose}
        />
      )}
      {detail?.evidence && (
        <EvidenceDialog
          brain={brain}
          evidence={detail.evidence}
          returnLabel="Back to graph"
          onClose={onClose}
        />
      )}
    </>
  );
}
