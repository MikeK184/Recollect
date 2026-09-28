import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Stack,
  Text,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useFocusReturn } from "@mantine/hooks";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { CapturedSource } from "./CapturePanel";
import { useContentDeadline } from "./useContentDeadline";
type Observation = components["schemas"]["McpObservation"];
const label = (value: string) => value.replaceAll("_", " ");

function ObservationRow({
  observation: o,
  inspect,
}: {
  observation: Observation;
  inspect: () => void;
}) {
  const expired = useContentDeadline(o.expires_at);
  return (
    <Stack gap={3}>
      <Group>
        <Text size="sm">
          {label(o.stage)} · {label(o.outcome)}
        </Text>
        <Badge>{expired && o.state !== "removed" ? "expired" : o.state}</Badge>
      </Group>
      <Text size="xs" c="dimmed">
        Observed {new Date(o.captured_at).toLocaleString()} · {o.attempts}{" "}
        publication attempts
      </Text>
      {!expired && o.error_code && <Text size="sm">{label(o.error_code)}</Text>}
      {!expired && !!o.coverage.length && (
        <Text size="xs" c="dimmed">
          Coverage: {o.coverage.map(label).join(" · ")}
        </Text>
      )}
      {!expired &&
        o.state === "published" &&
        o.source_id &&
        o.source_version_id && (
          <Button variant="light" size="xs" onClick={inspect}>
            Inspect captured evidence
          </Button>
        )}
    </Stack>
  );
}
export function McpObservations({
  brain,
  call,
  disposition,
  onInspectChange,
}: {
  brain: Brain;
  call: string;
  disposition: string;
  onInspectChange: (opened: boolean) => void;
}) {
  const [offset, setOffset] = useState(0);
  const [viewing, setViewing] = useState<Observation | null>(null);
  const query = useQuery({
    queryKey: ["mcp-observations", brain.id, call, offset],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/calls/{id}/observations", {
          params: { path: { brain: brain.id, id: call }, query: { offset } },
          signal,
        }),
      ),
    refetchInterval: 2000,
    retry: false,
    gcTime: 0,
  });
  useFocusReturn({ opened: viewing !== null && !query.error });
  useEffect(() => {
    onInspectChange(viewing !== null && !query.error);
    return () => onInspectChange(false);
  }, [viewing, query.error, onInspectChange]);
  const messages: Record<string, string> = {
    eligible:
      "Capture was enabled for this call. Publication runs automatically after an outcome is recorded.",
    disabled: "Managed capture was disabled when this call was admitted.",
    knowledge_write_required:
      "This caller could run the tool but did not have knowledge-write permission.",
    receipt_lookup: "Receipt evidence is linked to the original call.",
    removed:
      "This call's captured evidence was erased. Late results cannot restore it.",
  };
  return (
    <Stack gap="xs" aria-label="Managed observations">
      <Text fw={600}>Captured observations</Text>
      <Text size="sm" c="dimmed">
        {messages[disposition] ?? label(disposition)}
      </Text>
      {query.isPending && <Loader size="sm" />}
      {query.error && (
        <Alert color="red">
          {query.error.message}
          <Button variant="subtle" onClick={() => void query.refetch()}>
            Refresh observations
          </Button>
        </Alert>
      )}
      {!query.error &&
        query.data?.items.map((o) => (
          <ObservationRow
            key={o.id}
            observation={o}
            inspect={() => setViewing(o)}
          />
        ))}
      {!query.error && query.data?.total === 0 && (
        <Text size="sm">No published observation yet.</Text>
      )}
      {query.data && query.data.total > 20 && (
        <Group>
          <Button
            variant="subtle"
            disabled={!offset}
            onClick={() => setOffset(offset - 20)}
          >
            Previous observations
          </Button>
          <Text size="xs">
            {offset + 1}–{Math.min(offset + 20, query.data.total)} of{" "}
            {query.data.total}
          </Text>
          <Button
            variant="subtle"
            disabled={offset + 20 >= query.data.total}
            onClick={() => setOffset(offset + 20)}
          >
            Next observations
          </Button>
        </Group>
      )}
      {viewing && !query.error && (
        <CapturedSource
          brain={brain}
          source={viewing.source_id!}
          version={viewing.source_version_id!}
          close={() => setViewing(null)}
        />
      )}
    </Stack>
  );
}
