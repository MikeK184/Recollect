import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  ActionIcon,
  Button,
  Group,
  Popover,
  SegmentedControl,
  Select,
  Stack,
  Text,
  Tooltip,
  px,
  useMantineTheme,
} from "@mantine/core";
import cytoscape, { type Core, type StylesheetJson } from "cytoscape";
import { palette, fonts, iconSize } from "./design/tokens";
import {
  Maximize,
  ZoomIn,
  ZoomOut,
  Focus,
  Search,
  SlidersHorizontal,
} from "lucide-react";
import "./features/feature-views.css";
import "./features/graph/graph-canvas.css";
import type { components } from "./api-schema";

export type GraphRenderNode = {
  key: string;
  evidence: Pick<components["schemas"]["RecallItem"], "label" | "kind">;
};
export type GraphRenderEdge = Pick<
  components["schemas"]["GraphEdge"],
  "id" | "from" | "to" | "family" | "relation"
>;
export type GraphChoice = { kind: "node" | "edge"; id: string } | null;
export type GraphCanvasProps = {
  nodes: GraphRenderNode[];
  edges: GraphRenderEdge[];
  choice: GraphChoice;
  onChoose: (choice: GraphChoice) => void;
  pathNodes: string[];
  pathEdges: string[];
  focusRequest?: number;
  tools?: ReactNode;
};
const labelMaxWidth = 145;
const typeFilters = [
  { value: "agents", label: "Agents", kinds: ["device"] },
  { value: "people", label: "People", kinds: ["contributor"] },
  {
    value: "sources",
    label: "Sources & inputs",
    kinds: ["source_version", "capture"],
  },
  {
    value: "repositories",
    label: "Repository evidence",
    kinds: [
      "repository",
      "repository_fact",
      "repository_file",
      "snapshot",
      "manifest_revision",
    ],
  },
  { value: "memory", label: "Memory", kinds: ["claim"] },
];
type Labels = "auto" | "all" | "none";
type Viewport = { zoom: number; pan: cytoscape.Position };
const kindLabel = (kind: string) => kind.replaceAll("_", " ");

const graphStyles = (labelFontSize: number, labels: Labels): StylesheetJson => [
  {
    selector: "node",
    style: {
      "background-color": palette.blue,
      label: labels === "none" ? "" : "data(label)",
      "min-zoomed-font-size": labels === "auto" ? 10 : 0,
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
    selector: "node.label-priority",
    style: {
      "min-zoomed-font-size": 0,
    },
  },
  {
    selector: 'node[kind = "claim"]',
    style: { "background-color": palette.amber, shape: "round-rectangle" },
  },
  {
    selector: 'node[kind = "source_version"], node[kind = "capture"]',
    style: { "background-color": palette.sage, shape: "round-rectangle" },
  },
  {
    selector: 'node[kind = "contributor"]',
    style: { "background-color": palette.ink },
  },
  {
    selector: 'node[kind = "device"]',
    style: { "background-color": palette.violet, shape: "diamond" },
  },
  {
    selector: 'node[kind = "repository"]',
    style: { "background-color": palette.sageDark, shape: "hexagon" },
  },
  {
    selector: 'node[kind = "snapshot"]',
    style: { "background-color": palette.amber, shape: "round-rectangle" },
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
  { selector: ".local-hidden", style: { display: "none" } },
];

function fitGraph(cy: Core, elements = cy.elements(":visible"), padding = 35) {
  const canvas = cy.container();
  if (!canvas || elements.empty()) return;
  const rect = canvas.getBoundingClientRect();
  const controls = canvas.parentElement
    ?.querySelector(".graph-canvas-controls")
    ?.getBoundingClientRect();
  const bottom = Math.max(
    padding,
    controls ? rect.bottom - controls.top + 16 : 0,
  );
  const width = Math.max(1, cy.width() - padding * 2),
    height = Math.max(1, cy.height() - padding - bottom);
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
    y: padding + height / 2 - (bounds.y1 + bounds.h / 2) * zoom,
  });
}

export default function GraphCanvas({
  nodes,
  edges,
  choice,
  onChoose,
  pathNodes,
  pathEdges,
  focusRequest = 0,
  tools,
}: GraphCanvasProps) {
  const theme = useMantineTheme(),
    labelFontSize = Number(px(theme.fontSizes.xs));
  const container = useRef<HTMLDivElement>(null),
    core = useRef<Core | null>(null);
  const choose = useRef(onChoose);
  choose.current = onChoose;
  const selection = useRef(choice);
  selection.current = choice;
  const priorChoice = useRef(choice ? `${choice.kind}:${choice.id}` : null);
  const hovered = useRef<string | null>(null);
  const [hoverLabel, setHoverLabel] = useState<string | null>(null);
  const [layout, setLayout] = useState("cose"),
    [labels, setLabels] = useState<Labels>("auto");
  const [typeFilter, setTypeFilter] = useState("all"),
    [focused, setFocused] = useState<GraphChoice>(null);
  const [optionsOpened, setOptionsOpened] = useState(false),
    [ready, setReady] = useState(false);
  const [shown, setShown] = useState({
    nodes: nodes.map((node) => node.key),
    edges: edges.map((edge) => edge.id),
  });
  const overview = useRef<Viewport | null>(null),
    needsFit = useRef(false),
    initialFit = useRef(true);
  const laidOut = useRef(false),
    previousLayout = useRef(layout);
  const consumedFocus = useRef(focusRequest);
  const pathActive = pathNodes.length > 0 || pathEdges.length > 0;
  const topology = useMemo(
    () =>
      JSON.stringify([
        nodes.map((node) => node.key).sort(),
        edges
          .map((edge) => [edge.id, edge.from, edge.to])
          .sort((a, b) => a[0].localeCompare(b[0])),
      ]),
    [nodes, edges],
  );
  const availableFilters = typeFilters.filter((filter) =>
    nodes.some((node) => filter.kinds.includes(node.evidence.kind)),
  );
  const nodeMap = useMemo(
    () => new Map(nodes.map((node) => [node.key, node])),
    [nodes],
  );
  const shownNodes = new Set(shown.nodes),
    shownEdges = new Set(shown.edges);
  const currentFilter = typeFilters.find(
    (filter) => filter.value === typeFilter,
  );
  const localView = !pathActive && (typeFilter !== "all" || !!focused);

  function rememberOverview() {
    const cy = core.current;
    if (cy && !overview.current)
      overview.current = { zoom: cy.zoom(), pan: { ...cy.pan() } };
  }
  function showAll() {
    setTypeFilter("all");
    setFocused(null);
    needsFit.current = false;
    const cy = core.current;
    if (cy) {
      cy.elements().removeClass("local-hidden");
      if (overview.current) cy.viewport(overview.current);
    }
    overview.current = null;
  }
  function focusSelection() {
    const cy = core.current;
    if (!cy || !choice || pathActive || cy.getElementById(choice.id).empty())
      return;
    rememberOverview();
    setTypeFilter("all");
    setFocused({ ...choice });
    needsFit.current = true;
  }
  function prioritizeLabels() {
    const cy = core.current;
    if (!cy) return;
    cy.batch(() => {
      cy.nodes().removeClass("label-priority");
      const selected =
        selection.current && cy.getElementById(selection.current.id);
      if (selected && !selected.empty()) {
        if (selection.current?.kind === "node")
          selected.addClass("label-priority");
        else selected.connectedNodes().addClass("label-priority");
      }
      if (hovered.current)
        cy.getElementById(hovered.current).addClass("label-priority");
    });
  }

  useEffect(() => {
    if (!container.current) return;
    const cy = cytoscape({
      container: container.current,
      style: graphStyles(labelFontSize, "auto"),
      elements: [],
      layout: { name: "preset" },
      minZoom: 0.08,
      maxZoom: 5,
      boxSelectionEnabled: false,
      selectionType: "single",
    });
    core.current = cy;
    const tap = (event: cytoscape.EventObject) => {
      const target = event.target;
      choose.current(
        target === cy
          ? null
          : { kind: target.isNode() ? "node" : "edge", id: target.id() },
      );
    };
    const hover = (event: cytoscape.EventObject) => {
      hovered.current = event.type === "mouseover" ? event.target.id() : null;
      setHoverLabel(
        hovered.current
          ? `${event.target.data("label")} · ${kindLabel(event.target.data("kind"))}`
          : null,
      );
      prioritizeLabels();
    };
    cy.on("tap", tap);
    cy.on("mouseover mouseout", "node", hover);
    const resize = new ResizeObserver(() => {
      // Opening the inspector changes dimensions, never the user's viewport.
      cy.resize();
      if (
        initialFit.current &&
        cy.nodes().length &&
        cy.width() > 0 &&
        cy.height() > 0
      ) {
        fitGraph(cy);
        initialFit.current = false;
      }
    });
    resize.observe(container.current);
    setReady(true);
    return () => {
      resize.disconnect();
      cy.off("tap", tap);
      cy.off("mouseover mouseout", "node", hover);
      cy.destroy();
      core.current = null;
    };
  }, []);

  // Only topology or explicit layout changes run bounded positioning. Unchanged
  // polling, hover, labels, filters and resize never run a layout.
  useEffect(() => {
    const cy = core.current;
    if (!cy || !ready) return;
    hovered.current = null;
    setHoverLabel(null);
    const positions = new Map(
      cy.nodes().map((node) => [node.id(), { ...node.position() }] as const),
    );
    const viewport = { zoom: cy.zoom(), pan: { ...cy.pan() } },
      layoutChanged = previousLayout.current !== layout;
    previousLayout.current = layout;
    cy.batch(() => {
      cy.elements().remove();
      cy.add([
        ...nodes.map((node, index) => ({
          data: {
            id: node.key,
            label: node.evidence.label,
            kind: node.evidence.kind,
          },
          position: positions.get(node.key) ?? {
            x: (index % 10) * 160,
            y: Math.floor(index / 10) * 160,
          },
        })),
        ...edges.map((edge) => ({
          data: {
            id: edge.id,
            source: edge.from,
            target: edge.to,
            family: edge.family,
            relation: edge.relation,
          },
        })),
      ]);
    });
    const run = cy.layout(
      layout === "concentric"
        ? {
            name: "concentric",
            animate: false,
            fit: false,
            avoidOverlap: true,
            nodeDimensionsIncludeLabels: true,
            minNodeSpacing: 35,
            levelWidth: (items) => Math.max(1, items.maxDegree()),
          }
        : layout === "cose"
          ? {
              name: "cose",
              animate: false,
              fit: false,
              numIter: 250,
              randomize: false,
              padding: 35,
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
                fit: false,
                padding: 35,
                spacingFactor: 1.25,
                nodeDimensionsIncludeLabels: true,
              }
            : {
                name: "grid",
                animate: false,
                fit: false,
                padding: 35,
                nodeDimensionsIncludeLabels: true,
              },
    );
    run.run();
    if (!laidOut.current || layoutChanged) {
      fitGraph(cy);
      initialFit.current = cy.width() <= 0 || cy.height() <= 0;
    } else cy.viewport(viewport);
    laidOut.current = true;
    return () => {
      run.stop();
    };
  }, [ready, topology, layout]);
  useEffect(() => {
    const cy = core.current;
    if (!cy || !ready) return;
    cy.batch(() => {
      for (const node of nodes)
        cy.getElementById(node.key).data({
          label: node.evidence.label,
          kind: node.evidence.kind,
        });
      for (const edge of edges)
        cy.getElementById(edge.id).data({
          family: edge.family,
          relation: edge.relation,
        });
    });
    if (hovered.current) {
      const node = cy.getElementById(hovered.current);
      setHoverLabel(
        node.empty()
          ? null
          : `${node.data("label")} · ${kindLabel(node.data("kind"))}`,
      );
    }
  }, [nodes, edges, ready]);
  useEffect(() => {
    core.current?.style(graphStyles(labelFontSize, labels));
  }, [labelFontSize, labels, ready]);

  useEffect(() => {
    const cy = core.current;
    if (!cy || !ready) return;
    if (pathActive) {
      setTypeFilter("all");
      setFocused(null);
      overview.current = null;
      needsFit.current = false;
    }
    const effectiveFocus = pathActive ? null : focused,
      effectiveFilter = pathActive ? null : currentFilter;
    let keep = cy.nodes();
    if (effectiveFocus) {
      const anchor = cy.getElementById(effectiveFocus.id);
      if (anchor.empty()) {
        setFocused(null);
        if (overview.current) cy.viewport(overview.current);
        overview.current = null;
      } else
        keep =
          effectiveFocus.kind === "node"
            ? anchor.closedNeighborhood().nodes()
            : anchor.connectedNodes();
    } else if (effectiveFilter) {
      const seeds = cy
        .nodes()
        .filter((node) => effectiveFilter.kinds.includes(node.data("kind")));
      keep = seeds.closedNeighborhood().nodes();
    }
    const choiceKey = choice ? `${choice.kind}:${choice.id}` : null;
    const choiceChanged = priorChoice.current !== choiceKey;
    priorChoice.current = choiceKey;
    if (choice && choiceChanged && !pathActive) {
      const target = cy.getElementById(choice.id);
      const revealed =
        choice.kind === "node"
          ? keep.contains(target)
          : target.connectedNodes().every((node) => keep.contains(node));
      // Inspector connections are navigation. Reveal a newly selected loaded
      // target outside the local view; a type change still clears an old choice.
      if (!target.empty() && !revealed) {
        showAll();
        keep = cy.nodes();
      }
    }
    const keepIds = new Set(keep.map((node) => node.id()));
    const keepEdges = cy
      .edges()
      .filter(
        (edge) =>
          keepIds.has(edge.source().id()) && keepIds.has(edge.target().id()),
      );
    const keepEdgeIds = new Set(keepEdges.map((edge) => edge.id()));
    if (hovered.current && !keepIds.has(hovered.current)) {
      hovered.current = null;
      setHoverLabel(null);
    }
    cy.batch(() => {
      cy.nodes().forEach((node) => {
        node.toggleClass("local-hidden", !keepIds.has(node.id()));
      });
      cy.edges().forEach((edge) => {
        edge.toggleClass("local-hidden", !keepEdgeIds.has(edge.id()));
      });
    });
    const next = { nodes: [...keepIds], edges: [...keepEdgeIds] };
    setShown((old) =>
      JSON.stringify(old) === JSON.stringify(next) ? old : next,
    );
    if (
      choice &&
      !(choice.kind === "node" ? keepIds : keepEdgeIds).has(choice.id)
    )
      choose.current(null);
    if (needsFit.current) {
      fitGraph(cy, keep.union(keepEdges));
      needsFit.current = false;
    }
  }, [ready, topology, nodes, choice, typeFilter, focused, pathActive, layout]);
  useEffect(() => {
    const cy = core.current;
    if (!cy || !ready) return;
    cy.batch(() => {
      cy.elements().removeClass("chosen path");
      for (const id of [...pathNodes, ...pathEdges])
        cy.getElementById(id).addClass("path");
      if (choice) cy.getElementById(choice.id).addClass("chosen");
    });
    prioritizeLabels();
  }, [choice, pathNodes, pathEdges, ready, topology, layout]);
  useEffect(() => {
    if (ready && focusRequest !== consumedFocus.current) {
      consumedFocus.current = focusRequest;
      focusSelection();
    }
  }, [focusRequest, ready]);

  function zoom(factor: number) {
    const cy = core.current;
    if (cy)
      cy.zoom({
        level: Math.min(5, Math.max(0.08, cy.zoom() * factor)),
        renderedPosition: { x: cy.width() / 2, y: cy.height() / 2 },
      });
  }
  const mode = pathActive
    ? "Qualified path"
    : focused
      ? "Focused connections"
      : currentFilter
        ? `${currentFilter.label} + direct connections`
        : "All loaded";
  return (
    <div className="graph-stage rc-graph-stage">
      <div className="graph-canvas-toolbar" data-graph-chrome="">
        <Select
          className="graph-canvas-finder"
          aria-label="Inspect graph entity"
          placeholder="Find entity…"
          leftSection={<Search size={iconSize.small} />}
          searchable
          clearable
          allowDeselect={false}
          limit={50}
          value={
            choice?.kind === "node" && shownNodes.has(choice.id)
              ? choice.id
              : null
          }
          data={nodes
            .filter((node) => shownNodes.has(node.key))
            .map((node) => ({
              value: node.key,
              label: `${node.evidence.label} · ${kindLabel(node.evidence.kind)}`,
            }))}
          onOptionSubmit={(id) => choose.current({ kind: "node", id })}
          onChange={(id) => {
            if (!id) choose.current(null);
          }}
        />
        {tools}
        <Popover
          opened={optionsOpened}
          onChange={setOptionsOpened}
          position="bottom-end"
          width={320}
          trapFocus
        >
          <Popover.Target>
            <Button
              variant="default"
              leftSection={<SlidersHorizontal size={iconSize.small} />}
              onClick={() => setOptionsOpened((open) => !open)}
            >
              View
            </Button>
          </Popover.Target>
          <Popover.Dropdown>
            <Stack gap="sm">
              <Select
                aria-label="Graph layout"
                label="Layout"
                value={layout}
                onChange={(value) => setLayout(value ?? "cose")}
                data={[
                  { value: "cose", label: "Connected clusters" },
                  { value: "concentric", label: "Connected rings" },
                  { value: "breadthfirst", label: "Directed layers" },
                  { value: "grid", label: "Grid" },
                ]}
              />
              <div>
                <Text size="sm" mb="xs">
                  Labels
                </Text>
                <SegmentedControl
                  fullWidth
                  aria-label="Graph labels"
                  value={labels}
                  onChange={(value) => setLabels(value as Labels)}
                  data={[
                    { value: "auto", label: "Auto" },
                    { value: "all", label: "All" },
                    { value: "none", label: "None" },
                  ]}
                />
              </div>
              <Select
                aria-label="Inspect graph relationship"
                label="Find relationship"
                placeholder="Select relationship…"
                searchable
                clearable
                allowDeselect={false}
                limit={50}
                value={
                  choice?.kind === "edge" && shownEdges.has(choice.id)
                    ? choice.id
                    : null
                }
                data={edges
                  .filter((edge) => shownEdges.has(edge.id))
                  .map((edge) => ({
                    value: edge.id,
                    label: `${nodeMap.get(edge.from)?.evidence.label ?? edge.from} → ${nodeMap.get(edge.to)?.evidence.label ?? edge.to} · ${kindLabel(edge.relation)}`,
                  }))}
                onOptionSubmit={(id) => {
                  choose.current({ kind: "edge", id });
                  setOptionsOpened(false);
                }}
                onChange={(id) => {
                  if (!id) choose.current(null);
                }}
              />
            </Stack>
          </Popover.Dropdown>
        </Popover>
      </div>
      <div className="graph-canvas-viewbar" data-graph-chrome="">
        <Group
          gap={4}
          className="graph-canvas-filters"
          aria-label="Filter loaded graph entities"
        >
          {availableFilters.length > 0 && (
            <Button
              size="compact-xs"
              variant={typeFilter === "all" ? "light" : "subtle"}
              disabled={pathActive}
              aria-pressed={typeFilter === "all"}
              onClick={showAll}
            >
              All
            </Button>
          )}
          {availableFilters.map((filter) => (
            <Button
              key={filter.value}
              size="compact-xs"
              variant={typeFilter === filter.value ? "light" : "subtle"}
              disabled={pathActive}
              aria-pressed={typeFilter === filter.value}
              onClick={() => {
                if (typeFilter === filter.value && !focused) {
                  if (core.current) fitGraph(core.current);
                  return;
                }
                rememberOverview();
                setFocused(null);
                setTypeFilter(filter.value);
                needsFit.current = true;
              }}
            >
              {filter.label}
            </Button>
          ))}
        </Group>
        <Text
          size="xs"
          c="dimmed"
          className="graph-canvas-counts"
          role="status"
          data-testid="graph-local-counts"
        >
          {mode} · {shown.nodes.length} of {nodes.length} loaded entities ·{" "}
          {shown.edges.length} of {edges.length} relationships
        </Text>
        {localView && (
          <Button size="compact-xs" variant="subtle" onClick={showAll}>
            Show all loaded
          </Button>
        )}
      </div>
      <div className="graph-canvas-viewport">
        {hoverLabel && (
          <Text size="sm" className="graph-canvas-hover-label" role="tooltip">
            {hoverLabel}
          </Text>
        )}
        {localView && shown.nodes.length === 0 && (
          <Text
            size="sm"
            c="dimmed"
            className="graph-canvas-empty"
            role="status"
          >
            No matching entities in this loaded read.
          </Text>
        )}
        <div
          ref={container}
          className="graph-canvas"
          role="img"
          aria-label={`Interactive graph: ${shown.nodes.length} of ${nodes.length} loaded entities and ${shown.edges.length} of ${edges.length} directed relationships. Use Find entity for keyboard inspection.`}
          data-testid="graph-canvas"
          data-ready={ready}
          data-labels={labels}
        />
        <Group className="graph-canvas-controls" gap={4} data-graph-chrome="">
          <Tooltip label="Fit graph">
            <ActionIcon
              variant="subtle"
              aria-label="Fit graph"
              onClick={() => core.current && fitGraph(core.current)}
            >
              <Maximize size={iconSize.small} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label="Zoom in">
            <ActionIcon
              variant="subtle"
              aria-label="Zoom graph in"
              onClick={() => zoom(1.3)}
            >
              <ZoomIn size={iconSize.small} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label="Zoom out">
            <ActionIcon
              variant="subtle"
              aria-label="Zoom graph out"
              onClick={() => zoom(1 / 1.3)}
            >
              <ZoomOut size={iconSize.small} />
            </ActionIcon>
          </Tooltip>
          <Button
            size="compact-xs"
            variant="subtle"
            leftSection={<Focus size={iconSize.small} />}
            disabled={!choice || pathActive}
            onClick={focusSelection}
          >
            Focus selection
          </Button>
        </Group>
      </div>
    </div>
  );
}
