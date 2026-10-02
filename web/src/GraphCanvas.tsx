import { useEffect, useRef, useState } from "react";
import { Button, Group, Select, px, useMantineTheme } from "@mantine/core";
import cytoscape, { type Core, type StylesheetJson } from "cytoscape";
import { palette, fonts, iconSize } from "./design/tokens";
import { Maximize, ZoomIn, ZoomOut, Focus } from "lucide-react";
import "./features/feature-views.css";
import type { components } from "./api-schema";

type Node = components["schemas"]["GraphNode"];
type Edge = components["schemas"]["GraphEdge"];
export type GraphChoice = { kind: "node" | "edge"; id: string } | null;
const labelMaxWidth = 145;

const graphStyles = (labelFontSize: number): StylesheetJson => [
  {
    selector: "node",
    style: {
      "background-color": palette.blue,
      label: "data(label)",
      color: palette.ink,
      "font-size": labelFontSize,
      "font-family": fonts.ui,
      "text-valign": "bottom",
      "text-margin-y": 7,
      "text-wrap": "ellipsis",
      "text-max-width": `${labelMaxWidth}px`,
      width: 30,
      height: 30,
      "border-width": 2,
      "border-color": palette.paperLight,
    },
  },
  {
    selector: 'node[kind = "claim"]',
    style: { "background-color": palette.amber, shape: "round-rectangle" },
  },
  {
    selector: 'node[kind = "source_version"]',
    style: { "background-color": palette.sage, shape: "round-rectangle" },
  },
  {
    selector: "edge",
    style: {
      width: 1.5,
      "line-color": palette.lineDark,
      "target-arrow-color": palette.lineDark,
      "target-arrow-shape": "triangle",
      "curve-style": "bezier",
      "arrow-scale": 0.9,
    },
  },
  {
    selector: 'edge[family = "cross_repository"]',
    style: {
      "line-style": "dashed",
      "line-color": palette.violet,
      "target-arrow-color": palette.violet,
    },
  },
  {
    selector: ".path",
    style: {
      "line-color": palette.sageDark,
      "target-arrow-color": palette.sageDark,
      "border-color": palette.sageDark,
      "border-width": 4,
      width: 3,
    },
  },
  { selector: "node.path", style: { width: 28 } },
  {
    selector: ".chosen",
    style: {
      "line-color": palette.ink,
      "target-arrow-color": palette.ink,
      "border-color": palette.ink,
      "border-width": 5,
      "overlay-opacity": 0.15,
      "overlay-color": palette.ink,
    },
  },
];

function fitGraph(cy: Core, elements = cy.elements(), padding = 35) {
  const canvas = cy.container();
  if (!canvas || elements.empty()) return;
  const rect = canvas.getBoundingClientRect();
  const overlay = canvas
    .closest(".graph-stage-area")
    ?.querySelector(".graph-stage-overlay")
    ?.getBoundingClientRect();
  const controls = canvas.parentElement
    ?.querySelector(".graph-canvas-controls")
    ?.getBoundingClientRect();
  // Fit captions inside the usable viewport, including wrapping controls.
  const top = Math.max(padding, overlay ? overlay.bottom - rect.top + 16 : 0);
  const bottom = Math.max(
    padding,
    controls ? rect.bottom - controls.top + 16 : 0,
  );
  const width = Math.max(1, cy.width() - padding * 2);
  const height = Math.max(1, cy.height() - top - bottom);
  const bounds = elements.boundingBox({ includeLabels: true });
  cy.zoom(
    Math.min(
      1.35,
      width / Math.max(1, bounds.w),
      height / Math.max(1, bounds.h),
    ),
  );
  const zoom = cy.zoom();
  cy.pan({
    x: padding + width / 2 - (bounds.x1 + bounds.w / 2) * zoom,
    y: top + height / 2 - (bounds.y1 + bounds.h / 2) * zoom,
  });
}

export default function GraphCanvas({
  nodes,
  edges,
  choice,
  onChoose,
  pathNodes,
  pathEdges,
}: {
  nodes: Node[];
  edges: Edge[];
  choice: GraphChoice;
  onChoose: (choice: GraphChoice) => void;
  pathNodes: string[];
  pathEdges: string[];
}) {
  const theme = useMantineTheme();
  const labelFontSize = Number(px(theme.fontSizes.xs));
  const container = useRef<HTMLDivElement>(null);
  const core = useRef<Core | null>(null);
  const choose = useRef(onChoose);
  choose.current = onChoose;
  const [layout, setLayout] = useState("concentric");
  const [ready, setReady] = useState(false);
  useEffect(() => {
    if (!container.current) return;
    setReady(false);
    const cy = cytoscape({
      container: container.current,
      style: graphStyles(labelFontSize),
      elements: [
        ...nodes.map((n) => ({
          data: { id: n.key, label: n.evidence.label, kind: n.evidence.kind },
        })),
        ...edges.map((e) => ({
          data: { id: e.id, source: e.from, target: e.to, family: e.family },
        })),
      ],
      layout: { name: "grid" },
      minZoom: 0.08,
      maxZoom: 5,
      boxSelectionEnabled: false,
      selectionType: "single",
      hideEdgesOnViewport: edges.length > 500,
    });
    core.current = cy;
    const run = cy.layout(
      layout === "concentric"
        ? {
            name: "concentric",
            animate: false,
            fit: false,
            avoidOverlap: true,
            nodeDimensionsIncludeLabels: true,
            minNodeSpacing: 35,
            // Small degree differences should not create many large rings
            // that force every caption down to an unreadable fitted size.
            levelWidth: (items) => Math.max(1, items.maxDegree()),
          }
        : layout === "cose"
          ? {
              name: "cose",
              animate: false,
              numIter: 250,
              randomize: false,
              padding: 35,
              // The default short spring can overlap the captions of small graphs.
              idealEdgeLength: labelMaxWidth + 48,
              nodeRepulsion: labelMaxWidth ** 2,
              componentSpacing: labelMaxWidth,
              nodeDimensionsIncludeLabels: true,
            }
          : layout === "breadthfirst"
            ? {
                name: "breadthfirst",
                directed: true,
                animate: false,
                padding: 35,
                spacingFactor: 1.25,
                nodeDimensionsIncludeLabels: true,
              }
            : {
                name: "grid",
                animate: false,
                padding: 35,
                nodeDimensionsIncludeLabels: true,
              },
    );
    run.run();
    const fitLayout = () => {
      // Choose the orientation with the larger real rendered fit, including
      // labels. Evaluate again after resize because initial size can be zero.
      fitGraph(cy);
      if (layout !== "cose") return;
      const originalZoom = cy.zoom();
      const positions = new Map(
        cy.nodes().map((node) => [node.id(), { ...node.position() }] as const),
      );
      cy.nodes().positions((node) => {
        const { x, y } = positions.get(node.id())!;
        return { x: y, y: -x };
      });
      fitGraph(cy);
      if (cy.zoom() <= originalZoom) {
        cy.nodes().positions((node) => positions.get(node.id())!);
        fitGraph(cy);
      }
    };
    fitLayout();
    const tap = (event: cytoscape.EventObject) => {
      const target = event.target;
      choose.current(
        target === cy
          ? null
          : { kind: target.isNode() ? "node" : "edge", id: target.id() },
      );
    };
    cy.on("tap", tap);
    const resize = new ResizeObserver(() => {
      cy.resize();
      fitLayout();
    });
    resize.observe(container.current);
    setReady(true);
    return () => {
      resize.disconnect();
      run.stop();
      cy.off("tap", tap);
      cy.destroy();
      core.current = null;
    };
  }, [nodes, edges, layout, labelFontSize]);
  useEffect(() => {
    const cy = core.current;
    if (!cy) return;
    cy.batch(() => {
      cy.elements().removeClass("chosen path");
      for (const id of [...pathNodes, ...pathEdges])
        cy.getElementById(id).addClass("path");
      if (choice) cy.getElementById(choice.id).addClass("chosen");
    });
  }, [choice, pathNodes, pathEdges, nodes, edges, layout]);
  function zoom(factor: number) {
    const cy = core.current;
    if (cy)
      cy.zoom({
        level: Math.min(5, Math.max(0.08, cy.zoom() * factor)),
        renderedPosition: { x: cy.width() / 2, y: cy.height() / 2 },
      });
  }
  return (
    <div className="graph-stage">
      <div
        ref={container}
        className="graph-canvas"
        role="img"
        aria-label={`Interactive graph: ${nodes.length} entities and ${edges.length} directed relationships. Use the entity search or List view for keyboard inspection.`}
        data-testid="graph-canvas"
        data-ready={ready}
      />
      <Group
        className="graph-canvas-controls"
        align="end"
        justify="end"
        data-graph-chrome=""
      >
        <Select
          aria-label="Graph layout"
          value={layout}
          onChange={(value) => setLayout(value ?? "concentric")}
          data={[
            { value: "concentric", label: "Connected rings" },
            { value: "cose", label: "Connected clusters" },
            { value: "breadthfirst", label: "Directed layers" },
            { value: "grid", label: "Grid" },
          ]}
        />
        <Button
          variant="subtle"
          aria-label="Fit graph"
          onClick={() => core.current && fitGraph(core.current)}
        >
          <Maximize size={iconSize.small} />
        </Button>
        <Button
          variant="subtle"
          aria-label="Zoom graph in"
          onClick={() => zoom(1.3)}
        >
          <ZoomIn size={iconSize.small} />
        </Button>
        <Button
          variant="subtle"
          aria-label="Zoom graph out"
          onClick={() => zoom(1 / 1.3)}
        >
          <ZoomOut size={iconSize.small} />
        </Button>
        <Button
          variant="subtle"
          disabled={!choice}
          onClick={() => {
            if (choice) {
              const cy = core.current;
              if (cy)
                fitGraph(
                  cy,
                  cy.getElementById(choice.id).closedNeighborhood(),
                  65,
                );
            }
          }}
        >
          Focus selection
        </Button>
      </Group>
    </div>
  );
}
