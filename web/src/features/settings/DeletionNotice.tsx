import { Alert, Button, Group, Text } from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result } from "../../api";
import type { components } from "../../api-schema";
type Receipt = components["schemas"]["BrainDeletionResult"];
export const deletionReceiptKey = ["last-brain-deletion"];

// Session-only receipt contains no Brain name or content. It survives leaving
// the deleted Brain; logout clears it with the rest of the query cache.
export function DeletionNotice() {
  const cache = useQueryClient();
  const receipt = useQuery<Receipt | null>({
    queryKey: deletionReceiptKey,
    queryFn: () => null,
    initialData: null,
    enabled: false,
  });
  return receipt.data ? (
    <Progress
      key={receipt.data.request.id}
      receipt={receipt.data}
      dismiss={() => cache.setQueryData(deletionReceiptKey, null)}
    />
  ) : null;
}
function Progress({
  receipt,
  dismiss,
}: {
  receipt: Receipt;
  dismiss: () => void;
}) {
  const query = useQuery({
    queryKey: ["deletion-status", receipt.request.id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/deletions/{request}", {
          params: {
            path: {
              brain: receipt.request.brain_id,
              request: receipt.request.id,
            },
          },
          signal,
        }),
      ),
    initialData: receipt.request,
    staleTime: 0,
    gcTime: 0,
    retry: false,
    refetchInterval: (query) =>
      query.state.data?.state === "complete" ? false : 2000,
  });
  const state = query.data;
  return (
    <Alert
      mb="lg"
      color={
        query.isError || state.state === "error"
          ? "red"
          : state.state === "complete"
            ? "teal"
            : "yellow"
      }
      title={
        query.isError
          ? "Brain deleted · cleanup status unavailable"
          : state.state === "complete"
            ? "Deletion complete"
            : state.state === "error"
              ? "Brain deleted · cleanup needs attention"
              : "Deletion in progress"
      }
    >
      <Text size="sm">The Brain is absent from listings and recall.</Text>
      {query.isError ? (
        <Button variant="subtle" size="xs" onClick={() => void query.refetch()}>
          Retry cleanup status
        </Button>
      ) : (
        <Text size="sm">
          {state.state === "complete"
            ? "Central cleanup is complete."
            : `${state.pending_artifacts} files pending · journal ${state.journaled ? "saved" : "pending"} · graph cleanup ${state.graph_pending ? "pending" : "complete"}.`}
        </Text>
      )}
      <Text size="xs" mt="xs">
        Managed backups can retain earlier copies for up to{" "}
        {receipt.backup_days} days. Offline host copies require check-in;
        central completion does not prove their removal.
      </Text>
      <Group justify="flex-end">
        <Button variant="subtle" size="xs" onClick={dismiss}>
          Dismiss deletion status
        </Button>
      </Group>
    </Alert>
  );
}
