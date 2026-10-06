import { createContext, useContext, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useRouterState } from "@tanstack/react-router";
import { ArrowUpRight, Check, CircleDashed, TriangleAlert } from "lucide-react";
import { Button } from "@mantine/core";
import { client, result } from "../../api";
import { useBrain } from "../../app/context";
import type { BrainSearch } from "../../app/useBrainSearch";
import "./assurance.css";

type Exception = {
  id: string;
  blocking: boolean;
  title: string;
  detail: string;
  section: "activity" | "settings" | "agents";
  search: BrainSearch;
};

function useAssuranceReads(active: boolean) {
  const brain = useBrain();
  const admin = brain.role === "admin";
  // Separate role/revision positions, no payload cache across a changed Brain.
  // Each read returns only the metadata this surface needs. Private tool output,
  // captured content and working scopes never enter the assurance cache.
  const base = {
    gcTime: 0,
    staleTime: 5000,
    enabled: active,
    refetchInterval: 10_000,
    retry: false,
  };
  const key = (name: string) => [
    "assurance",
    brain.id,
    brain.role,
    brain.updated_at,
    name,
  ];
  const path = { brain: brain.id };
  const automation = useQuery({
    ...base,
    queryKey: key("automation"),
    queryFn: async ({ signal }) => {
      const data = result(
        await client.GET("/api/brains/{brain}/automation", {
          params: { path },
          signal,
        }),
      );
      return {
        enabled:
          data.models.current.policy.enabled &&
          data.models.current.policy.autonomous_memory,
        credentials: data.models.installed.credentials_present,
      };
    },
  });
  const learning = useQuery({
    ...base,
    queryKey: key("learning"),
    queryFn: async ({ signal }) => {
      const data = result(
        await client.GET("/api/brains/{brain}/learning", {
          params: { path },
          signal,
        }),
      );
      const latest = data.items.find((run) => run.state === "succeeded");
      return {
        total: data.total,
        latest: latest
          ? {
              accepted: latest.accepted,
              revised: latest.revised,
              retired: latest.retired,
              at: latest.finished_at ?? latest.created_at,
            }
          : undefined,
      };
    },
  });
  const processing = useQuery({
    ...base,
    queryKey: key("processing"),
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{id}/processing", {
          params: { path: { id: brain.id } },
          signal,
        }),
      ),
  });
  const jobs = useQuery({
    ...base,
    queryKey: key("jobs"),
    queryFn: async ({ signal }) => {
      const data = result(
        await client.GET("/api/brains/{id}/jobs", {
          params: { path: { id: brain.id } },
          signal,
        }),
      );
      return data
        .filter((job) => job.state === "failed")
        .map((job) => ({ id: job.id, at: job.updated_at }));
    },
  });
  const captured = useQuery({
    ...base,
    queryKey: key("captured"),
    queryFn: async ({ signal }) => {
      const data = result(
        await client.GET("/api/brains/{brain}/capture/events", {
          params: { path },
          signal,
        }),
      );
      return { total: data.total };
    },
  });
  const devices = useQuery({
    ...base,
    queryKey: key("devices"),
    enabled: active && admin,
    queryFn: async ({ signal }) => {
      const data = result(
        await client.GET("/api/brains/{brain}/capture/devices", {
          params: { path },
          signal,
        }),
      );
      return data.items
        .filter(
          (device) =>
            device.report &&
            (device.report.device_gap_count > 0 ||
              device.report.denied > 0 ||
              device.report.issue),
        )
        .map((device) => ({
          id: device.device_id,
          gaps: device.report!.device_gap_count,
        }));
    },
  });
  const calls = useQuery({
    ...base,
    queryKey: key("calls"),
    enabled: active && admin,
    queryFn: async ({ signal }) => {
      const data = result(
        await client.GET("/api/brains/{brain}/mcp/calls", {
          params: { path },
          signal,
        }),
      );
      return data.calls
        .filter(
          (call) =>
            ["unknown", "failed", "tool_error"].includes(call.state) &&
            call.resolutions.length === 0,
        )
        .map((call) => ({ id: call.id, state: call.state }));
    },
  });
  const erasures = useQuery({
    ...base,
    queryKey: key("erasures"),
    enabled: active && admin,
    queryFn: async ({ signal }) => {
      const data = result(
        await client.GET("/api/brains/{brain}/erasures", {
          params: { path },
          signal,
        }),
      );
      return data.items
        .filter((item) => item.state !== "complete")
        .map((item) => ({
          id: item.id,
          state: item.state,
          journaled: item.journaled,
        }));
    },
  });
  const feeds = [
    { name: "Automation", query: automation },
    { name: "Learning", query: learning },
    { name: "Processing", query: processing },
    { name: "Jobs", query: jobs },
    { name: "Capture", query: captured },
    ...(admin
      ? [
          { name: "Capture coverage", query: devices },
          { name: "Tool calls", query: calls },
          { name: "Data removal", query: erasures },
        ]
      : []),
  ];
  const exceptions: Exception[] = [];
  if (
    !brain.archived &&
    !automation.isError &&
    automation.data?.enabled &&
    !automation.data.credentials
  )
    exceptions.push({
      id: "provider",
      blocking: true,
      title: "Learning needs a provider credential",
      detail:
        "Capture and evidence search remain available. An administrator can check the installation.",
      section: "settings",
      search: { tab: "ai" },
    });
  if (!processing.isError && processing.data?.state === "failed")
    exceptions.push({
      id: "processing",
      blocking: true,
      title: "Brain processing is blocked",
      detail:
        "Processing reports a failure with no queued work. Inspect processing for the cause.",
      section: "activity",
      search: { tab: "processing" },
    });
  if (!jobs.isError)
    for (const job of jobs.data ?? [])
      exceptions.push({
        id: job.id,
        blocking: false,
        title: "Processing did not finish",
        detail: `Recorded failure${job.at ? ` · ${new Date(job.at).toLocaleString()}` : ""}. This record alone does not mean current processing is blocked.`,
        section: "activity",
        search: { tab: "processing", job: job.id },
      });
  if (admin && !calls.isError)
    for (const call of calls.data ?? [])
      exceptions.push({
        id: call.id,
        blocking: call.state === "unknown",
        title:
          call.state === "unknown"
            ? "A tool outcome is uncertain"
            : "A tool call did not succeed",
        detail: "Inspect the original call before deciding what to do next.",
        section: "activity",
        search: { tab: "tools", call: call.id },
      });
  if (admin && !erasures.isError)
    for (const item of erasures.data ?? [])
      exceptions.push({
        id: item.id,
        blocking: item.state === "error",
        title:
          item.state === "error"
            ? "Data cleanup needs attention"
            : "Data cleanup is pending",
        detail: item.journaled
          ? "Content is unavailable while central cleanup finishes. Offline copies require check-in."
          : "The deletion journal is still pending. Cleanup is not complete.",
        section: "activity",
        search: { tab: "removal", erasure: item.id },
      });
  if (admin && !devices.isError)
    for (const device of devices.data ?? [])
      exceptions.push({
        id: device.id,
        blocking: false,
        title: "A device reported a capture gap",
        detail:
          device.gaps > 0
            ? `${device.gaps} gaps in its latest report. Offline coverage cannot be confirmed.`
            : "The latest capture report needs attention. Inspect coverage for the details.",
        section: "agents",
        search: { tab: "sessions", device: device.id },
      });
  return {
    brain,
    exceptions,
    blockers: exceptions.filter((item) => item.blocking),
    history: exceptions.filter((item) => !item.blocking),
    feeds,
    pending: feeds.some((feed) => feed.query.isPending),
    failed: feeds.filter((feed) => feed.query.isError),
    enabled: !automation.isError && automation.data?.enabled,
    latest: learning.isError ? undefined : learning.data?.latest,
    captured: captured.isError ? undefined : captured.data?.total,
    pendingJobs: processing.isError ? undefined : processing.data?.pending_jobs,
    empty:
      !learning.isError &&
      learning.data?.total === 0 &&
      !captured.isError &&
      captured.data?.total === 0,
  };
}

const AssuranceContext = createContext<ReturnType<
  typeof useAssuranceReads
> | null>(null);
function useAssurance() {
  const value = useContext(AssuranceContext);
  if (!value) throw new Error("Assurance is outside its Brain");
  return value;
}
export function AssuranceProvider({ children }: { children: ReactNode }) {
  const active = useRouterState({
    select: (state) =>
      state.location.pathname.endsWith("/dashboard") ||
      (state.location.pathname.endsWith("/activity") &&
        (state.location.search as BrainSearch).tab === "attention"),
  });
  const value = useAssuranceReads(active);
  return (
    <AssuranceContext.Provider value={value}>
      {active && <AssuranceBand />}
      {children}
    </AssuranceContext.Provider>
  );
}

function AssuranceBand() {
  const data = useAssurance();
  const first = data.blockers[0];
  const attention = !!first;
  const waiting = data.pending || data.failed.length > 0;
  // The band exists to surface attention. A healthy Brain shows no band at
  // all: "nothing needs you" is the absence of the strip, not its content.
  if (
    !attention &&
    !waiting &&
    !data.brain.archived &&
    data.enabled &&
    !data.empty
  ) {
    return null;
  }
  const title = attention
    ? first.title
    : data.pending
      ? "Checking recent activity"
      : data.failed.length
        ? "Some activity is unavailable"
        : data.brain.archived
          ? "Memory preserved · Brain archived"
          : !data.enabled
            ? "Autonomous memory is not enabled"
            : data.empty
              ? "Ready for your first session"
              : "Nothing needs you";
  const Icon = attention
    ? TriangleAlert
    : waiting || !data.enabled
      ? CircleDashed
      : Check;
  return (
    <section
      className={`assurance-band${attention ? " assurance-attention" : ""}`}
      aria-label="Brain assurance"
    >
      <Icon size={18} aria-hidden />
      <div className="assurance-position">
        <strong>{title}</strong>
        <span>
          {first ? (
            <>
              {first.detail}
              {data.blockers.length > 1 &&
                ` · ${data.blockers.length - 1} other current blocker${data.blockers.length > 2 ? "s" : ""}`}
            </>
          ) : data.latest ? (
            <>
              Latest completed learning run · {data.latest.accepted} learned
              {data.latest.revised !== undefined && (
                <> · {data.latest.revised} revised</>
              )}
              {data.latest.retired !== undefined && (
                <> · {data.latest.retired} retired</>
              )}
            </>
          ) : data.pending ? (
            "Reading permitted activity…"
          ) : data.failed.length ? (
            "Unavailable figures are omitted."
          ) : !data.enabled ? (
            "An administrator can enable autonomous memory in Settings."
          ) : data.empty ? (
            "Connect an agent. Recollect handles the memory work."
          ) : (
            "Recent authorized activity; no individual memory approvals."
          )}
        </span>
      </div>
      {!attention && (
        <div className="assurance-counts">
          {data.captured !== undefined && (
            <span>
              <strong>{data.captured}</strong> captured events{" "}
              <small>Brain total</small>
            </span>
          )}
          {!!data.pendingJobs && <span>{data.pendingJobs} jobs pending</span>}
        </div>
      )}
      <Link
        to={
          first && data.blockers.length === 1
            ? `/brains/$brainId/${first.section}`
            : "/brains/$brainId/activity"
        }
        params={{ brainId: data.brain.id }}
        search={
          first && data.blockers.length === 1
            ? first.search
            : { tab: "attention" }
        }
        className="assurance-link"
      >
        {first && data.blockers.length === 1
          ? "Inspect problem"
          : "View activity"}{" "}
        <ArrowUpRight size={15} aria-hidden />
      </Link>
    </section>
  );
}

export function AssuranceActivity() {
  const data = useAssurance();
  return (
    <section
      className="assurance-activity"
      aria-label="Attention and autonomous activity"
    >
      <div className="assurance-heading">
        <h2>
          {data.blockers.length ? "Current blockers" : "Your memory, at work"}
        </h2>
        <p>
          {data.enabled
            ? "Recent authorized activity. Recollect learns, revises and retires memory automatically."
            : "Recent authorized activity. Autonomous memory can be enabled in Settings."}
        </p>
      </div>
      {data.failed.length > 0 && (
        <div className="assurance-incomplete" role="status">
          <strong>Some activity could not be checked</strong>
          <p>
            {data.failed.map((feed) => feed.name).join(", ")} unavailable. This
            is not a successful health check.
          </p>
          <Button
            variant="subtle"
            size="xs"
            onClick={() => {
              for (const feed of data.failed) void feed.query.refetch();
            }}
          >
            Retry unavailable activity
          </Button>
        </div>
      )}
      {data.pending && <p role="status">Checking recent activity…</p>}
      {data.blockers.map((item) => (
        <Link
          key={item.id}
          className="assurance-exception"
          to={`/brains/$brainId/${item.section}`}
          params={{ brainId: data.brain.id }}
          search={item.search}
        >
          <TriangleAlert size={18} aria-hidden />
          <div>
            <strong>{item.title}</strong>
            <p>{item.detail}</p>
          </div>
          <ArrowUpRight size={18} aria-hidden />
        </Link>
      ))}
      {!data.pending && !data.failed.length && !data.blockers.length && (
        <div className="assurance-quiet">
          <Check size={24} aria-hidden />
          <div>
            <h3>
              {data.empty
                ? "No activity yet"
                : data.enabled
                  ? "No action required"
                  : "Connect once, then let memory grow"}
            </h3>
            <p>
              {data.enabled
                ? "Nothing in the recent activity checked needs you. You can inspect or correct memory whenever you choose."
                : "An administrator can enable autonomous memory once in Settings. Individual memories do not need approval."}
            </p>
            <Link
              to={
                data.enabled
                  ? "/brains/$brainId/agents"
                  : "/brains/$brainId/settings"
              }
              params={{ brainId: data.brain.id }}
              search={data.enabled ? { tab: "setup" } : { tab: "ai" }}
            >
              {data.enabled ? "Connect an agent" : "Autonomous memory settings"}
            </Link>
          </div>
        </div>
      )}
      {data.history.length > 0 && (
        <section aria-label="Activity history and pending work">
          <h3>History and pending work</h3>
          <p className="assurance-footnote">
            These records do not establish a current blocker. Past failures
            remain available for inspection.
          </p>
          {data.history.map((item) => (
            <Link
              key={item.id}
              className="assurance-exception"
              to={`/brains/$brainId/${item.section}`}
              params={{ brainId: data.brain.id }}
              search={item.search}
            >
              <CircleDashed size={18} aria-hidden />
              <div>
                <strong>{item.title}</strong>
                <p>{item.detail}</p>
              </div>
              <ArrowUpRight size={18} aria-hidden />
            </Link>
          ))}
        </section>
      )}
      {data.latest && (
        <p className="assurance-footnote">
          Learning figures describe one completed run, recorded{" "}
          {new Date(data.latest.at).toLocaleString()}. They are not lifetime
          totals.
        </p>
      )}
      <p className="assurance-footnote">
        Checks cover the latest 100 jobs, 20 learning runs and permitted
        capture, tool and removal pages. Independent events may overlap. Offline
        device coverage and external copies cannot be confirmed here.
      </p>
    </section>
  );
}
