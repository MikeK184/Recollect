import { useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Loader,
  Modal,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { ContributionPicker, combineContributions } from "./MemoryForms";
import { ClaimDialog } from "./ClaimsPanel";
import { useIdempotency } from "./useIdempotency";
type View = components["schemas"]["ClaimView"];
function GenerateDialog({
  brain,
  onClose,
  onQueued,
}: {
  brain: Brain;
  onClose: () => void;
  onQueued: () => void;
}) {
  const [title, setTitle] = useState("");
  const [rows, setRows] = useState<View[]>([]);
  const command = useIdempotency();
  const send = useMutation({
    mutationFn: async () => {
      const body = {
        title,
        contributions: rows.map((row) => row.revision.id),
        operation_id: null,
      };
      return result(
        await client.POST("/api/brains/{brain}/handovers", {
          params: { path: { brain: brain.id } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: () => {
      command.reset();
      onQueued();
      onClose();
    },
  });
  const scope = combineContributions(rows).selection;
  return (
    <Modal opened onClose={onClose} size="xl" title="Generate handover">
      <form
        onSubmit={(e) => {
          e.preventDefault();
          send.mutate();
        }}
      >
        <Stack>
          <Text size="sm">
            The selected text model will compose a handover from these records.
            It retains every contribution and follows this Brain's acceptance
            policy. Human review is optional.
          </Text>
          <TextInput
            label="Handover title"
            required
            maxLength={256}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
          <ContributionPicker
            brain={brain}
            ids={rows.map((row) => row.revision.id)}
            onChange={setRows}
          />
          <Text size="sm">
            {rows.length} contributions · {scope.repository_ids.length} selected
            repositories · {scope.area_ids.length} selected areas
          </Text>
          <Text size="sm" c="dimmed">
            Requires the Brain model policy to allow synthesis, claim content
            and the requested title as query content.
          </Text>
          {send.error && (
            <Alert color="red" title="Handover could not be queued">
              {send.error.message}
            </Alert>
          )}
          <Button
            type="submit"
            disabled={!rows.length}
            loading={send.isPending}
          >
            Queue handover
          </Button>
        </Stack>
      </form>
    </Modal>
  );
}
export function HandoversPanel({ brain }: { brain: Brain }) {
  const [offset, setOffset] = useState(0);
  const [creating, setCreating] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [queued, setQueued] = useState(false);
  const command = useIdempotency();
  const cache = useQueryClient();
  const runs = useQuery({
    queryKey: ["handovers", brain.id, offset],
    refetchInterval: 3000,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/handovers", {
          params: { path: { brain: brain.id }, query: { offset } },
        }),
      ),
  });
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
        }),
      ),
  });
  const refresh = () => {
    for (const key of [
      "handovers",
      "claims",
      "claim",
      "models-usage",
      "jobs",
      "audit",
    ])
      void cache.invalidateQueries({ queryKey: [key, brain.id] });
  };
  const retry = useMutation({
    mutationFn: async (id: string) =>
      result(
        await client.POST("/api/brains/{brain}/handovers/{run}/retry", {
          params: { path: { brain: brain.id, run: id } },
          body: { operation_id: null },
          headers: { "Idempotency-Key": command.forInput({ id }) },
        }),
      ),
    onSuccess: () => {
      command.reset();
      refresh();
    },
  });
  const cancel = useMutation({
    mutationFn: async (id: string) =>
      result(
        await client.POST("/api/brains/{id}/jobs/{job}/cancel", {
          params: { path: { id: brain.id, job: id } },
        }),
      ),
    onSuccess: refresh,
  });
  const error = runs.error ?? retry.error ?? cancel.error;
  return (
    <Card withBorder p="xl" mt="lg">
      <Stack>
        <Group justify="space-between">
          <Title order={2} fz={21}>
            Generated handovers
          </Title>
          {brain.role !== "reader" && !brain.archived && (
            <Button size="xs" onClick={() => setCreating(true)}>
              Generate handover
            </Button>
          )}
        </Group>
        <Text size="sm" c="dimmed">
          Bring selected claims, decisions and procedures together while
          preserving their evidence, applicability and review state.
        </Text>
        {queued && (
          <Alert color="blue" withCloseButton onClose={() => setQueued(false)}>
            Handover queued. Its result will appear below.
          </Alert>
        )}
        {error && (
          <Alert color="red" title="Handover request failed">
            {error.message}
          </Alert>
        )}
        {runs.isPending && <Loader />}
        {runs.data?.total === 0 && (
          <Text c="dimmed">No generated handovers yet.</Text>
        )}
        {runs.data?.items.map((run) => (
          <Card withBorder key={run.id}>
            <Stack gap="sm">
              <Group justify="space-between">
                <Text fw={600}>{run.title || "Handover attempt"}</Text>
                <Badge
                  color={
                    run.state === "succeeded"
                      ? "teal"
                      : ["failed", "removed"].includes(run.state)
                        ? "orange"
                        : "gray"
                  }
                >
                  {run.state}
                </Badge>
              </Group>
              <Text size="xs" c="dimmed">
                {new Date(run.created_at).toLocaleString()} ·{" "}
                {run.contributions.length} contributions
              </Text>
              {run.error_code && (
                <Text size="sm">{run.error_code.replaceAll("_", " ")}</Text>
              )}
              <Group>
                {run.claim_id && run.state === "succeeded" && (
                  <Button
                    variant="light"
                    size="xs"
                    onClick={() => setSelected(run.claim_id!)}
                  >
                    Inspect generated handover
                  </Button>
                )}
                {["failed", "cancelled"].includes(run.state) &&
                  brain.role !== "reader" &&
                  !brain.archived && (
                    <Button
                      variant="light"
                      size="xs"
                      loading={retry.isPending && retry.variables === run.id}
                      disabled={retry.isPending && retry.variables !== run.id}
                      onClick={() => retry.mutate(run.id)}
                    >
                      Start new handover attempt
                    </Button>
                  )}
                {["queued", "running"].includes(run.state) &&
                  brain.role === "admin" && (
                    <Button
                      variant="subtle"
                      size="xs"
                      loading={
                        cancel.isPending && cancel.variables === run.job_id
                      }
                      onClick={() => cancel.mutate(run.job_id)}
                    >
                      Cancel handover
                    </Button>
                  )}
              </Group>
            </Stack>
          </Card>
        ))}
        {!!runs.data && runs.data.total > 20 && (
          <Group>
            <Button
              variant="subtle"
              disabled={!offset}
              onClick={() => setOffset(Math.max(0, offset - 20))}
            >
              Previous handovers
            </Button>
            <Button
              variant="subtle"
              disabled={offset + 20 >= runs.data.total}
              onClick={() => setOffset(offset + 20)}
            >
              More handovers
            </Button>
          </Group>
        )}
        {creating && (
          <GenerateDialog
            brain={brain}
            onClose={() => setCreating(false)}
            onQueued={() => {
              setQueued(true);
              setOffset(0);
              refresh();
            }}
          />
        )}
        {selected && (
          <ClaimDialog
            key={selected}
            brain={brain}
            id={selected}
            catalogue={catalogue.data}
            onClose={() => setSelected(null)}
            onSaved={refresh}
          />
        )}
      </Stack>
    </Card>
  );
}
