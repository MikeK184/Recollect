import { useEffect, useState, type ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, Inbox, ScanLine } from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { PolicyRow } from "./features/settings/PolicyRow";
import { useIdempotency } from "./useIdempotency";
import { EraseAction } from "./RetentionPanel";
import { Markdown } from "./components/Markdown";
import { EmptyState } from "./components/AsyncState";
import { SkeletonRows } from "./components/Skeleton";
import {
  Alert,
  Badge,
  Button,
  CardBar,
  Code,
  Drawer,
  Modal,
  NumberInput,
  Select,
  Spinner,
  StatusBadge,
  Switch,
  Textarea,
} from "./components/ui";
import { cn } from "./lib/utils";
import { useContentDeadline } from "./useContentDeadline";
import { usePollInterval } from "./usePollInterval";
import { useFirstLoadStagger } from "./features/activity/firstLoad";

type Settings = components["schemas"]["CaptureSettings"];
type Event = components["schemas"]["CaptureEventView"];
type Device = components["schemas"]["CaptureDeviceView"];
const labels: Record<string, string> = {
  prompt: "Questions",
  reply: "Replies",
  tool_result: "Tool result",
  lifecycle: "Session events",
  codex: "Codex",
  claude_code: "Claude Code",
  opencode: "OpenCode",
  managed_mcp: "Managed tool",
  ManagedTool: "Tool outcome",
  ManagedReceipt: "Later receipt",
  ManagedResolution: "Evidence resolution",
};
const label = (value: string) => labels[value] ?? value.replaceAll("_", " ");
const time = (value: string) => new Date(value).toLocaleString();
const clock = (value: string) =>
  new Date(value).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });
const capturedAt = (item: Event) =>
  item.event?.captured_at ?? item.receipt.received_at;
const eventTitle = (item: Event) =>
  item.event?.tool_name ||
  (item.event ? label(item.event.kind) : "Removed event");

/** Bounded captured events grouped by their local capture date, in feed order. */
function byDay(items: Event[]) {
  const days = new Map<string, Event[]>();
  for (const item of items) {
    const day = new Date(capturedAt(item)).toLocaleDateString(undefined, {
      day: "numeric",
      month: "short",
      year: "numeric",
    });
    days.set(day, [...(days.get(day) ?? []), item]);
  }
  return [...days];
}

function CompactCaptureTimeline({
  items,
  openEvidence,
}: {
  items: Event[];
  openEvidence: (event: Event) => void;
}) {
  return (
    <div className="grid gap-4">
      {byDay(items).map(([day, events]) => (
        <section className="grid gap-1" key={day} aria-label={day}>
          <h4 className="m-0 px-1 text-caption font-medium text-muted-foreground">
            {day}
          </h4>
          <div className="overflow-hidden rounded-lg border border-border bg-card">
            {events.map((item) => {
              const captured = capturedAt(item);
              return (
                <details
                  className="group border-b border-border last:border-b-0"
                  key={item.receipt.event_id}
                  data-testid="agent-activity-event"
                >
                  <summary className="flex cursor-pointer list-none items-center gap-2.5 px-3 py-2 text-body transition-colors duration-fast hover:bg-accent [&::-webkit-details-marker]:hidden">
                    <strong className="min-w-0 flex-1 truncate font-medium text-foreground">
                      {eventTitle(item)}
                    </strong>
                    <time
                      className="mono text-caption text-muted-foreground"
                      dateTime={captured}
                      title={time(captured)}
                    >
                      {clock(captured)}
                    </time>
                    <span className="text-caption text-foreground-secondary">
                      {item.receipt.state === "accepted"
                        ? "Captured"
                        : label(item.receipt.state)}
                    </span>
                    <ChevronDown
                      className="size-3.5 text-muted-foreground transition-transform duration-fast group-open:rotate-180"
                      aria-hidden="true"
                    />
                  </summary>
                  <div className="grid gap-1 px-3 pb-3 text-caption text-foreground-secondary [&_p]:m-0">
                    {item.source_available &&
                      item.receipt.source_id &&
                      item.receipt.source_version_id && (
                        <div className="pb-1">
                          <Button
                            variant="secondary"
                            size="sm"
                            onClick={() => openEvidence(item)}
                          >
                            Open evidence
                          </Button>
                        </div>
                      )}
                    <p>
                      {label(item.host)} {item.host_version} ·{" "}
                      {item.event
                        ? label(item.event.outcome)
                        : "Event content removed"}
                    </p>
                    {item.processing && (
                      <p>Processing: {label(item.processing)}</p>
                    )}
                    {item.learning && (
                      <p>
                        Learning: {label(item.learning.state)}
                        {item.learning.state === "succeeded"
                          ? ` · ${item.learning.accepted} accepted, ${item.learning.proposed} proposed`
                          : ""}{" "}
                        ·{" "}
                        {time(
                          item.learning.finished_at ??
                            item.learning.job.updated_at,
                        )}
                      </p>
                    )}
                    <p>
                      Scope: {item.selection.repository_ids?.length ?? 0}{" "}
                      repositories · {item.selection.area_ids?.length ?? 0}{" "}
                      areas
                      {item.selection.environment_id
                        ? " · selected environment"
                        : ""}
                    </p>
                    {item.managed_call_id && (
                      <p className="mono">Managed call {item.managed_call_id}</p>
                    )}
                    {!!item.event?.coverage.length && (
                      <p>
                        Coverage: {item.event.coverage.map(label).join(" · ")}
                      </p>
                    )}
                    {item.receipt.expires_at && (
                      <p>Content deadline: {time(item.receipt.expires_at)}</p>
                    )}
                    {!item.source_available && (
                      <p>No retained source text available.</p>
                    )}
                    <p className="mono text-muted-foreground">
                      Event {item.receipt.event_id}
                    </p>
                  </div>
                </details>
              );
            })}
          </div>
        </section>
      ))}
    </div>
  );
}

/** Activity's captured-event timeline: date groups, mono times, one outcome. */
function CaptureTimeline({
  items,
  openEvidence,
}: {
  items: Event[];
  openEvidence: (event: Event) => void;
}) {
  const stagger = useFirstLoadStagger(items.map((item) => item.receipt.event_id));
  let index = 0;
  return (
    <div className="grid">
      {byDay(items).map(([day, events]) => (
        <section key={day} aria-label={day}>
          <h4 className="sticky top-0 z-[1] m-0 border-b border-border bg-muted/85 px-4 py-1.5 text-caption font-medium text-muted-foreground backdrop-blur-chrome">
            {day}
          </h4>
          {events.map((item) => {
            const captured = capturedAt(item);
            const entrance = stagger(item.receipt.event_id, index++);
            return (
              <div
                key={item.receipt.event_id}
                className={cn(
                  "border-b border-border px-4 py-2.5 last:border-b-0",
                  entrance.className,
                )}
                style={entrance.style}
              >
                <div className="grid grid-cols-[56px_minmax(0,1fr)_auto] items-start gap-x-3">
                  <time
                    className="mono pt-0.5 text-caption text-muted-foreground"
                    dateTime={captured}
                    title={time(captured)}
                  >
                    {clock(captured)}
                  </time>
                  <div className="grid min-w-0 gap-0.5">
                    <span className="truncate text-body font-medium text-foreground">
                      {eventTitle(item)}
                    </span>
                    <span className="truncate text-caption text-muted-foreground">
                      {item.user_name} · {item.agent_name ?? label(item.host)}
                      {item.event ? ` · ${label(item.event.outcome)}` : ""}
                    </span>
                    {item.processing && (
                      <span className="text-caption text-muted-foreground">
                        Processing · {label(item.processing)}
                        {item.learning
                          ? ` · Last recorded learning ${label(item.learning.state)}${item.learning.state === "succeeded" ? ` · ${item.learning.accepted} accepted, ${item.learning.proposed} proposed` : ""}`
                          : " · No learning outcome recorded"}
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    {item.source_available &&
                      item.receipt.source_id &&
                      item.receipt.source_version_id && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => openEvidence(item)}
                        >
                          Open evidence
                        </Button>
                      )}
                    {item.receipt.state === "accepted" ? (
                      <StatusBadge state="positive">Captured</StatusBadge>
                    ) : (
                      <StatusBadge state="neutral">
                        <span className="first-letter:uppercase">
                          {label(item.receipt.state)}
                        </span>
                      </StatusBadge>
                    )}
                  </div>
                </div>
                <details className="group ml-[68px] mt-1">
                  <summary className="inline-flex cursor-pointer list-none items-center gap-1 rounded-sm text-caption text-muted-foreground hover:text-foreground [&::-webkit-details-marker]:hidden">
                    Technical details
                    <ChevronDown
                      className="size-3 transition-transform duration-fast group-open:rotate-180"
                      aria-hidden="true"
                    />
                  </summary>
                  <div className="mt-1.5 grid gap-1 rounded-md border border-border bg-muted/50 px-3 py-2 text-caption text-foreground-secondary [&_p]:m-0">
                    <p>
                      {label(item.host)} {item.host_version} · event{" "}
                      <span className="mono">{item.receipt.event_id}</span>
                    </p>
                    <p>
                      Scope: {item.selection.repository_ids?.length ?? 0}{" "}
                      repositories · {item.selection.area_ids?.length ?? 0}{" "}
                      areas
                      {item.selection.environment_id
                        ? " · selected environment"
                        : ""}
                    </p>
                    {item.managed_call_id && (
                      <p>
                        Managed call{" "}
                        <span className="mono">{item.managed_call_id}</span>
                      </p>
                    )}
                    {item.learning && (
                      <p>
                        Learning state recorded{" "}
                        {time(
                          item.learning.finished_at ??
                            item.learning.job.updated_at,
                        )}
                      </p>
                    )}
                    {!!item.event?.coverage.length && (
                      <p>
                        Coverage: {item.event.coverage.map(label).join(" · ")}
                      </p>
                    )}
                    {item.receipt.expires_at && (
                      <p>Content deadline: {time(item.receipt.expires_at)}</p>
                    )}
                    {!item.source_available && (
                      <p>No retained source text available.</p>
                    )}
                  </div>
                </details>
              </div>
            );
          })}
        </section>
      ))}
    </div>
  );
}
const kinds = ["prompt", "reply", "tool_result", "lifecycle"].map((value) => ({
  value,
  label: label(value),
}));
function Failure({ error }: { error: Error | null }) {
  return error ? <Alert variant="destructive">{error.message}</Alert> : null;
}
function Pages({
  offset,
  total,
  change,
}: {
  offset: number;
  total: number;
  change: (n: number) => void;
}) {
  return total > 20 ? (
    <div className="flex items-center gap-2">
      <Button
        variant="ghost"
        size="sm"
        disabled={!offset}
        onClick={() => change(offset - 20)}
      >
        Previous
      </Button>
      <span className="mono text-caption text-muted-foreground">
        {offset + 1}–{Math.min(offset + 20, total)} of {total}
      </span>
      <Button
        variant="ghost"
        size="sm"
        disabled={offset + 20 >= total}
        onClick={() => change(offset + 20)}
      >
        Next
      </Button>
    </div>
  ) : null;
}
export function CapturePolicyEditor({
  brain,
  settings,
  close,
  inline = false,
  readOnly = false,
}: {
  brain: string;
  settings: Settings;
  close: () => void;
  inline?: boolean;
  readOnly?: boolean;
}) {
  const [draft, setDraft] = useState(structuredClone(settings.policy));
  const [base] = useState(settings.change_id);
  const [tools, setTools] = useState(draft.excluded_tools.join("\n"));
  const [content, setContent] = useState(draft.excluded_content.join("\n"));
  const cache = useQueryClient();
  const command = useIdempotency();
  const lines = (value: string) =>
    value
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean);
  const save = useMutation({
    mutationFn: async () => {
      const body = {
        base_change: base,
        policy: {
          ...draft,
          excluded_tools: lines(tools),
          excluded_content: lines(content),
        },
      };
      return result(
        await client.PUT("/api/brains/{brain}/capture/policy", {
          params: { path: { brain } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: async () => {
      await cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain),
      });
      close();
    },
  });
  const form = (
    <div className="grid">
      <PolicyRow title="Automatic session capture">
        <Switch
          aria-label="Enable automatic session capture"
          checked={readOnly ? settings.policy.enabled : draft.enabled}
          disabled={readOnly}
          onChange={(e) =>
            setDraft({ ...draft, enabled: e.currentTarget.checked })
          }
        />
      </PolicyRow>
      {[
        {
          title: "Questions & replies",
          description: "User questions and agent responses",
          values: ["prompt", "reply"],
        },
        {
          title: "Tool results",
          description: "Outputs from tools",
          values: ["tool_result"],
        },
        {
          title: "Session events",
          description: "Agent actions, tool calls and other events",
          values: ["lifecycle"],
        },
      ].map((row) => (
        <PolicyRow
          key={row.title}
          title={row.title}
          description={row.description}
        >
          <div className="flex flex-wrap gap-4">
            {row.values.map((value) => (
              <Switch
                key={value}
                aria-label={
                  "Capture " +
                  (kinds.find((k) => k.value === value)?.label.toLowerCase() ??
                    value)
                }
                checked={(readOnly ? settings.policy : draft).kinds.includes(
                  value,
                )}
                disabled={readOnly}
                onChange={(e) =>
                  setDraft({
                    ...draft,
                    kinds: e.currentTarget.checked
                      ? [...draft.kinds, value]
                      : draft.kinds.filter((k) => k !== value),
                  })
                }
              />
            ))}
          </div>
        </PolicyRow>
      ))}
      <section className="grid" aria-label="Capture limits">
        <PolicyRow title="Managed tool results">
          <Switch
            aria-label="Capture managed tool results"
            checked={
              (readOnly ? settings.policy : draft).managed_tools ?? false
            }
            disabled={readOnly}
            onChange={(e) =>
              setDraft({ ...draft, managed_tools: e.currentTarget.checked })
            }
          />
        </PolicyRow>
        <PolicyRow title="Maximum text per event" description="KiB">
          <NumberInput
            aria-label="Maximum text per event · KiB"
            min={1}
            max={64}
            allowDecimal={false}
            value={(readOnly ? settings.policy : draft).max_event_bytes / 1024}
            readOnly={readOnly}
            hideControls={readOnly}
            suffix="KiB"
            onChange={(value) =>
              setDraft({
                ...draft,
                max_event_bytes: typeof value === "number" ? value * 1024 : 0,
              })
            }
          />
        </PolicyRow>
        {(!readOnly || settings.policy.excluded_tools.length > 0) && (
          <PolicyRow
            title="Excluded tools"
            description="One name or pattern per line; up to 20."
          >
            {readOnly ? (
              <span className="mono text-label text-foreground">
                {settings.policy.excluded_tools.join(" · ") || "None"}
              </span>
            ) : (
              <Textarea
                aria-label="Excluded tools"
                autosize
                minRows={2}
                value={tools}
                onChange={(e) => setTools(e.currentTarget.value)}
              />
            )}
          </PolicyRow>
        )}
        {(!readOnly || settings.policy.excluded_content.length > 0) && (
          <PolicyRow
            title="Excluded content"
            description="Literal phrases. Do not enter credentials."
          >
            {readOnly ? (
              <span className="text-label text-foreground">
                {settings.policy.excluded_content.join(" · ") || "None"}
              </span>
            ) : (
              <Textarea
                aria-label="Excluded content"
                autosize
                minRows={2}
                value={content}
                onChange={(e) => setContent(e.currentTarget.value)}
              />
            )}
          </PolicyRow>
        )}
      </section>
      <div className="pt-3">
        <Failure error={save.error} />
      </div>
      {!readOnly && inline && (
        <div className="flex justify-end gap-2 pt-1">
          <Button variant="secondary" disabled={save.isPending} onClick={close}>
            Cancel
          </Button>
          <Button loading={save.isPending} onClick={() => save.mutate()}>
            Save capture policy
          </Button>
        </div>
      )}
    </div>
  );
  return inline ? (
    form
  ) : (
    <Modal
      opened
      onClose={() => !save.isPending && close()}
      title="Capture permissions"
      size="lg"
      footer={
        readOnly ? undefined : (
          <>
            <Button
              variant="secondary"
              disabled={save.isPending}
              onClick={close}
            >
              Cancel
            </Button>
            <Button loading={save.isPending} onClick={() => save.mutate()}>
              Save capture policy
            </Button>
          </>
        )
      }
    >
      {form}
    </Modal>
  );
}
// Retained connection hint; Agents owns the actual setup flow.
export function CaptureSetup({
  brain,
  close,
}: {
  brain: string;
  close: () => void;
}) {
  return (
    <Modal opened onClose={close} title="Connect session capture" size="lg">
      <div className="grid gap-3 text-body text-foreground-secondary [&_p]:m-0">
        <p>
          The Recollect plugin supplies automatic capture and memory recall in
          normal Codex, Claude Code and OpenCode sessions. Install it and
          connect once from Agents.
        </p>
        <div>
          <Button variant="secondary" size="sm" asChild>
            <Link
              to="/brains/$brainId/agents"
              params={{ brainId: brain }}
              search={{ tab: "setup" }}
            >
              Connect an agent
            </Link>
          </Button>
        </div>
        <p>
          Capture follows this Brain&apos;s policy. Delivered events and
          coverage gaps appear here after the host runs.
        </p>
      </div>
    </Modal>
  );
}
function CaptureDevice({ item }: { item: Device }) {
  const stale = item.reported_at
    ? Date.now() - Date.parse(item.reported_at) > 30_000
    : false;
  const report = item.report;
  const state = !report
    ? "Configured only"
    : stale
      ? "Offline or stopped"
      : report.denied
        ? "Delivery denied"
        : report.issue
          ? "Partial delivery"
          : report.pending
            ? "Queued"
            : "Connected · queue empty";
  return (
    <div className="grid gap-1.5 rounded-lg border border-border bg-card px-3.5 py-3">
      <div className="flex items-center justify-between gap-3">
        <span className="text-body font-semibold text-foreground">
          Device <span className="mono">{item.device_id.slice(0, 8)}</span>
        </span>
        <StatusBadge
          state={
            !report || stale
              ? "neutral"
              : report.denied
                ? "negative"
                : report.issue || report.pending
                  ? "attention"
                  : "positive"
          }
        >
          {state}
        </StatusBadge>
      </div>
      <p className="m-0 text-label text-foreground-secondary">
        {item.bindings} configured{" "}
        {item.bindings === 1 ? "binding" : "bindings"}
        {report
          ? ` · ${report.pending} queued · ${report.denied} denied`
          : " · no delivery report yet"}
      </p>
      {report?.issue && (
        <p className="m-0 text-label text-foreground-secondary">
          {label(report.issue)}. Start the coding host again or run the plugin
          drain command after resolving access or connectivity.
        </p>
      )}
      {!!report?.device_gap_count && (
        <p className="m-0 text-label text-warning-text">
          {report.device_gap_count} gaps reported across this device. Use local
          capture status for the causes.
        </p>
      )}
      <p className="m-0 mono text-caption text-muted-foreground">
        Last report: {item.reported_at ? time(item.reported_at) : "none"} ·
        Last publication:{" "}
        {item.last_publication ? time(item.last_publication) : "none"}
      </p>
    </div>
  );
}
function capturedResultText(content: string): string | null {
  try {
    const record = JSON.parse(content);
    if (
      record?.provenance !== "reported_tool_observation" ||
      !Array.isArray(record.result?.content)
    )
      return null;
    const text = record.result.content
      .filter(
        (part: { type?: string; text?: string }) =>
          part?.type === "text" && typeof part.text === "string",
      )
      .map((part: { text: string }) => part.text)
      .join("\n\n");
    return text || null;
  } catch {
    return null;
  }
}
export function CapturedSource({
  brain,
  source,
  version,
  close,
}: {
  brain: Brain;
  source: string;
  version: string;
  close: () => void;
}) {
  const pollNormal = usePollInterval("normal");
  const evidence = useQuery({
    queryKey: ["capture-source", brain.id, source, version],
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/sources/{source}/versions/{version}",
          { signal, params: { path: { brain: brain.id, source, version } } },
        ),
      ),
    refetchInterval: pollNormal,
    gcTime: 0,
    retry: false,
  });
  const expired = useContentDeadline(evidence.data?.version.expires_at);
  const capturedText = evidence.data?.content
    ? capturedResultText(evidence.data.content)
    : null;
  return (
    <Modal opened onClose={close} title="Captured source evidence" size="lg">
      <div className="grid gap-3">
        <Failure error={evidence.error} />
        {evidence.isPending ? (
          <SkeletonRows rows={3} label="Loading captured evidence…" />
        ) : (
          !evidence.error &&
          evidence.data && (
            <>
              <div className="grid gap-0.5">
                <p className="m-0 text-title font-semibold text-foreground">
                  {evidence.data.version.title}
                </p>
                <p className="m-0 text-caption text-muted-foreground">
                  Captured{" "}
                  <span className="mono">
                    {time(evidence.data.version.created_at)}
                  </span>{" "}
                  ·{" "}
                  {label(evidence.data.version.retention_class ?? "raw_session")}
                </p>
              </div>
              {evidence.data.content != null && !expired ? (
                <div
                  data-testid="capture-source-content"
                  className="grid gap-3"
                >
                  <Markdown text={capturedText ?? evidence.data.content} />
                  {capturedText && (
                    <details className="group">
                      <summary className="cursor-pointer text-label font-medium text-foreground-secondary hover:text-foreground">
                        Full captured record
                      </summary>
                      <div className="mt-2">
                        <Code block>{evidence.data.content}</Code>
                      </div>
                    </details>
                  )}
                </div>
              ) : (
                <Alert role="status">
                  This source content is unavailable or has been removed.
                </Alert>
              )}
            </>
          )
        )}
        {brain.role === "admin" && !brain.archived && (
          <div>
            <EraseAction
              brain={brain.id}
              target={{ kind: "source", id: source }}
              permitted
              name="captured source"
            />
          </div>
        )}
      </div>
    </Modal>
  );
}
export function CapturePanel({
  brain,
  section = "sessions",
  deviceId,
  initialCoverage = false,
  selectedSource,
  selectedVersion,
  onInspectorChange,
  compactTimeline = false,
}: {
  brain: Brain;
  section?: "sessions" | "settings";
  simple?: boolean;
  deviceId?: string;
  initialCoverage?: boolean;
  selectedSource?: string;
  selectedVersion?: string;
  onInspectorChange?: (open: boolean) => void;
  compactTimeline?: boolean;
}) {
  const pollNormal = usePollInterval("normal");
  const [editing, setEditing] = useState(false);
  const [coverage, setCoverage] = useState(initialCoverage);
  const [kind, setKind] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [deviceOffset, setDeviceOffset] = useState(0);
  const [viewing, setViewing] = useState<Event | null>(null);
  const [linkedEvidence, setLinkedEvidence] = useState<{
    source: string;
    version: string;
  } | null>(null);
  useEffect(() => {
    setLinkedEvidence(
      selectedSource && selectedVersion
        ? { source: selectedSource, version: selectedVersion }
        : null,
    );
  }, [selectedSource, selectedVersion, brain.id]);
  useEffect(() => {
    onInspectorChange?.(!!(viewing || linkedEvidence || coverage || editing));
    return () => onInspectorChange?.(false);
  }, [viewing, linkedEvidence, coverage, editing, onInspectorChange]);
  const path = { brain: brain.id };
  const policy = useQuery({
    queryKey: ["capture-policy", brain.id],
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/capture/policy", {
          signal,
          params: { path },
        }),
      ),
    refetchInterval: pollNormal,
  });
  const events = useQuery({
    queryKey: ["capture-events", brain.id, deviceId, offset, kind],
    enabled: section === "sessions",
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/capture/events", {
          signal,
          params: {
            path,
            query: { offset, device_id: deviceId, kind: kind ?? undefined },
          },
        }),
      ),
    refetchInterval: pollNormal,
  });
  const managed = useQuery({
    queryKey: ["capture-managed", brain.id],
    enabled: section === "sessions" && coverage && !deviceId,
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/capture/managed", {
          signal,
          params: { path },
        }),
      ),
    refetchInterval: pollNormal,
  });
  const devices = useQuery({
    queryKey: ["capture-devices", brain.id, deviceOffset],
    enabled: section === "sessions" && coverage && !deviceId,
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/capture/devices", {
          signal,
          params: { path, query: { offset: deviceOffset } },
        }),
      ),
    refetchInterval: pollNormal,
  });
  useEffect(() => {
    setOffset(0);
    setViewing(null);
  }, [brain.id, deviceId, kind]);
  useEffect(() => {
    if (policy.isError || events.isError) {
      setViewing(null);
      setLinkedEvidence(null);
      setEditing(false);
    }
  }, [policy.isError, events.isError]);
  useEffect(() => {
    if (brain.role !== "admin" || brain.archived) setEditing(false);
  }, [brain.role, brain.archived]);
  const current = policy.isError ? undefined : policy.data?.policy;
  const heading = section === "settings" ? "Capture" : "Captured activity";
  const controls: ReactNode =
    section === "settings" ? (
      brain.role === "admin" &&
      !brain.archived && (
        <Button
          variant="secondary"
          size="sm"
          disabled={!current}
          onClick={() => setEditing(true)}
        >
          Edit capture permissions
        </Button>
      )
    ) : (
      <div className="flex items-center gap-2">
        {!deviceId && (
          <Button variant="ghost" size="sm" onClick={() => setCoverage(true)}>
            <ScanLine aria-hidden="true" />
            Capture diagnostics
          </Button>
        )}
        <Select
          aria-label="Activity kind"
          placeholder="All captured events"
          clearable
          size="sm"
          data={kinds}
          value={kind}
          onChange={setKind}
          className="w-48"
        />
      </div>
    );
  const body = (
    <>
      <Failure error={policy.error ?? events.error} />
      {section === "settings" &&
        (policy.isPending ? (
          <Spinner label="Loading capture permissions" />
        ) : (
          current && (
            <div className="grid gap-1.5">
              <div className="flex flex-wrap items-center gap-2">
                <StatusBadge state={current.enabled ? "positive" : "neutral"}>
                  {current.enabled ? "Allowed" : "Off"}
                </StatusBadge>
                <span className="text-body text-foreground-secondary">
                  {current.enabled
                    ? current.kinds.map(label).join(" · ") ||
                      "No event kinds allowed"
                    : "New session capture is disabled."}
                </span>
              </div>
              <p className="m-0 text-label text-muted-foreground">
                {current.excluded_tools.length} tool exclusions ·{" "}
                {current.excluded_content.length} content exclusions. Secret
                redaction always applies.
              </p>
              <p className="m-0 text-caption text-muted-foreground">
                Capture does not grant permission to send content to AI.
                Existing evidence follows its retention rules.
              </p>
            </div>
          )
        ))}
      {section === "sessions" &&
        compactTimeline &&
        !events.isError &&
        events.data?.items.some((item) =>
          item.event?.coverage.includes("partial_host_coverage"),
        ) && (
          <p className="m-0 text-caption font-medium text-warning-text">
            Partial host coverage
          </p>
        )}
      {section === "sessions" &&
        (events.isPending ? (
          <SkeletonRows rows={4} label="Loading captured events…" />
        ) : (
          !events.isError &&
          events.data?.total === 0 &&
          (compactTimeline ? (
            <p className="m-0 text-body text-muted-foreground">
              No captured events in this view.
            </p>
          ) : (
            <EmptyState
              icon={Inbox}
              title="No captured events in this view"
              description="Events captured by connected agents appear here under this Brain’s capture policy."
              className="py-10"
            />
          ))
        ))}
      {section === "sessions" &&
        !events.isError &&
        !policy.isError &&
        compactTimeline && (
          <CompactCaptureTimeline
            items={events.data?.items ?? []}
            openEvidence={setViewing}
          />
        )}
    </>
  );
  return (
    <section
      className={cn(
        "grid",
        compactTimeline
          ? "gap-3"
          : "overflow-clip rounded-lg border border-border bg-card",
      )}
      aria-label={
        section === "settings" ? "Capture permissions" : "Captured activity"
      }
    >
      {compactTimeline ? (
        <>
          <div className="flex items-center justify-between gap-3">
            <h3 className="m-0 text-title font-semibold text-foreground">
              {heading}
            </h3>
            {controls}
          </div>
          {body}
        </>
      ) : (
        <>
          <CardBar>
            <h3>{heading}</h3>
            {section === "sessions" && (
              <span className="hidden truncate text-caption text-muted-foreground xl:inline">
                Recorded capture events. Host coverage may be partial.
              </span>
            )}
            <div className="ml-auto">{controls}</div>
          </CardBar>
          {(policy.error ||
            events.error ||
            section === "settings" ||
            events.isPending ||
            events.data?.total === 0) && (
            <div className="grid gap-3 p-4">{body}</div>
          )}
          {section === "sessions" &&
            !events.isError &&
            !policy.isError &&
            !!events.data?.items.length && (
              <CaptureTimeline
                items={events.data.items}
                openEvidence={setViewing}
              />
            )}
        </>
      )}
      {section === "sessions" && !events.isError && events.data && (
        <div className={cn("empty:hidden", !compactTimeline && "border-t border-border px-4 py-2")}>
          <Pages
            offset={offset}
            total={events.data.total}
            change={setOffset}
          />
        </div>
      )}
      <Drawer
        opened={coverage && !deviceId}
        onClose={() => setCoverage(false)}
        position="right"
        title="Capture diagnostics"
        size={560}
      >
        {coverage && !deviceId && (
          <div className="grid gap-3">
            <Failure error={managed.error ?? devices.error} />
            {!managed.isError && managed.data && (
              <p className="m-0 mono text-label text-foreground-secondary">
                {managed.data.pending} pending · {managed.data.errors} retrying
                · {managed.data.published} published
                {managed.data.unknown
                  ? ` · ${managed.data.unknown} outcomes unknown`
                  : ""}
              </p>
            )}
            {(managed.isPending || devices.isPending) && (
              <SkeletonRows rows={3} label="Loading capture diagnostics…" />
            )}
            {!devices.isError &&
              devices.data?.items.map((item) => (
                <CaptureDevice key={item.device_id} item={item} />
              ))}
            {!devices.isError && devices.data?.items.length === 0 && (
              <p className="m-0 text-body text-muted-foreground">
                No capture devices have reported to this Brain.
              </p>
            )}
            {!devices.isError && devices.data && (
              <Pages
                offset={deviceOffset}
                total={devices.data.total}
                change={setDeviceOffset}
              />
            )}
          </div>
        )}
      </Drawer>
      {editing &&
        brain.role === "admin" &&
        !brain.archived &&
        policy.data &&
        !policy.isError && (
          <CapturePolicyEditor
            brain={brain.id}
            settings={policy.data}
            close={() => setEditing(false)}
          />
        )}
      {linkedEvidence && !events.isError && !policy.isError && (
        <CapturedSource
          brain={brain}
          source={linkedEvidence.source}
          version={linkedEvidence.version}
          close={() => setLinkedEvidence(null)}
        />
      )}
      {viewing && !linkedEvidence && !events.isError && !policy.isError && (
        <CapturedSource
          brain={brain}
          source={viewing.receipt.source_id!}
          version={viewing.receipt.source_version_id!}
          close={() => setViewing(null)}
        />
      )}
    </section>
  );
}
