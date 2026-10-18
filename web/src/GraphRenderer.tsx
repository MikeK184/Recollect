import {
  Component,
  lazy,
  Suspense,
  useMemo,
  useRef,
  type MutableRefObject,
  useState,
  type ReactNode,
} from "react";
import { Alert, Loader, SegmentedControl } from "@mantine/core";
import { spatialTopology, type SpatialSession } from "./graph-3d-model";
import type { GraphCanvasProps } from "./GraphCanvas";
import "./features/feature-views.css";
import "./features/graph/graph-canvas.css";
import "./features/graph/graph-3d.css";

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
  const [dimension, setDimension] = useState("3d");
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
  return (
    <div
      className="graph-renderer rc-graph-renderer"
      data-has-inspector={!!inspector}
    >
      <div className="graph-renderer-canvas">
        <div className="graph-renderer-dimensions" data-graph-chrome="">
          <SegmentedControl
            size="xs"
            aria-label="Graph dimension"
            value={dimension}
            onChange={setDimension}
            data={[
              { value: "2d", label: "2D" },
              { value: "3d", label: "3D", disabled: failed },
            ]}
          />
          {failed && (
            <Alert
              variant="light"
              role="status"
              className="graph-renderer-fallback"
            >
              3D is unavailable. The 2D graph supports the same evidence
              inspection.
            </Alert>
          )}
        </div>
        {displayGroups.length > 0 && (
          <div
            className="graph-canvas-viewbar graph-spatial-groups"
            aria-label="Loaded topic memberships"
          >
            {displayGroups.map((group) => (
              <span className="graph-spatial-group" key={group.id}>
                {group.label} ·{" "}
                {group.nodeKeys.filter((key) => nodeKeys.has(key)).length}
              </span>
            ))}
          </div>
        )}
        <Suspense
          fallback={
            <div className="graph-renderer-loading">
              <Loader aria-label="Loading graph renderer" />
            </div>
          }
        >
          {dimension === "3d" ? (
            <SpatialGraphBoundary onFailure={fallback}>
              <Canvas3D
                {...props}
                displayGroups={displayGroups}
                topology={topology}
                session={spatialSession}
                onFailure={fallback}
              />
            </SpatialGraphBoundary>
          ) : (
            <Canvas2D {...props} />
          )}
        </Suspense>
      </div>
      {inspector && <div className="graph-renderer-inspector">{inspector}</div>}
    </div>
  );
}
