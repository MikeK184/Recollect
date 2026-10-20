import { useEffect, useRef, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import {
  ArrowRight,
  CircleAlert,
  CircleCheck,
  CircleDashed,
  Clock3,
  LoaderCircle,
  Network,
  RefreshCw,
  Terminal,
  UserRound,
} from "lucide-react";
import { client, result, type Brain } from "../../api";
import { useBrain } from "../../app/context";
import { EmptyState, LoadingState } from "../../components/AsyncState";
import { DetailInspector } from "../../components/Inspector";
import { staggerStyle } from "../../components/Motion";
import { StatusDot } from "../../components/StatusDot";
import { useStableRows } from "../../components/useStableRows";
import {
  Alert,
  Badge,
  Button,
  Card,
  CardBar,
  IconButton,
  Stat,
  Tile,
  Tooltip,
} from "../../components/ui";
import { arrive, leave, listItem, settle } from "../../design/motion";
import { cn } from "../../lib/utils";
import { pollIntervals, usePollInterval } from "../../usePollInterval";
import { PipelineFlow } from "./PipelineFlow";
import { PipelineRecord } from "./PipelineRecord";
import {
  inputStatus,
  itemState,
  observedStages,
  origin,
  originKey,
  processingState,
  stageTone,
  stateLabel,
  type FlowStage,
  type PipelineFeed,
  type PipelineItem,
  type StageTone,
} from "./pipelineModel";

const NO_PULSES: Partial<Record<FlowStage, number>> = {};
const timeOf = (value: string) => new Date(value).toLocaleTimeString();

function CheckedAgo({ since }: { since: number }) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [since]);
  return <>{`Checked ${Math.max(0, Math.floor((now - since) / 1000))}s ago`}</>;
}

/** Status pairs its colour with an icon and a word (non-colour rule). */
function InputState({ tone, label }: { tone: StageTone; label: string }) {
  const variant =
    tone === "done" || tone === "running"
      ? "brand"
      : tone === "waiting"
        ? "warning"
        : tone === "failed"
          ? "destructive"
          : "default";
  const Icon =
    tone === "running"
      ? LoaderCircle
      : tone === "waiting"
        ? Clock3
        : tone === "failed"
          ? CircleAlert
          : tone === "done"
            ? CircleCheck
            : CircleDashed;
  return (
    <Badge variant={variant}>
      <Icon
        aria-hidden="true"
        className={cn(
          tone === "running" &&
            "animate-spin group-data-[paused=true]/pipeline:[animation-play-state:paused]",
        )}
      />
      {label}
    </Badge>
  );
}

function EventIcon({ tone }: { tone: StageTone }) {
  if (tone === "running")
    return <StatusDot live size={7} className="mx-[4.5px]" />;
  if (tone === "waiting")
    return <Clock3 aria-hidden="true" className="size-4 text-warning-text" />;
  if (tone === "failed")
    return <CircleAlert aria-hidden="true" className="size-4 text-destructive-text" />;
  if (tone === "done")
    return <CircleCheck aria-hidden="true" className="size-4 text-brand-text" />;
  return <CircleDashed aria-hidden="true" className="size-4 text-faint-foreground" />;
}

type LogRow = {
  key: string;
  at: string;
  tone: StageTone;
  text: string;
  meta: ReactNode;
};

/** The selected input's recorded events, newest first (at most eight). */
function eventRows(
  item: PipelineItem,
  at: string,
  outcomeAction: ReactNode,
): LogRow[] {
  const state = itemState(item, at);
  const run = item.learning;
  const summary: LogRow = {
    key: `state:${state}:${item.activity_at}`,
    at: item.activity_at,
    tone: stageTone(state),
    text: `${item.contributor} → ${origin(item)} → ${inputStatus(item, at).label}`,
    meta: (
      <time className="mono" dateTime={item.activity_at}>
        {new Date(item.activity_at).toLocaleString()}
      </time>
    ),
  };
  const recorded: LogRow[] = [];
  if (run?.state === "succeeded")
    recorded.push({
      key: `outcome:${run.id}`,
      at: run.finished_at ?? run.created_at,
      tone: "done",
      text: "Recorded outcome",
      meta: (
        <>
          <span>
            {run.accepted} accepted · {run.proposed} proposed
            {run.conflicting ? ` (${run.conflicting} conflicting)` : ""}
          </span>
          {outcomeAction}
        </>
      ),
    });
  if (run && run.state !== "succeeded")
    recorded.push({
      key: `learning:${run.id}:${run.job.state}:${run.job.updated_at}`,
      at: run.job.updated_at,
      tone: stageTone(run.state === "running" ? run.job.state : run.state),
      text: `Recollect learning · ${stateLabel(run.state === "running" ? run.job.state : run.state).toLowerCase()}`,
      meta: run.job.error_code ? <span className="mono">{run.job.error_code}</span> : null,
    });
  if (item.processing_job)
    recorded.push({
      key: `processing:${item.processing_job.id}:${item.processing_job.state}:${item.processing_job.updated_at}`,
      at: item.processing_job.updated_at,
      tone: stageTone(processingState(item, at)),
      text: `Source processing · ${stateLabel(processingState(item, at)).toLowerCase()}`,
      meta: item.processing_job.error_code ? (
        <span className="mono">{item.processing_job.error_code}</span>
      ) : null,
    });
  recorded.push({
    key: `received:${item.id}`,
    at: item.received_at,
    tone: "done",
    text: `Received · ${item.kind.replaceAll("_", " ")}${item.tool_name ? ` · ${item.tool_name}` : ""}`,
    meta: null,
  });
  recorded.sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
  return [summary, ...recorded].slice(0, 8);
}

function EventLog({ rows }: { rows: LogRow[] }) {
  return (
    <ol
      className="m-0 list-none border-t border-border py-1 pl-0"
      aria-label="Recorded events"
      aria-live="polite"
    >
      <AnimatePresence initial={false}>
        {rows.map((row) => (
          <motion.li
            key={row.key}
            layout="position"
            initial={{ opacity: 0, y: -6 }}
            animate={{ opacity: 1, y: 0, transition: arrive }}
            exit={{ opacity: 0, transition: leave }}
            transition={{ layout: settle }}
            className="grid h-8 grid-cols-[72px_16px_minmax(0,1fr)_auto] items-center gap-2.5 px-4 text-label"
          >
            <time className="mono text-caption text-muted-foreground" dateTime={row.at}>
              {timeOf(row.at)}
            </time>
            <EventIcon tone={row.tone} />
            <code className="mono truncate text-caption text-foreground-secondary" title={row.text}>
              {row.text}
            </code>
            <span className="flex items-center gap-2 whitespace-nowrap text-caption text-muted-foreground">
              {row.meta}
            </span>
          </motion.li>
        ))}
      </AnimatePresence>
    </ol>
  );
}

type StatCard = {
  label: string;
  value: ReactNode;
  unit?: string;
  caption: string;
  live?: boolean;
  link: ReactNode;
};

function StatTile({ card, index }: { card: StatCard; index: number }) {
  return (
    <motion.div variants={listItem} initial="initial" animate="animate" custom={index}>
      <Card interactive className="relative h-full focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ring">
        <p className="m-0 flex items-center gap-2 text-body text-muted-foreground">
          {card.link}
          {card.live && <StatusDot live size={7} label="Working now" />}
        </p>
        <Stat className="mono mt-1.5 flex items-baseline gap-1.5">
          <AnimatePresence mode="popLayout" initial={false}>
            <motion.span
              key={String(card.value)}
              initial={{ opacity: 0, y: 4 }}
              animate={{ opacity: 1, y: 0, transition: arrive }}
              exit={{ opacity: 0, transition: leave }}
            >
              {card.value}
            </motion.span>
          </AnimatePresence>
          {card.unit && (
            <span className="font-sans text-title font-medium text-muted-foreground">
              {card.unit}
            </span>
          )}
        </Stat>
        <p className="m-0 mt-0.5 text-caption text-muted-foreground">{card.caption}</p>
      </Card>
    </motion.div>
  );
}

const graphStateLabel = (graph: NonNullable<PipelineFeed["graph"]>) =>
  graph.state === "ready" && graph.input_epoch !== graph.current_epoch
    ? "Awaiting current projection"
    : graph.state === "ready"
      ? "Ready"
      : stateLabel(graph.state);

/** Summary cards built only from the pipeline feed the dashboard already reads. */
function DashboardStats({
  brain,
  data,
  pending,
}: {
  brain: Brain;
  data?: PipelineFeed;
  pending: boolean;
}) {
  const items = data?.items ?? [];
  const at = data?.observed_at ?? "";
  const running = items.filter((i) => itemState(i, at) === "running").length;
  const active = items.filter((i) => i.active).length;
  const runs = items.filter((i) => i.learning?.state === "succeeded");
  const accepted = runs.reduce((sum, i) => sum + (i.learning?.accepted ?? 0), 0);
  const proposed = runs.reduce((sum, i) => sum + (i.learning?.proposed ?? 0), 0);
  const contexts = new Set(items.map(originKey)).size;
  const none = pending ? "…" : "—";
  // The whole card is the link target; the visible label names it.
  const stretch =
    "text-muted-foreground no-underline after:absolute after:inset-0 after:rounded-lg after:content-[''] hover:text-muted-foreground focus-visible:outline-none";
  const cards: StatCard[] = [
    {
      label: "Inputs",
      value: data ? items.length : none,
      caption: data
        ? `${contexts} contributor ${contexts === 1 ? "context" : "contexts"} · ${data.has_more ? "latest 30" : "recorded window"}`
        : "Recent inputs unavailable",
      link: (
        <Link
          to="/brains/$brainId/activity"
          params={{ brainId: brain.id }}
          search={{ tab: "pipeline" }}
          className={stretch}
        >
          Inputs
        </Link>
      ),
    },
    {
      label: "In progress",
      value: data ? active : none,
      caption: !data
        ? "Processing state unavailable"
        : active
          ? `${running} running · ${active - running} queued or waiting`
          : "Nothing is processing now",
      live: running > 0,
      link: (
        <Link
          to="/brains/$brainId/activity"
          params={{ brainId: brain.id }}
          search={{ tab: "processing" }}
          className={stretch}
        >
          In progress
        </Link>
      ),
    },
    {
      label: "Learning outcomes",
      value: data ? accepted : none,
      unit: data ? "accepted" : undefined,
      caption: data
        ? `${proposed} proposed · ${runs.length} ${runs.length === 1 ? "run" : "runs"} in this window`
        : "Learning results unavailable",
      link: (
        <Link
          to="/brains/$brainId/memory"
          params={{ brainId: brain.id }}
          search={{}}
          className={stretch}
        >
          Learning outcomes
        </Link>
      ),
    },
    {
      label: "Graph",
      value: data?.graph ? data.graph.node_count : none,
      unit: data?.graph ? "nodes" : undefined,
      caption: data?.graph
        ? `${data.graph.edge_count} edges · ${graphStateLabel(data.graph).toLowerCase()}`
        : data
          ? "No projection recorded"
          : "Graph state unavailable",
      link: (
        <Link
          to="/brains/$brainId/graph"
          params={{ brainId: brain.id }}
          search={{}}
          className={stretch}
        >
          Graph
        </Link>
      ),
    },
  ];
  return (
    <div className="@container" aria-label="Brain summary" role="group">
      <div className="grid grid-cols-2 gap-4 @3xl:grid-cols-4">
        {cards.map((card, index) => (
          <StatTile key={card.label} card={card} index={index} />
        ))}
      </div>
    </div>
  );
}

function GraphCard({ brain, data }: { brain: Brain; data: PipelineFeed }) {
  const graph = data.graph;
  const label = graph ? graphStateLabel(graph) : "Not recorded";
  return (
    <Card className="flex min-w-0 flex-col gap-3.5">
      <div className="flex items-center gap-2">
        <h3 id="pipeline-graph-title" className="m-0 text-title font-semibold text-foreground">
          Brain graph
        </h3>
        <Badge variant={label === "Ready" ? "brand" : graph ? "warning" : "default"}>
          {label}
        </Badge>
        <Link
          to="/brains/$brainId/graph"
          params={{ brainId: brain.id }}
          search={{}}
          className="ml-auto text-label text-brand-text hover:underline"
        >
          Explore graph
        </Link>
      </div>
      <div className="dot-grid grid min-h-[132px] flex-1 place-items-center rounded-md border border-border">
        {graph ? (
          <div className="flex items-end gap-8">
            <div className="grid justify-items-center">
              <Stat className="mono">{graph.node_count}</Stat>
              <span className="text-caption text-muted-foreground">nodes</span>
            </div>
            <div className="grid justify-items-center">
              <Stat className="mono">{graph.edge_count}</Stat>
              <span className="text-caption text-muted-foreground">edges</span>
            </div>
          </div>
        ) : (
          <span className="flex items-center gap-2 text-label text-muted-foreground">
            <Network className="size-4" aria-hidden="true" />
            No projection recorded
          </span>
        )}
      </div>
      <div className="flex items-center gap-2 text-caption text-muted-foreground">
        <span>Independent projection</span>
        {graph?.published_at && (
          <time className="mono ml-auto" dateTime={graph.published_at}>
            Published {new Date(graph.published_at).toLocaleString()}
          </time>
        )}
      </div>
    </Card>
  );
}

export function PipelineView() {
  const brain = useBrain();
  return <BrainPipeline key={brain.id} brain={brain} />;
}

export function BrainPipeline({
  brain,
  compact = false,
  dashboard = false,
}: {
  brain: Brain;
  compact?: boolean;
  dashboard?: boolean;
}) {
  const pollFast = usePollInterval("fast");
  const reduced = useReducedMotion();
  const [visible, setVisible] = useState(() => !document.hidden),
    [now, setNow] = useState(Date.now);
  const [selected, setSelected] = useState<string | null>(null),
    [inspecting, setInspecting] = useState(false),
    [expanded, setExpanded] = useState(false);
  const [changed, setChanged] = useState<
    Map<string, Partial<Record<FlowStage, number>>>
  >(new Map());
  const previous = useRef<PipelineFeed | null>(null);
  const firstRows = useRef<Set<string> | null>(null);
  const pulseTimer = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  useEffect(() => {
    const update = () => {
      setVisible(!document.hidden);
      setNow(Date.now());
      previous.current = null;
      setChanged(new Map());
      clearTimeout(pulseTimer.current);
    };
    document.addEventListener("visibilitychange", update);
    return () => {
      clearTimeout(pulseTimer.current);
      document.removeEventListener("visibilitychange", update);
    };
  }, []);
  const query = useQuery({
    queryKey: ["pipeline-traces", brain.id],
    enabled: visible,
    queryFn: async ({ signal }) => {
      const started = performance.now();
      const feed = result(
        await client.GET("/api/brains/{brain}/pipeline", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      );
      return {
        ...feed,
        clientExpiresAt:
          Date.now() +
          Math.max(
            0,
            Date.parse(feed.valid_until) -
              Date.parse(feed.observed_at) -
              (performance.now() - started),
          ),
      };
    },
    refetchInterval: pollFast,
    refetchIntervalInBackground: false,
    staleTime: 0,
    gcTime: 0,
    retry: false,
    refetchOnMount: "always",
  });
  // Subtract the entire request duration: latency and clock skew cannot extend a deadline.
  useEffect(() => {
    if (!query.data) return;
    const timer = setTimeout(
      () => setNow(Date.now()),
      Math.max(0, query.data.clientExpiresAt - Date.now()),
    );
    return () => clearTimeout(timer);
  }, [query.data]);
  const stale =
    !!query.data && Math.max(now, Date.now()) >= query.data.clientExpiresAt;
  // Visibility pauses polling/motion, not a still-valid snapshot. Clearing it
  // on every focus event tears down the graph and closes inspection while the
  // replacement read is pending. Errors and the original deadline still clear it.
  const data = !query.isError && !stale ? query.data : undefined;
  useEffect(() => {
    if (!data) {
      previous.current = null;
      setChanged(new Map());
      setInspecting(false);
      clearTimeout(pulseTimer.current);
      return;
    }
    const changes = observedStages(previous.current, data);
    previous.current = data;
    if (!visible || reduced || !changes.size) return;
    clearTimeout(pulseTimer.current);
    const token = Date.now();
    setChanged(
      new Map(
        [...changes].map(([id, stages]) => [
          id,
          Object.fromEntries(stages.map((stage) => [stage, token])),
        ]),
      ),
    );
    pulseTimer.current = setTimeout(() => setChanged(new Map()), 1600);
  }, [data, reduced, visible]);
  useEffect(() => {
    if (inspecting && !data?.items.some((row) => row.id === selected))
      setInspecting(false);
  }, [data, selected, inspecting]);
  const item = data?.items.find((i) => i.id === selected) ?? data?.items[0];
  const running =
    data?.items.filter((i) => itemState(i, data.observed_at) === "running")
      .length ?? 0;
  const active = data?.items.filter((i) => i.active).length ?? 0;
  const identities = new Set(data?.items.map(originKey));
  const stable = useStableRows(
    data?.items,
    brain.id,
    !!selected,
    (row) => row.id,
  );
  if (data && !firstRows.current)
    firstRows.current = new Set(data.items.map((row) => row.id));
  const shown =
    (compact || dashboard) && !expanded ? stable.rows.slice(0, 5) : stable.rows;
  const choose = (id: string) => {
    setSelected(id);
    setInspecting(false);
    setChanged(new Map());
  };
  const inspect = () => {
    if (!item) return;
    setSelected(item.id);
    setInspecting(true);
  };
  const outcomeAction =
    dashboard && item?.learning?.state === "succeeded" ? (
      <Button variant="ghost" size="sm" className="-my-1" onClick={inspect}>
        Inspect outcome
      </Button>
    ) : null;
  return (
    <>
      {dashboard && (
        <DashboardStats brain={brain} data={data} pending={query.isPending} />
      )}
      <section
        className="group/pipeline @container grid gap-6"
        aria-label={`${brain.name} activity pipeline`}
        data-testid="live-pipeline"
        data-paused={!visible}
      >
        <div className="overflow-hidden rounded-lg border border-border bg-card">
          <div className="flex h-12 min-w-0 items-center gap-2.5 border-b border-border pl-4 pr-2">
            <h3 className="m-0 shrink-0 text-body font-semibold text-foreground">
              Activity pipeline
            </h3>
            {!data ? (
              <Badge variant="outline" className="shrink-0">Unavailable</Badge>
            ) : active ? (
              <Badge variant="brand" className="shrink-0">
                <StatusDot live size={6} className="-ml-0.5" />
                {running} running · {active - running} queued / waiting
              </Badge>
            ) : (
              <Badge variant="outline" className="shrink-0">Last recorded activity</Badge>
            )}
            {item && (
              <span className="min-w-0 truncate text-label text-muted-foreground">
                <span className="text-foreground-secondary">{item.title}</span>
                {" · "}
                {origin(item)} · {item.kind.replaceAll("_", " ")}
                {item.tool_name ? ` · ${item.tool_name}` : ""}
              </span>
            )}
            <div className="ml-auto flex shrink-0 items-center gap-1">
              <span className="mono mr-1 text-caption text-muted-foreground">
                {data ? (
                  <CheckedAgo since={query.dataUpdatedAt} />
                ) : query.isPending ? (
                  "Checking activity"
                ) : (
                  "Updates paused"
                )}
              </span>
              <IconButton
                size="icon-sm"
                label="Refresh pipeline"
                onClick={() => void query.refetch()}
              >
                <RefreshCw
                  aria-hidden="true"
                  className={cn(query.isFetching && "animate-spin")}
                />
              </IconButton>
              {item && (
                <Button variant="ghost" size="sm" onClick={inspect}>
                  Inspect input
                </Button>
              )}
              {compact && (
                <Button asChild variant="secondary" size="sm">
                  <Link
                    to="/brains/$brainId/ask"
                    params={{ brainId: brain.id }}
                    search={{}}
                  >
                    Open Brain
                    <ArrowRight aria-hidden="true" />
                  </Link>
                </Button>
              )}
            </div>
          </div>
          {query.isPending ? (
            <div className="p-4">
              <LoadingState label="Loading recorded activity…" />
            </div>
          ) : !data ? (
            <div className="p-4">
              <Alert
                variant={query.isError ? "destructive" : "default"}
                title={
                  query.isError
                    ? "Activity unavailable"
                    : !visible
                      ? "Updates paused"
                      : "Activity needs a fresh check"
                }
                actions={
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => void query.refetch()}
                  >
                    Retry
                  </Button>
                }
              >
                {query.isError
                  ? "The last snapshot has been cleared. Retry to read current activity."
                  : "No stale processing state is shown."}
              </Alert>
            </div>
          ) : !item ? (
            <div className="dot-grid">
              <EmptyState
                icon={Network}
                title="No recorded activity yet"
                description="Published agent captures and source imports will appear here as they arrive."
              />
            </div>
          ) : (
            <>
              <PipelineFlow
                brain={brain}
                item={item}
                at={data.observed_at}
                pulses={
                  !visible || reduced
                    ? NO_PULSES
                    : (changed.get(item.id) ?? NO_PULSES)
                }
                onInspect={inspect}
              />
              <EventLog
                key={item.id}
                rows={eventRows(item, data.observed_at, outcomeAction)}
              />
            </>
          )}
        </div>
        {data && item && (
          <div className="grid grid-cols-1 gap-4 @4xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
            <section
              className="min-w-0 overflow-hidden rounded-lg border border-border bg-card"
              aria-labelledby="pipeline-inputs-title"
            >
              <CardBar>
                <h3 id="pipeline-inputs-title">
                  {active ? "In progress & recent inputs" : "Recent inputs"}
                </h3>
                <span className="text-caption text-muted-foreground">
                  {identities.size} contributor{" "}
                  {identities.size === 1 ? "context" : "contexts"} ·{" "}
                  {data.has_more ? "Latest 30" : "Recorded window"}
                </span>
                {stable.pending > 0 && (
                  <Button
                    variant="secondary"
                    size="sm"
                    className="ml-auto"
                    onClick={stable.reveal}
                  >
                    {stable.pending} new inputs · Show updates
                  </Button>
                )}
              </CardBar>
              <div className="grid">
                {shown.map((row, index) => {
                  const status = inputStatus(row, data.observed_at);
                  const tone = stageTone(itemState(row, data.observed_at));
                  const first = firstRows.current?.has(row.id);
                  return (
                    <motion.button
                      type="button"
                      layout={visible && !reduced ? "position" : false}
                      transition={{ layout: settle }}
                      key={row.id}
                      className={cn(
                        "flex w-full items-center gap-3 border-b border-border px-4 py-2.5 text-left transition-colors duration-fast last:border-b-0 hover:bg-accent focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring",
                        row.id === item.id && "bg-accent",
                        first ? "animate-enter" : "animate-learned",
                        changed.has(row.id) && "animate-learned",
                      )}
                      style={first ? staggerStyle(index) : undefined}
                      onClick={() => choose(row.id)}
                      aria-pressed={row.id === item.id}
                      aria-description={status.detail ?? undefined}
                      data-testid={`pipeline-input-${row.id}`}
                    >
                      <Tile size="md" tone="muted">
                        {row.device_id || row.host ? (
                          <Terminal aria-hidden="true" />
                        ) : (
                          <UserRound aria-hidden="true" />
                        )}
                      </Tile>
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-body font-medium text-foreground">
                          {row.title}
                        </span>
                        <span className="block truncate text-caption text-muted-foreground">
                          {row.contributor} · {origin(row)}
                          {row.agent_id ? ` · Agent ${row.agent_id}` : ""}
                        </span>
                      </span>
                      <Tooltip label={status.detail} disabled={!status.detail}>
                        <span className="inline-flex">
                          <InputState tone={tone} label={status.label} />
                        </span>
                      </Tooltip>
                      <time
                        className="mono w-[72px] shrink-0 text-right text-caption text-muted-foreground"
                        dateTime={row.activity_at}
                      >
                        {timeOf(row.activity_at)}
                      </time>
                    </motion.button>
                  );
                })}
              </div>
              {(compact || dashboard) && data.items.length > 5 && (
                <div className="border-t border-border px-2 py-1.5">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setExpanded(!expanded)}
                  >
                    {expanded
                      ? "Show fewer inputs"
                      : `Show ${data.items.length} inputs`}
                  </Button>
                </div>
              )}
            </section>
            <GraphCard brain={brain} data={data} />
          </div>
        )}
        <p className="m-0 -mt-2 text-caption text-muted-foreground">
          Checks every {pollIntervals.fast / 1000}s · Observed states only.
          {data?.has_more
            ? " More activity exists outside this bounded window."
            : ""}
        </p>
        <DetailInspector
          opened={inspecting && !!data && !!item && item.id === selected}
          onClose={() => setInspecting(false)}
          title="Processing record"
        >
          {data && item && item.id === selected && (
            <PipelineRecord brain={brain} item={item} at={data.observed_at} />
          )}
        </DetailInspector>
      </section>
    </>
  );
}
