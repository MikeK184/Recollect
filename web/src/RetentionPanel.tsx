import { useEffect, useRef, useState } from "react";
import "./features/feature-views.css";
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Loader,
  Modal,
  NumberInput,
  Stack,
  Switch,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import { useLineageOverlay } from "./features/knowledge/selection";
type Policy = components["schemas"]["RetentionPolicy"];
type Settings = components["schemas"]["RetentionSettings"];
type Erasure = components["schemas"]["ErasureStatus"];
type Target = components["schemas"]["ErasureTarget"];
const label = (s: string) => s.replaceAll("_", " ");
const time = (s: string) => new Date(s).toLocaleString();
function Failure({ error }: { error: Error | null }) {
  return error ? <Alert color="red">{error.message}</Alert> : null;
}
const policyFields: {
  key: keyof Policy;
  title: string;
  optional?: boolean;
  max?: number;
}[] = [
  { key: "raw_session_days", title: "Raw sessions" },
  { key: "tool_output_days", title: "Raw tool output" },
  { key: "document_days", title: "Documents", optional: true },
  { key: "support_excerpt_days", title: "Supporting excerpts", optional: true },
  { key: "repository_days", title: "Repository snapshots", optional: true },
  {
    key: "claim_days",
    title: "Claims, decisions, procedures and handovers",
    optional: true,
  },
  { key: "audit_days", title: "Activity detail" },
  { key: "backup_days", title: "Managed backup window", max: 365 },
];
function PolicyEditor({
  brain,
  settings,
  onClose,
}: {
  brain: string;
  settings: Settings;
  onClose: () => void;
}) {
  const [draft, setDraft] = useState(settings.policy);
  const [base] = useState(settings.change_id);
  const cache = useQueryClient();
  const command = useIdempotency();
  const save = useMutation({
    mutationFn: async () => {
      const body = { base_change: base, policy: draft };
      return result(
        await client.PUT("/api/brains/{brain}/retention", {
          params: { path: { brain } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: async () => {
      await cache.resetQueries({
        predicate: (q) => q.queryKey.includes(brain),
      });
      onClose();
    },
  });
  return (
    <Modal opened onClose={onClose} title="Retention policy" size="lg">
      <Stack>
        <Text size="sm">
          Durations apply from capture time, including existing records.
          Shortening a duration can make content expire immediately. Extending
          it does not recover removed content.
        </Text>
        {policyFields.map((f) => (
          <NumberInput
            key={f.key}
            label={f.title + " · days"}
            description={
              f.optional ? "Leave blank to retain until erased." : undefined
            }
            min={1}
            max={f.max ?? 3650}
            allowDecimal={false}
            value={(draft[f.key] as number | null) ?? ""}
            onChange={(v) =>
              setDraft({
                ...draft,
                [f.key]: typeof v === "number" ? v : f.optional ? null : 0,
              })
            }
          />
        ))}
        <Switch
          label="Allow explicitly retained supporting excerpts"
          checked={draft.allow_support_excerpts}
          onChange={(e) =>
            setDraft({
              ...draft,
              allow_support_excerpts: e.currentTarget.checked,
            })
          }
        />
        <Text size="xs" c="dimmed">
          The backup window is recorded for the forthcoming backup adapter.
          External copies remain outside this installation's control.
        </Text>
        <Failure error={save.error} />
        <Button loading={save.isPending} onClick={() => save.mutate()}>
          Save retention policy
        </Button>
      </Stack>
    </Modal>
  );
}
function ErasureProgress({ brain, item }: { brain: string; item: Erasure }) {
  const cache = useQueryClient();
  const retry = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/brains/{brain}/erasures/{request}/retry", {
          params: { path: { brain, request: item.id } },
        }),
      ),
    onSuccess: async () => {
      await Promise.all([
        cache.resetQueries({
          predicate: (q) =>
            q.queryKey.includes(brain) && q.queryKey[0] !== "brain",
        }),
        cache.invalidateQueries({ queryKey: ["brain", brain] }),
      ]);
    },
  });
  const current = item.state === "complete" ? item : (retry.data ?? item);
  return (
    <Stack gap="xs">
      <Group>
        <Badge
          color={
            current.state === "complete"
              ? "teal"
              : current.state === "error"
                ? "red"
                : "orange"
          }
        >
          {current.state === "complete"
            ? "Central cleanup complete"
            : label(current.state)}
        </Badge>
        <Text size="sm">
          {current.pending_artifacts} files pending · journal{" "}
          {current.journaled ? "saved" : "pending"}
          {current.graph_pending
            ? " · graph cleanup pending"
            : " · graph cleanup complete"}
        </Text>
      </Group>
      {current.error_code && (
        <Alert color="red">
          {label(current.error_code)}. Content is unavailable while cleanup
          retries.
        </Alert>
      )}
      <Text size="xs" c="dimmed">
        {current.acknowledged_devices ?? 0} devices have acknowledged this
        deletion position. Host-side copies require device check-in or their
        known expiry. Central completion does not confirm removal from offline
        devices or external backups.
      </Text>
      {current.state !== "complete" && (
        <Button
          variant="light"
          size="xs"
          loading={retry.isPending}
          onClick={() => retry.mutate()}
        >
          Retry cleanup
        </Button>
      )}
      <Failure error={retry.error} />
    </Stack>
  );
}
export function EraseAction({
  brain,
  permitted,
  target,
  name,
}: {
  brain: string;
  permitted: boolean;
  target: Target;
  name: string;
}) {
  const [opened, setOpened] = useState(false);
  useLineageOverlay(opened);
  const [attempt, setAttempt] = useState(0);
  const cache = useQueryClient();
  const command = useIdempotency();
  const preview = useQuery({
    queryKey: ["erasure-preview", brain, target.kind, target.id, attempt],
    enabled: opened && permitted,
    queryFn: async () =>
      result(
        await client.POST("/api/brains/{brain}/erasures/preview", {
          params: { path: { brain } },
          body: target,
        }),
      ),
    retry: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });
  const erase = useMutation({
    mutationFn: async () => {
      if (!preview.data) throw new Error("Preview this erasure first.");
      const body = {
        target,
        eligibility_epoch: preview.data.eligibility_epoch,
      };
      return result(
        await client.POST("/api/brains/{brain}/erasures", {
          params: { path: { brain } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: async () => {
      await Promise.all([
        cache.resetQueries({
          predicate: (q) =>
            q.queryKey.includes(brain) && q.queryKey[0] !== "brain",
        }),
        cache.invalidateQueries({ queryKey: ["brain", brain] }),
      ]);
    },
  });
  if (!permitted) return null;
  return (
    <>
      <Button
        color="red"
        variant="subtle"
        size="xs"
        onClick={() => {
          erase.reset();
          setAttempt((a) => a + 1);
          setOpened(true);
        }}
      >
        Erase {label(target.kind)}
      </Button>
      {opened && (
        <Modal
          opened
          onClose={() => setOpened(false)}
          title="Erase controlled memory"
          size="lg"
        >
          <Stack>
            {erase.data ? (
              <>
                <Alert color="orange">
                  Content is now unavailable. Cleanup progress is shown below.
                </Alert>
                <ErasureProgress brain={brain} item={erase.data} />
              </>
            ) : (
              <>
                <Text fw={600} style={{ overflowWrap: "anywhere" }}>
                  {name}
                </Text>
                <Text size="sm">
                  This permanently removes controlled content and dependent
                  revisions. Erasing the current revision does not restore an
                  older assertion.
                </Text>
                {preview.isPending && <Loader size="sm" />}
                <Failure error={preview.error} />
                {preview.data && (
                  <>
                    <Text>
                      {preview.data.source_versions} source versions ·{" "}
                      {preview.data.claim_revisions} claim revisions ·{" "}
                      {preview.data.snapshots} snapshots ·{" "}
                      {preview.data.manifest_revisions} manifest revisions
                    </Text>
                    <Text size="sm">
                      {preview.data.artifacts} stored files ·{" "}
                      {preview.data.jobs} queued or running jobs ·{" "}
                      {preview.data.independent_claim_revisions} independent
                      claim revisions preserved
                    </Text>
                    {!!preview.data.model_input_fences && (
                      <Text size="sm">
                        {preview.data.model_input_fences} supporting source
                        versions will be fenced from further model transmission.
                        Independently retained evidence remains inspectable.
                      </Text>
                    )}
                    {!!preview.data.capture_event_fences && (
                      <Text size="sm">
                        {preview.data.capture_event_fences} capture events will
                        be removed from host inboxes during
                        synchronization.
                      </Text>
                    )}
                    {!!preview.data.model_claim_fences && (
                      <Text size="sm">
                        {preview.data.model_claim_fences} contributing claim
                        revisions will be fenced from further model
                        transmission. Independent records remain inspectable.
                      </Text>
                    )}
                    {preview.data.shared_sources && (
                      <Alert color="yellow">
                        Some selected sources also belong to other collections.
                        Erasing them removes their content throughout this
                        Brain.
                      </Alert>
                    )}
                  </>
                )}
                <Failure error={erase.error} />
                <Group>
                  <Button
                    variant="default"
                    disabled={erase.isPending}
                    onClick={() => {
                      erase.reset();
                      setAttempt((a) => a + 1);
                    }}
                  >
                    Refresh erasure preview
                  </Button>
                  <Button
                    color="red"
                    loading={erase.isPending}
                    disabled={
                      !preview.data || preview.isFetching || !!preview.error
                    }
                    onClick={() => erase.mutate()}
                  >
                    Confirm erasure
                  </Button>
                </Group>
              </>
            )}
          </Stack>
        </Modal>
      )}
    </>
  );
}
export function ExcerptAction({
  brain,
  source,
  version,
  title,
}: {
  brain: string;
  source: string;
  version: string;
  title: string;
}) {
  const [opened, setOpened] = useState(false);
  const [first, setFirst] = useState(1);
  const [last, setLast] = useState(1);
  const [name, setName] = useState((title + " excerpt").slice(0, 120));
  const cache = useQueryClient();
  const command = useIdempotency();
  const save = useMutation({
    mutationFn: async () => {
      const body = {
        source_id: source,
        version_id: version,
        first_line: first,
        last_line: last,
        title: name,
      };
      return result(
        await client.POST("/api/brains/{brain}/excerpts", {
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
    },
  });
  return (
    <>
      <Button
        size="xs"
        variant="light"
        onClick={() => {
          save.reset();
          setOpened(true);
        }}
      >
        Retain supporting excerpt
      </Button>
      {opened && (
        <Modal
          opened
          onClose={() => setOpened(false)}
          title="Retain supporting excerpt"
        >
          <Stack>
            <Text size="sm">
              Select exact lines from this source, up to 8 KiB. The excerpt has
              its own retention period and preserves provenance. Explicit source
              erasure also removes its excerpts.
            </Text>
            <TextInput
              label="Excerpt title"
              value={name}
              maxLength={120}
              onChange={(e) => setName(e.currentTarget.value)}
            />
            <NumberInput
              label="First line"
              value={first}
              min={1}
              allowDecimal={false}
              onChange={(v) => setFirst(Number(v))}
            />
            <NumberInput
              label="Last line"
              value={last}
              min={first}
              allowDecimal={false}
              onChange={(v) => setLast(Number(v))}
            />
            <Failure error={save.error} />
            {save.data ? (
              <Alert color="teal">
                Excerpt retained. It is available as a separate source.
              </Alert>
            ) : (
              <Button
                loading={save.isPending}
                disabled={!name.trim() || first < 1 || last < first}
                onClick={() => save.mutate()}
              >
                Create excerpt
              </Button>
            )}
          </Stack>
        </Modal>
      )}
    </>
  );
}
export function RetentionPanel({
  brain,
  section = "settings",
  simple = false,
  selectedRequest,
}: {
  brain: Brain;
  section?: "settings" | "activity";
  simple?: boolean;
  selectedRequest?: string;
}) {
  const focused = useRef<HTMLDivElement>(null);
  const [editing, setEditing] = useState(false);
  const [offset, setOffset] = useState(0);
  const policy = useQuery({
    queryKey: ["retention", brain.id],
    enabled: section === "settings",
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/retention", {
          params: { path: { brain: brain.id } },
        }),
      ),
  });
  const erasures = useQuery({
    queryKey: ["erasures", brain.id, offset],
    enabled: section === "activity",
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/erasures", {
          params: { path: { brain: brain.id }, query: { offset } },
        }),
      ),
    refetchInterval: 4000,
  });
  useEffect(() => {
    if (selectedRequest && erasures.data)
      focused.current?.scrollIntoView({ block: "center" });
  }, [selectedRequest, erasures.data]);
  return (
    <section className="feature-setting">
      <Stack>
        {simple && (
          <Text size="sm">
            Recollect enforces these retention rules in the background. It
            revises memory as evidence changes and excludes obsolete information
            from current recall. You do not need to manage individual expiry
            dates.
          </Text>
        )}
        <Group justify="space-between">
          <Title order={3}>
            {section === "settings" ? "Retention & privacy" : "Data removal"}
          </Title>
          {section === "settings" && !simple && brain.role === "admin" && (
            <Button
              variant="default"
              disabled={!policy.data}
              onClick={() => setEditing(true)}
            >
              Edit retention
            </Button>
          )}
        </Group>
        {section === "settings" && (
          <>
            <Failure error={policy.error} />
            {policy.isPending && <Loader size="sm" />}
            {!policy.error && policy.data && (
              <>
                <Text size="sm">
                  Raw sessions: {policy.data.policy.raw_session_days} days · raw
                  tool output: {policy.data.policy.tool_output_days} days.
                  Excerpts are{" "}
                  {policy.data.policy.allow_support_excerpts
                    ? "permitted"
                    : "disabled"}
                  .
                </Text>
                <Group gap="xs">
                  {policyFields
                    .filter((f) => f.optional)
                    .map((f) => (
                      <Badge variant="light" key={f.key}>
                        {f.title}:{" "}
                        {policy.data!.policy[f.key] == null
                          ? "until erased"
                          : String(policy.data!.policy[f.key]) + " days"}
                      </Badge>
                    ))}
                </Group>
              </>
            )}
            <Text size="sm" c="dimmed">
              Open a source, claim, snapshot or manifest to preview erasure.
              Collection erasure includes its associated sources.
            </Text>
          </>
        )}
        {section === "activity" && (
          <>
            <Failure error={erasures.error} />
            {erasures.isPending ? (
              <Loader size="sm" />
            ) : erasures.data?.items.length === 0 ? (
              <Text size="sm">No erasure or expiry requests.</Text>
            ) : (
              !erasures.error &&
              erasures.data?.items.map((item) => (
                <Card
                  withBorder
                  key={item.id}
                  p="md"
                  ref={item.id === selectedRequest ? focused : undefined}
                  className={
                    item.id === selectedRequest
                      ? "activity-selected"
                      : undefined
                  }
                  data-testid={
                    item.id === selectedRequest ? "selected-erasure" : undefined
                  }
                >
                  <Stack gap="sm">
                    <Text size="sm">
                      {label(item.cause)} · {label(item.target.kind)} ·{" "}
                      {time(item.created_at)}
                    </Text>
                    {brain.role === "admin" ? (
                      <ErasureProgress brain={brain.id} item={item} />
                    ) : (
                      <>
                        <Badge>{label(item.state)}</Badge>
                        <Text size="xs">
                          {item.pending_artifacts} files pending.
                          {item.graph_pending
                            ? " Graph cleanup pending. "
                            : " "}
                          Offline host copies require check-in.
                        </Text>
                      </>
                    )}
                  </Stack>
                </Card>
              ))
            )}
            {erasures.data && erasures.data.total > 20 && (
              <Group>
                <Button
                  variant="subtle"
                  disabled={offset === 0}
                  onClick={() => setOffset(Math.max(0, offset - 20))}
                >
                  Previous requests
                </Button>
                <Button
                  variant="subtle"
                  disabled={offset + 20 >= erasures.data.total}
                  onClick={() => setOffset(offset + 20)}
                >
                  Next requests
                </Button>
              </Group>
            )}
          </>
        )}
        {editing && policy.data && !policy.error && (
          <PolicyEditor
            brain={brain.id}
            settings={policy.data}
            onClose={() => setEditing(false)}
          />
        )}
      </Stack>
    </section>
  );
}
