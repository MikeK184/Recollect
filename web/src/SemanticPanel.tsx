import { useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Loader,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { useIdempotency } from "./useIdempotency";
import { TriangleAlert } from "lucide-react";

const label = (text: string) => text.replaceAll("_", " ");
type Action =
  | { kind: "reindex"; base_profile: string | null }
  | { kind: "retry"; batch: string };

export function SemanticPanel({ brain }: { brain: Brain }) {
  const [offset, setOffset] = useState(0);
  const cache = useQueryClient();
  const command = useIdempotency();
  const status = useQuery({
    queryKey: ["semantic", brain.id, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/semantic", {
          params: { path: { brain: brain.id }, query: { offset } },
        }),
      ),
    refetchInterval: 3000,
  });
  const action = useMutation({
    mutationFn: async (input: Action) => {
      const headers = { "Idempotency-Key": command.forInput(input) };
      if (input.kind === "reindex") {
        return result(
          await client.POST("/api/brains/{brain}/semantic/reindex", {
            params: { path: { brain: brain.id } },
            body: { base_profile: input.base_profile },
            headers,
          }),
        );
      }
      return result(
        await client.POST(
          "/api/brains/{brain}/semantic/batches/{batch}/retry",
          {
            params: { path: { brain: brain.id, batch: input.batch } },
            headers,
          },
        ),
      );
    },
    onSuccess: async () => {
      command.reset();
      await cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain.id),
      });
    },
  });
  const data = status.data;
  const writer = brain.role === "writer" || brain.role === "admin";
  return (
    <Stack component="section" aria-label="Semantic search readiness" gap="sm">
      <Group justify="space-between">
        <Title order={3}>Semantic search</Title>
        {data && (
          <Badge color={data.state === "ready" ? "teal" : "gray"}>
            {label(data.state)}
          </Badge>
        )}
      </Group>
      <Text size="sm">
        Embeddings find similar meanings. Each result still follows the Brain’s
        scope, evidence and memory rules.
      </Text>
      {status.isPending && <Loader aria-label="Loading semantic readiness" />}
      {(status.error || action.error) && (
        <Alert color="red">{(status.error ?? action.error)?.message}</Alert>
      )}
      {data && (
        <>
          <Text size="sm">
            {data.enabled
              ? "Automatic indexing is enabled for approved content."
              : "Automatic indexing is off. An administrator can enable it in the model policy."}
          </Text>
          {data.profile && (
            <Text size="xs" c="dimmed">
              {data.profile.model} · {data.profile.dimensions.toLocaleString()}{" "}
              dimensions
            </Text>
          )}
          <Text size="sm">
            {data.counts.ready.toLocaleString()} represented ·{" "}
            {(
              data.counts.pending +
              data.counts.queued +
              data.counts.running
            ).toLocaleString()}{" "}
            pending ·{" "}
            {(data.counts.blocked + data.counts.failed).toLocaleString()}{" "}
            blocked or failed · {data.counts.removed.toLocaleString()} removed
          </Text>
          {data.coverage.length > 0 && (
            <Alert title="Index coverage" color="yellow">
              <Stack gap={4}>
                {data.coverage.map((reason) => (
                  <Text size="sm" key={reason}>
                    {label(reason)}
                  </Text>
                ))}
              </Stack>
            </Alert>
          )}
          {data.state === "profile_mismatch" && (
            <Alert color="yellow">
              The existing index uses a different model or representation.
              Select compatible models in AI permissions, then Save and rebuild.
            </Alert>
          )}
          {brain.role === "admin" && !brain.archived && (
            <>
              <Button
                variant="light"
                w="fit-content"
                loading={
                  action.isPending && action.variables?.kind === "reindex"
                }
                disabled={
                  !data.enabled || data.state === "disabled" || action.isPending
                }
                onClick={() =>
                  action.mutate({
                    kind: "reindex",
                    base_profile: data.profile?.id ?? null,
                  })
                }
              >
                Rebuild semantic index
              </Button>
              <Text size="xs" c="dimmed">
                Rebuilding replaces the current index and queues new embedding
                requests under this Brain’s model budget. Search coverage grows
                as batches finish.
              </Text>
            </>
          )}
          {data.batches.length > 0 && (
            <Text fw={600} size="sm">
              Recent embedding batches
            </Text>
          )}
          {data.batches.map((batch) => (
            <Card withBorder padding="sm" key={batch.id}>
              <Group justify="space-between">
                <Text size="sm">
                  {batch.input_count} input{batch.input_count === 1 ? "" : "s"}{" "}
                  · {new Date(batch.created_at).toLocaleString()}
                </Text>
                <Badge>{label(batch.state)}</Badge>
              </Group>
              {batch.error_code && (
                <Text size="sm" c="dimmed">
                  {label(batch.error_code)}
                </Text>
              )}
              {batch.automatic_attempt > 0 && (
                <Text size="xs">
                  Automatic replacement {batch.automatic_attempt} of 2
                </Text>
              )}
              {writer &&
                !brain.archived &&
                data.enabled &&
                !["disabled", "profile_mismatch"].includes(data.state) &&
                batch.can_retry && (
                  <Group mt="xs">
                    <Button
                      size="xs"
                      variant="light"
                      loading={
                        action.isPending &&
                        action.variables?.kind === "retry" &&
                        action.variables.batch === batch.id
                      }
                      disabled={action.isPending}
                      onClick={() =>
                        action.mutate({ kind: "retry", batch: batch.id })
                      }
                    >
                      Start new embedding attempt
                    </Button>
                    <Text size="xs" c="dimmed">
                      Remaining eligible inputs only. A new attempt may incur
                      provider usage.
                    </Text>
                  </Group>
                )}
            </Card>
          ))}
          {data.total_batches > 20 && (
            <Group>
              <Button
                variant="subtle"
                disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - 20))}
              >
                Previous batches
              </Button>
              <Text size="xs">
                {offset + 1}–{Math.min(offset + 20, data.total_batches)} of{" "}
                {data.total_batches}
              </Text>
              <Button
                variant="subtle"
                disabled={offset + 20 >= data.total_batches}
                onClick={() => setOffset(offset + 20)}
              >
                Next batches
              </Button>
            </Group>
          )}
        </>
      )}
    </Stack>
  );
}

export function SemanticSummary({
  brain,
  inspect,
}: {
  brain: Brain;
  inspect: () => void;
}) {
  const query = useQuery({
    queryKey: ["semantic-summary", brain.id],
    gcTime: 0,
    retry: false,
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/semantic", {
          params: { path: { brain: brain.id }, query: { summary: true } },
          signal,
        }),
      ),
  });
  const data = query.error ? undefined : query.data;
  const reason: Record<string, string> = {
    semantic_work_pending: "Some approved content is still being indexed.",
    semantic_inputs_unavailable: "Some inputs are blocked or failed.",
    semantic_representation_truncated:
      "Some indexed inputs were shortened to fit the representation limit.",
    semantic_discovery_pending:
      "Some eligible content has not been discovered yet.",
    semantic_capacity_reached: "The current index has reached its capacity.",
  };
  return (
    <aside
      className="management-surface ai-search-coverage"
      aria-label="Semantic search coverage"
    >
      <Group justify="space-between" mb="md">
        <h3 className="management-title">Search coverage</h3>
        {data && (
          <Badge
            color={data.state === "ready" ? "brand" : "yellow"}
            leftSection={
              data.state !== "ready" ? <TriangleAlert size={13} /> : undefined
            }
          >
            {data.state === "ready" ? "Ready" : "Needs attention"}
          </Badge>
        )}
      </Group>
      {query.isPending && <Loader size="sm" mt="md" />}
      {query.error && (
        <Alert color="red" mt="md">
          {query.error.message}
        </Alert>
      )}
      {data && (
        <>
          <div className="management-stats">
            <div className="management-stat">
              <strong>{data.counts.ready.toLocaleString()}</strong>
              <span>Represented</span>
            </div>
            <div className="management-stat blocked-stat">
              <strong>
                {(data.counts.blocked + data.counts.failed).toLocaleString()}
              </strong>
              <span>Blocked or failed</span>
            </div>
          </div>
          {data.coverage.length > 0 && (
            <div className="semantic-coverage-note">
              <p>Coverage is incomplete.</p>
              {data.coverage.map((value) => (
                <p key={value}>{reason[value] ?? label(value)}</p>
              ))}
            </div>
          )}
        </>
      )}
      <Button mt="md" variant="default" onClick={inspect}>
        {data && data.counts.blocked + data.counts.failed > 0
          ? `Review ${data.counts.blocked + data.counts.failed} items →`
          : "Inspect search coverage →"}
      </Button>
    </aside>
  );
}
