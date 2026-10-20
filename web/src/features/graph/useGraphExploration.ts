import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { client, RequestError, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import type { GraphChoice } from "../../GraphCanvas";
import type { SceneGenerations } from "../../graph-generation-status";
import type {
  ExplorerNode,
  ExplorerPath,
  ExplorerView,
} from "./graphExplorerShared";

// A graph read answers 503 when its inputs changed during preparation (or the
// database is briefly busy) and asks to be read again. Retry that read a few
// times instead of leaving the canvas empty; other failures surface at once.
export const retryGraphRead = (failures: number, error: Error) =>
  failures < 3 && error instanceof RequestError && error.status === 503;
export const graphRetryDelay = (attempt: number) =>
  Math.min(500 * 2 ** attempt, 2000);

export type GraphExplorerProps = {
  brain: Brain;
  view: ExplorerView;
  path: ExplorerPath | null;
  inspect: (node: ExplorerNode) => void;
  onStart: (key: string) => void;
  onEnd: (key: string) => void;
  onExpired: () => void;
  initialCenter?: string;
  initialDirection?: string;
  initialHops?: number;
  initialExploration?: components["schemas"]["GraphExploration"];
  compactControls?: boolean;
  expectedMemoryEpoch?: number;
  onReadStarted?: () => void;
  onViewAccepted?: (scene: SceneGenerations) => void;
  onExplore?: (center: string | null, direction: string, hops: number) => void;
};

// The exploration request, its bounded read and the effects that keep the read,
// the selection and the accepted scene consistent. The explorer calls this
// unconditionally, before the scene hook derives anything from the read.
export function useGraphExploration({
  brain,
  view,
  path,
  onExpired,
  initialCenter = "",
  initialDirection = "outgoing",
  initialHops = 1,
  initialExploration,
  expectedMemoryEpoch,
  onReadStarted,
  onViewAccepted,
  onExplore,
}: GraphExplorerProps) {
  const [center, setCenter] = useState(initialCenter);
  const [controlsOpened, setControlsOpened] = useState(false);
  const [direction, setDirection] = useState(initialDirection);
  const [hops, setHops] = useState(initialHops);
  const [request, setRequest] = useState({
    nonce: crypto.randomUUID(),
    center: initialCenter || null,
    direction: initialDirection,
    max_hops: initialHops,
  });
  const startedRequest = useRef<string | null>(null);
  function beginRead(next: typeof request) {
    onReadStarted?.();
    startedRequest.current = next.nonce;
    setRequest(next);
  }
  useEffect(() => {
    if (
      request.center === (initialCenter || null) &&
      request.direction === initialDirection &&
      request.max_hops === initialHops
    )
      return;
    setCenter(initialCenter);
    setDirection(initialDirection);
    setHops(initialHops);
    setExpired(false);
    beginRead({
      nonce: crypto.randomUUID(),
      center: initialCenter || null,
      direction: initialDirection,
      max_hops: initialHops,
    });
  }, [initialCenter, initialDirection, initialHops]);
  const [choice, setGraphChoice] = useState<GraphChoice>(null);
  const [focusRequest, setFocusRequest] = useState(0);
  const setChoice = (next: GraphChoice) => {
    setGraphChoice(next);
  };
  const [showPath, setShowPath] = useState(false);
  const [expired, setExpired] = useState(false);
  const [coverageOpen, setCoverageOpen] = useState(false);
  const initialNonce = useRef(request.nonce);
  const read = useQuery({
    initialData:
      request.nonce === initialNonce.current &&
      JSON.stringify(initialExploration?.view.scope) ===
        JSON.stringify(view.scope) &&
      initialExploration?.view.generation.id === view.generation.id &&
      initialExploration.center === request.center &&
      initialExploration.direction === request.direction &&
      initialExploration.max_hops === request.max_hops
        ? initialExploration
        : undefined,
    queryKey: [
      "graph-explore",
      brain.id,
      view.scope,
      view.generation.id,
      request,
    ],
    retry: retryGraphRead,
    retryDelay: graphRetryDelay,
    gcTime: 0,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/explore", {
          params: { path: { brain: brain.id } },
          body: {
            scope: view.scope,
            center: request.center,
            direction: request.direction,
            max_hops: request.max_hops,
            windowed: !request.center || request.max_hops === 1,
          },
          signal,
        }),
      ),
  });
  useEffect(() => {
    setChoice(null);
    setShowPath(false);
    setCoverageOpen(false);
  }, [request, path]);
  useEffect(() => {
    if (startedRequest.current !== request.nonce) {
      startedRequest.current = request.nonce;
      onReadStarted?.();
    }
  }, [request.nonce, onReadStarted]);
  useEffect(() => {
    const deadline = read.data?.expires_at;
    if (!deadline) return;
    let timer: ReturnType<typeof setTimeout>;
    function check() {
      const remaining = new Date(deadline!).getTime() - Date.now();
      if (remaining <= 0) {
        setExpired(true);
        setChoice(null);
        onExpired();
      } else timer = setTimeout(check, Math.min(remaining + 1, 2_147_483_647));
    }
    check();
    return () => clearTimeout(timer);
  }, [read.data, onExpired]);
  function load(from: string | null) {
    beginRead({
      nonce: crypto.randomUUID(),
      center: from,
      direction,
      max_hops: hops,
    });
    onExplore?.(from, direction, hops);
    setChoice(null);
    setShowPath(false);
    setExpired(false);
  }
  const changed =
    expectedMemoryEpoch !== undefined &&
    read.data !== undefined &&
    read.data.view.memory_epoch !== expectedMemoryEpoch;
  useEffect(() => {
    if (changed) onExpired();
  }, [changed, onExpired]);
  const data = !read.isError && !expired && !changed ? read.data : undefined;
  const inPath = showPath && path?.status === "path";
  const shownView = inPath ? path.view : data?.view;
  useEffect(() => {
    if (shownView)
      onViewAccepted?.({
        generation: shownView.generation,
        inputs: shownView.inputs,
        memory_epoch: shownView.memory_epoch,
        state: shownView.state,
        scope: shownView.scope,
        total_nodes: shownView.total_nodes,
        total_edges: shownView.total_edges,
        coverage: shownView.coverage,
        link_issues: shownView.link_issues,
        link_issues_total: shownView.link_issues_total,
      });
  }, [shownView, onViewAccepted]);
  return {
    center,
    setCenter,
    controlsOpened,
    setControlsOpened,
    direction,
    setDirection,
    hops,
    setHops,
    request,
    choice,
    setChoice,
    focusRequest,
    setFocusRequest,
    showPath,
    setShowPath,
    expired,
    coverageOpen,
    setCoverageOpen,
    read,
    load,
    data,
    inPath,
    shownView,
  };
}

export type GraphExplorationState = ReturnType<typeof useGraphExploration>;
