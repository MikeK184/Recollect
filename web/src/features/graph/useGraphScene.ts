import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result } from "../../api";
import type { components } from "../../api-schema";
import {
  invalidatesScene,
  sceneGenerationIds,
  verifySceneStatus,
  type SceneGenerations,
} from "../../graph-generation-status";
import { usePollInterval } from "../../usePollInterval";
import { feedsGraphScene } from "./sceneKeys";
import type { GraphDetail, SubmittedPath, SubmittedRead } from "./graphScope";

// The parent-owned read state that a scene reset must also discard.
export type SceneResets = {
  setInitialExploration: Dispatch<
    SetStateAction<components["schemas"]["GraphExploration"] | undefined>
  >;
  setSubmitted: Dispatch<SetStateAction<SubmittedRead | null>>;
  setPathSubmitted: Dispatch<SetStateAction<SubmittedPath | null>>;
  setDetail: Dispatch<SetStateAction<GraphDetail | null>>;
};

// The accepted scene and its revision counters. `clear` must stay
// referentially stable: it is only recreated when the cache or Brain changes.
export function useGraphScene(brainId: string, resets: SceneResets) {
  const cache = useQueryClient();
  const { setInitialExploration, setSubmitted, setPathSubmitted, setDetail } =
    resets;
  const [generationError, setGenerationError] = useState<Error | null>(null);
  const sceneRevision = useRef(0);
  const acceptedScene = useRef<SceneGenerations | null>(null);
  const [scene, setScene] = useState<SceneGenerations | null>(null);
  const sceneStarted = useCallback(() => {
    sceneRevision.current++;
    acceptedScene.current = null;
    setScene(null);
  }, []);
  const sceneAccepted = useCallback((next: SceneGenerations) => {
    sceneRevision.current++;
    acceptedScene.current = next;
    setScene(next);
    setGenerationError(null);
  }, []);
  const clear = useCallback(() => {
    setGenerationError(null);
    sceneRevision.current++;
    acceptedScene.current = null;
    setScene(null);
    setInitialExploration(undefined);
    setSubmitted(null);
    setPathSubmitted(null);
    setDetail(null);
    void cache.cancelQueries({ queryKey: ["graph-read", brainId] });
    void cache.cancelQueries({ queryKey: ["graph-path", brainId] });
    void cache.cancelQueries({ queryKey: ["graph-explore", brainId] });
    void cache.cancelQueries({ queryKey: ["graph-entities", brainId] });
    void cache.cancelQueries({
      queryKey: ["graph-generation-status", brainId],
    });
  }, [cache, brainId]);
  return {
    scene,
    setScene,
    generationError,
    setGenerationError,
    sceneRevision,
    acceptedScene,
    sceneStarted,
    sceneAccepted,
    clear,
  };
}

export type GraphSceneState = ReturnType<typeof useGraphScene>;

// Discards the scene when relevant queries are invalidated or emptied, and
// verifies the accepted scene's generations against the server while it shows.
export function useSceneVerification(
  brainId: string,
  state: GraphSceneState,
  resets: SceneResets,
) {
  const pollFast = usePollInterval("fast");
  const cache = useQueryClient();
  const { setInitialExploration, setSubmitted, setPathSubmitted, setDetail } =
    resets;
  const {
    scene,
    setScene,
    setGenerationError,
    sceneRevision,
    acceptedScene,
    clear,
  } = state;
  useEffect(
    () =>
      cache.getQueryCache().subscribe((event) => {
        if (
          event.query.queryKey[1] === brainId &&
          event.type === "updated" &&
          ((event.action.type === "invalidate" &&
            feedsGraphScene(event.query.queryKey)) ||
            (event.action.type === "setState" &&
              event.query.state.data === undefined &&
              ["graph", "graph-read", "graph-path"].includes(
                String(event.query.queryKey[0]),
              )))
        ) {
          sceneRevision.current++;
          acceptedScene.current = null;
          setScene(null);
          setInitialExploration(undefined);
          setSubmitted(null);
          setPathSubmitted(null);
          setDetail(null);
          void cache.cancelQueries({ queryKey: ["graph-read", brainId] });
          void cache.cancelQueries({ queryKey: ["graph-path", brainId] });
          void cache.cancelQueries({ queryKey: ["graph-explore", brainId] });
          void cache.cancelQueries({ queryKey: ["graph-entities", brainId] });
        }
      }),
    [brainId, cache],
  );
  const trackedIds = scene ? sceneGenerationIds(scene) : [];
  const generationStatus = useQuery({
    queryKey: ["graph-generation-status", brainId, trackedIds],
    enabled: !!scene,
    retry: false,
    gcTime: 0,
    refetchInterval: pollFast,
    refetchOnWindowFocus: false,
    queryFn: async ({ signal }) => {
      const revision = sceneRevision.current;
      try {
        const observed = result(
          await client.GET("/api/brains/{brain}/graph", {
            params: {
              path: { brain: brainId },
              query: { tracked_generation_ids: trackedIds.join(",") },
            },
            signal,
          }),
        );
        verifySceneStatus(scene!, observed);
        return { observed, revision };
      } catch (error) {
        if (
          !signal.aborted &&
          revision === sceneRevision.current &&
          acceptedScene.current
        ) {
          clear();
          setGenerationError(
            error instanceof Error
              ? error
              : new Error(
                  "The graph inputs could not be verified. Load the graph again.",
                ),
          );
        }
        throw error;
      }
    },
  });
  useEffect(() => {
    const response = generationStatus.data;
    if (
      response &&
      response.revision === sceneRevision.current &&
      acceptedScene.current &&
      invalidatesScene(acceptedScene.current, response.observed)
    )
      clear();
  }, [generationStatus.data]);
}
