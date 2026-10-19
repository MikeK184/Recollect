import { useEffect, useMemo, useRef, type ReactNode } from "react";
import type { Brain } from "../../api";
import type { SpatialSession } from "../../graph-3d-model";
import { useKnowledgeSelection } from "../knowledge/selection";
import type {
  ExplorerNode,
  ExplorerPath,
  ExplorerView,
} from "./graphExplorerShared";
import type { GraphExplorationState } from "./useGraphExploration";

// The nodes, edges and selection derived from the accepted read or the shown
// path, the camera session that survives path toggles, and the evidence
// hand-off to the knowledge selection.
export function useExplorerScene(
  {
    brain,
    view,
    path,
    inspect,
  }: {
    brain: Brain;
    view: ExplorerView;
    path: ExplorerPath | null;
    inspect: (node: ExplorerNode) => void;
  },
  exploration: GraphExplorationState,
) {
  const { data, choice, request, setChoice, setFocusRequest } = exploration;
  // Recomputed here so the path check narrows `path` for the reads below.
  const inPath = exploration.showPath && path?.status === "path";
  const matchingPath =
    path?.view.generation.id === data?.view.generation.id &&
    path?.view.memory_epoch === data?.view.memory_epoch;
  const nodes = (inPath ? path.nodes : data?.nodes) ?? [];
  const edges = (inPath ? path.edges : data?.edges) ?? [];
  const nodeMap = useMemo(() => new Map(nodes.map((n) => [n.key, n])), [nodes]);
  const pathNodes = useMemo(
    () => (inPath || matchingPath ? (path?.nodes.map((n) => n.key) ?? []) : []),
    [path, inPath, matchingPath],
  );
  const pathEdges = useMemo(
    () => (inPath || matchingPath ? (path?.edges.map((e) => e.id) ?? []) : []),
    [path, inPath, matchingPath],
  );
  const chosenNode =
    choice?.kind === "node" ? nodeMap.get(choice.id) : undefined;
  const chosenEdge =
    choice?.kind === "edge" ? edges.find((e) => e.id === choice.id) : undefined;
  const witness =
    !inPath && chosenNode && data?.reach.find((r) => r.key === chosenNode.key);
  const edgeEvidence =
    chosenEdge &&
    nodes.find(
      (n) =>
        n.evidence.id === chosenEdge.evidence_id ||
        n.evidence.revision_id === chosenEdge.evidence_id,
    );
  // The renderer remounts per read, never per path toggle; its camera session
  // belongs to this key so toggling the path keeps the camera.
  const rendererKey = JSON.stringify([
    brain.id,
    view.scope,
    request.center,
    request.direction,
    request.max_hops,
  ]);
  const sessionKey = useRef(rendererKey);
  const spatialSession = useRef<SpatialSession>({
    value: null,
    invalid: false,
  });
  if (sessionKey.current !== rendererKey) {
    sessionKey.current = rendererKey;
    spatialSession.current = { value: null, invalid: false };
  }
  const { select, available } = useKnowledgeSelection();
  const ownsSelection = useRef(false);
  const focusInGraph = () => {
    ownsSelection.current = false;
    setFocusRequest((value) => value + 1);
    select(null);
  };
  const details = useRef<ReactNode>(null);
  function openEvidence(node: ExplorerNode) {
    if (available && node.evidence.kind === "claim") {
      ownsSelection.current = true;
      select(
        {
          kind: "graph-node",
          key: node.key,
          claimId: node.evidence.id,
          revision: node.evidence.revision_id,
        },
        details.current,
      );
    } else {
      select(null);
      inspect(node);
    }
  }
  useEffect(() => {
    if (!data && ownsSelection.current) {
      ownsSelection.current = false;
      select(null);
    }
  }, [data, select]);
  useEffect(() => {
    if (
      choice &&
      (!data ||
        (choice.kind === "node"
          ? !nodeMap.has(choice.id)
          : !edges.some((edge) => edge.id === choice.id)))
    )
      setChoice(null);
  }, [data, nodeMap, edges, choice]);
  return {
    nodes,
    edges,
    nodeMap,
    pathNodes,
    pathEdges,
    chosenNode,
    chosenEdge,
    witness,
    edgeEvidence,
    rendererKey,
    spatialSession,
    select,
    available,
    ownsSelection,
    focusInGraph,
    details,
    openEvidence,
  };
}

export type ExplorerSceneState = ReturnType<typeof useExplorerScene>;
