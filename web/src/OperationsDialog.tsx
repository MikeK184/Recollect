import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Activity, Database, Network, Server } from "lucide-react";
import { client, result } from "./api";
import { usePollInterval } from "./usePollInterval";
import { Alert } from "./components/ui/alert";
import { Button, IconButton } from "./components/ui/button";
import { Drawer } from "./components/ui/sheet";
import { Spinner } from "./components/ui/spinner";

export function OperationsDialog() {
  const [opened, setOpened] = useState(false);
  const pollNormalWhen0 = usePollInterval("normal", opened);
  const [openedAt, setOpenedAt] = useState(0);
  const query = useQuery({
    queryKey: ["operations"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/operations", { signal })),
    gcTime: 0,
    retry: false,
    enabled: opened,
    refetchInterval: pollNormalWhen0,
  });
  const health = useQuery({
    queryKey: ["diagnostic-status"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/status", { signal })),
    enabled: opened,
    refetchInterval: pollNormalWhen0,
    gcTime: 0,
    retry: false,
  });
  const data =
    !opened || query.error || query.dataUpdatedAt < openedAt
      ? undefined
      : query.data;
  const metric = (label: string, value: number) => (
    <div key={label} className="rounded-lg border border-border bg-card px-3 py-2.5">
      <p className="m-0 text-caption text-muted-foreground">{label}</p>
      <p className="mono m-0 mt-1 text-section font-semibold text-foreground">
        {value.toLocaleString()}
      </p>
    </div>
  );
  const total = Math.max(
    1,
    data?.http.response_header_latency_ms.at(-1)?.count ?? 0,
  );
  return (
    <>
      <IconButton
        label="Runtime diagnostics"
        size="icon-sm"
        onClick={() => {
          setOpenedAt(Date.now());
          setOpened(true);
        }}
      >
        <Activity aria-hidden="true" />
      </IconButton>
      <Drawer
        opened={opened}
        onClose={() => setOpened(false)}
        title="Runtime diagnostics"
        description="Checked while open · refreshes every 10 seconds"
        size={560}
      >
        <div className="grid gap-5">
          {health.isError ? (
            <Alert role="status">Core store checks unavailable.</Alert>
          ) : (
            opened &&
            health.dataUpdatedAt >= openedAt &&
            health.data && (
              <div className="grid grid-cols-[auto_24px_minmax(0,1fr)] items-center gap-y-2 rounded-lg border border-border p-4 dot-grid">
                <div className="flex items-center gap-2.5 rounded-lg border border-border bg-card px-3 py-2.5 shadow-xs">
                  <Server className="size-4 text-brand-text" aria-hidden="true" />
                  <div>
                    <strong className="block text-label font-semibold">Recollect API</strong>
                    <small className="block text-caption text-muted-foreground">Response received</small>
                  </div>
                </div>
                <span aria-hidden="true" className="h-px bg-brand/50" />
                <div className="grid gap-2">
                  {health.data.dependencies.map((dependency) => {
                    const Icon = dependency.name.toLowerCase().includes("neo4j")
                      ? Network
                      : Database;
                    return (
                      <div
                        key={dependency.name}
                        className="flex items-center gap-2.5 rounded-lg border border-border bg-card px-3 py-2.5 shadow-xs"
                      >
                        <Icon
                          className={dependency.connected ? "size-4 text-brand-text" : "size-4 text-destructive-text"}
                          aria-hidden="true"
                        />
                        <div className="min-w-0">
                          <strong className="block text-label font-semibold">{dependency.name}</strong>
                          <small className="block truncate text-caption text-muted-foreground">
                            {dependency.connected ? "Reachable" : "Unavailable"} · {dependency.detail}
                          </small>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )
          )}
          <p className="m-0 text-caption text-muted-foreground">
            These checks cover core stores. Model providers and agent presence
            are not checked here.
          </p>
          <p className="m-0 text-label text-muted-foreground">
            Request counters reset when the API restarts. Job counts cover only
            Brains you can access.
          </p>
          {(query.isPending || (query.isFetching && !data)) && (
            <Spinner label="Loading runtime diagnostics" />
          )}
          {query.error && (
            <Alert
              variant="destructive"
              title="Diagnostics unavailable"
              actions={
                <Button size="sm" variant="secondary" onClick={() => void query.refetch()}>
                  Retry
                </Button>
              }
            >
              {query.error.message}
            </Alert>
          )}
          {data && (
            <>
              <div className="grid grid-cols-3 gap-2">
                {metric("Requests started", data.http.requests_started)}
                {metric("Active requests", data.http.in_flight)}
                {metric("Server errors", data.http.responses_by_status_class[5] ?? 0)}
                {metric("Queued jobs", data.jobs_queued)}
                {metric("Running jobs", data.jobs_running)}
                {metric("Failed jobs", data.jobs_failed)}
              </div>
              <div className="mono flex justify-between gap-4 text-caption text-foreground-secondary">
                <span>API uptime: {Math.floor(data.http.uptime_seconds / 60)} minutes</span>
                <span>
                  Database connections: {data.database_pool_size} · {data.database_pool_idle} idle
                </span>
              </div>
              <h3 className="m-0 text-label font-semibold">Time until response headers</h3>
              <div
                className="grid gap-1.5 rounded-lg border border-border p-3"
                aria-label="Cumulative response header latency"
              >
                {data.http.response_header_latency_ms.map((bucket) => (
                  <div
                    className="mono grid grid-cols-[96px_minmax(0,1fr)_64px] items-center gap-3 text-caption text-muted-foreground"
                    key={bucket.upper_bound_ms ?? "all"}
                  >
                    <span>
                      {bucket.upper_bound_ms == null ? "Any duration" : `≤ ${bucket.upper_bound_ms} ms`}
                    </span>
                    <div className="h-1.5 overflow-hidden rounded-full bg-secondary">
                      <span
                        className="block h-full rounded-full bg-chart-1 transition-[width] duration-slow ease-out"
                        style={{ width: `${(100 * bucket.count) / total}%` }}
                      />
                    </div>
                    <span className="text-right text-foreground-secondary">{bucket.count.toLocaleString()}</span>
                  </div>
                ))}
              </div>
              <p className="m-0 text-caption text-muted-foreground">
                Counts are cumulative across API requests. Interrupted requests:{" "}
                {data.http.interrupted}. These observations do not replace Brain
                activity history.
              </p>
            </>
          )}
        </div>
      </Drawer>
    </>
  );
}
