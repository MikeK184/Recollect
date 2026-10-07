import { useEffect, useRef, useState, memo } from "react";
import {
  Alert,
  Button,
  Drawer,
  Group,
  Stack,
  Text,
  Tooltip,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { motion, useReducedMotion } from "motion/react";
import {
  Background,
  BaseEdge,
  Handle,
  Position,
  ReactFlow,
  getSmoothStepPath,
  type ReactFlowInstance,
  type Node,
  type NodeProps,
  type Edge,
  type EdgeProps,
} from "@xyflow/react";
import {
  ArrowRight,
  Boxes,
  FileInput,
  FileCog,
  Network,
  NotebookTabs,
  RefreshCw,
  Sparkles,
  Terminal,
  UserRound,
} from "lucide-react";
import { client, result, type Brain } from "../../api";
import { useBrain } from "../../app/context";
import { LoadingState } from "../../components/AsyncState";
import {
  observedStages,
  type FlowStage,
  itemState,
  inputStatus,
  learningFailure,
  learningState,
  origin,
  originKey,
  processingState,
  stateLabel,
  type PipelineFeed,
  type PipelineItem,
} from "./pipelineModel";
import "@xyflow/react/dist/style.css";
import "./pipeline.css";
import { useStableRows } from "../../components/useStableRows";

type StageData = {
  label: string;
  value: string;
  detail: string;
  kind: string;
  live: boolean;
};
type StageNode = Node<StageData, "stage">;
type FlowEdge = Edge<{ pulse?: number }, "observed">;
const Stage = memo(({ data }: NodeProps<StageNode>) => {
  const Icon =
    {
      origin: UserRound,
      input: Boxes,
      processing: FileCog,
      learning: Sparkles,
      memory: NotebookTabs,
    }[data.kind] ?? FileInput;
  return (
    <div
      className={`flow-stage ${data.live ? "is-running" : ""}`}
      data-stage={data.kind}
    >
      <Handle type="target" position={Position.Left} isConnectable={false} />
      <span className="flow-stage-icon">
        <Icon size={18} />
      </span>
      <div>
        <span className="flow-stage-label">{data.label}</span>
        <strong>{data.value}</strong>
        <small>{data.detail}</small>
      </div>
      <Handle type="source" position={Position.Right} isConnectable={false} />
    </div>
  );
});
function ObservedEdge(props: EdgeProps<FlowEdge>) {
  const [path] = getSmoothStepPath({ ...props, borderRadius: 24, offset: 30 });
  return (
    <>
      <BaseEdge path={path} />
      {props.data?.pulse ? (
        <path
          key={props.data.pulse}
          className="flow-packet"
          d={path}
          pathLength={1}
          data-testid="pipeline-pulse"
        />
      ) : null}
    </>
  );
}
const nodeTypes = { stage: Stage };
const edgeTypes = { observed: ObservedEdge };
function Diagram({
  brain,
  item,
  at,
  pulses,
  onInspect,
}: {
  brain: Brain;
  item: PipelineItem;
  at: string;
  pulses: Partial<Record<FlowStage, number>>;
  onInspect: () => void;
}) {
  const canvas = useRef<HTMLDivElement>(null);
  const flow = useRef<ReactFlowInstance<StageNode, FlowEdge> | null>(null);
  useEffect(() => {
    let frame = 0;
    const refit = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        void flow.current?.fitView({ padding: 0.06, duration: 0 });
      });
    };
    const observer = new ResizeObserver(refit);
    if (canvas.current) observer.observe(canvas.current);
    refit();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [item.id]);
  const processing = processingState(item, at),
    learning = learningState(item, at),
    run = item.learning;
  const memory =
    run?.state === "succeeded"
      ? run.claim_ids.length
        ? "Outcomes recorded"
        : "No memory changes"
      : "No outcome recorded";
  const nodes: StageNode[] = [
    {
      id: "origin",
      type: "stage",
      position: { x: 0, y: 118 },
      data: {
        kind: "origin",
        label: "Contributed by",
        value: item.contributor,
        detail: origin(item),
        live: false,
      },
    },
    {
      id: "input",
      type: "stage",
      position: { x: 265, y: 118 },
      data: {
        kind: "input",
        label: "Selected Brain",
        value: brain.name,
        detail: item.capture_id ? "Published capture" : "Source import",
        live: false,
      },
    },
    {
      id: "processing",
      type: "stage",
      position: { x: 555, y: 18 },
      data: {
        kind: "processing",
        label: "Source processing",
        value: stateLabel(processing),
        detail: item.source_version_id
          ? "Exact source version"
          : "No source to process",
        live: processing === "running",
      },
    },
    {
      id: "learning",
      type: "stage",
      position: { x: 805, y: 18 },
      data: {
        kind: "learning",
        label: "Recollect learning",
        value:
          learning === "failed"
            ? learningFailure(run?.job.error_code).label
            : stateLabel(learning),
        detail: run ? "Extract + reconcile" : "No learning run recorded",
        live: learning === "running",
      },
    },
    {
      id: "memory",
      type: "stage",
      position: { x: 805, y: 205 },
      data: {
        kind: "memory",
        label: "Memory",
        value: memory,
        detail:
          run?.state === "succeeded"
            ? `${run.accepted} accepted · ${run.proposed} proposed${run.conflicting ? ` (${run.conflicting} conflicting)` : ""}`
            : "Only recorded results appear",
        live: false,
      },
    },
  ];
  const edges: FlowEdge[] = [
    {
      id: "contribution",
      source: "origin",
      target: "input",
      type: "observed",
      data: { pulse: pulses.contribution },
    },
  ];
  if (item.source_version_id)
    edges.push({
      id: "process",
      source: "input",
      target: "processing",
      type: "observed",
      data: { pulse: pulses.process },
    });
  if (run)
    edges.push({
      id: "learn",
      source: "processing",
      target: "learning",
      type: "observed",
      data: { pulse: pulses.learn },
    });
  if (run?.state === "succeeded")
    edges.push({
      id: "outcome",
      source: "learning",
      target: "memory",
      type: "observed",
      data: { pulse: pulses.outcome },
    });
  return (
    <div
      ref={canvas}
      className="flow-canvas"
      aria-label="Recorded source processing path"
    >
      <ReactFlow
        onInit={(instance) => {
          flow.current = instance;
          void instance.fitView({ padding: 0.06, duration: 0 });
        }}
        key={brain.id}
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        fitView
        fitViewOptions={{ padding: 0.06 }}
        minZoom={0.4}
        maxZoom={1.1}
        nodesDraggable={false}
        nodesConnectable={false}
        edgesReconnectable={false}
        deleteKeyCode={null}
        panOnDrag={false}
        zoomOnScroll={false}
        zoomOnPinch={false}
        zoomOnDoubleClick={false}
        onNodeClick={onInspect}
        proOptions={{ hideAttribution: true }}
      >
        <Background color="var(--rc-border)" gap={19} size={0.7} />
      </ReactFlow>
    </div>
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
    const timer = setInterval(() => setNow(Date.now()), 500);
    return () => {
      clearTimeout(pulseTimer.current);
      clearInterval(timer);
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
    refetchInterval: visible ? 2000 : false,
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
  const shown =
    (compact || dashboard) && !expanded ? stable.rows.slice(0, 5) : stable.rows;
  const choose = (id: string) => {
    setSelected(id);
    setInspecting(false);
    setChanged(new Map());
  };
  return (
    <section
      className="control-pipeline"
      aria-label={`${brain.name} activity pipeline`}
      data-testid="live-pipeline"
      data-paused={!visible}
    >
      <div className="control-panel-heading">
        <div>
          <strong>Activity pipeline</strong>
          <span className="flow-mode">
            {!data
              ? "Unavailable"
              : active
                ? `${running} running · ${active - running} queued / waiting`
                : "Last recorded activity"}
          </span>
        </div>
        <Group gap="xs">
          <Text size="xs" c="dimmed">
            {data
              ? `Checked ${Math.max(0, Math.floor((now - query.dataUpdatedAt) / 1000))}s ago`
              : query.isPending
                ? "Checking activity"
                : "Updates paused"}
          </Text>
          <Button
            size="compact-xs"
            variant="subtle"
            aria-label="Refresh pipeline"
            onClick={() => void query.refetch()}
          >
            <RefreshCw size={15} />
          </Button>
          {compact && (
            <Button
              renderRoot={(props) => (
                <Link
                  {...props}
                  to="/brains/$brainId/ask"
                  params={{ brainId: brain.id }}
                  search={{}}
                />
              )}
              variant="default"
              size="xs"
              rightSection={<ArrowRight size={14} />}
            >
              Open Brain
            </Button>
          )}
        </Group>
      </div>
      {query.isPending ? (
        <LoadingState label="Loading recorded activity…" />
      ) : !data ? (
        <Alert
          m="md"
          color="gray"
          title={
            query.isError
              ? "Activity unavailable"
              : !visible
                ? "Updates paused"
                : "Activity needs a fresh check"
          }
        >
          {query.isError
            ? "The last snapshot has been cleared. Retry to read current activity."
            : "No stale processing state is shown."}
          <Button
            variant="subtle"
            size="xs"
            onClick={() => void query.refetch()}
          >
            Retry
          </Button>
        </Alert>
      ) : !item ? (
        <div className="flow-empty">
          <Network size={32} />
          <h3>No recorded activity yet</h3>
          <p>
            Published agent captures and source imports will appear here as they
            arrive.
          </p>
        </div>
      ) : (
        <>
          <div className="flow-input-heading">
            <FileInput size={17} />
            <div>
              <strong>{item.title}</strong>
              <span>
                {origin(item)} · {item.kind.replaceAll("_", " ")}
                {item.tool_name ? ` · ${item.tool_name}` : ""}
              </span>
            </div>
            <Button
              variant="subtle"
              size="xs"
              onClick={() => {
                setSelected(item.id);
                setInspecting(true);
              }}
            >
              Inspect input
            </Button>
          </div>
          <Diagram
            brain={brain}
            item={item}
            at={data.observed_at}
            pulses={!visible || reduced ? {} : (changed.get(item.id) ?? {})}
            onInspect={() => {
              setSelected(item.id);
              setInspecting(true);
            }}
          />
          <div className="flow-event-strip" aria-live="polite">
            <span
              className={`flow-state-dot ${itemState(item, data.observed_at) === "running" ? "running" : ""}`}
            />
            <span>
              {item.contributor} → {origin(item)} →{" "}
              {inputStatus(item, data.observed_at).label}
            </span>
            <time dateTime={item.activity_at}>
              {new Date(item.activity_at).toLocaleString()}
            </time>
          </div>
          {dashboard && item.learning?.state === "succeeded" && (
            <div className="dashboard-outcome-strip">
              <div>
                <span>Recorded outcome</span>
                <strong>
                  {item.learning.accepted} accepted · {item.learning.proposed}{" "}
                  proposed
                  {item.learning.conflicting
                    ? ` (${item.learning.conflicting} conflicting)`
                    : ""}
                </strong>
              </div>
              <Button
                size="xs"
                variant="default"
                onClick={() => {
                  setSelected(item.id);
                  setInspecting(true);
                }}
              >
                Inspect outcome
              </Button>
            </div>
          )}
          {stable.pending > 0 && (
            <Button variant="light" size="xs" m="sm" onClick={stable.reveal}>
              {stable.pending} new inputs · Show updates
            </Button>
          )}
          <div className="flow-history-heading">
            <strong>
              {active ? "In progress & recent inputs" : "Recent inputs"}
            </strong>
            <span>
              {identities.size} contributor{" "}
              {identities.size === 1 ? "context" : "contexts"} ·{" "}
              {data.has_more ? "Latest 30" : "Recorded window"}
            </span>
          </div>
          <div className="flow-input-list">
            {shown?.map((row) => (
              <motion.button
                type="button"
                layout={visible && !reduced}
                key={row.id}
                initial={{ opacity: 1 }}
                transition={{ duration: reduced ? 0 : 0.22 }}
                className={`flow-input-row ${row.id === item.id ? "selected" : ""} ${changed.has(row.id) ? "changed" : ""}`}
                onClick={() => choose(row.id)}
                aria-pressed={row.id === item.id}
                aria-description={
                  inputStatus(row, data.observed_at).detail ?? undefined
                }
                data-testid={`pipeline-input-${row.id}`}
              >
                <span className="flow-input-icon">
                  {row.device_id || row.host ? (
                    <Terminal size={17} />
                  ) : (
                    <UserRound size={17} />
                  )}
                </span>
                <span className="flow-input-copy">
                  <strong>{row.title}</strong>
                  <small>
                    {row.contributor} · {origin(row)}
                    {row.agent_id ? ` · Agent ${row.agent_id}` : ""}
                  </small>
                </span>
                <Tooltip
                  label={inputStatus(row, data.observed_at).detail}
                  disabled={!inputStatus(row, data.observed_at).detail}
                  multiline
                  w={300}
                  events={{ hover: true, focus: true, touch: true }}
                >
                  <span
                    className={`flow-state state-${itemState(row, data.observed_at)}`}
                  >
                    {inputStatus(row, data.observed_at).label}
                  </span>
                </Tooltip>
                <time dateTime={row.activity_at}>
                  {new Date(row.activity_at).toLocaleTimeString()}
                </time>
              </motion.button>
            ))}
          </div>
          {(compact || dashboard) && data.items.length > 5 && (
            <Button
              variant="subtle"
              size="xs"
              onClick={() => setExpanded(!expanded)}
            >
              {expanded
                ? "Show fewer inputs"
                : `Show ${data.items.length} inputs`}
            </Button>
          )}
          <div className="flow-graph-summary">
            <Network size={17} />
            <strong>Brain graph</strong>
            <span>
              {data.graph
                ? `${data.graph.state === "ready" && data.graph.input_epoch !== data.graph.current_epoch ? "Awaiting current projection" : data.graph.state === "ready" ? "Ready" : stateLabel(data.graph.state)} · ${data.graph.node_count} nodes · ${data.graph.edge_count} edges`
                : "No projection recorded"}
            </span>
            <small>Independent projection</small>
            <Link
              to="/brains/$brainId/graph"
              params={{ brainId: brain.id }}
              search={{}}
            >
              Explore graph
            </Link>
          </div>
        </>
      )}
      <div className="flow-footnote">
        Checks every 2s · Observed states only.
        {data?.has_more
          ? " More activity exists outside this bounded window."
          : ""}
      </div>
      <Drawer
        opened={inspecting && !!data && !!item && item.id === selected}
        onClose={() => setInspecting(false)}
        title="Processing record"
        position="right"
        size="lg"
        className="control-drawer"
      >
        {data && item && item.id === selected && (
          <Stack gap="lg">
            <Text fw={600}>{item.title}</Text>
            <div className="provenance-path">
              <UserRound size={18} />
              <span>{item.contributor}</span>
              <ArrowRight size={14} />
              <Terminal size={18} />
              <span>{origin(item)}</span>
            </div>
            <Text size="sm">
              Received {new Date(item.received_at).toLocaleString()} ·{" "}
              {item.kind.replaceAll("_", " ")}
            </Text>
            {item.agent_id && (
              <Text size="sm">Reported agent: {item.agent_id}</Text>
            )}
            {!!item.coverage.length && (
              <Alert color="yellow" title="Capture coverage">
                {item.coverage.map((c) => c.replaceAll("_", " ")).join(" · ")}
              </Alert>
            )}
            {item.source_id && item.source_version_id ? (
              <Button
                renderRoot={(props) => (
                  <Link
                    {...props}
                    to="/brains/$brainId/sources"
                    params={{ brainId: brain.id }}
                    search={{
                      source: item.source_id!,
                      version: item.source_version_id!,
                    }}
                  />
                )}
                variant="light"
                leftSection={<FileInput size={16} />}
              >
                Read exact source version
              </Button>
            ) : (
              <Alert color="gray">
                This capture has no retained source. No learning path is
                inferred.
              </Alert>
            )}
            <div className="control-detail-section">
              <h3>Source processing</h3>
              <Text size="sm">
                {stateLabel(processingState(item, data.observed_at))}
              </Text>
              {item.processing_job?.error_code && (
                <Text size="sm" c="red">
                  {item.processing_job.error_code}
                </Text>
              )}
            </div>
            <div className="control-detail-section">
              <h3>Recollect learning</h3>
              <Text size="sm">
                {learningState(item, data.observed_at) === "failed"
                  ? learningFailure(item.learning?.job.error_code).label
                  : stateLabel(learningState(item, data.observed_at))}
              </Text>
              <Text size="xs" c="dimmed">
                Background processing is separate from the contributing agent.
              </Text>
              {item.learning && (
                <>
                  <Text size="sm">
                    Recorded{" "}
                    {new Date(item.learning.created_at).toLocaleString()}
                    {item.learning.finished_at
                      ? ` · Finished ${new Date(item.learning.finished_at).toLocaleString()}`
                      : ""}
                  </Text>
                  {item.learning.job.error_code && (
                    <Alert
                      color="red"
                      title={
                        learningFailure(item.learning.job.error_code).label
                      }
                    >
                      {learningFailure(item.learning.job.error_code).detail}
                      <Text size="xs" mt="xs">
                        Reason: {item.learning.job.error_code}
                      </Text>
                    </Alert>
                  )}
                  {item.learning.state === "succeeded" && (
                    <>
                      <div className="flow-outcomes">
                        {(
                          [
                            "accepted",
                            "proposed",
                            "reused",
                            "revised",
                            "retired",
                            "blocked",
                            "conflicting",
                          ] as const
                        ).map((key) => (
                          <div key={key}>
                            <strong>{item.learning![key]}</strong>
                            <small>{key}</small>
                          </div>
                        ))}
                      </div>
                      <Text size="xs" c="dimmed">
                        Recorded dispositions can overlap. These are this run’s
                        results, not the current memory inventory.
                      </Text>
                      {!item.learning.claim_ids.length && (
                        <Text size="sm">Completed without memory changes.</Text>
                      )}
                    </>
                  )}
                  <Button
                    renderRoot={(props) => (
                      <Link
                        {...props}
                        to="/brains/$brainId/activity"
                        params={{ brainId: brain.id }}
                        search={{
                          tab: "processing",
                          job: item.learning!.job.id,
                        }}
                      />
                    )}
                    variant="subtle"
                  >
                    Open learning job
                  </Button>
                  {item.learning.claim_ids.slice(0, 5).map((id) => (
                    <Link
                      key={id}
                      to="/brains/$brainId/memory"
                      params={{ brainId: brain.id }}
                      search={{ claim: id }}
                    >
                      Open current memory · {id.slice(0, 8)}
                    </Link>
                  ))}
                </>
              )}
            </div>
          </Stack>
        )}
      </Drawer>
    </section>
  );
}
