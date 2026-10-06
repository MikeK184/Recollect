import { useState } from "react";
import {
  Alert,
  ActionIcon,
  Button,
  Group,
  Loader,
  Drawer,
  Paper,
  SimpleGrid,
  Stack,
  Text,
  Tooltip,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { Activity, Database, Network, Server } from "lucide-react";
import "./features/workspace/control-panel.css";
import { client, result } from "./api";

export function OperationsDialog() {
  const [opened, setOpened] = useState(false);
  const [openedAt, setOpenedAt] = useState(0);
  const query = useQuery({
    queryKey: ["operations"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/operations", { signal })),
    gcTime: 0,
    retry: false,
    enabled: opened,
    refetchInterval: opened ? 10_000 : false,
  });
  const health = useQuery({
    queryKey: ["diagnostic-status"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/status", { signal })),
    enabled: opened,
    refetchInterval: opened ? 10000 : false,
    gcTime: 0,
    retry: false,
  });
  const data =
    !opened || query.error || query.dataUpdatedAt < openedAt
      ? undefined
      : query.data;
  return (
    <>
      <Tooltip label="Runtime diagnostics">
        <ActionIcon
          variant="subtle"
          size="lg"
          color="teal"
          aria-label="Runtime diagnostics"
          onClick={() => {
            setOpenedAt(Date.now());
            setOpened(true);
          }}
        >
          <Activity size={19} aria-hidden="true" />
        </ActionIcon>
      </Tooltip>
      <Drawer
        opened={opened}
        onClose={() => setOpened(false)}
        title="Runtime diagnostics"
        size="xl"
        position="right"
        className="control-drawer"
      >
        <Stack gap="lg">
          <Text size="xs" c="dimmed">
            Checked while open · refreshes every 10 seconds
          </Text>
          {health.isError ? (
            <Alert color="gray">Core store checks unavailable.</Alert>
          ) : (
            opened &&
            health.dataUpdatedAt >= openedAt &&
            health.data && (
              <div className="control-surface">
                <div className="diagnostic-map">
                  <div className="diagnostic-service">
                    <Server size={21} />
                    <div>
                      <strong>Recollect API</strong>
                      <small>Response received</small>
                    </div>
                  </div>
                  <div className="diagnostic-join" />
                  <Stack gap="sm">
                    {health.data.dependencies.map((dependency) => (
                      <div key={dependency.name} className="diagnostic-service">
                        {dependency.name.toLowerCase().includes("neo4j") ? (
                          <Network size={21} />
                        ) : (
                          <Database size={21} />
                        )}
                        <div>
                          <strong>{dependency.name}</strong>
                          <small>
                            {dependency.connected ? "Reachable" : "Unavailable"}{" "}
                            · {dependency.detail}
                          </small>
                        </div>
                      </div>
                    ))}
                  </Stack>
                </div>
              </div>
            )
          )}
          <Text size="xs" c="dimmed">
            These checks cover core stores. Model providers and agent presence
            are not checked here.
          </Text>
          <Text size="sm" c="dimmed">
            Request counters reset when the API restarts. Job counts cover only
            Brains you can access.
          </Text>
          {(query.isPending || (query.isFetching && !data)) && (
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
                  <Paper
                    key={label}
                    p="sm"
                    withBorder
                    className="diagnostic-metric"
                  >
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
              <div
                className="control-surface diagnostic-cdf"
                aria-label="Cumulative response header latency"
              >
                {data.http.response_header_latency_ms.map((bucket) => (
                  <div
                    className="diagnostic-bar-row"
                    key={bucket.upper_bound_ms ?? "all"}
                  >
                    <span>
                      {bucket.upper_bound_ms == null
                        ? "Any duration"
                        : `≤ ${bucket.upper_bound_ms} ms`}
                    </span>
                    <div className="diagnostic-bar">
                      <span
                        style={{
                          width: `${(100 * bucket.count) / Math.max(1, data.http.response_header_latency_ms.at(-1)?.count ?? 0)}%`,
                        }}
                      />
                    </div>
                    <span>{bucket.count.toLocaleString()}</span>
                  </div>
                ))}
              </div>
              <Text size="xs" c="dimmed">
                Counts are cumulative across API requests. Interrupted requests:{" "}
                {data.http.interrupted}. These observations do not replace Brain
                activity history.
              </Text>
            </>
          )}
        </Stack>
      </Drawer>
    </>
  );
}
