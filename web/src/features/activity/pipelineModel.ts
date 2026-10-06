import type { components } from "../../api-schema";
export type PipelineItem = components["schemas"]["PipelineItem"];
export type PipelineFeed = components["schemas"]["PipelineFeed"];
export type PipelineJob = components["schemas"]["PipelineJob"];

export function jobState(job: PipelineJob | null | undefined, at: string) {
  if (!job) return "not_recorded";
  if (
    job.state === "running" &&
    (!job.lease_until || Date.parse(job.lease_until) <= Date.parse(at))
  )
    return "waiting";
  return job.state;
}
export function processingState(item: PipelineItem, at: string) {
  return item.processing_job
    ? jobState(item.processing_job, at)
    : item.processing;
}
export function learningState(item: PipelineItem, at: string) {
  if (!item.learning) return "not_recorded";
  return item.learning.state === "running"
    ? jobState(item.learning.job, at)
    : item.learning.state;
}
export function itemState(item: PipelineItem, at: string) {
  const processing = processingState(item, at),
    learning = learningState(item, at);
  // A current retry takes priority over a previous terminal learning outcome.
  for (const state of ["running", "queued", "waiting", "failed", "cancelled"]) {
    if (processing === state || learning === state) return state;
  }
  if (learning === "succeeded") return "completed";
  return item.source_version_id
    ? processing === "ready" || processing === "succeeded"
      ? item.processing === "reference_only"
        ? "reference_only"
        : "ready"
      : processing === "reference_only"
        ? "reference_only"
        : "received"
    : "capture_only";
}
export const hostName = (host?: string | null) =>
  ({
    codex: "Codex",
    opencode: "OpenCode",
    claude_code: "Claude Code",
    managed_mcp: "Managed MCP",
  })[host ?? ""] ?? host;
export function origin(item: PipelineItem) {
  return (
    item.agent_name ||
    hostName(item.host) ||
    (item.device_id ? "Agent · host unreported" : "Manual source")
  );
}
export function originKey(item: PipelineItem) {
  return [
    item.actor_id,
    item.device_id,
    item.binding_id,
    item.host_session_id,
    item.agent_id,
  ].join(":");
}
export const stateLabel = (value: string) =>
  ({
    running: "Running",
    queued: "Queued",
    waiting: "Waiting for worker",
    failed: "Failed",
    cancelled: "Cancelled",
    succeeded: "Completed",
    completed: "Completed",
    ready: "Processed",
    reference_only: "Reference only",
    missing: "Unavailable",
    received: "Source received",
    capture_only: "Capture only",
    no_source: "No retained source",
    not_recorded: "Not recorded",
  })[value] ?? value.replaceAll("_", " ");

export type FlowStage = "contribution" | "process" | "learn" | "outcome";
/** Pulses report a newly observed change on its own edge, never replay a path. */
export function observedStages(
  previous: PipelineFeed | null,
  next: PipelineFeed,
): Map<string, FlowStage[]> {
  const changes = new Map<string, FlowStage[]>();
  if (!previous || previous.brain_id !== next.brain_id) return changes;
  const old = new Map(previous.items.map((item) => [item.id, item]));
  for (const item of next.items) {
    const before = old.get(item.id),
      stages: FlowStage[] = [];
    if (!before) {
      if (Date.parse(item.received_at) > Date.parse(previous.observed_at))
        stages.push("contribution");
    } else {
      if (
        JSON.stringify([
          before.processing_job?.id,
          before.processing_job?.state,
        ]) !==
        JSON.stringify([item.processing_job?.id, item.processing_job?.state])
      )
        stages.push("process");
      if (
        JSON.stringify([
          before.learning?.id,
          before.learning?.state,
          before.learning?.job.state,
        ]) !==
        JSON.stringify([
          item.learning?.id,
          item.learning?.state,
          item.learning?.job.state,
        ])
      ) {
        stages.push("learn");
        if (item.learning?.state === "succeeded") stages.push("outcome");
      }
    }
    if (stages.length) changes.set(item.id, stages);
  }
  return changes;
}
