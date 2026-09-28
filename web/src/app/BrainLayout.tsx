import { useEffect } from "react";
import { Alert } from "@mantine/core";
import { Outlet, useParams } from "@tanstack/react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result } from "../api";
import { BrainContext } from "./context";
import { ErrorState, LoadingState } from "../components/AsyncState";

export function BrainLayout() {
  const { brainId } = useParams({ strict: false });
  return brainId ? <BrainBoundary key={brainId} id={brainId} /> : null;
}

function BrainBoundary({ id }: { id: string }) {
  const cache = useQueryClient();
  const query = useQuery({
    queryKey: ["brain", id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{id}", {
          params: { path: { id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
  });
  useEffect(
    () => () => {
      // Dismounting a Brain cancels all its consumers. Protected payloads are not a
      // cross-Brain cache; re-entry must pass a fresh authorized read.
      void cache.cancelQueries({ predicate: (q) => q.queryKey.includes(id) });
      cache.removeQueries({ predicate: (q) => q.queryKey.includes(id) });
    },
    [cache, id],
  );
  if (query.isError)
    return (
      <ErrorState error={query.error} retry={() => void query.refetch()} />
    );
  if (query.isPending) return <LoadingState label="Opening Brain…" />;
  if (!query.data) return null;
  return (
    <BrainContext.Provider value={query.data}>
      {query.data.archived && (
        <Alert color="yellow" mb="lg" title="This Brain is archived">
          Its content and history are preserved. Reopen it in Settings to
          continue using this space.
        </Alert>
      )}
      <Outlet />
    </BrainContext.Provider>
  );
}
