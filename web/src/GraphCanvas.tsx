import { useEffect, useRef, useState } from "react";
import { Button, Group, Select, Stack, Text } from "@mantine/core";
import cytoscape, { type Core, type StylesheetJson } from "cytoscape";
import type { components } from "./api-schema";

type Node = components["schemas"]["GraphNode"];
type Edge = components["schemas"]["GraphEdge"];
export type GraphChoice = { kind: "node" | "edge"; id: string } | null;

const style: StylesheetJson = [
  {
    selector: "node",
    style: {
      "background-color": "#1971c2",
      label: "data(label)",
      color: "#172b4d",
      "font-size": 11,
      "text-valign": "bottom",
      "text-margin-y": 7,
      "text-wrap": "ellipsis",
      "text-max-width": "145px",
      width: 24,
      height: 24,
      "border-width": 2,
      "border-color": "#fff",
    },
  },
  {
    selector: 'node[kind = "claim"]',
    style: { "background-color": "#7950f2", shape: "diamond" },
  },
  {
    selector: 'node[kind = "source_version"]',
    style: { "background-color": "#099268", shape: "round-rectangle" },
  },
  {
    selector: "edge",
    style: {
      width: 1.5,
      "line-color": "#8698ad",
      "target-arrow-color": "#8698ad",
      "target-arrow-shape": "triangle",
      "curve-style": "bezier",
      "arrow-scale": 0.9,
    },
  },
  {
    selector: 'edge[family = "cross_repository"]',
    style: {
      "line-style": "dashed",
      "line-color": "#c96f12",
      "target-arrow-color": "#c96f12",
    },
  },
  {
    selector: ".path",
    style: {
      "line-color": "#087f5b",
      "target-arrow-color": "#087f5b",
      "border-color": "#087f5b",
      "border-width": 4,
      width: 3,
    },
  },
  { selector: "node.path", style: { width: 28 } },
  {
    selector: ".chosen",
    style: {
      "line-color": "#e8590c",
      "target-arrow-color": "#e8590c",
      "border-color": "#e8590c",
      "border-width": 5,
      "overlay-opacity": 0.15,
      "overlay-color": "#e8590c",
    },
  },
];

function fitGraph(cy: Core, elements = cy.elements(), padding = 35) {
  cy.fit(elements, padding);
  // A small path should not enlarge labels to fill the entire canvas.
  if (cy.zoom() > 1.35) {
    cy.zoom(1.35);
    cy.center(elements);
  }
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
  const container = useRef<HTMLDivElement>(null);
  const core = useRef<Core | null>(null);
  const choose = useRef(onChoose);
  choose.current = onChoose;
  const [layout, setLayout] = useState("cose");
  const [ready, setReady] = useState(false);
  useEffect(() => {
    if (!container.current) return;
    setReady(false);
    const cy = cytoscape({
      container: container.current,
      style,
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
      wheelSensitivity: 0.25,
      boxSelectionEnabled: false,
      selectionType: "single",
      hideEdgesOnViewport: edges.length > 500,
    });
    core.current = cy;
    const run = cy.layout(
      layout === "cose"
        ? {
            name: "cose",
            animate: false,
            numIter: 250,
            randomize: false,
            padding: 35,
            componentSpacing: 80,
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
    fitGraph(cy);
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
      fitGraph(cy);
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
  }, [nodes, edges, layout]);
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
    <Stack gap="xs" style={{ minWidth: 0 }}>
      <Group align="end">
        <Select
          label="Graph layout"
          value={layout}
          onChange={(value) => setLayout(value ?? "cose")}
          data={[
            { value: "cose", label: "Connected clusters" },
            { value: "breadthfirst", label: "Directed layers" },
            { value: "grid", label: "Grid" },
          ]}
        />
        <Button
          variant="light"
          onClick={() => core.current && fitGraph(core.current)}
        >
          Fit graph
        </Button>
        <Button
          variant="light"
          aria-label="Zoom graph in"
          onClick={() => zoom(1.3)}
        >
          +
        </Button>
        <Button
          variant="light"
          aria-label="Zoom graph out"
          onClick={() => zoom(1 / 1.3)}
        >
          −
        </Button>
        <Button
          variant="light"
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
      <div
        ref={container}
        role="img"
        aria-label={`Interactive graph: ${nodes.length} entities and ${edges.length} directed relationships. Use the entity and relationship controls below for keyboard inspection.`}
        data-testid="graph-canvas"
        data-ready={ready}
        style={{
          height: "clamp(290px, 48vh, 520px)",
          width: "100%",
          minWidth: 0,
          overflow: "hidden",
          contain: "inline-size",
          background: "#f8fafc",
          border: "1px solid #ced4da",
          borderRadius: 8,
        }}
      />
      <Text size="xs" c="dimmed">
        Drag to pan; scroll to zoom; select an entity or arrow to inspect it.
        Blue: repository evidence · purple: claim · green: source. Orange marks
        selection; green outlines mark the requested path. Layout position does
        not indicate trust or importance.
      </Text>
    </Stack>
  );
}
