import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import "./features/feature-views.css";
import {
  Alert,
  Badge,
  Button,
  Card,
  Drawer,
  Code,
  Group,
  Loader,
  Modal,
  MultiSelect,
  NumberInput,
  Select,
  Stack,
  Switch,
  Text,
  Textarea,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { PolicyRow } from "./features/settings/PolicyRow";
import { useIdempotency } from "./useIdempotency";
import { EraseAction } from "./RetentionPanel";
import { Markdown } from "./components/Markdown";
import { useContentDeadline } from "./useContentDeadline";
import { ChevronDown } from "lucide-react";

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
function CompactCaptureTimeline({
  items,
  openEvidence,
}: {
  items: Event[];
  openEvidence: (event: Event) => void;
}) {
  const days = new Map<string, Event[]>();
  for (const item of items) {
    const day = new Date(
      item.event?.captured_at ?? item.receipt.received_at,
    ).toLocaleDateString(undefined, {
      day: "numeric",
      month: "short",
      year: "numeric",
    });
    days.set(day, [...(days.get(day) ?? []), item]);
  }
  return (
    <div className="agent-capture-days">
      {[...days].map(([day, events]) => (
        <section className="agent-capture-day" key={day} aria-label={day}>
          <h4>{day}</h4>
          {events.map((item) => {
            const captured =
              item.event?.captured_at ?? item.receipt.received_at;
            return (
              <details
                className="agent-capture-event"
                key={item.receipt.event_id}
                data-testid="agent-activity-event"
              >
                <summary>
                  <strong>
                    {item.event?.tool_name ||
                      (item.event ? label(item.event.kind) : "Removed event")}
                  </strong>
                  <time dateTime={captured} title={time(captured)}>
                    {new Date(captured).toLocaleTimeString(undefined, {
                      hour: "2-digit",
                      minute: "2-digit",
                    })}
                  </time>
                  <span className="agent-event-outcome">
                    {item.receipt.state === "accepted"
                      ? "Captured"
                      : label(item.receipt.state)}
                  </span>
                  <ChevronDown size={14} />
                </summary>
                <div className="agent-capture-event-details">
                  {item.source_available &&
                    item.receipt.source_id &&
                    item.receipt.source_version_id && (
                      <Button
                        variant="light"
                        size="compact-xs"
                        onClick={() => openEvidence(item)}
                      >
                        Open evidence
                      </Button>
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
                    repositories · {item.selection.area_ids?.length ?? 0} areas
                    {item.selection.environment_id
                      ? " · selected environment"
                      : ""}
                  </p>
                  {item.managed_call_id && (
                    <p>Managed call {item.managed_call_id}</p>
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
                  <p>Event {item.receipt.event_id}</p>
                </div>
              </details>
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
  return error ? <Alert color="red">{error.message}</Alert> : null;
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
    <Group>
      <Button
        variant="subtle"
        disabled={!offset}
        onClick={() => change(offset - 20)}
      >
        Previous
      </Button>
      <Text size="xs">
        {offset + 1}–{Math.min(offset + 20, total)} of {total}
      </Text>
      <Button
        variant="subtle"
        disabled={offset + 20 >= total}
        onClick={() => change(offset + 20)}
      >
        Next
      </Button>
    </Group>
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
    <Stack className={inline ? "privacy-editor" : undefined}>
      <div className="privacy-capture-enabled">
        <span>Automatic session capture</span>
        <Switch
          aria-label="Enable automatic session capture"
          checked={readOnly ? settings.policy.enabled : draft.enabled}
          disabled={readOnly}
          onChange={(e) =>
            setDraft({ ...draft, enabled: e.currentTarget.checked })
          }
        />
      </div>
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
          <div className="privacy-capture-kind-switches">
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
      <section className="privacy-capture-options" aria-label="Capture limits">
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
              settings.policy.excluded_tools.join(" · ") || "None"
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
              settings.policy.excluded_content.join(" · ") || "None"
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
      <Failure error={save.error} />
      {!readOnly && (
        <Group justify="flex-end">
          <Button variant="default" disabled={save.isPending} onClick={close}>
            Cancel
          </Button>
          <Button loading={save.isPending} onClick={() => save.mutate()}>
            Save capture policy
          </Button>
        </Group>
      )}
    </Stack>
  );
  return inline ? (
    form
  ) : (
    <Modal
      opened
      onClose={() => !save.isPending && close()}
      title="Capture permissions"
      size="lg"
    >
      {form}
    </Modal>
  );
}
function Setup({ brain, close }: { brain: string; close: () => void }) {
  return (
    <Modal opened onClose={close} title="Connect session capture" size="lg">
      <Stack>
        <Text size="sm">
          The Recollect plugin supplies automatic capture and memory recall in
          normal Codex, Claude Code and OpenCode sessions. Install it and
          connect once from Agents.
        </Text>
        <Link
          to="/brains/$brainId/agents"
          params={{ brainId: brain }}
          search={{ tab: "setup" }}
        >
          Connect an agent
        </Link>
        <Text size="sm">
          Capture follows this Brain&apos;s policy. Delivered events and
          coverage gaps appear here after the host runs.
        </Text>
      </Stack>
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
    <Card withBorder padding="sm">
      <Stack gap="xs">
        <Group justify="space-between">
          <Text fw={600}>Device {item.device_id.slice(0, 8)}</Text>
          <Badge
            color={
              !report || stale
                ? "gray"
                : report.denied
                  ? "red"
                  : report.issue || report.pending
                    ? "yellow"
                    : "teal"
            }
          >
            {state}
          </Badge>
        </Group>
        <Text size="sm">
          {item.bindings} configured{" "}
          {item.bindings === 1 ? "binding" : "bindings"}
          {report
            ? ` · ${report.pending} queued · ${report.denied} denied`
            : " · no delivery report yet"}
        </Text>
        {report?.issue && (
          <Text size="sm">
            {label(report.issue)}. Start the coding host again or run the plugin
            drain command after resolving access or connectivity.
          </Text>
        )}
        {!!report?.device_gap_count && (
          <Text size="sm" c="orange">
            {report.device_gap_count} gaps reported across this device. Use
            local capture status for the causes.
          </Text>
        )}
        <Text size="xs" c="dimmed">
          Last report: {item.reported_at ? time(item.reported_at) : "none"} ·
          Last publication:{" "}
          {item.last_publication ? time(item.last_publication) : "none"}
        </Text>
      </Stack>
    </Card>
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
  const evidence = useQuery({
    queryKey: ["capture-source", brain.id, source, version],
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/sources/{source}/versions/{version}",
          { signal, params: { path: { brain: brain.id, source, version } } },
        ),
      ),
    refetchInterval: 4000,
    gcTime: 0,
    retry: false,
  });
  const expired = useContentDeadline(evidence.data?.version.expires_at);
  const capturedText = evidence.data?.content
    ? capturedResultText(evidence.data.content)
    : null;
  return (
    <Modal opened onClose={close} title="Captured source evidence" size="lg">
      <Stack>
        <Failure error={evidence.error} />
        {evidence.isPending ? (
          <Loader />
        ) : (
          !evidence.error &&
          evidence.data && (
            <>
              <Text fw={600}>{evidence.data.version.title}</Text>
              <Text size="xs">
                Captured {time(evidence.data.version.created_at)} ·{" "}
                {label(evidence.data.version.retention_class ?? "raw_session")}
              </Text>
              {evidence.data.content != null && !expired ? (
                <div data-testid="capture-source-content">
                  <Markdown text={capturedText ?? evidence.data.content} />
                  {capturedText && (
                    <details className="feature-details">
                      <summary>Full captured record</summary>
                      <Code block>{evidence.data.content}</Code>
                    </details>
                  )}
                </div>
              ) : (
                <Alert color="gray">
                  This source content is unavailable or has been removed.
                </Alert>
              )}
            </>
          )
        )}
        {brain.role === "admin" && !brain.archived && (
          <EraseAction
            brain={brain.id}
            target={{ kind: "source", id: source }}
            permitted
            name="captured source"
          />
        )}
      </Stack>
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
    refetchInterval: 4000,
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
    refetchInterval: 4000,
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
    refetchInterval: 4000,
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
    refetchInterval: 4000,
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
  return (
    <section
      className={
        section === "settings"
          ? "feature-setting"
          : compactTimeline
            ? "agent-capture-timeline"
            : "feature-view"
      }
      aria-label={
        section === "settings" ? "Capture permissions" : "Captured activity"
      }
    >
      <Stack gap="md">
        <Group
          justify="space-between"
          className={compactTimeline ? "agent-activity-toolbar" : undefined}
        >
          <Title order={3}>
            {section === "settings" ? "Capture" : "Captured activity"}
          </Title>
          {section === "settings" ? (
            brain.role === "admin" &&
            !brain.archived && (
              <Button
                variant="default"
                disabled={!current}
                onClick={() => setEditing(true)}
              >
                Edit capture permissions
              </Button>
            )
          ) : (
            <Group>
              {!deviceId && (
                <Button variant="subtle" onClick={() => setCoverage(true)}>
                  Capture diagnostics
                </Button>
              )}
              <Select
                aria-label="Activity kind"
                placeholder="All captured events"
                clearable
                data={kinds}
                value={kind}
                onChange={setKind}
              />
            </Group>
          )}
        </Group>
        <Failure error={policy.error ?? events.error} />
        {section === "settings" &&
          (policy.isPending ? (
            <Loader size="sm" />
          ) : (
            current && (
              <>
                <Group>
                  <Badge color={current.enabled ? "teal" : "gray"}>
                    {current.enabled ? "Allowed" : "Off"}
                  </Badge>
                  <Text size="sm">
                    {current.enabled
                      ? current.kinds.map(label).join(" · ") ||
                        "No event kinds allowed"
                      : "New session capture is disabled."}
                  </Text>
                </Group>
                <Text size="sm" c="dimmed">
                  {current.excluded_tools.length} tool exclusions ·{" "}
                  {current.excluded_content.length} content exclusions. Secret
                  redaction always applies.
                </Text>
                <Text size="xs" c="dimmed">
                  Capture does not grant permission to send content to AI.
                  Existing evidence follows its retention rules.
                </Text>
              </>
            )
          ))}
        {section === "sessions" && (
          <>
            {!compactTimeline && (
              <Text size="xs" c="dimmed">
                Recorded capture events. Host coverage may be partial.
              </Text>
            )}
            {compactTimeline &&
              !events.isError &&
              events.data?.items.some((item) =>
                item.event?.coverage.includes("partial_host_coverage"),
              ) && (
                <p className="agent-capture-partial">Partial host coverage</p>
              )}
            {events.isPending ? (
              <Loader size="sm" />
            ) : (
              !events.isError &&
              events.data?.total === 0 && (
                <Text size="sm" c="dimmed">
                  No captured events in this view.
                </Text>
              )
            )}
            {!events.isError &&
              !policy.isError &&
              (compactTimeline ? (
                <CompactCaptureTimeline
                  items={events.data?.items ?? []}
                  openEvidence={setViewing}
                />
              ) : (
                events.data?.items.map((item) => (
                  <Card withBorder padding="md" key={item.receipt.event_id}>
                    <Stack gap="sm">
                      <Group justify="space-between" align="start">
                        <div>
                          <Text fw={600} size="sm">
                            {item.event?.tool_name ||
                              (item.event
                                ? label(item.event.kind)
                                : "Removed event")}
                          </Text>
                          <Text size="xs" c="dimmed">
                            {item.user_name} ·{" "}
                            {item.agent_name ?? label(item.host)}
                          </Text>
                        </div>
                        <Badge
                          color={
                            item.receipt.state === "accepted" ? "teal" : "gray"
                          }
                        >
                          {item.receipt.state === "accepted"
                            ? "Captured"
                            : label(item.receipt.state)}
                        </Badge>
                      </Group>
                      <Group justify="space-between">
                        <Text size="xs" c="dimmed">
                          {time(
                            item.event?.captured_at ?? item.receipt.received_at,
                          )}
                          {item.event ? ` · ${label(item.event.outcome)}` : ""}
                        </Text>
                        {item.source_available &&
                          item.receipt.source_id &&
                          item.receipt.source_version_id && (
                            <Button
                              variant="subtle"
                              size="compact-sm"
                              onClick={() => setViewing(item)}
                            >
                              Open evidence
                            </Button>
                          )}
                      </Group>
                      {item.processing && (
                        <Text size="xs" c="dimmed">
                          Processing · {label(item.processing)}
                          {item.learning
                            ? ` · Last recorded learning ${label(item.learning.state)}${item.learning.state === "succeeded" ? ` · ${item.learning.accepted} accepted, ${item.learning.proposed} proposed` : ""}`
                            : " · No learning outcome recorded"}
                        </Text>
                      )}
                      <details className="feature-details">
                        <summary>Technical details</summary>
                        <Stack gap={4} mt="sm">
                          <Text size="xs">
                            {label(item.host)} {item.host_version} · event{" "}
                            {item.receipt.event_id}
                          </Text>
                          <Text size="xs">
                            Scope: {item.selection.repository_ids?.length ?? 0}{" "}
                            repositories ·{" "}
                            {item.selection.area_ids?.length ?? 0} areas
                            {item.selection.environment_id
                              ? " · selected environment"
                              : ""}
                          </Text>
                          {item.managed_call_id && (
                            <Text size="xs">
                              Managed call {item.managed_call_id}
                            </Text>
                          )}
                          {item.learning && (
                            <Text size="xs">
                              Learning state recorded{" "}
                              {time(
                                item.learning.finished_at ??
                                  item.learning.job.updated_at,
                              )}
                            </Text>
                          )}
                          {!!item.event?.coverage.length && (
                            <Text size="xs">
                              Coverage:{" "}
                              {item.event.coverage.map(label).join(" · ")}
                            </Text>
                          )}
                          {item.receipt.expires_at && (
                            <Text size="xs">
                              Content deadline: {time(item.receipt.expires_at)}
                            </Text>
                          )}
                          {!item.source_available && (
                            <Text size="xs">
                              No retained source text available.
                            </Text>
                          )}
                        </Stack>
                      </details>
                    </Stack>
                  </Card>
                ))
              ))}
            {!events.isError && events.data && (
              <Pages
                offset={offset}
                total={events.data.total}
                change={setOffset}
              />
            )}
          </>
        )}
        <Drawer
          className="feature-drawer"
          opened={coverage && !deviceId}
          onClose={() => setCoverage(false)}
          position="right"
          title="Capture diagnostics"
          size="lg"
        >
          {coverage && !deviceId && (
            <Stack>
              <Failure error={managed.error ?? devices.error} />
              {!managed.isError && managed.data && (
                <Text size="sm">
                  {managed.data.pending} pending · {managed.data.errors}{" "}
                  retrying · {managed.data.published} published
                  {managed.data.unknown
                    ? ` · ${managed.data.unknown} outcomes unknown`
                    : ""}
                </Text>
              )}
              {!devices.isError &&
                devices.data?.items.map((item) => (
                  <CaptureDevice key={item.device_id} item={item} />
                ))}
              {!devices.isError && devices.data && (
                <Pages
                  offset={deviceOffset}
                  total={devices.data.total}
                  change={setDeviceOffset}
                />
              )}
            </Stack>
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
      </Stack>
    </section>
  );
}
