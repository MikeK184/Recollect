import type { GraphCanvasProps, GraphRenderNode } from "./GraphCanvas";
import type { GraphDisplayGroup } from "./GraphRenderer";

export type SpatialNode = {
  id: string;
  label: string;
  kind: string;
  groupId: string;
  x: number;
  y: number;
  z: number;
  vx?: number;
  vy?: number;
  vz?: number;
  fx?: number;
  fy?: number;
  fz?: number;
  anchor: { x: number; y: number; z: number };
};
export type SpatialLink = {
  id: string;
  source: string | SpatialNode;
  target: string | SpatialNode;
  from: string;
  to: string;
  relation: string;
  family: string;
  curvature: number;
  rotation: number;
};
export type SpatialViewState = {
  topology: string;
  positions: Map<string, { x: number; y: number; z: number }>;
  camera: { x: number; y: number; z: number };
  target: { x: number; y: number; z: number };
};
export type SpatialSession = {
  value: SpatialViewState | null;
  invalid: boolean;
};
export type SpatialModel = { nodes: SpatialNode[]; links: SpatialLink[] };

export function spatialTopology(
  nodes: GraphCanvasProps["nodes"],
  edges: GraphCanvasProps["edges"],
  groups: GraphDisplayGroup[],
) {
  return JSON.stringify([
    nodes.map((n) => n.key).sort(),
    edges
      .map((e) => [e.id, e.from, e.to])
      .sort((a, b) => a[0].localeCompare(b[0])),
    groups
      .map((g) => [g.id, [...g.nodeKeys].sort()])
      .sort((a, b) => String(a[0]).localeCompare(String(b[0]))),
  ]);
}

/** The engine mutates its inputs. Never give it canonical API objects. */
export function spatialModel(
  nodes: GraphRenderNode[],
  edges: GraphCanvasProps["edges"],
  groups: GraphDisplayGroup[],
  previous?: SpatialModel,
): SpatialModel {
  const prior = new Map(previous?.nodes.map((n) => [n.id, n]));
  const ordered = [...nodes].sort((a, b) => a.key.localeCompare(b.key));
  const memberships = new Map<string, string>();
  const groupIds = groups.map((g) => g.id).sort();
  for (const group of [...groups].sort((a, b) => a.id.localeCompare(b.id)))
    for (const key of group.nodeKeys)
      if (!memberships.has(key)) memberships.set(key, group.id);
  const indices = new Map<string, number>();
  const counts = new Map<string, number>();
  for (const node of ordered) {
    const group = memberships.get(node.key) ?? "";
    counts.set(group, (counts.get(group) ?? 0) + 1);
  }
  const newNodes = ordered.map((node) => {
    const group = memberships.get(node.key) ?? "";
    const groupIndex = group ? groupIds.indexOf(group) : groupIds.length;
    const groupColumns = Math.max(1, Math.ceil(Math.sqrt(groupIds.length + 1)));
    const anchor = {
      x: ((groupIndex % groupColumns) - (groupColumns - 1) / 2) * 280,
      y:
        (-Math.floor(groupIndex / groupColumns) +
          (Math.ceil((groupIds.length + 1) / groupColumns) - 1) / 2) *
        230,
      z: 0,
    };
    const index = indices.get(group) ?? 0;
    indices.set(group, index + 1);
    const columns = Math.ceil(Math.sqrt(counts.get(group) ?? 1));
    const old = prior.get(node.key);
    return {
      id: node.key,
      groupId: group,
      label: node.evidence.label,
      kind: node.evidence.kind,
      anchor,
      x: old?.x ?? anchor.x + ((index % columns) - (columns - 1) / 2) * 35,
      y:
        old?.y ??
        anchor.y +
          (-Math.floor(index / columns) +
            (Math.ceil((counts.get(group) ?? 1) / columns) - 1) / 2) *
            35,
      z: old?.z ?? ((index % 3) - 1) * 12,
    };
  });
  const ids = new Set(newNodes.map((n) => n.id));
  const validEdges = edges.filter((e) => ids.has(e.from) && ids.has(e.to));
  const pairs = new Map<string, string[]>();
  for (const edge of validEdges) {
    const key = JSON.stringify([edge.from, edge.to].sort());
    pairs.set(key, [...(pairs.get(key) ?? []), edge.id]);
  }
  for (const pair of pairs.values()) pair.sort();
  return {
    nodes: newNodes,
    links: validEdges.map((edge) => {
      const siblings = pairs.get(JSON.stringify([edge.from, edge.to].sort()))!;
      const index = siblings.indexOf(edge.id);
      // Reverse edges use the same positive magnitude but opposite curve orientation.
      // Every parallel edge (including self loops) gets a distinct arc.
      const direction = edge.from.localeCompare(edge.to) <= 0 ? 1 : -1;
      return {
        id: edge.id,
        from: edge.from,
        to: edge.to,
        source: edge.from,
        target: edge.to,
        relation: edge.relation,
        family: edge.family,
        curvature:
          edge.from === edge.to
            ? 0.6 + index * 0.22
            : siblings.length > 1
              ? direction * (0.16 + index * 0.13)
              : 0,
        rotation: edge.from === edge.to ? index * 1.2 : 0,
      };
    }),
  };
}
