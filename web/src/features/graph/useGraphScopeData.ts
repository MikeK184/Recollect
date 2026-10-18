import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "../../api";
import { useContentDeadline } from "../../useContentDeadline";
import { useIdempotency } from "../../useIdempotency";
import { usePollInterval } from "../../usePollInterval";
import type { Scope, SubmittedPath, SubmittedRead } from "./graphScope";

// Snapshot and manifest choices for the selected repository or environment,
// with their paging offsets.
export function useGraphScopeOptions(brainId: string, scope: Scope) {
  const [snapshotOffset, setSnapshotOffset] = useState(0);
  const [manifestOffset, setManifestOffset] = useState(0);
  const repository = (scope.selection.repository_ids ?? [])[0] ?? "";
  const snapshots = useQuery({
    queryKey: ["graph-snapshots", brainId, repository, snapshotOffset],
    enabled: scope.kind === "repository" && !!repository,
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/repositories/{repository}/snapshots",
          {
            params: {
              path: { brain: brainId, repository },
              query: { offset: snapshotOffset },
            },
          },
        ),
      ),
  });
  const manifests = useQuery({
    queryKey: [
      "graph-manifests",
      brainId,
      scope.selection.environment_id,
      manifestOffset,
    ],
    enabled: !!scope.selection.environment_id,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/revision-manifests", {
          params: {
            path: { brain: brainId },
            query: {
              environment_id: scope.selection.environment_id ?? undefined,
              offset: manifestOffset,
            },
          },
        }),
      ),
  });
  return {
    snapshots,
    manifests,
    snapshotOffset,
    setSnapshotOffset,
    manifestOffset,
    setManifestOffset,
  };
}

export function useGraphRebuild(brain: Brain, scope: Scope, clear: () => void) {
  const cache = useQueryClient();
  const idem = useIdempotency();
  return useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/brains/{brain}/graph/rebuild", {
          params: { path: { brain: brain.id } },
          body: {
            kind: scope.kind,
            snapshot_id: scope.kind === "repository" ? scope.snapshot_id : null,
            manifest_revision_id:
              scope.kind === "combined" ? scope.manifest_revision_id : null,
          },
          headers: {
            "Idempotency-Key": idem.forInput({
              brain: brain.id,
              kind: scope.kind,
              snapshot: scope.snapshot_id,
              manifest:
                scope.kind === "combined" ? scope.manifest_revision_id : null,
            }),
          },
        }),
      ),
    onSuccess: () => {
      idem.reset();
      clear();
      void cache.invalidateQueries({ queryKey: ["graph", brain.id] });
      void cache.invalidateQueries({ queryKey: ["jobs", brain.id] });
    },
  });
}

// The windowed entity pages for the read being shown; only fetched while the
// entities drawer is open.
export function useGraphEntities(
  brainId: string,
  {
    enabled,
    submitted,
    entityOffset,
    memoryEpoch,
    linkEpoch,
  }: {
    enabled: boolean;
    submitted: SubmittedRead | null;
    entityOffset: number;
    memoryEpoch: number | undefined;
    linkEpoch: number | undefined;
  },
) {
  const entities = useQuery({
    queryKey: [
      "graph-entities",
      brainId,
      submitted,
      entityOffset,
      memoryEpoch,
      linkEpoch,
    ],
    enabled,
    gcTime: 0,
    retry: false,
    refetchOnWindowFocus: false,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/view", {
          params: { path: { brain: brainId } },
          body: {
            scope: submitted!.scope,
            offset: entityOffset,
            windowed: true,
          },
          signal,
        }),
      ),
  });
  const expired = useContentDeadline(entities.data?.expires_at);
  return { entities, expired };
}

export function useGraphStatus(brainId: string, offset: number) {
  const pollNormal = usePollInterval("normal");
  return useQuery({
    queryKey: ["graph", brainId, offset],
    refetchInterval: pollNormal,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/graph", {
          params: { path: { brain: brainId }, query: { offset } },
        }),
      ),
  });
}

// Disabled by design: the read is fetched imperatively through the cache and
// this query only observes it under the same key.
export function useGraphRead(brainId: string, submitted: SubmittedRead | null) {
  return useQuery({
    queryKey: ["graph-read", brainId, submitted],
    enabled: false,
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/view", {
          params: { path: { brain: brainId } },
          body: { scope: submitted!.scope, offset: submitted!.offset },
          signal,
        }),
      ),
  });
}

export function useGraphPath(
  brainId: string,
  pathSubmitted: SubmittedPath | null,
) {
  return useQuery({
    queryKey: ["graph-path", brainId, pathSubmitted],
    enabled: false,
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/path", {
          params: { path: { brain: brainId } },
          body: pathSubmitted!.request,
          signal,
        }),
      ),
  });
}
