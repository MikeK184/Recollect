import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type MutableRefObject,
} from "react";
import { Button, Select, Text } from "@mantine/core";
import { useReducedMotion } from "@mantine/hooks";
import ForceGraph3D, { type ForceGraphMethods } from "react-force-graph-3d";
import {
  CanvasTexture,
  Group as ObjectGroup,
  Mesh,
  MeshLambertMaterial,
  SphereGeometry,
  Sprite,
  SpriteMaterial,
  Vector2,
  Vector3,
} from "three";
import type { GraphCanvasProps } from "./GraphCanvas";
import type { GraphDisplayGroup } from "./GraphRenderer";
import { palette, fonts } from "./design/tokens";
import {
  spatialModel,
  type SpatialModel,
  type SpatialNode,
  type SpatialLink,
  type SpatialSession,
} from "./graph-3d-model";

const rendererConfig = {
  antialias: true,
  alpha: false,
  powerPreference: "low-power" as const,
};
const kindLabel = (value: string) => value.replaceAll("_", " ");
const baseColor = (kind: string) =>
  kind === "claim"
    ? palette.amber
    : kind === "device"
      ? palette.violet
      : kind === "contributor"
        ? palette.inkSoft
        : kind === "repository"
          ? palette.sageDark
          : palette.sage;

function disposeLabel(object: ObjectGroup) {
  const label = object.getObjectByName("evidence-label");
  if (label instanceof Sprite) {
    label.geometry.dispose();
    label.material.map?.dispose();
    label.material.dispose();
    object.remove(label);
  }
}
// A label is rebuilt only when its text or on-screen scale changes.
function ensureLabel(object: ObjectGroup, text: string, scale: number) {
  const existing = object.getObjectByName("evidence-label");
  if (
    existing instanceof Sprite &&
    existing.userData.text === text &&
    existing.userData.scale === scale
  )
    return;
  disposeLabel(object);
  addLabel(object, text, scale);
}
function addLabel(object: ObjectGroup, text: string, scale: number) {
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  if (!context) return;
  const short = text.length > 44 ? `${text.slice(0, 43)}…` : text;
  context.font = `24px ${fonts.ui}`;
  canvas.width = Math.ceil(context.measureText(short).width) + 28;
  canvas.height = 48;
  context.fillStyle = palette.paperLight;
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.font = `24px ${fonts.ui}`;
  context.fillStyle = palette.ink;
  context.textBaseline = "middle";
  context.fillText(short, 14, canvas.height / 2);
  const label = new Sprite(
    new SpriteMaterial({
      map: new CanvasTexture(canvas),
      depthTest: false,
      sizeAttenuation: false,
    }),
  );
  // Three shares one default Sprite geometry globally. Give this label its own
  // geometry so disposal releases its renderer listeners without invalidating
  // another mounted view or retaining an old WebGLRenderer.
  label.geometry = label.geometry.clone();
  label.name = "evidence-label";
  label.userData = { text, scale };
  label.scale.set((scale * canvas.width) / canvas.height, scale, 1);
  label.position.set(0, 14, 0);
  label.renderOrder = 2;
  const viewportSize = new Vector2(),
    projected = new Vector3();
  label.onBeforeRender = (renderer, _scene, camera) => {
    renderer.getSize(viewportSize);
    label.getWorldPosition(projected).project(camera);
    // Match Three's non-attenuated Sprite shader and move only its screen
    // anchor. Resizing and orbiting must not clip a selected source's name or
    // change node positions, the camera, or the finite layout.
    const width =
      (label.scale.x *
        viewportSize.y *
        Math.abs(camera.projectionMatrix.elements[5])) /
      2;
    if (!Number.isFinite(width) || width <= 0) return;
    const x = ((projected.x + 1) * viewportSize.x) / 2;
    const left = Math.max(
      8,
      Math.min(x - width / 2, viewportSize.x - width - 8),
    );
    label.center.x = (x - left) / width;
  };
  object.add(label);
}

export default function GraphCanvas3D({
  nodes,
  edges,
  choice,
  onChoose,
  pathNodes,
  pathEdges,
  focusRequest = 0,
  tools,
  displayGroups,
  session,
  onFailure,
  topology,
}: GraphCanvasProps & {
  displayGroups: GraphDisplayGroup[];
  topology: string;
  session: MutableRefObject<SpatialSession>;
  onFailure: () => void;
}) {
  const graph = useRef<ForceGraphMethods<SpatialNode, SpatialLink> | undefined>(
    undefined,
  );
  const settleFrame = useRef<number | null>(null);
  const intersecting = useRef(true);
  const topicHeadings = useRef<ObjectGroup | null>(null);
  const mountedApi = useRef<
    ForceGraphMethods<SpatialNode, SpatialLink> | undefined
  >(undefined);
  const container = useRef<HTMLDivElement>(null),
    viewport = useRef<HTMLDivElement>(null);
  const previous = useRef<SpatialModel | undefined>(undefined);
  const failure = useRef(onFailure);
  failure.current = onFailure;
  const choose = useRef(onChoose);
  choose.current = onChoose;
  const selection = useRef(choice);
  selection.current = choice;
  const reducedMotion = useReducedMotion();
  const [size, setSize] = useState({ width: 1, height: 1 });
  const [ready, setReady] = useState(false),
    [settled, setSettled] = useState(false);
  const [hover, setHover] = useState<string | null>(null);
  const [hoverEdge, setHoverEdge] = useState<string | null>(null);
  const labelScale =
    52 /
    (Math.max(1, size.height) *
      (graph.current?.camera().projectionMatrix.elements[5] ?? 2.145));
  const currentTopology = useRef(topology);
  currentTopology.current = topology;
  const restored = useRef(session.current.value?.topology === topology);
  const needsInitialFit = useRef(session.current.value?.topology !== topology),
    consumedFocus = useRef(focusRequest);
  const model = useMemo(() => {
    const next = spatialModel(nodes, edges, displayGroups, previous.current);
    const remembered = session.current.value;
    restored.current = !previous.current && remembered?.topology === topology;
    if (!previous.current && remembered?.topology === topology) {
      for (const node of next.nodes) {
        const position = remembered.positions.get(node.id);
        if (position) Object.assign(node, position);
      }
    }
    previous.current = next;
    return next;
    // Only topology changes create mutable inputs; inspection never reheats.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [topology]);
  // Every node shares one sphere; only materials are per node.
  const sphere = useMemo(() => new SphereGeometry(5, 10, 8), []);
  const nodeObjects = useMemo(
    () =>
      new Map(
        model.nodes.map((node) => {
          const object = new ObjectGroup();
          object.add(
            new Mesh(
              sphere,
              new MeshLambertMaterial({ color: baseColor(node.kind) }),
            ),
          );
          return [node.id, object] as const;
        }),
      ),
    [model, sphere],
  );
  const objectFor = useCallback(
    (node: SpatialNode) => nodeObjects.get(node.id)!,
    [nodeObjects],
  );
  const nodePath = useMemo(
      () => new Set(pathNodes),
      [JSON.stringify(pathNodes)],
    ),
    edgePath = useMemo(() => new Set(pathEdges), [JSON.stringify(pathEdges)]);
  const selectedEdge = choice?.kind === "edge" ? choice.id : null;
  const linkColor = useCallback(
    (link: SpatialLink) =>
      selectedEdge === link.id
        ? palette.ink
        : edgePath.has(link.id)
          ? palette.sageDark
          : link.family === "cross_repository"
            ? palette.violet
            : palette.lineDark,
    [selectedEdge, edgePath],
  );
  const linkWidth = useCallback(
    (link: SpatialLink) =>
      selectedEdge === link.id ? 2.3 : edgePath.has(link.id) ? 1.8 : 0.55,
    [selectedEdge, edgePath],
  );

  useEffect(() => {
    const lookup = new Map(nodes.map((n) => [n.key, n])),
      edgeLookup = new Map(edges.map((e) => [e.id, e]));
    for (const node of model.nodes) {
      const canonical = lookup.get(node.id);
      if (canonical) {
        node.label = canonical.evidence.label;
        node.kind = canonical.evidence.kind;
      }
      const object = nodeObjects.get(node.id)!,
        mesh = object.children[0] as Mesh<SphereGeometry, MeshLambertMaterial>;
      const highlighted = choice?.kind === "node" && choice.id === node.id;
      mesh.material.color.set(
        highlighted
          ? palette.ink
          : nodePath.has(node.id)
            ? palette.sageDark
            : baseColor(node.kind),
      );
      mesh.scale.setScalar(
        highlighted ? 1.5 : nodePath.has(node.id) ? 1.25 : 1,
      );
      if (
        highlighted ||
        hover === node.id ||
        (model.nodes.length <= 12 && !choice && displayGroups.length === 0)
      )
        ensureLabel(object, node.label, labelScale);
      else disposeLabel(object);
    }
    for (const link of model.links) {
      const edge = edgeLookup.get(link.id);
      if (edge) {
        link.relation = edge.relation;
        link.family = edge.family;
      }
    }
  }, [
    nodes,
    edges,
    model,
    nodeObjects,
    choice,
    nodePath,
    hover,
    labelScale,
    displayGroups.length,
  ]);
  useEffect(
    () => () => {
      for (const object of nodeObjects.values()) {
        disposeLabel(object);
        const mesh = object.children[0] as Mesh<
          SphereGeometry,
          MeshLambertMaterial
        >;
        mesh.material.dispose();
      }
    },
    [nodeObjects],
  );
  useEffect(() => () => sphere.dispose(), [sphere]);

  useLayoutEffect(() => {
    const holder = container.current,
      area = viewport.current;
    if (!holder || !area) return;
    let disposed = false;
    const resize = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      // Hidden panes report zero size. Keep the measured scene mounted and
      // pause it through visibility, preserving its camera and GPU lifecycle.
      if (width <= 0 || height <= 0) return;
      setSize((old) =>
        old.width === Math.floor(width) && old.height === Math.floor(height)
          ? old
          : {
              width: Math.max(1, Math.floor(width)),
              height: Math.max(1, Math.floor(height)),
            },
      );
    });
    resize.observe(area);
    const visibility = () => {
      if (!graph.current) return;
      if (document.hidden || !intersecting.current)
        graph.current.pauseAnimation();
      else graph.current.resumeAnimation();
    };
    const observer = new IntersectionObserver(([entry]) => {
      intersecting.current = entry.isIntersecting;
      visibility();
    });
    observer.observe(area);
    document.addEventListener("visibilitychange", visibility);
    const lost = (event: Event) => {
      event.preventDefault();
      if (!disposed) failure.current();
    };
    holder.addEventListener("webglcontextlost", lost, true);
    return () => {
      disposed = true;
      if (settleFrame.current !== null)
        cancelAnimationFrame(settleFrame.current);
      resize.disconnect();
      observer.disconnect();
      document.removeEventListener("visibilitychange", visibility);
      holder.removeEventListener("webglcontextlost", lost, true);
      // The library destructor also disposes simulation, geometry, controls and
      // renderer. Release its GPU context during repeated view/scope switches.
      const api = mountedApi.current ?? graph.current;
      if (api) {
        if (!session.current.invalid && previous.current?.nodes.length) {
          const camera = api.camera().position;
          const target = (
            api.controls() as { target?: { x: number; y: number; z: number } }
          ).target;
          session.current.value = {
            topology: currentTopology.current,
            positions: new Map(
              previous.current.nodes.map((n) => [
                n.id,
                { x: n.x, y: n.y, z: n.z },
              ]),
            ),
            camera: { x: camera.x, y: camera.y, z: camera.z },
            target: target
              ? { x: target.x, y: target.y, z: target.z }
              : { x: 0, y: 0, z: 0 },
          };
        }
        api.pauseAnimation();
        // Layout cleanup runs while the canvas is attached. OrbitControls
        // resolves its keyboard listener target through getRootNode(); after
        // DOM removal that resolves to the detached canvas, leaking the old
        // document listener and its complete scene even after dispose().
        (api.controls() as { dispose?: () => void }).dispose?.();
        api.postProcessingComposer().dispose();
        api.renderer().dispose();
        api.renderer().forceContextLoss();
      }
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    const frame = requestAnimationFrame(() => {
      if (disposed || !graph.current) return;
      try {
        const api = graph.current;
        mountedApi.current = api;
        // The dependency assigns window.scene for debugging. Qualified evidence
        // belongs to this mounted view, not a browser-global debug reference.
        const globalScene = window as Window & { scene?: unknown };
        if (globalScene.scene === api.scene()) delete globalScene.scene;
        api.d3Force("charge")?.strength(-65);
        api
          .d3Force("link")
          ?.distance(48)
          .strength((link: SpatialLink) => {
            const source =
              typeof link.source === "string" ? undefined : link.source;
            const target =
              typeof link.target === "string" ? undefined : link.target;
            return source?.groupId &&
              target?.groupId &&
              source.groupId !== target.groupId
              ? 0.008
              : 0.25;
          });
        api.d3Force("center", null);
        api.d3Force("topic", (alpha: number) => {
          for (const node of previous.current?.nodes ?? []) {
            node.vx = (node.vx ?? 0) + (node.anchor.x - node.x) * alpha * 0.15;
            node.vy = (node.vy ?? 0) + (node.anchor.y - node.y) * alpha * 0.15;
            node.vz = (node.vz ?? 0) - node.z * alpha * 0.15;
          }
        });
        (api.controls() as { enableDamping?: boolean }).enableDamping = false;
        const remembered = session.current.value;
        if (remembered?.topology === currentTopology.current)
          api.cameraPosition(remembered.camera, remembered.target, 0);
        setReady(true);
        if (document.hidden || !intersecting.current) api.pauseAnimation();
      } catch {
        failure.current();
      }
    });
    return () => {
      disposed = true;
      cancelAnimationFrame(frame);
    };
  }, [size.width > 1 && size.height > 1]);

  useEffect(() => {
    const api = graph.current;
    if (!ready || !api) return;
    const headingObjects = new ObjectGroup();
    for (const group of displayGroups) {
      const members = model.nodes.filter((node) => node.groupId === group.id);
      if (!members.length) continue;
      const heading = new ObjectGroup();
      addLabel(heading, group.label, labelScale);
      const anchor = members[0].anchor;
      heading.position.set(anchor.x, anchor.y + 35, anchor.z);
      const label = heading.children[0] as Sprite;
      label.scale.multiplyScalar(1.4);
      headingObjects.add(heading);
    }
    headingObjects.visible = !selection.current && !hover && !hoverEdge;
    topicHeadings.current = headingObjects;
    api.scene().add(headingObjects);
    return () => {
      api.scene().remove(headingObjects);
      for (const heading of [...headingObjects.children])
        disposeLabel(heading as ObjectGroup);
      headingObjects.clear();
      if (topicHeadings.current === headingObjects)
        topicHeadings.current = null;
    };
  }, [ready, model, displayGroups, labelScale]);

  useEffect(() => {
    if (topicHeadings.current)
      topicHeadings.current.visible = !choice && !hover && !hoverEdge;
  }, [choice, hover, hoverEdge]);

  const modelTopology = useRef(topology);
  useEffect(() => {
    setSettled(false);
    // A different node set (for example the shortest path) re-fits once it
    // settles, as a fresh mount would.
    if (modelTopology.current !== topology) {
      modelTopology.current = topology;
      needsInitialFit.current = true;
    }
    if (model.nodes.length === 0) {
      needsInitialFit.current = true;
      previous.current = undefined;
      session.current.value = null;
    }
  }, [model]);
  const fit = (duration = reducedMotion ? 0 : 240, initial = false) => {
    const api = graph.current;
    const visible = model.nodes.filter((node) =>
      [node.x, node.y, node.z].every(Number.isFinite),
    );
    if (!api || !visible.length) return;
    // Fit finite layout coordinates directly. The dependency's geometry bbox
    // can still contain stale world matrices during a zero-tick/hidden first
    // render, and its tiny single-node bbox can place the camera in a sphere.
    const center = new Vector3(
      ...(["x", "y", "z"].map((axis) => {
        const values = visible.map((node) => node[axis as "x" | "y" | "z"]);
        return (Math.min(...values) + Math.max(...values)) / 2;
      }) as [number, number, number]),
    );
    const camera = api.camera(),
      forward = camera.getWorldDirection(new Vector3());
    if (initial && visible.length === 2) {
      const [a, b] = visible;
      const side = new Vector3(-(b.z - a.z), 0, b.x - a.x);
      if (side.lengthSq() > 0.000001) {
        side.normalize();
        if (side.dot(forward) < 0) side.negate();
        forward.copy(side);
      }
    }
    const right = new Vector3().crossVectors(forward, camera.up).normalize();
    if (!right.lengthSq()) right.set(1, 0, 0);
    const up = new Vector3().crossVectors(right, forward).normalize();
    const tanV =
      Math.max(1, size.height - 130) /
      size.height /
      Math.abs(camera.projectionMatrix.elements[5]);
    const tanH =
      Math.max(1, size.width - 130) /
      size.width /
      Math.abs(camera.projectionMatrix.elements[0]);
    const offset = new Vector3();
    let distance = 200;
    for (const node of visible) {
      offset.set(node.x, node.y, node.z).sub(center);
      distance = Math.max(
        distance,
        (Math.abs(offset.dot(up)) + 16) / tanV - offset.dot(forward),
        (Math.abs(offset.dot(right)) + 16) / tanH - offset.dot(forward),
      );
    }
    if (!Number.isFinite(distance)) return;
    const position = center.clone().addScaledVector(forward, -distance);
    api.cameraPosition(position, center, duration);
  };
  const focus = () => {
    const selected = selection.current,
      target =
        selected?.kind === "node"
          ? model.nodes.find((n) => n.id === selected.id)
          : undefined;
    if (!target || !graph.current) return;
    const { x, y, z } = target;
    graph.current.cameraPosition(
      { x: x + 35, y: y + 25, z: z + 170 },
      { x, y, z },
      reducedMotion ? 0 : 240,
    );
  };
  useEffect(() => {
    if (ready && focusRequest !== consumedFocus.current) {
      consumedFocus.current = focusRequest;
      focus();
    }
  }, [focusRequest, ready]);
  const labels = useMemo(
    () => new Map(nodes.map((node) => [node.key, node.evidence.label])),
    [nodes],
  );
  const nodeOptions = useMemo(
    () =>
      nodes.map((node) => ({
        value: node.key,
        label: `${node.evidence.label} · ${kindLabel(node.evidence.kind)}`,
      })),
    [nodes],
  );
  const edgeOptions = useMemo(
    () =>
      edges
        .filter((edge) => labels.has(edge.from) && labels.has(edge.to))
        .map((edge) => ({
          value: edge.id,
          label: `${labels.get(edge.from)} → ${labels.get(edge.to)} · ${kindLabel(edge.relation)}`,
        })),
    [edges, labels],
  );
  const hovered = model.nodes.find((n) => n.id === hover);
  const hoveredLink = model.links.find((link) => link.id === hoverEdge);
  return (
    <div className="graph-stage rc-graph-stage graph-spatial-stage">
      <div className="graph-canvas-toolbar" data-graph-chrome="">
        <Select
          className="graph-canvas-finder"
          aria-label="Inspect graph entity"
          placeholder="Find entity…"
          searchable
          clearable
          allowDeselect={false}
          limit={50}
          value={
            choice?.kind === "node" && labels.has(choice.id) ? choice.id : null
          }
          data={nodeOptions}
          onOptionSubmit={(id) => choose.current({ kind: "node", id })}
          onChange={(id) => {
            if (!id) choose.current(null);
          }}
        />
        {tools}
      </div>
      <div className="graph-spatial-viewport" ref={viewport}>
        {hovered && (
          <Text className="graph-canvas-hover-label" role="tooltip" size="sm">
            {hovered.label} · {kindLabel(hovered.kind)}
          </Text>
        )}
        {hoveredLink && (
          <Text className="graph-canvas-hover-label" role="tooltip" size="sm">
            {labels.get(hoveredLink.from)} → {labels.get(hoveredLink.to)} ·{" "}
            {kindLabel(hoveredLink.relation)}
          </Text>
        )}
        {nodes.length === 0 && (
          <Text className="graph-canvas-empty" role="status">
            No eligible entities in this loaded read.
          </Text>
        )}
        <div
          ref={container}
          className="graph-spatial-canvas"
          role="img"
          aria-label={`Interactive 3D graph: ${model.nodes.length} loaded nodes and ${model.links.length} directed relationships. Use Find entity and Find relationship for keyboard inspection.`}
          data-testid="graph-canvas"
          data-dimension="3d"
          data-ready={ready}
          data-layout-ready={settled}
          data-nodes={model.nodes.length}
          data-edges={model.links.length}
          data-path-nodes={pathNodes.length}
          data-path-edges={pathEdges.length}
        >
          {size.width > 1 && size.height > 1 && (
            <ForceGraph3D
              ref={graph}
              graphData={model}
              width={size.width}
              height={size.height}
              rendererConfig={rendererConfig}
              controlType="orbit"
              backgroundColor={palette.paper}
              showNavInfo={false}
              nodeThreeObject={objectFor}
              nodeLabel={() => ""}
              linkLabel={() => ""}
              linkColor={linkColor}
              linkWidth={linkWidth}
              linkOpacity={0.7}
              linkCurvature="curvature"
              linkCurveRotation="rotation"
              linkResolution={4}
              linkDirectionalArrowLength={4}
              linkDirectionalArrowRelPos={0.9}
              linkDirectionalArrowResolution={4}
              linkDirectionalArrowColor={linkColor}
              enableNodeDrag={false}
              warmupTicks={restored.current ? 0 : reducedMotion ? 100 : 30}
              cooldownTicks={restored.current ? 0 : reducedMotion ? 0 : 80}
              cooldownTime={1600}
              d3AlphaDecay={0.05}
              onEngineStop={() => {
                if (settleFrame.current !== null)
                  cancelAnimationFrame(settleFrame.current);
                // The library emits stop before it updates object positions and
                // world matrices. Fit on the next rendered frame, including the
                // zero-tick reduced-motion path; fitting immediately sees every
                // object at the origin and can put the camera inside the scene.
                settleFrame.current = requestAnimationFrame(() => {
                  if (needsInitialFit.current) {
                    needsInitialFit.current = false;
                    fit(0, true);
                  }
                  setSettled(true);
                });
              }}
              onNodeClick={(node) =>
                choose.current({ kind: "node", id: node.id })
              }
              onLinkClick={(link) =>
                choose.current({ kind: "edge", id: link.id })
              }
              onNodeHover={(node) => setHover(node?.id ?? null)}
              onLinkHover={(link) => setHoverEdge(link?.id ?? null)}
              onBackgroundClick={() => choose.current(null)}
            />
          )}
        </div>
      </div>
      <div className="graph-spatial-controls" data-graph-chrome="">
        <Select
          aria-label="Inspect graph relationship"
          placeholder="Find relationship…"
          searchable
          clearable
          allowDeselect={false}
          limit={50}
          value={choice?.kind === "edge" ? choice.id : null}
          data={edgeOptions}
          onOptionSubmit={(id) => choose.current({ kind: "edge", id })}
          onChange={(id) => {
            if (!id) choose.current(null);
          }}
        />
        <Button size="compact-xs" variant="subtle" onClick={() => fit()}>
          Fit graph
        </Button>
        <Button
          size="compact-xs"
          variant="subtle"
          disabled={choice?.kind !== "node"}
          onClick={focus}
        >
          Focus selection
        </Button>
        <Button
          size="compact-xs"
          variant="subtle"
          onClick={() => {
            restored.current = false;
            setSettled(false);
            graph.current?.d3ReheatSimulation();
          }}
        >
          Rearrange
        </Button>
      </div>
      <div className="graph-spatial-caption" role="status">
        {model.nodes.length} loaded nodes · {model.links.length} directed
        relationships · {settled ? "Layout settled" : "Arranging"}. Drag to
        orbit, scroll to zoom. Position is presentation; evidence defines
        relationships.
      </div>
    </div>
  );
}
