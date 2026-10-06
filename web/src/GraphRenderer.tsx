import { lazy, Suspense, type ReactNode } from "react";
import { Loader } from "@mantine/core";
import type { GraphCanvasProps } from "./GraphCanvas";
import "./features/graph/graph-canvas.css";
const Canvas2D = lazy(() => import("./GraphCanvas"));

export function GraphRenderer({
  inspector,
  ...props
}: GraphCanvasProps & { inspector?: ReactNode }) {
  return (
    <div
      className="graph-renderer rc-graph-renderer"
      data-has-inspector={!!inspector}
    >
      <div className="graph-renderer-canvas">
        <Suspense fallback={<Loader aria-label="Loading graph renderer" />}>
          <Canvas2D {...props} />
        </Suspense>
      </div>
      {inspector && <div className="graph-renderer-inspector">{inspector}</div>}
    </div>
  );
}
