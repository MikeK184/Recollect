import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "../../api";
import { ErrorState, LoadingState } from "../../components/AsyncState";
import { PrivacyForm } from "./PrivacyForm";
import type { PrivacySnapshot } from "./privacyDraft";

export function PrivacyOverview({ brain }: { brain: Brain }) {
  const cache = useQueryClient();
  const path = { brain: brain.id };
  const capture = useQuery({
    queryKey: ["capture-policy", brain.id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/capture/policy", {
          params: { path },
          signal,
        }),
      ),
    refetchInterval: 4000,
    retry: false,
    gcTime: 0,
  });
  const retention = useQuery({
    queryKey: ["retention", brain.id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/retention", {
          params: { path },
          signal,
        }),
      ),
    refetchInterval: 4000,
    retry: false,
    gcTime: 0,
  });
  const documents = useQuery({
    queryKey: ["evidence", brain.id, "storage-policy"],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          params: { path },
          signal,
        }),
      ),
    refetchInterval: 4000,
    retry: false,
    gcTime: 0,
  });
  const repository = useQuery({
    queryKey: ["repository-policy", brain.id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/repositories/policy", {
          params: { path },
          signal,
        }),
      ),
    refetchInterval: 4000,
    retry: false,
    gcTime: 0,
  });
  const error = capture.error || retention.error || documents.error || repository.error;
  const refresh = async (): Promise<PrivacySnapshot> => {
    const [c, r, d, p] = await Promise.all([
      capture.refetch(), retention.refetch(), documents.refetch(), repository.refetch(),
    ]);
    const failure = c.error || r.error || d.error || p.error;
    if (failure) throw failure;
    if (!c.data || !r.data || !d.data || !p.data) throw new Error("Privacy settings unavailable.");
    return {
      capture: c.data, retention: r.data,
      documents: d.data.policy.allow_document_content,
      repository: p.data.allow_file_content,
    };
  };
  if (error) return <ErrorState error={error} retry={() => { void refresh().catch(() => undefined); }} />;
  if (!capture.data || !retention.data || !documents.data || !repository.data)
    return <LoadingState label="Loading privacy policy…" />;
  const snapshot: PrivacySnapshot = {
    capture: capture.data, retention: retention.data,
    documents: documents.data.policy.allow_document_content,
    repository: repository.data.allow_file_content,
  };
  return <PrivacyForm key={brain.id} brain={brain} snapshot={snapshot} refresh={refresh}
    acknowledge={(value, domain) => {
      if (domain === "capture") cache.setQueryData(["capture-policy", brain.id], value.capture);
      if (domain === "retention") cache.setQueryData(["retention", brain.id], value.retention);
      if (domain === "documents") {
        cache.setQueryData(["evidence", brain.id, "storage-policy"], (old: typeof documents.data) =>
          old && ({ ...old, policy: { ...old.policy, allow_document_content: value.documents } }));
        void cache.invalidateQueries({ queryKey: ["evidence", brain.id], exact: true });
      }
      if (domain === "repository") cache.setQueryData(["repository-policy", brain.id], (old: typeof repository.data) =>
        old && ({ ...old, allow_file_content: value.repository }));
      if (domain === "retention") void cache.invalidateQueries({
        predicate: (query) => query.queryKey.includes(brain.id) && !["retention", "capture-policy", "repository-policy"].includes(String(query.queryKey[0])),
      });
    }} />;
}
