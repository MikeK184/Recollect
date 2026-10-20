import { useEffect } from "react";
import { Alert } from "../components/ui/alert";
import { Outlet, useParams } from "@tanstack/react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result } from "../api";
import { BrainContext } from "./context";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { AssuranceProvider } from "../features/activity/Assurance";
import { usePollInterval } from "../usePollInterval";

export function BrainLayout() {
  const { brainId } = useParams({ strict: false });
  return brainId ? <BrainBoundary key={brainId} id={brainId} /> : null;
}

function BrainBoundary({ id }: { id: string }) {
  const pollSlow = usePollInterval("slow");
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
    refetchInterval: pollSlow,
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
      <div className="mx-auto w-full max-w-[1248px] px-6 pt-6">
        <ErrorState error={query.error} retry={() => void query.refetch()} />
      </div>
    );
  if (query.isPending)
    return (
      <div className="mx-auto w-full max-w-[1248px] px-6 pt-6">
        <LoadingState label="Opening Brain…" />
      </div>
    );
  if (!query.data) return null;
  return (
    <BrainContext.Provider value={query.data}>
      {query.data.archived && (
        <div className="mx-auto w-full max-w-[1248px] px-6 pt-6">
          <Alert variant="warning" title="This Brain is archived">
            Its content and history are preserved. Reopen it in Settings to
            continue using this space.
          </Alert>
        </div>
      )}
      {/* Metadata revisions invalidate assurance queries, not the entire page.
          Effective role changes still discard the protected route subtree. */}
      <AssuranceProvider key={query.data.role}>
        <Outlet />
      </AssuranceProvider>
    </BrainContext.Provider>
  );
}
