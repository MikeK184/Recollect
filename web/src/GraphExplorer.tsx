import { Network, Settings2 } from "lucide-react";
import { EmptyState } from "./components/AsyncState";
import { Alert, Badge, Button, Spinner } from "./components/ui";
import { cn } from "./lib/utils";
import { GraphRenderer } from "./GraphRenderer";
import {
  GraphCanvasTools,
  GraphCoverageIndicator,
  GraphExploreDrawer,
} from "./features/graph/GraphExplorerControls";
import { GraphExplorerDetails } from "./features/graph/GraphExplorerDetails";
import { GraphExplorerSelection } from "./features/graph/GraphExplorerSelection";
import {
  ChromeSlot,
  EnsureGraphStage,
  chromeCard,
} from "./features/graph/GraphStage";
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
  const { nodes, edges, rendererKey } = scene;
  scene.details.current = (
    <GraphExplorerDetails
      exploration={exploration}
      scene={scene}
      onStart={onStart}
      onEnd={onEnd}
    />
  );
  const windowed = !!shownView?.coverage.reasons.includes("graph_window");
  return (
    <EnsureGraphStage className="min-h-[480px]">
      <section
        className="graph-explorer relative flex min-h-0 min-w-0 flex-1 flex-col"
        aria-label="Interactive graph exploration"
      >
        {read.isFetching && (
          <ChromeSlot zone="top-left" order={20}>
            <p
              role="status"
              className={cn(
                chromeCard,
                "m-0 flex items-center gap-2 px-3 py-2 text-label text-foreground-secondary",
              )}
            >
              <Spinner />
              Reading the qualified graph…
            </p>
          </ChromeSlot>
        )}
        {read.error && (
          <ChromeSlot zone="top-left" order={21} className="w-[26rem]">
            <Alert
              variant="destructive"
              title="Graph could not be displayed"
              className="shadow-md"
            >
              {read.error.message}
            </Alert>
          </ChromeSlot>
        )}
        {expired && (
          <ChromeSlot zone="top-left" order={22} className="w-[26rem]">
            <Alert variant="warning" role="status" className="shadow-md">
              An input reached its retention deadline. Load the current graph
              again.
            </Alert>
          </ChromeSlot>
        )}
        {data && nodes.length === 0 && (
          <div className="grid min-h-0 flex-1 place-items-center p-6">
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
          </div>
        )}
        {/* When the bounded read has no data to display (an error such as an
            ineligible center, or a retention deadline), the canvas chrome is
            unmounted with it. Keep the exploration entry point reachable so a
            failed read can be corrected and reloaded from its new location. */}
        {!read.isFetching && (!data || nodes.length === 0) && (
          <ChromeSlot zone="top-right" order={0}>
            <div
              className={cn(
                chromeCard,
                "graph-explorer-recover flex items-center gap-2 p-1",
              )}
              data-graph-chrome=""
            >
              {data && (
                <GraphCoverageIndicator
                  shownView={shownView}
                  exploration={exploration}
                />
              )}
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setControlsOpened(true)}
              >
                <Settings2 aria-hidden="true" />
                Explore
              </Button>
            </div>
          </ChromeSlot>
        )}
        {data && nodes.length > 0 && (
          <>
            <GraphRenderer
              key={rendererKey}
              session={scene.spatialSession}
              tools={<GraphCanvasTools path={path} exploration={exploration} />}
              inspector={
                <GraphExplorerSelection
                  exploration={exploration}
                  scene={scene}
                  details={scene.details.current}
                />
              }
              nodes={nodes}
              edges={edges}
              choice={choice}
              focusRequest={focusRequest}
              onChoose={setChoice}
              pathNodes={scene.pathNodes}
              pathEdges={scene.pathEdges}
            />
            {data.center && nodes.length === 1 && (
              <ChromeSlot zone="top-left" order={23} className="w-[26rem]">
                <Alert variant="default" role="status" className="shadow-md">
                  {data.view.coverage.reasons.includes("graph_window")
                    ? "No other eligible entity was found in this window. Other candidates may exist."
                    : "No other eligible entity is reachable within this direction and hop bound."}
                </Alert>
              </ChromeSlot>
            )}
            <ChromeSlot
              zone="header-meta"
              order={10}
              className="graph-explorer-status flex min-w-0 flex-wrap items-center gap-2"
            >
              <Badge variant={inPath ? "brand" : "default"}>
                {inPath
                  ? "Shortest eligible path"
                  : data.center
                    ? "Bounded reachability"
                    : "Eligible overview"}
              </Badge>
              <span
                className="mono text-caption text-muted-foreground"
                data-testid="exploration-counts"
              >
                {nodes.length} loaded nodes · {edges.length} directed
                relationships
              </span>
            </ChromeSlot>
            {shownView?.coverage.partial && (
              <ChromeSlot zone="header-extra" order={10}>
                <GraphCoverageIndicator
                  shownView={shownView}
                  exploration={exploration}
                />
              </ChromeSlot>
            )}
            <ChromeSlot zone="bottom-right" order={10}>
              <p
                className={cn(
                  chromeCard,
                  "graph-display-note m-0 max-w-[26rem] px-2.5 py-1.5 text-caption text-muted-foreground",
                )}
                data-graph-chrome=""
              >
                Recorded edges describe this evidence only.{" "}
                {windowed
                  ? "Window limit: 250 entities, 1,000 relationships."
                  : "Limit: 500 entities, 2,000 relationships."}
              </p>
            </ChromeSlot>
          </>
        )}
        <GraphExploreDrawer view={view} exploration={exploration} />
      </section>
    </EnsureGraphStage>
  );
}
