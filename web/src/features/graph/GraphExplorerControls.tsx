import {
  Button,
  Drawer,
  NumberInput,
  Select,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { Settings2, TriangleAlert } from "lucide-react";
import { iconSize } from "../../design/tokens";
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
    <SimpleGrid cols={{ base: 1, sm: 2 }}>
      <TextInput
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
      <NumberInput
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
    </SimpleGrid>
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
    <span className="graph-coverage">
      <button
        type="button"
        className="graph-coverage-toggle"
        aria-expanded={coverageOpen}
        onClick={() => setCoverageOpen((value) => !value)}
      >
        <TriangleAlert size={iconSize.small} />
        Coverage limits
      </button>
      {coverageOpen && (
        <span className="graph-coverage-reasons">
          {shownView.coverage.reasons.map(label).join(" · ")}
        </span>
      )}
    </span>
  ) : null;
}

// The buttons that sit in the canvas chrome: open the drawer, toggle the path.
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
      <Button
        variant="default"
        leftSection={<Settings2 size={iconSize.small} />}
        onClick={() => setControlsOpened(true)}
      >
        Explore
      </Button>
      {path?.status === "path" && data && (
        <Button
          variant="light"
          onClick={() => {
            setShowPath((value) => !value);
            setChoice(null);
          }}
        >
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
      className="feature-drawer"
      opened={controlsOpened}
      onClose={() => setControlsOpened(false)}
      position="right"
      title="Explore relationships"
      size="lg"
    >
      <Stack>
        <Text size="sm" c="dimmed">
          Choose an exact entity and a bounded traversal. Scope and eligibility
          stay the same as the current graph.
        </Text>
        <GraphExplorationSettings view={view} exploration={exploration} />
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
          variant="default"
          onClick={() => {
            load(null);
            setControlsOpened(false);
          }}
          disabled={read.isFetching}
        >
          Show eligible overview
        </Button>
        {data && (
          <Text size="xs" className="feature-meta">
            Generation {shownView!.generation.id} · {label(shownView!.state)} ·{" "}
            {shownView!.total_nodes} eligible entities in this{" "}
            {shownView!.coverage.reasons.includes("graph_window")
              ? "window"
              : "selection"}
          </Text>
        )}
      </Stack>
    </Drawer>
  );
}
