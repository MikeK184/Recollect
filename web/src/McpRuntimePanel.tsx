import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Power, Server } from "lucide-react";
import { client, result, type Brain } from "./api";
import { SkeletonRows } from "./components/Skeleton";
import {
  Alert,
  Badge,
  Button,
  StatusBadge,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableWrap,
} from "./components/ui";
import { StatusDot } from "./components/StatusDot";
import { useFocusReturn } from "./lib/hooks";
import type { McpCatalogue } from "./McpPanel";
import { McpCallDialog } from "./McpCallDialog";
import { usePollInterval } from "./usePollInterval";

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

/** Call state as colour + icon + word; live states pulse only while active. */
export function CallState({ state }: { state: string }) {
  if (activeCall(state))
    return (
      <Badge variant="brand">
        <StatusDot live size={6} className="mr-0.5" />
        {callLabel(state)}
      </Badge>
    );
  return (
    <StatusBadge
      state={
        state === "succeeded"
          ? "positive"
          : state === "unknown"
            ? "attention"
            : ["tool_error", "failed"].includes(state)
              ? "negative"
              : "neutral"
      }
    >
      {callLabel(state)}
    </StatusBadge>
  );
}

export function McpRuntimePanel({
  brain,
  actor,
  catalogue,
  selected,
  select,
  session,
  released,
  overlay = false,
}: {
  brain: Brain;
  actor: string;
  catalogue: McpCatalogue;
  selected: string | null;
  select: (id: string | null) => void;
  session: string;
  released: () => void;
  /** Open calls as a dialog above a modal drawer instead of docking. */
  overlay?: boolean;
}) {
  const pollFast = usePollInterval("fast");
  const pollNormal = usePollInterval("normal");
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
    refetchInterval: pollFast,
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
    refetchInterval: pollNormal,
    retry: false,
    gcTime: 0,
  });
  const data = calls.error ? undefined : calls.data;
  const status = runtime.error ? undefined : runtime.data;
  return (
    <section aria-label="MCP activity" className="grid min-w-0 gap-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="min-w-0">
          <h2 className="m-0 text-title font-semibold text-foreground">
            Tool calls and runners
          </h2>
          <p className="m-0 mt-0.5 text-label text-muted-foreground">
            Release this page’s idle connections. Active calls can finish.
          </p>
        </div>
        <Button
          size="sm"
          variant="secondary"
          loading={releasing}
          onClick={() => void release()}
        >
          <Power aria-hidden="true" />
          End tool session
        </Button>
      </div>
      {releaseError && <Alert variant="destructive">{releaseError}</Alert>}
      {(calls.error || runtime.error) && (
        <Alert
          variant="destructive"
          title="Activity unavailable"
          actions={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => {
                void calls.refetch();
                void runtime.refetch();
              }}
            >
              Refresh activity
            </Button>
          }
        >
          {(calls.error || runtime.error)?.message}
        </Alert>
      )}
      {calls.isPending && (
        <div className="rounded-lg border border-border bg-card px-4 py-1">
          <SkeletonRows rows={3} label="Loading tool calls…" />
        </div>
      )}
      {status && (
        <div className="grid gap-2">
          {status.runners.length === 0 && (
            <p className="m-0 text-label text-muted-foreground">
              No configured runner targets.
            </p>
          )}
          {status.runners.length > 0 && (
            <ul className="m-0 grid list-none gap-1.5 p-0">
              {status.runners.map((r) => (
                <li
                  key={r.runner_reference}
                  className="flex min-w-0 items-center gap-2.5 rounded-md border border-border bg-card px-3 py-2"
                >
                  <Server
                    aria-hidden="true"
                    className="size-4 shrink-0 text-muted-foreground"
                  />
                  <span className="mono min-w-0 flex-1 truncate text-caption text-foreground">
                    {r.runner_reference}
                  </span>
                  <StatusBadge state={r.available ? "positive" : "attention"}>
                    {r.available ? "Runner available" : "Runner unavailable"}
                  </StatusBadge>
                </li>
              ))}
            </ul>
          )}
          {status.instances.map((i) => (
            <div
              key={i.id}
              className="grid gap-1 rounded-md border border-border bg-card px-3 py-2.5"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="min-w-0 truncate text-label font-medium text-foreground">
                  {catalogue.connections.find((c) => c.id === i.connection_id)
                    ?.name ?? "Connection"}
                </span>
                {i.state === "ready" ? (
                  <Badge variant="brand">
                    <StatusDot live={i.active_calls > 0} size={6} />
                    Connected
                  </Badge>
                ) : (
                  <Badge>{i.state}</Badge>
                )}
              </div>
              <span className="mono text-caption text-muted-foreground">
                {i.active_calls} active calls · useful idle {i.idle_seconds}s ·{" "}
                {i.runner_reference}
              </span>
            </div>
          ))}
        </div>
      )}
      {data && data.calls.length === 0 && (
        <p className="m-0 text-body text-muted-foreground">
          No tool calls yet. Open a profile’s cached tools to run an approved
          tool.
        </p>
      )}
      {data && data.calls.length > 0 && (
        <TableWrap>
          <Table>
            <TableHead>
              <tr>
                <TableHeader>Tool</TableHeader>
                <TableHeader>State</TableHeader>
                <TableHeader>Runner</TableHeader>
                <TableHeader>Started</TableHeader>
                <TableHeader className="text-right">Details</TableHeader>
              </tr>
            </TableHead>
            <TableBody>
              {data.calls.map((call) => (
                <TableRow key={call.id} selected={selected === call.id}>
                  <TableCell primary className="mono text-label">
                    {call.tool_name}
                  </TableCell>
                  <TableCell>
                    <CallState state={call.state} />
                  </TableCell>
                  <TableCell className="mono max-w-[200px] truncate text-caption">
                    {call.runner_reference}
                  </TableCell>
                  <TableCell className="mono text-caption">
                    {new Date(call.created_at).toLocaleString()}
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => select(call.id)}
                    >
                      Inspect call
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableWrap>
      )}
      {data && (cursor || data.next_cursor) && (
        <div className="flex gap-2">
          <Button
            variant="secondary"
            size="sm"
            disabled={!cursor}
            onClick={() => setCursor(undefined)}
          >
            Latest calls
          </Button>
          <Button
            variant="secondary"
            size="sm"
            disabled={!data.next_cursor}
            onClick={() => setCursor(data.next_cursor ?? undefined)}
          >
            Older calls
          </Button>
        </div>
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
          overlay={overlay}
        />
      )}
    </section>
  );
}
