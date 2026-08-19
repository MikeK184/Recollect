import {
  Badge,
  Button,
  Card,
  Group,
  Loader,
  Progress,
  Stack,
  Text,
  Alert,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result } from "./api";

const stateLabels: Record<string, string> = {
  current: "Up to date",
  queued: "Queued",
  running: "Running",
  succeeded: "Completed",
  failed: "Needs attention",
  cancelled: "Cancelled",
  missing: "Not processed",
};
const errorLabels: Record<string, string> = {
  database_error:
    "A database call failed. Processing will retry within its attempt limit.",
  permission_revoked: "The submitting account no longer has access.",
  unsupported_kind: "This installation cannot process this job type.",
  input_missing: "The input is unavailable.",
  content_removed:
    "The input expired or was erased. This work cannot be retried.",
  artifact_unavailable:
    "Evidence storage is unavailable. Processing will retry within its attempt limit.",
  lease_expired: "The worker stopped before completing this job.",
  cancelled_by_admin: "An administrator cancelled this job.",
};

export function JobsPanel({ id, admin }: { id: string; admin: boolean }) {
  const cache = useQueryClient();
  const processing = useQuery({
    queryKey: ["processing", id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{id}/processing", {
          params: { path: { id } },
        }),
      ),
    refetchInterval: 1500,
  });
  const jobs = useQuery({
    queryKey: ["jobs", id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{id}/jobs", { params: { path: { id } } }),
      ),
    refetchInterval: 1500,
  });
  const action = useMutation({
    mutationFn: async ({
      job,
      action,
    }: {
      job: string;
      action: "retry" | "cancel";
    }) =>
      result(
        await client.POST(
          action === "retry"
            ? "/api/brains/{id}/jobs/{job}/retry"
            : "/api/brains/{id}/jobs/{job}/cancel",
          { params: { path: { id, job } } },
        ),
      ),
    onSuccess: () => {
      void cache.invalidateQueries({ queryKey: ["jobs", id] });
      void cache.invalidateQueries({ queryKey: ["processing", id] });
      void cache.invalidateQueries({ queryKey: ["audit", id] });
    },
  });
  const error = processing.error ?? jobs.error ?? action.error;
  return (
    <Card
      component="section"
      aria-label="Background processing"
      withBorder
      p="lg"
      mt="xl"
    >
      <Group justify="space-between" mb="sm">
        <Text fw={600}>Processing</Text>
        {processing.data && (
          <Badge
            color={
              processing.data.state === "current"
                ? "teal"
                : processing.data.state === "failed"
                  ? "red"
                  : "gray"
            }
            variant="light"
          >
            {stateLabels[processing.data.state]}
          </Badge>
        )}
      </Group>
      <Text size="sm" c="dimmed" mb="lg">
        Your changes are saved immediately. Background work updates the views
        that use them.
      </Text>
      {error && (
        <Alert color="red" mb="md">
          {error.message}
          <Button
            size="xs"
            variant="subtle"
            onClick={() => {
              void processing.refetch();
              void jobs.refetch();
            }}
          >
            Refresh processing
          </Button>
        </Alert>
      )}
      {jobs.isPending ? (
        <Loader size="sm" />
      ) : !jobs.data?.length ? (
        <Text c="dimmed" size="sm">
          No background work yet.
        </Text>
      ) : (
        <Stack gap="md">
          {jobs.data.map((job) => (
            <div className="job-row" key={job.id}>
              <div className="job-info">
                <Text size="sm" fw={500}>
                  {job.kind === "brain.refresh"
                    ? "Update Brain details"
                    : job.kind === "source.learn"
                      ? "Learn from source"
                      : job.kind === "handover.generate"
                        ? "Generate handover"
                        : job.kind === "source.process"
                          ? "Process source text"
                          : job.kind === "repository.process"
                            ? "Process repository snapshot"
                            : job.kind}
                </Text>
                <Text size="xs" c="dimmed">
                  {stateLabels[job.state]} · Attempt {job.attempts} of{" "}
                  {job.max_attempts}
                </Text>
                {job.state === "running" && (
                  <Progress
                    aria-label="Job progress"
                    value={job.progress}
                    size="xs"
                    mt="sm"
                    animated
                  />
                )}
                {job.error_code && (
                  <Text
                    size="xs"
                    c={job.state === "failed" ? "red" : "dimmed"}
                    mt={4}
                  >
                    {errorLabels[job.error_code] ??
                      "Processing did not complete."}
                  </Text>
                )}
              </div>
              {admin &&
                ["queued", "running", "failed", "cancelled"].includes(
                  job.state,
                ) && (
                  <Button
                    size="xs"
                    variant="light"
                    color={
                      ["queued", "running"].includes(job.state)
                        ? "gray"
                        : "teal"
                    }
                    loading={
                      action.isPending && action.variables?.job === job.id
                    }
                    disabled={
                      action.isPending ||
                      job.error_code === "content_removed" ||
                      (["failed", "cancelled"].includes(job.state) &&
                        [
                          "source.learn",
                          "handover.generate",
                          "semantic.generate",
                          "graph.analyze",
                        ].includes(job.kind))
                    }
                    onClick={() =>
                      action.mutate({
                        job: job.id,
                        action: ["queued", "running"].includes(job.state)
                          ? "cancel"
                          : "retry",
                      })
                    }
                  >
                    {["queued", "running"].includes(job.state)
                      ? "Cancel job"
                      : "Retry job"}
                  </Button>
                )}
            </div>
          ))}
        </Stack>
      )}
    </Card>
  );
}
