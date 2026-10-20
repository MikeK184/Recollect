import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { useIdempotency } from "./useIdempotency";
import { usePollInterval } from "./usePollInterval";
import { SkeletonRows } from "./components/Skeleton";
import { Alert, Badge, Button, StatusBadge } from "./components/ui";

const label = (text: string) => text.replaceAll("_", " ");
type Action =
  | { kind: "reindex"; base_profile: string | null }
  | { kind: "retry"; batch: string };

export function SemanticPanel({ brain }: { brain: Brain }) {
  const pollFast = usePollInterval("fast");
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
    refetchInterval: pollFast,
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
    <section
      aria-label="Semantic search readiness"
      className="grid gap-3 border-t border-border pt-4"
    >
      <div className="flex items-center justify-between gap-3">
        <h3 className="m-0 text-title font-semibold text-foreground">
          Semantic search
        </h3>
        {data && (
          <StatusBadge state={data.state === "ready" ? "positive" : "neutral"}>
            <span className="capitalize">{label(data.state)}</span>
          </StatusBadge>
        )}
      </div>
      <p className="m-0 text-body text-foreground-secondary">
        Embeddings find similar meanings. Each result still follows the Brain’s
        scope, evidence and memory rules.
      </p>
      {status.isPending && (
        <SkeletonRows rows={2} label="Loading semantic readiness" />
      )}
      {(status.error || action.error) && (
        <Alert variant="destructive">
          {(status.error ?? action.error)?.message}
        </Alert>
      )}
      {data && (
        <>
          <p className="m-0 text-body text-foreground-secondary">
            {data.enabled
              ? "Automatic indexing is enabled for approved content."
              : "Automatic indexing is off. An administrator can enable it in the model policy."}
          </p>
          {data.profile && (
            <p className="m-0 mono text-caption text-muted-foreground">
              {data.profile.model} · {data.profile.dimensions.toLocaleString()}{" "}
              dimensions
            </p>
          )}
          <p className="m-0 mono text-label text-foreground-secondary">
            {data.counts.ready.toLocaleString()} represented ·{" "}
            {(
              data.counts.pending +
              data.counts.queued +
              data.counts.running
            ).toLocaleString()}{" "}
            pending ·{" "}
            {(data.counts.blocked + data.counts.failed).toLocaleString()}{" "}
            blocked or failed · {data.counts.removed.toLocaleString()} removed
          </p>
          {data.coverage.length > 0 && (
            <Alert title="Index coverage" variant="warning">
              <div className="grid gap-1">
                {data.coverage.map((reason) => (
                  <p className="first-letter:uppercase" key={reason}>
                    {label(reason)}
                  </p>
                ))}
              </div>
            </Alert>
          )}
          {data.state === "profile_mismatch" && (
            <Alert variant="warning">
              The existing index uses a different model or representation.
              Select compatible models in AI permissions, then Save and rebuild.
            </Alert>
          )}
          {brain.role === "admin" && !brain.archived && (
            <div className="grid gap-1.5">
              <div>
                <Button
                  variant="secondary"
                  loading={
                    action.isPending && action.variables?.kind === "reindex"
                  }
                  disabled={
                    !data.enabled ||
                    data.state === "disabled" ||
                    action.isPending
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
              </div>
              <p className="m-0 text-caption text-muted-foreground">
                Rebuilding replaces the current index and queues new embedding
                requests under this Brain’s model budget. Search coverage grows
                as batches finish.
              </p>
            </div>
          )}
          {data.batches.length > 0 && (
            <h4 className="m-0 mt-1 text-label font-semibold text-foreground">
              Recent embedding batches
            </h4>
          )}
          {data.batches.length > 0 && (
            <div className="overflow-hidden rounded-lg border border-border">
              {data.batches.map((batch) => (
                <div
                  key={batch.id}
                  className="grid gap-1 border-b border-border px-3.5 py-2.5 last:border-b-0"
                >
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-label text-foreground">
                      <span className="mono">{batch.input_count}</span> input
                      {batch.input_count === 1 ? "" : "s"} ·{" "}
                      <span className="mono text-muted-foreground">
                        {new Date(batch.created_at).toLocaleString()}
                      </span>
                    </span>
                    <Badge className="capitalize">{label(batch.state)}</Badge>
                  </div>
                  {batch.error_code && (
                    <p className="m-0 text-caption text-muted-foreground">
                      {label(batch.error_code)}
                    </p>
                  )}
                  {batch.automatic_attempt > 0 && (
                    <p className="m-0 text-caption text-foreground-secondary">
                      Automatic replacement {batch.automatic_attempt} of 2
                    </p>
                  )}
                  {writer &&
                    !brain.archived &&
                    data.enabled &&
                    !["disabled", "profile_mismatch"].includes(data.state) &&
                    batch.can_retry && (
                      <div className="mt-1 flex flex-wrap items-center gap-2">
                        <Button
                          size="sm"
                          variant="secondary"
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
                        <span className="text-caption text-muted-foreground">
                          Remaining eligible inputs only. A new attempt may
                          incur provider usage.
                        </span>
                      </div>
                    )}
                </div>
              ))}
            </div>
          )}
          {data.total_batches > 20 && (
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="sm"
                disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - 20))}
              >
                Previous batches
              </Button>
              <span className="mono text-caption text-muted-foreground">
                {offset + 1}–{Math.min(offset + 20, data.total_batches)} of{" "}
                {data.total_batches}
              </span>
              <Button
                variant="ghost"
                size="sm"
                disabled={offset + 20 >= data.total_batches}
                onClick={() => setOffset(offset + 20)}
              >
                Next batches
              </Button>
            </div>
          )}
        </>
      )}
    </section>
  );
}
