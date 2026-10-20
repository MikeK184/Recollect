import { memo, useEffect, useMemo, useRef } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import {
  Handle,
  Position,
  ReactFlow,
  getStraightPath,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeProps,
  type ReactFlowInstance,
} from "@xyflow/react";
import {
  Boxes,
  CircleAlert,
  CircleCheck,
  CircleDashed,
  Clock3,
  FileCog,
  FileInput,
  LoaderCircle,
  NotebookTabs,
  Sparkles,
  UserRound,
} from "lucide-react";
import "@xyflow/react/dist/style.css";
import type { Brain } from "../../api";
import { Tile } from "../../components/ui";
import { arrive, leave, settle, staggerDelay } from "../../design/motion";
import { cn } from "../../lib/utils";
import {
  learningFailure,
  learningState,
  origin,
  processingState,
  stageTone,
  stateLabel,
  type FlowStage,
  type PipelineItem,
  type StageTone,
} from "./pipelineModel";

// The recorded processing path for one input, drawn by React Flow as five
// stages in a row (motion.md "Workflows and pipelines"): stages never move,
// only their state transitions; done edges are brand at half opacity, the edge
// feeding a running stage flows, and one packet travels an edge per recorded
// event. Nothing loops unless the server reports running work.

type StageKind = "origin" | "input" | "processing" | "learning" | "memory";
type StageData = {
  kind: StageKind;
  label: string;
  value: string;
  detail: string;
  tone: StageTone;
  index: number;
};
type StageNode = Node<StageData, "stage">;
type EdgeTone = "done" | "active" | "rest";
type FlowEdge = Edge<{ tone: EdgeTone; pulse?: number }, "observed">;

const STAGE_WIDTH = 196;
const STAGE_GAP = 32;
const stageIcon = {
  origin: UserRound,
  input: Boxes,
  processing: FileCog,
  learning: Sparkles,
  memory: NotebookTabs,
} as const;

function ToneIcon({ tone }: { tone: StageTone }) {
  const base = "size-4 shrink-0";
  if (tone === "running")
    return (
      <LoaderCircle
        aria-hidden="true"
        className={cn(
          base,
          "animate-spin text-brand-text group-data-[paused=true]/pipeline:[animation-play-state:paused]",
        )}
      />
    );
  if (tone === "waiting")
    return <Clock3 aria-hidden="true" className={cn(base, "text-faint-foreground")} />;
  if (tone === "failed")
    return <CircleAlert aria-hidden="true" className={cn(base, "text-destructive-text")} />;
  if (tone === "done")
    return <CircleCheck aria-hidden="true" className={cn(base, "text-brand-text")} />;
  return <CircleDashed aria-hidden="true" className={cn(base, "text-faint-foreground")} />;
}

const handleClass =
  "!size-px !min-h-0 !min-w-0 !border-0 !bg-transparent !opacity-0 !pointer-events-none";

const Stage = memo(({ data }: NodeProps<StageNode>) => {
  const reduced = useReducedMotion();
  const Icon = stageIcon[data.kind] ?? FileInput;
  return (
    <motion.div
      initial={reduced ? false : { opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ ...settle, delay: staggerDelay(data.index) }}
      data-stage={data.kind}
      data-state={data.tone}
      className={cn(
        "relative flex h-[124px] flex-col gap-2 rounded-lg border bg-card p-3 text-left shadow-xs transition-[border-color,box-shadow,opacity] duration-fast ease-standard",
        data.tone === "running" && "border-brand shadow-glow",
        data.tone === "waiting" && "border-dashed border-border-strong shadow-none opacity-(--opacity-waiting)",
        data.tone === "failed" && "border-destructive",
        (data.tone === "done" || data.tone === "idle") && "border-border",
      )}
      style={{ width: STAGE_WIDTH }}
    >
      <Handle
        type="target"
        position={Position.Left}
        isConnectable={false}
        className={handleClass}
      />
      <div className="flex items-center gap-2">
        <Tile
          size="sm"
          tone={
            data.tone === "failed"
              ? "destructive"
              : data.tone === "idle"
                ? "muted"
                : "brand"
          }
        >
          <Icon aria-hidden="true" />
        </Tile>
        <span className="section-label truncate">{data.label}</span>
        <span className="ml-auto flex">
          <ToneIcon tone={data.tone} />
        </span>
      </div>
      <div className="relative min-h-0">
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.p
            key={data.value}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1, transition: arrive }}
            exit={{ opacity: 0, transition: leave }}
            className="m-0 line-clamp-2 text-[15px] font-semibold leading-[22px] tracking-[-0.011em] text-foreground [overflow-wrap:anywhere]"
          >
            {data.value}
          </motion.p>
        </AnimatePresence>
      </div>
      <p className="-mt-1.5 m-0 truncate text-caption text-muted-foreground" title={data.detail}>
        {data.detail}
      </p>
      <Handle
        type="source"
        position={Position.Right}
        isConnectable={false}
        className={handleClass}
      />
    </motion.div>
  );
});

function ObservedEdge(props: EdgeProps<FlowEdge>) {
  const [path] = getStraightPath(props);
  const tone = props.data?.tone ?? "rest";
  return (
    <>
      <path
        d={path}
        fill="none"
        className={cn(
          "[stroke-width:1.5] transition-[stroke] duration-fast",
          tone === "done" && "stroke-brand/50",
          tone === "rest" && "stroke-border-strong",
          tone === "active" &&
            "stroke-brand [stroke-dasharray:4_6] animate-flow-dash group-data-[paused=true]/pipeline:[animation-play-state:paused]",
        )}
      />
      {props.data?.pulse ? (
        <motion.path
          key={props.data.pulse}
          d={path}
          fill="none"
          pathLength={1}
          data-testid="pipeline-pulse"
          className="stroke-brand [stroke-width:6] [stroke-linecap:round] [stroke-dasharray:0_2] [filter:drop-shadow(0_0_4px_var(--brand))]"
          initial={{ strokeDashoffset: 0, opacity: 0 }}
          animate={{ strokeDashoffset: -1, opacity: [0, 1, 1, 0] }}
          transition={{
            duration: 1.1,
            ease: [0.4, 0, 0.2, 1],
            opacity: { duration: 1.1, times: [0, 0.12, 0.85, 1] },
          }}
        />
      ) : null}
    </>
  );
}

const nodeTypes = { stage: Stage };
const edgeTypes = { observed: ObservedEdge };
const fitOptions = { padding: 0.04, maxZoom: 1 };

function edgeTone(target: StageTone): EdgeTone {
  if (target === "running") return "active";
  if (target === "done") return "done";
  return "rest";
}

export function PipelineFlow({
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
  // Fit once per input and on resize (the inspector docking narrows the
  // panel); the canvas never pans on its own.
  useEffect(() => {
    let frame = 0;
    const refit = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        void flow.current?.fitView({ ...fitOptions, duration: 0 });
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
  const { nodes, edges } = useMemo(() => {
    const processingTone: StageTone = item.source_version_id
      ? stageTone(processing)
      : "idle";
    const learningTone = stageTone(learning);
    const memoryTone: StageTone =
      run?.state === "succeeded"
        ? "done"
        : learningTone === "running" || learningTone === "waiting"
          ? "waiting"
          : "idle";
    const stages: Omit<StageData, "index">[] = [
      {
        kind: "origin",
        label: "Contributed",
        value: item.contributor,
        detail: origin(item),
        tone: "done",
      },
      {
        kind: "input",
        label: "Brain",
        value: brain.name,
        detail: item.capture_id ? "Published capture" : "Source import",
        tone: "done",
      },
      {
        kind: "processing",
        label: "Processing",
        value: stateLabel(processing),
        detail: item.source_version_id
          ? "Exact source version"
          : "No source to process",
        tone: processingTone,
      },
      {
        kind: "learning",
        label: "Learning",
        value:
          learning === "failed"
            ? learningFailure(run?.job.error_code).label
            : stateLabel(learning),
        detail: run ? "Extract and reconcile" : "No learning run recorded",
        tone: learningTone,
      },
      {
        kind: "memory",
        label: "Memory",
        value:
          run?.state === "succeeded"
            ? run.claim_ids.length
              ? "Outcomes recorded"
              : "No memory changes"
            : "No outcome recorded",
        detail:
          run?.state === "succeeded"
            ? `${run.accepted} accepted · ${run.proposed} proposed${run.conflicting ? ` (${run.conflicting} conflicting)` : ""}`
            : "Only recorded results appear",
        tone: memoryTone,
      },
    ];
    const nodes: StageNode[] = stages.map((data, index) => ({
      id: data.kind,
      type: "stage",
      position: { x: index * (STAGE_WIDTH + STAGE_GAP), y: 0 },
      data: { ...data, index },
      width: STAGE_WIDTH,
      height: 124,
    }));
    const edges: FlowEdge[] = [
      {
        id: "contribution",
        source: "origin",
        target: "input",
        type: "observed",
        data: { tone: "done", pulse: pulses.contribution },
      },
    ];
    // Only recorded relationships are drawn; nothing infers a path.
    if (item.source_version_id)
      edges.push({
        id: "process",
        source: "input",
        target: "processing",
        type: "observed",
        data: { tone: edgeTone(processingTone), pulse: pulses.process },
      });
    if (run)
      edges.push({
        id: "learn",
        source: "processing",
        target: "learning",
        type: "observed",
        data: { tone: edgeTone(learningTone), pulse: pulses.learn },
      });
    if (run?.state === "succeeded")
      edges.push({
        id: "outcome",
        source: "learning",
        target: "memory",
        type: "observed",
        data: { tone: "done", pulse: pulses.outcome },
      });
    return { nodes, edges };
  }, [brain.name, item, processing, learning, run, pulses]);
  return (
    <div
      ref={canvas}
      className="flow-canvas relative h-[208px] w-full dot-grid"
      aria-label="Recorded source processing path"
    >
      <ReactFlow
        onInit={(instance) => {
          flow.current = instance;
          void instance.fitView({ ...fitOptions, duration: 0 });
        }}
        key={brain.id}
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        fitView
        fitViewOptions={fitOptions}
        minZoom={0.4}
        maxZoom={1}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={false}
        edgesReconnectable={false}
        deleteKeyCode={null}
        panOnDrag={false}
        panOnScroll={false}
        zoomOnScroll={false}
        zoomOnPinch={false}
        zoomOnDoubleClick={false}
        preventScrolling={false}
        onNodeClick={onInspect}
        proOptions={{ hideAttribution: true }}
        className="[&_.react-flow__node]:cursor-pointer"
      />
    </div>
  );
}
