import { useState } from "react";
import {
  Alert,
  Button,
  Group,
  Loader,
  Modal,
  Paper,
  SimpleGrid,
  Stack,
  Table,
  Text,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { client, result } from "./api";

export function OperationsDialog() {
  const [opened, setOpened] = useState(false);
  const query = useQuery({
    queryKey: ["operations"],
    queryFn: async () => result(await client.GET("/api/operations")),
    enabled: opened,
    refetchInterval: opened ? 10_000 : false,
  });
  const data = query.error ? undefined : query.data;
  return (
    <>
      <Button variant="subtle" size="xs" onClick={() => setOpened(true)}>
        Runtime diagnostics
      </Button>
      <Modal
        opened={opened}
        onClose={() => setOpened(false)}
        title="Runtime diagnostics"
        size="lg"
      >
        <Stack>
          <Text size="sm" c="dimmed">
            Request counters reset when the API restarts. Job counts cover only
            Brains you can access.
          </Text>
          {query.isPending && (
            <Loader
              size="sm"
              role="status"
              aria-label="Loading runtime diagnostics"
            />
          )}
          {query.error && (
            <Alert color="red" title="Diagnostics unavailable">
              <Text size="sm">{query.error.message}</Text>
              <Button
                size="xs"
                variant="light"
                mt="sm"
                onClick={() => void query.refetch()}
              >
                Retry
              </Button>
            </Alert>
          )}
          {data && (
            <>
              <SimpleGrid cols={3}>
                {(
                  [
                    ["Requests started", data.http.requests_started],
                    ["Active requests", data.http.in_flight],
                    [
                      "Server errors",
                      data.http.responses_by_status_class[5] ?? 0,
                    ],
                    ["Queued jobs", data.jobs_queued],
                    ["Running jobs", data.jobs_running],
                    ["Failed jobs", data.jobs_failed],
                  ] as const
                ).map(([label, value]) => (
                  <Paper key={label} p="sm" withBorder>
                    <Text size="xs" c="dimmed">
                      {label}
                    </Text>
                    <Text size="xl" fw={600}>
                      {value.toLocaleString()}
                    </Text>
                  </Paper>
                ))}
              </SimpleGrid>
              <Group justify="space-between">
                <Text size="sm">
                  API uptime: {Math.floor(data.http.uptime_seconds / 60)}{" "}
                  minutes
                </Text>
                <Text size="sm">
                  Database connections: {data.database_pool_size} ·{" "}
                  {data.database_pool_idle} idle
                </Text>
              </Group>
              <Text size="sm" fw={600}>
                Time until response headers
              </Text>
              <Table>
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th>Within</Table.Th>
                    <Table.Th>Completed requests</Table.Th>
                  </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                  {data.http.response_header_latency_ms.map((bucket) => (
                    <Table.Tr key={bucket.upper_bound_ms ?? "all"}>
                      <Table.Td>
                        {bucket.upper_bound_ms == null
                          ? "Any duration"
                          : `${bucket.upper_bound_ms.toLocaleString()} ms`}
                      </Table.Td>
                      <Table.Td>{bucket.count.toLocaleString()}</Table.Td>
                    </Table.Tr>
                  ))}
                </Table.Tbody>
              </Table>
              <Text size="xs" c="dimmed">
                Counts are cumulative across API requests. Interrupted requests:{" "}
                {data.http.interrupted}. These observations do not replace Brain
                activity history.
              </Text>
            </>
          )}
        </Stack>
      </Modal>
    </>
  );
}
