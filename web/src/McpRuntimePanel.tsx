import { useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Loader,
  Stack,
  Table,
  Text,
  Title,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useFocusReturn } from "@mantine/hooks";
import { client, result, type Brain } from "./api";
import type { McpCatalogue } from "./McpPanel";
import { McpCallDialog } from "./McpCallDialog";

export const callLabel = (state: string) =>
  ({
    queued: "Queued",
    starting: "Starting",
    running: "Running",
    succeeded: "Succeeded",
    tool_error: "Tool reported an error",
    failed: "Failed",
    cancelled: "Cancelled before dispatch",
    unknown: "Completion unknown",
  })[state] ?? state;
export const activeCall = (state: string) =>
  ["queued", "starting", "running"].includes(state);
export function McpRuntimePanel({
  brain,
  actor,
  catalogue,
  selected,
  select,
  session,
  released,
}: {
  brain: Brain;
  actor: string;
  catalogue: McpCatalogue;
  selected: string | null;
  select: (id: string | null) => void;
  session: string;
  released: () => void;
}) {
  const [cursor, setCursor] = useState<string | undefined>();
  const [releasing, setReleasing] = useState(false);
  const [releaseError, setReleaseError] = useState<string | null>(null);
  useFocusReturn({ opened: selected !== null });
  const release = async () => {
    setReleasing(true);
    setReleaseError(null);
    try {
      result(
        await client.POST("/api/brains/{brain}/mcp/session/release", {
          params: { path: { brain: brain.id } },
          body: { client_session_id: session },
        }),
      );
      released();
    } catch (e) {
      setReleaseError(
        e instanceof Error
          ? e.message
          : "Session release could not be confirmed.",
      );
    } finally {
      setReleasing(false);
    }
  };
  const calls = useQuery({
    queryKey: ["mcp", brain.id, "calls", cursor],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/calls", {
          params: { path: { brain: brain.id }, query: { cursor } },
          signal,
        }),
      ),
    refetchInterval: 2000,
    retry: false,
    gcTime: 0,
  });
  const runtime = useQuery({
    queryKey: ["mcp", brain.id, "runtime"],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/runtime", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  const data = calls.error ? undefined : calls.data;
  const status = runtime.error ? undefined : runtime.data;
  return (
    <Stack component="section" aria-label="MCP activity" mt="lg">
      <Title order={4}>Tool calls and runners</Title>
      <Group>
        <Button
          size="xs"
          variant="light"
          loading={releasing}
          onClick={() => void release()}
        >
          End tool session
        </Button>
        <Text size="xs" c="dimmed">
          Release this page’s idle connections. Active calls can finish.
        </Text>
      </Group>
      {releaseError && <Alert color="red">{releaseError}</Alert>}
      {(calls.error || runtime.error) && (
        <Alert color="red" title="Activity unavailable">
          {(calls.error || runtime.error)?.message}
          <Button
            variant="light"
            size="xs"
            onClick={() => {
              void calls.refetch();
              void runtime.refetch();
            }}
          >
            Refresh activity
          </Button>
        </Alert>
      )}
      {calls.isPending && <Loader size="sm" />}
      {status && (
        <>
          {status.runners.length === 0 && (
            <Text size="sm" c="dimmed">
              No configured runner targets.
            </Text>
          )}
          {status.runners.map((r) => (
            <Group key={r.runner_reference}>
              <Text size="sm">{r.runner_reference}</Text>
              <Badge color={r.available ? "teal" : "yellow"}>
                {r.available ? "Runner available" : "Runner unavailable"}
              </Badge>
            </Group>
          ))}
          {status.instances.map((i) => (
            <Card withBorder key={i.id} padding="sm">
              <Group justify="space-between">
                <Text size="sm">
                  {catalogue.connections.find((c) => c.id === i.connection_id)
                    ?.name ?? "Connection"}
                </Text>
                <Badge color={i.state === "ready" ? "teal" : "gray"}>
                  {i.state === "ready" ? "Connected" : i.state}
                </Badge>
              </Group>
              <Text size="xs">
                {i.active_calls} active calls · useful idle {i.idle_seconds}s ·{" "}
                {i.runner_reference}
              </Text>
            </Card>
          ))}
        </>
      )}
      {data && data.calls.length === 0 && (
        <Text size="sm">
          No tool calls yet. Open a profile’s cached tools to run an approved
          tool.
        </Text>
      )}
      {data && data.calls.length > 0 && (
        <Table striped withTableBorder>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Tool</Table.Th>
              <Table.Th>State</Table.Th>
              <Table.Th>Runner</Table.Th>
              <Table.Th>Started</Table.Th>
              <Table.Th>Details</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {data.calls.map((call) => (
              <Table.Tr key={call.id}>
                <Table.Td>{call.tool_name}</Table.Td>
                <Table.Td>{callLabel(call.state)}</Table.Td>
                <Table.Td>{call.runner_reference}</Table.Td>
                <Table.Td>
                  {new Date(call.created_at).toLocaleString()}
                </Table.Td>
                <Table.Td>
                  <Button
                    size="xs"
                    variant="light"
                    onClick={() => select(call.id)}
                  >
                    Inspect call
                  </Button>
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      )}
      {data && (cursor || data.next_cursor) && (
        <Group>
          <Button
            variant="light"
            disabled={!cursor}
            onClick={() => setCursor(undefined)}
          >
            Latest calls
          </Button>
          <Button
            variant="light"
            disabled={!data.next_cursor}
            onClick={() => setCursor(data.next_cursor ?? undefined)}
          >
            Older calls
          </Button>
        </Group>
      )}
      {selected && (
        <McpCallDialog
          key={selected}
          brain={brain}
          actor={actor}
          catalogue={catalogue}
          id={selected}
          close={() => select(null)}
          select={select}
        />
      )}
    </Stack>
  );
}
