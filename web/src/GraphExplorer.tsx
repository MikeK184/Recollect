import { Alert, Badge, Button, Group, Loader, Text } from "@mantine/core";
import { Network, Settings2 } from "lucide-react";
import { EmptyState } from "./components/AsyncState";
import { iconSize } from "./design/tokens";
import { GraphRenderer } from "./GraphRenderer";
import "./features/feature-views.css";
import {
  GraphCanvasTools,
  GraphCoverageIndicator,
  GraphExploreDrawer,
} from "./features/graph/GraphExplorerControls";
import { GraphExplorerDetails } from "./features/graph/GraphExplorerDetails";
import { GraphExplorerSelection } from "./features/graph/GraphExplorerSelection";
import { useExplorerScene } from "./features/graph/useExplorerScene";
import {
  useGraphExploration,
  type GraphExplorerProps,
} from "./features/graph/useGraphExploration";

export type { GraphExplorerProps };

export function GraphExplorer(props: GraphExplorerProps) {
  const { brain, view, path, inspect, onStart, onEnd } = props;
  const exploration = useGraphExploration(props);
  const scene = useExplorerScene({ brain, view, path, inspect }, exploration);
  const {
    read,
    expired,
    data,
    inPath,
    shownView,
    choice,
    setChoice,
    setControlsOpened,
    focusRequest,
  } = exploration;
  const { nodes, edges, chosenNode, chosenEdge, rendererKey } = scene;
  scene.details.current = (
    <GraphExplorerDetails
      exploration={exploration}
      scene={scene}
      onStart={onStart}
      onEnd={onEnd}
    />
  );
  return (
    <section
      className="graph-explorer"
      aria-label="Interactive graph exploration"
    >
      {read.isFetching && (
        <Group role="status">
          <Loader size="sm" />
          <Text>Reading the qualified graph…</Text>
        </Group>
      )}
      {read.error && (
        <Alert color="red" title="Graph could not be displayed">
          {read.error.message}
        </Alert>
      )}
      {expired && (
        <Alert color="yellow">
          An input reached its retention deadline. Load the current graph again.
        </Alert>
      )}
      {data && nodes.length === 0 && (
        <EmptyState
          icon={Network}
          title={
            data.view.coverage.reasons.includes("graph_window")
              ? "No eligible entities in this window"
              : "No eligible entities"
          }
          description={
            data.view.coverage.reasons.includes("graph_window")
              ? "Other pages may contain evidence. Browse entities in Graph tools, choose an entity, or adjust the filters."
              : "This selection has no graph evidence to display. Adjust the filters or add permitted evidence to this Brain."
          }
        />
      )}
      {/* When the bounded read has no data to display (an error such as an
          ineligible center, or a retention deadline), the canvas chrome is
          unmounted with it. Keep the exploration entry point reachable so a
          failed read can be corrected and reloaded from its new location. */}
      {!read.isFetching && (!data || nodes.length === 0) && (
        <div
          className="graph-chrome graph-explorer-recover"
          data-graph-chrome=""
        >
          {data && (
            <GraphCoverageIndicator
              shownView={shownView}
              exploration={exploration}
            />
          )}
          <Button
            variant="default"
            leftSection={<Settings2 size={iconSize.small} />}
            onClick={() => setControlsOpened(true)}
          >
            Explore
          </Button>
        </div>
      )}
      {data && nodes.length > 0 && (
        <>
          <div className="graph-stage-area">
            <GraphRenderer
              key={rendererKey}
              session={scene.spatialSession}
              tools={<GraphCanvasTools path={path} exploration={exploration} />}
              inspector={
                choice && (chosenNode || chosenEdge) ? (
                  <GraphExplorerSelection
                    exploration={exploration}
                    scene={scene}
                    details={scene.details.current}
                  />
                ) : null
              }
              nodes={nodes}
              edges={edges}
              choice={choice}
              focusRequest={focusRequest}
              onChoose={setChoice}
              pathNodes={scene.pathNodes}
              pathEdges={scene.pathEdges}
            />
          </div>
          {data.center && nodes.length === 1 && (
            <Text size="sm">
              {data.view.coverage.reasons.includes("graph_window")
                ? "No other eligible entity was found in this window. Other candidates may exist."
                : "No other eligible entity is reachable within this direction and hop bound."}
            </Text>
          )}
          <div
            className="graph-chrome graph-explorer-status"
            data-graph-chrome=""
          >
            <Text size="xs" c="dimmed" data-testid="exploration-counts">
              {nodes.length} loaded nodes · {edges.length} directed
              relationships
            </Text>
            <Badge variant="light">
              {inPath
                ? "Shortest eligible path"
                : data.center
                  ? "Bounded reachability"
                  : "Eligible overview"}
            </Badge>
            <GraphCoverageIndicator
              shownView={shownView}
              exploration={exploration}
            />
            <Text size="xs" c="dimmed" className="graph-display-note">
              Blue repository · amber memory · sage source; recorded edges
              describe this evidence only.{" "}
              {shownView!.coverage.reasons.includes("graph_window")
                ? "Window limit: 250 entities, 1,000 relationships."
                : "Limit: 500 entities, 2,000 relationships."}
            </Text>
          </div>
        </>
      )}
      <GraphExploreDrawer view={view} exploration={exploration} />
    </section>
  );
}
