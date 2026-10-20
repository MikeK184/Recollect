import {
  Component,
  lazy,
  Suspense,
  useContext,
  useMemo,
  useRef,
  type MutableRefObject,
  useState,
  type ReactNode,
} from "react";
import { SegmentedControl, Spinner } from "./components/ui";
import { cn } from "./lib/utils";
import { spatialTopology, type SpatialSession } from "./graph-3d-model";
import type { GraphCanvasProps } from "./GraphCanvas";
import {
  ChromeSlot,
  EnsureGraphStage,
  GraphDockContext,
  chromeCard,
} from "./features/graph/GraphStage";

const Canvas2D = lazy(() => import("./GraphCanvas"));
const Canvas3D = lazy(() => import("./GraphCanvas3D"));

/** Qualified topic membership supplied by the current graph read, never inferred by layout. */
export type GraphDisplayGroup = {
  id: string;
  label: string;
  nodeKeys: string[];
};

const emptyGroups: GraphDisplayGroup[] = [];

class SpatialGraphBoundary extends Component<
  { children: ReactNode; onFailure: () => void },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch() {
    this.props.onFailure();
  }
  render() {
    return this.state.failed ? null : this.props.children;
  }
}

/**
 * The lazy 2D (Cytoscape) / 3D (three.js) graph renderer on a full-bleed
 * canvas stage. Its chrome floats on the canvas; the record inspector docks
 * beside the canvas on the Graph page and sits in a column elsewhere.
 */
export function GraphRenderer({
  inspector,
  displayGroups = emptyGroups,
  session,
  ...props
}: GraphCanvasProps & {
  inspector?: ReactNode;
  displayGroups?: GraphDisplayGroup[];
  /** Owned by the caller when the renderer may remount without losing its camera. */
  session?: MutableRefObject<SpatialSession>;
}) {
  const docked = useContext(GraphDockContext);
  const ownSession = useRef<SpatialSession>({
    value: null,
    invalid: false,
  });
  const spatialSession = session ?? ownSession;
  const topology = useMemo(
    () => spatialTopology(props.nodes, props.edges, displayGroups),
    [props.nodes, props.edges, displayGroups],
  );
  if (
    props.nodes.length === 0 ||
    spatialSession.current.value?.topology !== topology
  )
    spatialSession.current.value = null;
  const [dimension, setDimension] = useState<"2d" | "3d">("3d");
  const [failed, setFailed] = useState(false);
  const nodeKeys = useMemo(
    () => new Set(props.nodes.map((node) => node.key)),
    [props.nodes],
  );
  const fallback = () => {
    spatialSession.current = { value: null, invalid: true };
    setFailed(true);
    setDimension("2d");
  };
  const leading = (
    <SegmentedControl
      size="sm"
      label="Graph dimension"
      value={dimension}
      onChange={setDimension}
      data={[
        { value: "2d", label: "2D" },
        { value: "3d", label: "3D", disabled: failed },
      ]}
    />
  );
  return (
    <div
      className="graph-renderer flex min-h-0 min-w-0 flex-1"
      data-has-inspector={!!inspector}
    >
      <EnsureGraphStage className="min-h-[420px]">
        {failed && (
          <ChromeSlot zone="top-right" order={2}>
            <p
              role="status"
              className={cn(
                chromeCard,
                "m-0 max-w-[22rem] px-3 py-2 text-caption text-foreground-secondary",
              )}
              data-graph-chrome=""
            >
              3D is unavailable. The 2D graph supports the same evidence
              inspection.
            </p>
          </ChromeSlot>
        )}
        {displayGroups.length > 0 && (
          <ChromeSlot zone="bottom-left" order={1}>
            <div
              aria-label="Loaded topic memberships"
              className="flex max-w-[640px] flex-wrap gap-1.5"
              data-graph-chrome=""
            >
              {displayGroups.map((group) => (
                <span
                  className={cn(
                    chromeCard,
                    "inline-flex h-7 items-center gap-1.5 px-2.5 text-caption font-medium",
                  )}
                  key={group.id}
                >
                  {group.label} ·{" "}
                  <span className="mono text-muted-foreground">
                    {group.nodeKeys.filter((key) => nodeKeys.has(key)).length}
                  </span>
                </span>
              ))}
            </div>
          </ChromeSlot>
        )}
        <Suspense
          fallback={
            <div className="grid min-h-0 flex-1 place-items-center">
              <Spinner label="Loading graph renderer" size={20} />
            </div>
          }
        >
          {dimension === "3d" ? (
            <SpatialGraphBoundary onFailure={fallback}>
              <Canvas3D
                {...props}
                leading={leading}
                displayGroups={displayGroups}
                topology={topology}
                session={spatialSession}
                onFailure={fallback}
              />
            </SpatialGraphBoundary>
          ) : (
            <Canvas2D {...props} leading={leading} />
          )}
        </Suspense>
      </EnsureGraphStage>
      {docked ? (
        inspector
      ) : (
        <div className="graph-renderer-inspector w-[19rem] min-w-0 shrink-0 overflow-auto border-l border-border bg-card empty:hidden">
          {inspector}
        </div>
      )}
    </div>
  );
}
