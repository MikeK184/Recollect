import { useState } from "react";
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
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import { EraseAction } from "./RetentionPanel";
import { useContentDeadline } from "./useContentDeadline";

type Settings = components["schemas"]["CaptureSettings"];
type Event = components["schemas"]["CaptureEventView"];
type Device = components["schemas"]["CaptureDeviceView"];
const labels: Record<string, string> = {
  prompt: "Prompt",
  reply: "Reply",
  tool_result: "Tool result",
  lifecycle: "Lifecycle",
  codex: "Codex",
  claude_code: "Claude Code",
  managed_mcp: "Managed tool",
  ManagedTool: "Tool outcome",
  ManagedReceipt: "Later receipt",
  ManagedResolution: "Evidence resolution",
};
const label = (value: string) => labels[value] ?? value.replaceAll("_", " ");
const time = (value: string) => new Date(value).toLocaleString();
const kinds = ["prompt", "reply", "tool_result", "lifecycle"].map((value) => ({
  value,
  label: label(value),
}));
const word = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;
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
function PolicyEditor({
  brain,
  settings,
  close,
}: {
  brain: string;
  settings: Settings;
  close: () => void;
}) {
  const [draft, setDraft] = useState(structuredClone(settings.policy));
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
        base_change: settings.change_id,
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
  return (
    <Modal opened onClose={close} title="Session capture policy" size="lg">
      <Stack>
        <Text size="sm">
          Supported prompts, replies and tool results are captured automatically
          under this policy. Model learning uses the separate model policy;
          memories do not wait for individual approval.
        </Text>
        <Switch
          label="Enable automatic session capture"
          checked={draft.enabled}
          onChange={(e) =>
            setDraft({ ...draft, enabled: e.currentTarget.checked })
          }
        />
        <MultiSelect
          label="Captured events"
          data={kinds}
          value={draft.kinds}
          onChange={(value) => setDraft({ ...draft, kinds: value })}
        />
        <Switch
          label="Capture managed tool results"
          description="Future permitted results from writers become evidence shared with this Brain's readers. Profile Use and model-provider permissions remain separate."
          checked={draft.managed_tools ?? false}
          onChange={(e) =>
            setDraft({ ...draft, managed_tools: e.currentTarget.checked })
          }
        />
        <NumberInput
          label="Maximum text per event · KiB"
          min={1}
          max={64}
          allowDecimal={false}
          value={draft.max_event_bytes / 1024}
          onChange={(value) =>
            setDraft({
              ...draft,
              max_event_bytes: typeof value === "number" ? value * 1024 : 0,
            })
          }
        />
        <Textarea
          label="Excluded tools"
          description="One name or pattern per line; up to 20."
          autosize
          minRows={2}
          value={tools}
          onChange={(e) => setTools(e.currentTarget.value)}
        />
        <Textarea
          label="Excluded content"
          description="One literal phrase per line; matching events are omitted. Do not enter credentials."
          autosize
          minRows={2}
          value={content}
          onChange={(e) => setContent(e.currentTarget.value)}
        />
        <Text size="sm" c="dimmed">
          Credential redaction and sensitive file exclusions always apply. Raw
          session and tool content follow this Brain's retention policy,
          initially 30 days.
        </Text>
        <Failure error={save.error} />
        <Button loading={save.isPending} onClick={() => save.mutate()}>
          Save capture policy
        </Button>
      </Stack>
    </Modal>
  );
}
function Setup({ brain, close }: { brain: string; close: () => void }) {
  const [host, setHost] = useState("codex");
  const [directory, setDirectory] = useState(".");
  const command = `cargo run -p recollect-agent -- capture setup ${host} ${word(directory)} --brain ${brain}`;
  return (
    <Modal opened onClose={close} title="Connect session capture" size="lg">
      <Stack>
        <Text size="sm">
          Use your paired native companion. Run this command from the Recollect
          checkout; the workspace can be an existing repository elsewhere on
          this computer.
        </Text>
        <Select
          label="Agent host"
          value={host}
          onChange={(value) => setHost(value ?? "codex")}
          data={[
            { value: "codex", label: "Codex" },
            { value: "claude_code", label: "Claude Code" },
          ]}
        />
        <TextInput
          label="Host workspace directory"
          value={directory}
          onChange={(e) => setDirectory(e.currentTarget.value)}
        />
        <Code
          block
          style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}
        >
          {command}
        </Code>
        <Text size="sm">
          Setup returns the launch command and Brain URL. Run the launch command
          to start the host with automatic background upload. To use an existing
          task's scope, add <Code>--task TASK_UUID</Code> during setup.
        </Text>
        {host === "codex" && (
          <Text size="sm">
            The launch registers Recollect's capture plugin through Codex. It
            stays inactive in sessions that were not launched with a Recollect
            capture binding. The bundle also carries memory skills describing
            the existing scoped MCP tools; skills grant no authority.
          </Text>
        )}
        <Text size="sm">
          The host's initial hook trust and workspace permissions still apply.
          Check companion activity below for actual delivery. Unsupported events
          and ambiguous child attribution are shown as coverage gaps.
        </Text>
      </Stack>
    </Modal>
  );
}
function Companion({ item }: { item: Device }) {
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
          <Text fw={600}>Companion {item.device_id.slice(0, 8)}</Text>
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
            : " · no companion report yet"}
        </Text>
        {report?.issue && (
          <Text size="sm">
            {label(report.issue)}. Restart the companion or run its drain
            command after resolving access or connectivity.
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
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/sources/{source}/versions/{version}",
          { params: { path: { brain: brain.id, source, version } } },
        ),
      ),
    refetchInterval: 4000,
    gcTime: 0,
    retry: false,
  });
  const expired = useContentDeadline(evidence.data?.version.expires_at);
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
                <pre
                  className="source-content"
                  data-testid="capture-source-content"
                >
                  {evidence.data.content}
                </pre>
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
  simple = false,
}: {
  brain: Brain;
  section?: "sessions" | "settings";
  simple?: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [coverage, setCoverage] = useState(false);
  const [setup, setSetup] = useState(false);
  const [kind, setKind] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [deviceOffset, setDeviceOffset] = useState(0);
  const [viewing, setViewing] = useState<Event | null>(null);
  const path = { brain: brain.id };
  const managed = useQuery({
    queryKey: ["capture-managed", brain.id],
    enabled: section === "sessions" && coverage,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/capture/managed", {
          params: { path },
        }),
      ),
    refetchInterval: 4000,
  });
  const policy = useQuery({
    queryKey: ["capture-policy", brain.id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/capture/policy", {
          params: { path },
        }),
      ),
    refetchInterval: 4000,
  });
  const devices = useQuery({
    queryKey: ["capture-devices", brain.id, deviceOffset],
    enabled: section === "sessions" && coverage,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/capture/devices", {
          params: { path, query: { offset: deviceOffset } },
        }),
      ),
    refetchInterval: 4000,
  });
  const events = useQuery({
    queryKey: ["capture-events", brain.id, offset, kind],
    enabled: section === "sessions",
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/capture/events", {
          params: { path, query: { offset, ...(kind ? { kind } : {}) } },
        }),
      ),
    refetchInterval: 4000,
  });
  return (
    <section
      className={section === "settings" ? "feature-setting" : "feature-view"}
      aria-label="Session capture"
    >
      <Stack>
        <Group justify="space-between">
          <Title order={3}>
            {section === "settings"
              ? simple
                ? "Connect agent capture"
                : "Capture policy"
              : "Captured sessions"}
          </Title>
          <Group>
            {section === "settings" &&
              brain.role !== "reader" &&
              !brain.archived && (
                <Button variant="light" onClick={() => setSetup(true)}>
                  Connect capture
                </Button>
              )}
            {section === "settings" &&
              !simple &&
              brain.role === "admin" &&
              !brain.archived && (
                <Button
                  variant="light"
                  disabled={!policy.data}
                  onClick={() => setEditing(true)}
                >
                  Capture policy
                </Button>
              )}
            {section === "sessions" && (
              <>
                <Button variant="default" onClick={() => setCoverage(true)}>
                  Capture coverage
                </Button>
                <Button
                  variant="subtle"
                  component="a"
                  href={`/brains/${brain.id}/settings?tab=capture`}
                >
                  Capture settings
                </Button>
              </>
            )}
          </Group>
        </Group>
        <Text size="sm">
          Automatic evidence from supported agent hooks and enabled managed
          tools. Learning and revision follow the model policy; expiry and
          erasure follow the retention policy.
        </Text>
        <Failure
          error={policy.error ?? devices.error ?? events.error ?? managed.error}
        />
        {simple && !policy.error && policy.data && (
          <Text size="sm">
            {policy.data.policy.enabled
              ? "Supported prompts, replies and tool results are captured automatically once your agent is connected. Secrets are redacted before storage."
              : "Enable autonomous memory once under AI & automation, then connect your agent here. No capture tuning is required."}
          </Text>
        )}
        {!simple && !policy.error && policy.data ? (
          <Group>
            <Badge color={policy.data.policy.enabled ? "teal" : "gray"}>
              {policy.data.policy.enabled
                ? "Capture enabled"
                : "Capture disabled"}
            </Badge>
            <Badge
              color={
                policy.data.policy.enabled && policy.data.policy.managed_tools
                  ? "teal"
                  : "gray"
              }
            >
              Managed tools{" "}
              {policy.data.policy.enabled && policy.data.policy.managed_tools
                ? "enabled"
                : "disabled"}
            </Badge>
            <Text size="sm">
              {policy.data.policy.kinds.map(label).join(" · ")} · up to{" "}
              {policy.data.policy.max_event_bytes / 1024} KiB per event
            </Text>
          </Group>
        ) : (
          policy.isPending && <Loader size="sm" />
        )}
        <Drawer
          className="feature-drawer"
          opened={coverage}
          onClose={() => setCoverage(false)}
          position="right"
          title="Capture coverage"
          size="lg"
        >
          <Stack>
            {!managed.error && managed.data && (
              <Stack gap={2}>
                <Text size="sm">
                  Visible managed calls: {managed.data.pending} pending ·{" "}
                  {managed.data.errors} retrying · {managed.data.published}{" "}
                  published · {managed.data.filtered} without retained text ·{" "}
                  {managed.data.skipped} skipped
                </Text>
                {!!managed.data.unknown && (
                  <Text size="sm" c="dimmed">
                    {managed.data.unknown} outcomes remain unknown; later
                    receipts have separate records.
                  </Text>
                )}
                {managed.data.last_publication && (
                  <Text size="xs" c="dimmed">
                    Last managed publication:{" "}
                    {time(managed.data.last_publication)}
                  </Text>
                )}
                {managed.data.oldest_pending && (
                  <Text size="xs" c="dimmed">
                    Oldest pending outcome: {time(managed.data.oldest_pending)}
                  </Text>
                )}
              </Stack>
            )}
            {devices.data?.total === 0 && (
              <Alert color="gray">
                No capture companion is configured for this Brain.
              </Alert>
            )}
            {!devices.error &&
              devices.data?.items.map((item) => (
                <Companion key={item.device_id} item={item} />
              ))}
            {devices.data && (
              <Pages
                offset={deviceOffset}
                total={devices.data.total}
                change={setDeviceOffset}
              />
            )}
          </Stack>
        </Drawer>
        {section === "settings" && !simple && (
          <Text size="sm" c="dimmed">
            Capture permission allows supported events to be recorded. Sending
            those records to a model requires a separate AI policy. Expiry and
            erasure are controlled in Retention &amp; privacy.
          </Text>
        )}
        {section === "sessions" && (
          <>
            <Group justify="space-between">
              <Title order={3}>Published activity</Title>
              <Select
                aria-label="Capture event kind"
                placeholder="All event kinds"
                clearable
                data={kinds}
                value={kind}
                onChange={(v) => {
                  setKind(v);
                  setOffset(0);
                }}
              />
            </Group>
            {events.isPending ? (
              <Loader size="sm" />
            ) : (
              events.data?.total === 0 && (
                <Text size="sm" c="dimmed">
                  No published events match this view. A configured hook does
                  not prove delivery.
                </Text>
              )
            )}
            {!events.error &&
              !policy.error &&
              events.data?.items.map((item) => (
                <Card withBorder padding="sm" key={item.receipt.event_id}>
                  <Stack gap="xs">
                    <Group justify="space-between">
                      <Text fw={600}>
                        {item.event ? label(item.event.kind) : "Removed event"}{" "}
                        · {label(item.host)}
                      </Text>
                      <Badge
                        color={
                          item.receipt.state === "accepted" ? "teal" : "gray"
                        }
                      >
                        {item.receipt.state}
                      </Badge>
                    </Group>
                    <Text size="xs" c="dimmed">
                      {item.event
                        ? time(item.event.captured_at)
                        : time(item.receipt.received_at)}{" "}
                      · host {item.host_version}
                    </Text>
                    <Text size="sm">
                      Scope:{" "}
                      {!item.selection.repository_ids?.length &&
                      !item.selection.area_ids?.length &&
                      !item.selection.environment_id
                        ? "Brain-wide"
                        : `${item.selection.repository_ids?.length ?? 0} repositories · ${item.selection.area_ids?.length ?? 0} areas${item.selection.environment_id ? " · selected environment" : ""}`}
                    </Text>
                    {item.managed_call_id && (
                      <Text size="xs">Managed call {item.managed_call_id}</Text>
                    )}
                    {item.event && (
                      <Text size="sm">
                        {label(item.event.host_event)} ·{" "}
                        {label(item.event.outcome)}
                        {item.event.tool_name
                          ? ` · ${item.event.tool_name}`
                          : ""}
                      </Text>
                    )}
                    {!!item.event?.coverage.length && (
                      <Text size="xs" c="dimmed">
                        Coverage: {item.event.coverage.map(label).join(" · ")}
                      </Text>
                    )}
                    {item.receipt.expires_at && (
                      <Text size="xs">
                        Content deadline: {time(item.receipt.expires_at)}
                      </Text>
                    )}
                    {item.source_available &&
                    item.receipt.source_id &&
                    item.receipt.source_version_id ? (
                      <Button
                        variant="subtle"
                        size="compact-sm"
                        onClick={() => setViewing(item)}
                      >
                        View captured source
                      </Button>
                    ) : (
                      <Text size="xs" c="dimmed">
                        No retained source text
                      </Text>
                    )}
                  </Stack>
                </Card>
              ))}
            {events.data && (
              <Pages
                offset={offset}
                total={events.data.total}
                change={setOffset}
              />
            )}
          </>
        )}
        {editing && policy.data && !policy.error && (
          <PolicyEditor
            brain={brain.id}
            settings={policy.data}
            close={() => setEditing(false)}
          />
        )}
        {setup && <Setup brain={brain.id} close={() => setSetup(false)} />}
        {viewing && (
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
