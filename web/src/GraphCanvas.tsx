import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import cytoscape, {
  type Collection,
  type Core,
  type StylesheetJson,
} from "cytoscape";
import { useReducedMotion } from "motion/react";
import {
  Focus,
  Scan,
  Search,
  SlidersHorizontal,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { fonts } from "./design/tokens";
import {
  Button,
  IconButton,
  Label,
  Popover,
  PopoverContent,
  PopoverTrigger,
  SegmentedControl,
  Select,
} from "./components/ui";
import { cn } from "./lib/utils";
import {
  ChromeSlot,
  chromeCard,
  useGraphStage,
} from "./features/graph/GraphStage";
import {
  groupShape,
  groupSwatch,
  kindGroup,
  useGraphColors,
  type GraphColors,
  type KindGroup,
} from "./features/graph/graphColors";
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
  /** Extra canvas actions (explore, path toggle) placed in the toolbar. */
  tools?: ReactNode;
  /** Leading toolbar control supplied by the renderer (the 2D/3D choice). */
  leading?: ReactNode;
};
const labelMaxWidth = 145;
const typeFilters: { value: string; label: string; group: KindGroup }[] = [
  { value: "memory", label: "Memory", group: "memory" },
  { value: "sources", label: "Sources & inputs", group: "sources" },
  { value: "repositories", label: "Repository evidence", group: "repositories" },
  { value: "people", label: "People", group: "people" },
  { value: "agents", label: "Agents", group: "agents" },
];
type Labels = "auto" | "all" | "none";
type Viewport = { zoom: number; pan: cytoscape.Position };
type Insets = () => { top: number; bottom: number };
const kindLabel = (kind: string) => kind.replaceAll("_", " ");
const slow = 360;

const nodeSize: Record<KindGroup, number> = {
  memory: 14,
  sources: 14,
  repositories: 16,
  people: 14,
  agents: 16,
  other: 14,
};
const nodeShape: Record<KindGroup, string> = {
  memory: "round-rectangle",
  sources: "ellipse",
  repositories: "hexagon",
  people: "round-triangle",
  agents: "diamond",
  other: "ellipse",
};

// Token-themed stylesheet: kind colours fill small shaped nodes, labels sit on
// popover pills, rest edges are border-strong and selection/path are brand.
// Re-applied on theme, label or motion changes; it never runs a layout.
const graphStyles = (
  colors: GraphColors,
  labels: Labels,
  motion: boolean,
): StylesheetJson => [
  {
    selector: "node",
    style: {
      "background-color": colors.kinds.other,
      shape: "ellipse",
      width: nodeSize.other,
      height: nodeSize.other,
      "border-width": 2,
      "border-color": colors.card,
      label: labels === "none" ? "" : "data(label)",
      "min-zoomed-font-size": labels === "auto" ? 10 : 0,
      color: colors.foreground,
      "font-size": 12,
      "font-weight": 500,
      "font-family": fonts.ui,
      "text-valign": "bottom",
      "text-margin-y": 8,
      "text-wrap": "ellipsis",
      "text-max-width": `${labelMaxWidth}px`,
      "text-background-color": colors.popover,
      "text-background-opacity": 1,
      "text-background-shape": "roundrectangle",
      "text-background-padding": "4px",
      "text-border-color": colors.border,
      "text-border-width": 1,
      "text-border-opacity": 1,
      "overlay-color": colors.brand,
      "overlay-padding": 6,
      "overlay-opacity": 0,
      "transition-property":
        "border-width, border-color, background-color, overlay-opacity, text-border-color",
      "transition-duration": motion ? 0.16 : 0,
    },
  },
  ...(Object.keys(nodeShape) as KindGroup[]).map((group) => ({
    selector: `node[group = "${group}"]`,
    style: {
      "background-color": colors.kinds[group],
      shape: nodeShape[group] as "ellipse",
      width: nodeSize[group],
      height: nodeSize[group],
    },
  })),
  {
    selector: "node.label-priority",
    style: { "min-zoomed-font-size": 0 },
  },
  {
    selector: "edge",
    style: {
      width: 1.25,
      "line-color": colors.edge,
      "target-arrow-color": colors.edge,
      "target-arrow-shape": "triangle",
      "curve-style": "bezier",
      "arrow-scale": 0.8,
      "overlay-color": colors.brand,
      "overlay-opacity": 0,
      "transition-property": "line-color, target-arrow-color, width, overlay-opacity",
      "transition-duration": motion ? 0.16 : 0,
    },
  },
  {
    selector: 'edge[family = "cross_repository"]',
    style: {
      "line-style": "dashed",
      "line-color": colors.cross,
      "target-arrow-color": colors.cross,
    },
  },
  {
    selector: "edge.hot",
    style: {
      "line-color": colors.brand,
      "target-arrow-color": colors.brand,
      width: 1.75,
    },
  },
  {
    selector: "edge.path",
    style: {
      "line-color": colors.brand,
      "target-arrow-color": colors.brand,
      width: 3,
    },
  },
  {
    selector: "node.path",
    style: { "border-color": colors.brand, "border-width": 3 },
  },
  {
    selector: "node.chosen",
    style: {
      "border-color": colors.brand,
      "border-width": 4,
      "overlay-opacity": 0.14,
      "text-border-color": colors.brand,
      "font-weight": 600,
    },
  },
  {
    selector: "edge.chosen",
    style: {
      "line-color": colors.brand,
      "target-arrow-color": colors.brand,
      width: 3,
      "overlay-opacity": 0.12,
    },
  },
  { selector: ".local-hidden", style: { display: "none" } },
];

/**
 * Fit the visible graph inside the free band between the canvas chrome rows.
 * Explicit fits glide over motion-slow; the first fit on load is immediate.
 */
/**
 * Elements shown on the canvas. Not `:visible`: Cytoscape counts a node that
 * is fading in (opacity 0) as invisible, which would skip the first fit.
 */
const shownElements = (cy: Core) => cy.elements().not(".local-hidden");

function fitGraph(
  cy: Core,
  {
    elements = shownElements(cy),
    insets,
    duration = 0,
  }: { elements?: Collection; insets?: Insets; duration?: number } = {},
) {
  if (!cy.container() || elements.empty()) return;
  const pad = 32;
  const chrome = insets?.() ?? { top: 0, bottom: 0 };
  let top = Math.max(pad, chrome.top + 16),
    bottom = Math.max(pad, chrome.bottom + 16);
  // A short canvas keeps the whole height rather than a sliver between rows.
  if (cy.height() - top - bottom < 160) top = bottom = pad;
  const width = Math.max(1, cy.width() - pad * 2),
    height = Math.max(1, cy.height() - top - bottom);
  const bounds = elements.boundingBox({ includeLabels: true });
  const zoom = Math.max(
    0.08,
    Math.min(
      1.35,
      width / Math.max(1, bounds.w),
      height / Math.max(1, bounds.h),
    ),
  );
  const pan = {
    x: pad + width / 2 - (bounds.x1 + bounds.w / 2) * zoom,
    y: top + height / 2 - (bounds.y1 + bounds.h / 2) * zoom,
  };
  if (duration > 0) {
    cy.stop(true);
    cy.animate({ zoom, pan }, { duration, easing: "ease-out-cubic" });
  } else cy.viewport({ zoom, pan });
}

function overlappingLabels(cy: Core) {
  const boxes = shownElements(cy)
    .nodes()
    .map((node) => node.boundingBox({ includeLabels: true }));
  return boxes.some((a, index) =>
    boxes
      .slice(index + 1)
      .some(
        (b) =>
          a.x1 < b.x2 + 8 &&
          b.x1 < a.x2 + 8 &&
          a.y1 < b.y2 + 8 &&
          b.y1 < a.y2 + 8,
      ),
  );
}

export function GraphSwatch({ group }: { group: KindGroup }) {
  return (
    <span
      aria-hidden="true"
      className={cn("inline-block size-2 shrink-0", groupSwatch[group], groupShape[group])}
    />
  );
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
  leading,
}: GraphCanvasProps) {
  const colors = useGraphColors();
  const motion = !useReducedMotion();
  const stage = useGraphStage();
  const insets = useRef<Insets | undefined>(stage?.insets);
  insets.current = stage?.insets;
  const motionRef = useRef(motion);
  motionRef.current = motion;
  const container = useRef<HTMLDivElement>(null),
    core = useRef<Core | null>(null);
  const choose = useRef(onChoose);
  choose.current = onChoose;
  const selection = useRef(choice);
  selection.current = choice;
  const priorChoice = useRef(choice ? `${choice.kind}:${choice.id}` : null);
  const hovered = useRef<string | null>(null);
  const [hover, setHover] = useState<{
    label: string;
    x: number;
    y: number;
  } | null>(null);
  const [layout, setLayout] = useState("cose"),
    [labels, setLabels] = useState<Labels>("auto");
  const [typeFilter, setTypeFilter] = useState("all"),
    [focused, setFocused] = useState<GraphChoice>(null);
  const [optionsOpened, setOptionsOpened] = useState(false),
    [ready, setReady] = useState(false);
  const [layoutReady, setLayoutReady] = useState(false);
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
    nodes.some((node) => kindGroup(node.evidence.kind) === filter.group),
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
  const glide = () => (motionRef.current ? slow : 0);

  function rememberOverview() {
    const cy = core.current;
    if (cy && !overview.current)
      overview.current = { zoom: cy.zoom(), pan: { ...cy.pan() } };
  }
  function restoreOverview(cy: Core) {
    if (!overview.current) return;
    cy.stop(true);
    if (glide())
      cy.animate(
        { zoom: overview.current.zoom, pan: overview.current.pan },
        { duration: glide(), easing: "ease-out-cubic" },
      );
    else cy.viewport(overview.current);
  }
  function showAll() {
    setTypeFilter("all");
    setFocused(null);
    needsFit.current = false;
    const cy = core.current;
    if (cy) {
      cy.elements().removeClass("local-hidden");
      restoreOverview(cy);
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
  function hoverAt(node: cytoscape.NodeSingular) {
    const box = node.renderedBoundingBox({ includeLabels: false });
    setHover({
      label: `${node.data("label")} · ${kindLabel(node.data("kind"))}`,
      x: (box.x1 + box.x2) / 2,
      y: box.y1,
    });
  }

  useEffect(() => {
    if (!container.current) return;
    const cy = cytoscape({
      container: container.current,
      style: graphStyles(colors, "auto", motionRef.current),
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
      if (hovered.current) hoverAt(event.target);
      else setHover(null);
      prioritizeLabels();
    };
    // A person panning or zooming takes over the camera: stop any glide and
    // hide the hover label rather than leaving it at a stale position.
    const takeOver = () => cy.stop(true);
    const moved = () => setHover(null);
    cy.on("tap", tap);
    cy.on("mouseover mouseout", "node", hover);
    cy.on("tapstart scrollzoom", takeOver);
    cy.on("viewport", moved);
    const resize = new ResizeObserver(() => {
      // Opening the inspector changes dimensions, never the user's viewport.
      cy.resize();
      if (
        initialFit.current &&
        laidOut.current &&
        cy.nodes().length &&
        cy.width() > 0 &&
        cy.height() > 0
      ) {
        fitGraph(cy, { insets: insets.current });
        initialFit.current = false;
      }
    });
    resize.observe(container.current);
    setReady(true);
    return () => {
      resize.disconnect();
      cy.off("tap", tap);
      cy.off("mouseover mouseout", "node", hover);
      cy.off("tapstart scrollzoom", takeOver);
      cy.off("viewport", moved);
      cy.stop(true);
      cy.destroy();
      core.current = null;
    };
    // The core is created once; theme and labels restyle it in place.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Only topology or explicit layout changes run bounded positioning. Unchanged
  // polling, hover, labels, filters, theme and resize never run a layout.
  useEffect(() => {
    const cy = core.current;
    if (!cy || !ready) return;
    setLayoutReady(false);
    hovered.current = null;
    setHover(null);
    const positions = new Map(
      cy.nodes().map((node) => [node.id(), { ...node.position() }] as const),
    );
    const knownEdges = new Set(cy.edges().map((edge) => edge.id()));
    const viewport = { zoom: cy.zoom(), pan: { ...cy.pan() } },
      layoutChanged = previousLayout.current !== layout;
    const columns = Math.max(1, Math.ceil(Math.sqrt(nodes.length)));
    previousLayout.current = layout;
    cy.stop(true);
    cy.batch(() => {
      cy.elements().remove();
      cy.add([
        ...nodes.map((node, index) => ({
          data: {
            id: node.key,
            label: node.evidence.label,
            kind: node.evidence.kind,
            group: kindGroup(node.evidence.kind),
          },
          position: positions.get(node.key) ?? {
            x: (index % columns) * 200,
            y: Math.floor(index / columns) * 140,
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
    // New nodes and edges fade and scale in from 0.9 (motion.md); unchanged
    // ones keep their place. Reduced motion shows them at once.
    const freshNodes = cy.nodes().filter((node) => !positions.has(node.id()));
    const freshEdges = cy.edges().filter((edge) => !knownEdges.has(edge.id()));
    // Only incremental arrivals animate: per-element animation of a large
    // first load (hundreds of nodes, thousands of edges) would hold the main
    // thread, so a big graph appears at once.
    const entering =
      motionRef.current &&
      !layoutChanged &&
      freshNodes.length <= 60 &&
      freshEdges.length <= 240;
    if (entering)
      cy.batch(() => {
        freshNodes.forEach((node) => {
          const size = nodeSize[node.data("group") as KindGroup] ?? 14;
          node.style({ opacity: 0, width: size * 0.9, height: size * 0.9 });
        });
        freshEdges.style({ opacity: 0 });
      });
    const enter = () => {
      if (!entering) return;
      freshNodes.forEach((node) => {
        const size = nodeSize[node.data("group") as KindGroup] ?? 14;
        node.animate(
          { style: { opacity: 1, width: size, height: size } },
          {
            duration: slow,
            easing: "ease-out-cubic",
            complete: () => node.removeStyle("opacity width height"),
          },
        );
      });
      freshEdges.animate(
        { style: { opacity: 1 } },
        {
          duration: slow,
          easing: "ease-out-cubic",
          complete: () => freshEdges.removeStyle("opacity"),
        },
      );
    };
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
    let fallback: cytoscape.Layouts | null = null;
    const fitted = () => {
      if (!laidOut.current || layoutChanged) {
        // The camera fits once on load (and for an explicit layout choice);
        // later topology changes keep the person's viewport.
        fitGraph(cy, {
          insets: insets.current,
          duration: laidOut.current ? glide() : 0,
        });
        initialFit.current = cy.width() <= 0 || cy.height() <= 0;
      } else cy.viewport(viewport);
      laidOut.current = true;
      enter();
      setLayoutReady(true);
    };
    const completed = () => {
      // COSE can settle disconnected, collinear components on top of each
      // other. Use the library's label-aware grid only when settled bounds
      // collide; do this before publishing readiness or fitting the viewport.
      if (layout === "cose" && overlappingLabels(cy)) {
        fallback = cy.layout({
          name: "grid",
          animate: false,
          fit: false,
          avoidOverlap: true,
          avoidOverlapPadding: 20,
          nodeDimensionsIncludeLabels: true,
        });
        fallback.one("layoutstop", fitted);
        fallback.run();
      } else fitted();
    };
    run.one("layoutstop", completed);
    run.run();
    return () => {
      run.off("layoutstop", completed);
      run.stop();
      fallback?.off("layoutstop", fitted);
      fallback?.stop();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, topology, layout]);
  useEffect(() => {
    const cy = core.current;
    if (!cy || !ready) return;
    cy.batch(() => {
      for (const node of nodes)
        cy.getElementById(node.key).data({
          label: node.evidence.label,
          kind: node.evidence.kind,
          group: kindGroup(node.evidence.kind),
        });
      for (const edge of edges)
        cy.getElementById(edge.id).data({
          family: edge.family,
          relation: edge.relation,
        });
    });
    if (hovered.current) {
      const node = cy.getElementById(hovered.current);
      if (node.empty()) setHover(null);
      else hoverAt(node);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nodes, edges, ready]);
  // Light/Ink, label and motion changes restyle in place without layout.
  useEffect(() => {
    core.current?.style(graphStyles(colors, labels, motion));
  }, [colors, labels, motion, ready]);

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
        restoreOverview(cy);
        overview.current = null;
      } else
        keep =
          effectiveFocus.kind === "node"
            ? anchor.closedNeighborhood().nodes()
            : anchor.connectedNodes();
    } else if (effectiveFilter) {
      const seeds = cy
        .nodes()
        .filter((node) => node.data("group") === effectiveFilter.group);
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
      setHover(null);
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
      fitGraph(cy, {
        elements: keep.union(keepEdges),
        insets: insets.current,
        duration: glide(),
      });
      needsFit.current = false;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, topology, nodes, choice, typeFilter, focused, pathActive, layout]);
  useEffect(() => {
    const cy = core.current;
    if (!cy || !ready) return;
    cy.batch(() => {
      cy.elements().removeClass("chosen path hot");
      for (const id of [...pathNodes, ...pathEdges])
        cy.getElementById(id).addClass("path");
      if (choice) {
        const chosen = cy.getElementById(choice.id);
        chosen.addClass("chosen");
        if (choice.kind === "node") chosen.connectedEdges().addClass("hot");
      }
    });
    prioritizeLabels();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [choice, pathNodes, pathEdges, ready, topology, layout]);
  useEffect(() => {
    if (ready && focusRequest !== consumedFocus.current) {
      consumedFocus.current = focusRequest;
      focusSelection();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusRequest, ready]);

  function zoom(factor: number) {
    const cy = core.current;
    if (!cy) return;
    const level = Math.min(5, Math.max(0.08, cy.zoom() * factor));
    const center = { x: cy.width() / 2, y: cy.height() / 2 };
    cy.stop(true);
    if (glide()) {
      // Zoom about the canvas centre: keep the centre's model point fixed.
      const ratio = level / cy.zoom(),
        pan = cy.pan();
      cy.animate(
        {
          zoom: level,
          pan: {
            x: center.x - (center.x - pan.x) * ratio,
            y: center.y - (center.y - pan.y) * ratio,
          },
        },
        { duration: 160, easing: "ease-out-cubic" },
      );
    } else cy.zoom({ level, renderedPosition: center });
  }
  const mode = pathActive
    ? "Qualified path"
    : focused
      ? "Focused connections"
      : currentFilter
        ? `${currentFilter.label} + direct connections`
        : "All loaded";
  return (
    <div className="graph-stage relative flex min-h-0 min-w-0 flex-1 flex-col">
      <ChromeSlot zone="top-right" order={0}>
        <div
          className={cn(chromeCard, "flex flex-wrap items-center justify-end gap-1 p-1")}
          data-graph-chrome=""
        >
          {leading}
          <Select
            className="w-52"
            size="sm"
            aria-label="Inspect graph entity"
            placeholder="Find entity…"
            leftSection={<Search />}
            searchable
            limit={50}
            clearable
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
            onChange={(id) =>
              choose.current(id ? { kind: "node", id } : null)
            }
          />
          {tools}
          <Popover open={optionsOpened} onOpenChange={setOptionsOpened}>
            <PopoverTrigger asChild>
              <Button variant="ghost" size="sm">
                <SlidersHorizontal aria-hidden="true" />
                View
              </Button>
            </PopoverTrigger>
            <PopoverContent align="end" className="grid w-80 gap-3">
              <Select
                aria-label="Graph layout"
                label="Layout"
                size="sm"
                value={layout}
                onChange={(value) => setLayout(value ?? "cose")}
                data={[
                  { value: "cose", label: "Connected clusters" },
                  { value: "concentric", label: "Connected rings" },
                  { value: "breadthfirst", label: "Directed layers" },
                  { value: "grid", label: "Grid" },
                ]}
              />
              <div className="grid gap-1.5">
                <Label>Labels</Label>
                <SegmentedControl
                  fullWidth
                  size="sm"
                  label="Graph labels"
                  value={labels}
                  onChange={setLabels}
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
                size="sm"
                searchable
                limit={50}
                clearable
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
                onChange={(id) => {
                  choose.current(id ? { kind: "edge", id } : null);
                  if (id) setOptionsOpened(false);
                }}
              />
            </PopoverContent>
          </Popover>
        </div>
      </ChromeSlot>
      <ChromeSlot zone="top-right" order={1}>
        <div
          className={cn(chromeCard, "flex items-center gap-0.5 p-1")}
          data-graph-chrome=""
        >
          <IconButton size="icon-sm" label="Zoom graph in" onClick={() => zoom(1.3)}>
            <ZoomIn aria-hidden="true" />
          </IconButton>
          <IconButton
            size="icon-sm"
            label="Zoom graph out"
            onClick={() => zoom(1 / 1.3)}
          >
            <ZoomOut aria-hidden="true" />
          </IconButton>
          <IconButton
            size="icon-sm"
            label="Fit graph"
            onClick={() =>
              core.current &&
              fitGraph(core.current, {
                insets: insets.current,
                duration: glide(),
              })
            }
          >
            <Scan aria-hidden="true" />
          </IconButton>
          <IconButton
            size="icon-sm"
            label="Focus selection"
            disabled={!choice || pathActive}
            onClick={focusSelection}
          >
            <Focus aria-hidden="true" />
          </IconButton>
        </div>
      </ChromeSlot>
      <ChromeSlot zone="bottom-left" order={0}>
        <div
          className={cn(
            chromeCard,
            "flex max-w-[640px] flex-wrap items-center gap-x-1 gap-y-1 p-1",
          )}
          data-graph-chrome=""
        >
          {availableFilters.length > 0 && (
            <div
              role="group"
              aria-label="Filter loaded graph entities"
              className="flex flex-wrap items-center gap-0.5"
            >
              <Button
                size="sm"
                variant="ghost"
                className="aria-pressed:bg-accent aria-pressed:text-foreground"
                disabled={pathActive}
                aria-pressed={typeFilter === "all"}
                onClick={showAll}
              >
                All
              </Button>
              {availableFilters.map((filter) => (
                <Button
                  key={filter.value}
                  size="sm"
                  variant="ghost"
                  className="aria-pressed:bg-accent aria-pressed:text-foreground"
                  disabled={pathActive}
                  aria-pressed={typeFilter === filter.value}
                  onClick={() => {
                    if (typeFilter === filter.value && !focused) {
                      if (core.current)
                        fitGraph(core.current, {
                          insets: insets.current,
                          duration: glide(),
                        });
                      return;
                    }
                    rememberOverview();
                    setFocused(null);
                    setTypeFilter(filter.value);
                    needsFit.current = true;
                  }}
                >
                  <GraphSwatch group={filter.group} />
                  {filter.label}
                </Button>
              ))}
            </div>
          )}
          <p
            className="mono m-0 px-2 text-caption text-muted-foreground"
            role="status"
            data-testid="graph-local-counts"
          >
            {mode} · {shown.nodes.length} of {nodes.length} loaded nodes ·{" "}
            {shown.edges.length} of {edges.length} relationships
          </p>
          {localView && (
            <Button size="sm" variant="ghost" onClick={showAll}>
              Show all loaded
            </Button>
          )}
        </div>
      </ChromeSlot>
      <div className="relative min-h-0 flex-1">
        <div
          ref={container}
          // Cytoscape adds an unlayered `position: relative` class to its
          // container; keep it pinned to the full canvas region.
          className="graph-canvas !absolute inset-0"
          role="img"
          aria-label={`Interactive graph: ${shown.nodes.length} of ${nodes.length} loaded nodes and ${shown.edges.length} of ${edges.length} directed relationships. Use Find entity for keyboard inspection.`}
          data-testid="graph-canvas"
          data-ready={ready}
          data-layout-ready={layoutReady}
          data-labels={labels}
        />
        {hover && (
          <p
            role="tooltip"
            className="pointer-events-none absolute z-20 m-0 max-w-[26rem] -translate-x-1/2 -translate-y-[calc(100%+8px)] animate-pop rounded-sm bg-primary px-2 py-[5px] text-caption font-medium text-primary-foreground shadow-md [overflow-wrap:anywhere]"
            style={{ left: hover.x, top: hover.y }}
          >
            {hover.label}
          </p>
        )}
        {localView && shown.nodes.length === 0 && (
          <p
            className="pointer-events-none absolute inset-x-4 top-1/2 m-0 text-center text-body text-muted-foreground"
            role="status"
          >
            No matching entities in this loaded read.
          </p>
        )}
      </div>
    </div>
  );
}
