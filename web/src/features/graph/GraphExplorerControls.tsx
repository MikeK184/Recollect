import { Route, Settings2, TriangleAlert } from "lucide-react";
import {
  Button,
  Drawer,
  NumberField,
  Select,
  TextField,
} from "../../components/ui";
import { label } from "./graphExplorerShared";
import type { ExplorerPath, ExplorerView } from "./graphExplorerShared";
import type { GraphExplorationState } from "./useGraphExploration";

// The center, direction, hop and entity-picker fields of the explore drawer.
export function GraphExplorationSettings({
  view,
  exploration,
}: {
  view: ExplorerView;
  exploration: GraphExplorationState;
}) {
  const { center, setCenter, direction, setDirection, hops, setHops } =
    exploration;
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <TextField
        label="Exploration center"
        description="An exact entity key; select a node below to fill it."
        value={center}
        onChange={(e) => setCenter(e.currentTarget.value)}
      />
      <Select
        label="Exploration direction"
        value={direction}
        data={["outgoing", "incoming", "both"]}
        onChange={(value) => setDirection(value ?? "outgoing")}
      />
      <NumberField
        label="Exploration hops"
        min={1}
        max={8}
        value={hops}
        onChange={(value) => setHops(Number(value) || 1)}
      />
      <Select
        label="Center from current entity page"
        placeholder="Choose an eligible entity"
        searchable
        clearable
        value={view.nodes.some((n) => n.key === center) ? center : null}
        data={view.nodes.map((n) => ({
          value: n.key,
          label: `${n.evidence.label} · ${n.evidence.kind}`,
        }))}
        onChange={(value) => setCenter(value ?? "")}
      />
    </div>
  );
}

// Partial-coverage notice with its expandable reasons; absent when complete.
export function GraphCoverageIndicator({
  shownView,
  exploration,
}: {
  shownView?: ExplorerView;
  exploration: GraphExplorationState;
}) {
  const { coverageOpen, setCoverageOpen } = exploration;
  return shownView?.coverage.partial ? (
    <span className="graph-coverage flex min-w-0 flex-wrap items-center gap-2">
      <button
        type="button"
        className="inline-flex h-[22px] items-center gap-1 rounded-full bg-warning-soft px-2 text-caption font-medium text-warning-text transition-colors duration-fast hover:bg-warning-soft/70 focus-visible:outline-2 focus-visible:outline-ring"
        aria-expanded={coverageOpen}
        onClick={() => setCoverageOpen((value) => !value)}
      >
        <TriangleAlert className="size-3.5" aria-hidden="true" />
        Coverage limits
      </button>
      {coverageOpen && (
        <span className="min-w-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
          {shownView.coverage.reasons.map(label).join(" · ")}
        </span>
      )}
    </span>
  ) : null;
}

// The buttons that sit in the canvas toolbar: open the drawer, toggle the path.
export function GraphCanvasTools({
  path,
  exploration,
}: {
  path: ExplorerPath | null;
  exploration: GraphExplorationState;
}) {
  const { setControlsOpened, data, setShowPath, setChoice, inPath } =
    exploration;
  return (
    <>
      <Button variant="ghost" size="sm" onClick={() => setControlsOpened(true)}>
        <Settings2 aria-hidden="true" />
        Explore
      </Button>
      {path?.status === "path" && data && (
        <Button
          variant={inPath ? "secondary" : "ghost"}
          size="sm"
          onClick={() => {
            setShowPath((value) => !value);
            setChoice(null);
          }}
        >
          <Route aria-hidden="true" />
          {inPath ? "Return to exploration" : "Show shortest path on canvas"}
        </Button>
      )}
    </>
  );
}

export function GraphExploreDrawer({
  view,
  exploration,
}: {
  view: ExplorerView;
  exploration: GraphExplorationState;
}) {
  const {
    controlsOpened,
    setControlsOpened,
    center,
    load,
    read,
    data,
    shownView,
  } = exploration;
  return (
    <Drawer
      opened={controlsOpened}
      onClose={() => setControlsOpened(false)}
      title="Explore relationships"
      description="Choose an exact entity and a bounded traversal. Scope and eligibility stay the same as the current graph."
      size={520}
      bodyClassName="grid content-start gap-4"
    >
      <GraphExplorationSettings view={view} exploration={exploration} />
      <div className="flex flex-wrap gap-2">
        <Button
          onClick={() => {
            load(center);
            setControlsOpened(false);
          }}
          disabled={!center || read.isFetching}
        >
          Explore from center
        </Button>
        <Button
          variant="secondary"
          onClick={() => {
            load(null);
            setControlsOpened(false);
          }}
          disabled={read.isFetching}
        >
          Show eligible overview
        </Button>
      </div>
      {data && (
        <p className="m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
          Generation <span className="mono">{shownView!.generation.id}</span> ·{" "}
          {label(shownView!.state)} ·{" "}
          <span className="mono">{shownView!.total_nodes}</span> eligible
          entities in this{" "}
          {shownView!.coverage.reasons.includes("graph_window")
            ? "window"
            : "selection"}
        </p>
      )}
    </Drawer>
  );
}
