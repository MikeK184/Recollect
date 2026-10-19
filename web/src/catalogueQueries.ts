import { useQuery } from "@tanstack/react-query";
import { client, result } from "./api";
import type { paths } from "./api-schema";
import { usePollInterval, type PollKind } from "./usePollInterval";

type WorkspaceQuery = NonNullable<
  paths["/api/brains/{brain}/workspace"]["get"]["parameters"]["query"]
>;
export type EvidenceListParams = NonNullable<
  paths["/api/brains/{brain}/evidence"]["get"]["parameters"]["query"]
>;

// Canonical keys: every reader of the same catalogue shares one cache entry,
// and ["workspace", brain] / ["evidence", brain] invalidate all of them.
export const workspaceKey = (brainId: string, params?: WorkspaceQuery) =>
  params
    ? (["workspace", brainId, params] as const)
    : (["workspace", brainId] as const);
export const evidenceKey = (brainId: string, params?: EvidenceListParams) =>
  ["evidence", brainId, compact(params)] as const;

function compact(params?: EvidenceListParams) {
  const out: Record<string, string | number> = {};
  for (const [key, value] of Object.entries(params ?? {}))
    if (value !== undefined && value !== "" && !(key === "offset" && !value))
      out[key] = value;
  return out;
}

type Options = { enabled?: boolean; poll?: PollKind | false };

// Unparameterised calls share one entry; the paged workspace view passes params.
export function useWorkspaceCatalogue(
  brainId: string,
  params?: WorkspaceQuery,
  { enabled = true, poll = false }: Options = {},
) {
  const interval = usePollInterval(poll || "normal", !!poll);
  return useQuery({
    queryKey: workspaceKey(brainId, params),
    enabled,
    gcTime: 0,
    refetchInterval: interval,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          signal,
          params: { path: { brain: brainId }, query: params },
        }),
      ),
  });
}

export function useEvidenceList(
  brainId: string,
  params?: EvidenceListParams,
  { enabled = true, poll = false }: Options = {},
) {
  const interval = usePollInterval(poll || "normal", !!poll);
  return useQuery({
    queryKey: evidenceKey(brainId, params),
    enabled,
    gcTime: 0,
    refetchInterval: interval,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          signal,
          params: { path: { brain: brainId }, query: params },
        }),
      ),
  });
}
