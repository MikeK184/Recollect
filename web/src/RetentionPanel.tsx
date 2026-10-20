import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Eraser, Trash2, TriangleAlert } from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { PolicyRow } from "./features/settings/PolicyRow";
import { useIdempotency } from "./useIdempotency";
import { useLineageOverlay } from "./features/knowledge/selection";
import { usePollInterval } from "./usePollInterval";
import { EmptyState } from "./components/AsyncState";
import { SkeletonRows } from "./components/Skeleton";
import {
  Alert,
  Badge,
  Button,
  Card,
  CardBar,
  Modal,
  NumberField,
  NumberInput,
  Select,
  Spinner,
  StatusBadge,
  Switch,
  TextField,
} from "./components/ui";
import { cn } from "./lib/utils";
import { useFirstLoadStagger } from "./features/activity/firstLoad";
type Policy = components["schemas"]["RetentionPolicy"];
type Settings = components["schemas"]["RetentionSettings"];
type Erasure = components["schemas"]["ErasureStatus"];
type Target = components["schemas"]["ErasureTarget"];
const label = (s: string) => s.replaceAll("_", " ");
const time = (s: string) => new Date(s).toLocaleString();
function Failure({ error }: { error: Error | null }) {
  return error ? <Alert variant="destructive">{error.message}</Alert> : null;
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
export function RetentionPolicyEditor({
  brain,
  settings,
  onClose,
  inline = false,
  fields,
  readOnly = false,
}: {
  brain: string;
  settings: Settings;
  onClose: () => void;
  inline?: boolean;
  fields?: (keyof Policy)[];
  readOnly?: boolean;
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
      await Promise.all([
        cache.resetQueries({
          predicate: (q) =>
            q.queryKey.includes(brain) && q.queryKey[0] !== "brain",
        }),
        cache.invalidateQueries({ queryKey: ["brain", brain] }),
      ]);
      onClose();
    },
  });
  const form = (
    <div className="grid gap-4">
      <div className={cn("grid", inline ? "gap-0" : "grid-cols-2 gap-x-4 gap-y-3")}>
        {policyFields
          .filter((f) => !fields || fields.includes(f.key))
          .map((f) =>
            inline ? (
              <PolicyRow
                key={f.key}
                title={f.key === "claim_days" ? "Memory" : f.title}
                description={
                  f.key === "raw_session_days"
                    ? "Conversation events and session text"
                    : f.key === "tool_output_days"
                      ? "Captured results from managed tools"
                      : f.optional
                        ? "Blank keeps content until erased."
                        : "Expiry runs automatically."
                }
              >
                {readOnly && f.optional ? (
                  <span className="mono text-label text-foreground">
                    {settings.policy[f.key] == null
                      ? "Until erased"
                      : `${settings.policy[f.key]} days`}
                  </span>
                ) : (
                  <div className="flex items-center gap-2">
                    <NumberInput
                      aria-label={f.title + " · days"}
                      min={1}
                      max={f.max ?? 3650}
                      allowDecimal={false}
                      value={
                        readOnly
                          ? ((settings.policy[f.key] as number | null) ?? "")
                          : ((draft[f.key] as number | null) ?? "")
                      }
                      placeholder={f.optional ? "Until erased" : undefined}
                      readOnly={readOnly}
                      hideControls={readOnly}
                      className="w-32"
                      onChange={(v) =>
                        setDraft({
                          ...draft,
                          [f.key]:
                            typeof v === "number" ? v : f.optional ? null : 0,
                        })
                      }
                    />
                    <Select
                      aria-label={f.title + " duration unit"}
                      data={["days"]}
                      value="days"
                      onChange={() => undefined}
                      readOnly
                      className="w-24"
                    />
                  </div>
                )}
              </PolicyRow>
            ) : (
              <NumberField
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
            ),
          )}
      </div>
      {(!fields || fields.includes("allow_support_excerpts")) && (
        <PolicyRow title="Supporting excerpt capture">
          {readOnly ? (
            <span className="text-body text-foreground">
              {settings.policy.allow_support_excerpts ? "Allowed" : "Disabled"}
            </span>
          ) : (
            <Switch
              aria-label="Allow explicitly retained supporting excerpts"
              checked={draft.allow_support_excerpts}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  allow_support_excerpts: e.currentTarget.checked,
                })
              }
            />
          )}
        </PolicyRow>
      )}
      {(!fields || fields.includes("backup_days")) && (
        <p className="m-0 text-caption text-muted-foreground">
          The backup window is recorded for the forthcoming backup adapter.
          External copies remain outside this installation’s control.
        </p>
      )}
      <Failure error={save.error} />
      {!readOnly && (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="m-0 flex items-start gap-2 text-caption text-warning-text">
            <TriangleAlert className="mt-px size-3.5 shrink-0" aria-hidden="true" />
            <span>
              Shortening a duration can expire existing content immediately.
              Extending it does not recover removed content.
            </span>
          </p>
          <div className="flex gap-2">
            <Button
              variant="secondary"
              disabled={save.isPending}
              onClick={onClose}
            >
              Cancel
            </Button>
            <Button loading={save.isPending} onClick={() => save.mutate()}>
              Save changes
            </Button>
          </div>
        </div>
      )}
    </div>
  );
  return inline ? (
    form
  ) : (
    <Modal
      opened
      onClose={() => !save.isPending && onClose()}
      title="Retention policy"
      size="lg"
    >
      {form}
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
    <div className="grid gap-2.5">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
        <StatusBadge
          state={
            current.state === "complete"
              ? "positive"
              : current.state === "error"
                ? "negative"
                : "attention"
          }
        >
          {current.state === "complete"
            ? "Central cleanup complete"
            : label(current.state)}
        </StatusBadge>
        <span className="mono text-caption text-foreground-secondary">
          {current.pending_artifacts} files pending · journal{" "}
          {current.journaled ? "saved" : "pending"}
          {current.graph_pending
            ? " · graph cleanup pending"
            : " · graph cleanup complete"}
        </span>
      </div>
      {current.error_code && (
        <Alert variant="destructive">
          {label(current.error_code)}. Content is unavailable while cleanup
          retries.
        </Alert>
      )}
      <p className="m-0 text-caption text-muted-foreground">
        <span className="mono">{current.acknowledged_devices ?? 0}</span>{" "}
        devices have acknowledged this deletion position. Host-side copies
        require device check-in or their known expiry. Central completion does
        not confirm removal from offline devices or external backups.
      </p>
      {current.state !== "complete" && (
        <div>
          <Button
            variant="secondary"
            size="sm"
            loading={retry.isPending}
            onClick={() => retry.mutate()}
          >
            Retry cleanup
          </Button>
        </div>
      )}
      <Failure error={retry.error} />
    </div>
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
        variant="destructive-ghost"
        size="sm"
        onClick={() => {
          erase.reset();
          setAttempt((a) => a + 1);
          setOpened(true);
        }}
      >
        <Eraser aria-hidden="true" />
        Erase {label(target.kind)}
      </Button>
      {opened && (
        <Modal
          opened
          onClose={() => setOpened(false)}
          title="Erase controlled memory"
          size="lg"
          footer={
            erase.data ? undefined : (
              <>
                <Button
                  variant="secondary"
                  disabled={erase.isPending}
                  onClick={() => {
                    erase.reset();
                    setAttempt((a) => a + 1);
                  }}
                >
                  Refresh erasure preview
                </Button>
                <Button
                  variant="destructive"
                  loading={erase.isPending}
                  disabled={
                    !preview.data || preview.isFetching || !!preview.error
                  }
                  onClick={() => erase.mutate()}
                >
                  <Trash2 aria-hidden="true" />
                  Confirm erasure
                </Button>
              </>
            )
          }
        >
          <div className="grid gap-3">
            {erase.data ? (
              <>
                <Alert variant="warning" role="status">
                  Content is now unavailable. Cleanup progress is shown below.
                </Alert>
                <ErasureProgress brain={brain} item={erase.data} />
              </>
            ) : (
              <>
                <p className="m-0 break-words text-title font-semibold text-foreground [overflow-wrap:anywhere]">
                  {name}
                </p>
                <p className="m-0 text-body text-foreground-secondary">
                  This permanently removes controlled content and dependent
                  revisions. Erasing the current revision does not restore an
                  older assertion.
                </p>
                {preview.isPending && <Spinner label="Previewing erasure" />}
                <Failure error={preview.error} />
                {preview.data && (
                  <div className="grid gap-2 rounded-lg border border-border bg-muted/50 px-3.5 py-3">
                    <p className="m-0 mono text-label text-foreground">
                      {preview.data.source_versions} source versions ·{" "}
                      {preview.data.claim_revisions} claim revisions ·{" "}
                      {preview.data.snapshots} snapshots ·{" "}
                      {preview.data.manifest_revisions} manifest revisions
                    </p>
                    <p className="m-0 mono text-caption text-muted-foreground">
                      {preview.data.artifacts} stored files ·{" "}
                      {preview.data.jobs} queued or running jobs ·{" "}
                      {preview.data.independent_claim_revisions} independent
                      claim revisions preserved
                    </p>
                    {!!preview.data.model_input_fences && (
                      <p className="m-0 text-label text-foreground-secondary">
                        {preview.data.model_input_fences} supporting source
                        versions will be fenced from further model transmission.
                        Independently retained evidence remains inspectable.
                      </p>
                    )}
                    {!!preview.data.capture_event_fences && (
                      <p className="m-0 text-label text-foreground-secondary">
                        {preview.data.capture_event_fences} capture events will
                        be removed from host inboxes during synchronization.
                      </p>
                    )}
                    {!!preview.data.model_claim_fences && (
                      <p className="m-0 text-label text-foreground-secondary">
                        {preview.data.model_claim_fences} contributing claim
                        revisions will be fenced from further model
                        transmission. Independent records remain inspectable.
                      </p>
                    )}
                  </div>
                )}
                {preview.data?.shared_sources && (
                  <Alert variant="warning">
                    Some selected sources also belong to other collections.
                    Erasing them removes their content throughout this Brain.
                  </Alert>
                )}
                <Failure error={erase.error} />
              </>
            )}
          </div>
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
  useLineageOverlay(opened);
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
        size="sm"
        variant="secondary"
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
          description="Select exact lines from this source, up to 8 KiB. The excerpt has its own retention period and preserves provenance. Explicit source erasure also removes its excerpts."
          footer={
            save.data ? undefined : (
              <Button
                loading={save.isPending}
                disabled={!name.trim() || first < 1 || last < first}
                onClick={() => save.mutate()}
              >
                Create excerpt
              </Button>
            )
          }
        >
          <div className="grid gap-4">
            <TextField
              label="Excerpt title"
              value={name}
              maxLength={120}
              onChange={(e) => setName(e.currentTarget.value)}
            />
            <div className="grid grid-cols-2 gap-3">
              <NumberField
                label="First line"
                value={first}
                min={1}
                allowDecimal={false}
                onChange={(v) => setFirst(Number(v))}
              />
              <NumberField
                label="Last line"
                value={last}
                min={first}
                allowDecimal={false}
                onChange={(v) => setLast(Number(v))}
              />
            </div>
            <Failure error={save.error} />
            {save.data && (
              <Alert variant="brand" role="status">
                Excerpt retained. It is available as a separate source.
              </Alert>
            )}
          </div>
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
  onInspectorChange,
}: {
  brain: Brain;
  section?: "settings" | "activity";
  simple?: boolean;
  selectedRequest?: string;
  onInspectorChange?: (opened: boolean) => void;
}) {
  const pollSlow = usePollInterval("slow");
  const pollNormal = usePollInterval("normal");
  const focused = useRef<HTMLDivElement>(null);
  const [editing, setEditing] = useState(false);
  useEffect(() => {
    onInspectorChange?.(editing);
    return () => onInspectorChange?.(false);
  }, [editing, onInspectorChange]);
  const [offset, setOffset] = useState(0);
  const policy = useQuery({
    queryKey: ["retention", brain.id],
    enabled: section === "settings",
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/retention", {
          signal,
          params: { path: { brain: brain.id } },
        }),
      ),
    refetchInterval: pollSlow,
    retry: false,
    gcTime: 0,
  });
  useEffect(() => {
    if (brain.role !== "admin" || brain.archived || policy.isError)
      setEditing(false);
  }, [brain.role, brain.archived, policy.isError]);
  const erasures = useQuery({
    queryKey: ["erasures", brain.id, offset],
    enabled: section === "activity",
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/erasures", {
          params: { path: { brain: brain.id }, query: { offset } },
        }),
      ),
    refetchInterval: pollNormal,
  });
  const stagger = useFirstLoadStagger(erasures.data?.items.map((i) => i.id));
  useEffect(() => {
    if (selectedRequest && erasures.data)
      focused.current?.scrollIntoView({ block: "center" });
  }, [selectedRequest, erasures.data]);
  return (
    <Card className="overflow-hidden p-0" role="region" aria-label={section === "settings" ? "Retention" : "Data removal"}>
      <CardBar>
        <h3>{section === "settings" ? "Retention" : "Data removal"}</h3>
        {section === "activity" && (
          <span className="text-caption text-muted-foreground">
            Erasure and expiry requests with their cleanup progress.
          </span>
        )}
        {section === "settings" && brain.role === "admin" && !brain.archived && (
          <Button
            variant="secondary"
            size="sm"
            className="ml-auto"
            disabled={!policy.data}
            onClick={() => setEditing(true)}
          >
            Edit retention
          </Button>
        )}
      </CardBar>
      <div className="grid gap-3 p-4">
        {simple && (
          <p className="m-0 text-body text-foreground-secondary">
            Retention runs automatically. Durations differ by content type.
          </p>
        )}
        {section === "settings" && (
          <>
            <Failure error={policy.error} />
            {policy.isPending && <SkeletonRows rows={2} label="Loading retention…" />}
            {!policy.error && policy.data && (
              <>
                <p className="m-0 text-body text-foreground-secondary">
                  Raw sessions:{" "}
                  <span className="mono">
                    {policy.data.policy.raw_session_days} days
                  </span>{" "}
                  · raw tool output:{" "}
                  <span className="mono">
                    {policy.data.policy.tool_output_days} days
                  </span>
                  . Excerpts are{" "}
                  {policy.data.policy.allow_support_excerpts
                    ? "permitted"
                    : "disabled"}
                  .
                </p>
                <div className="flex flex-wrap gap-1.5">
                  {policyFields
                    .filter((f) => f.optional)
                    .map((f) => (
                      <Badge variant="outline" key={f.key}>
                        {f.title}:{" "}
                        {policy.data!.policy[f.key] == null
                          ? "until erased"
                          : String(policy.data!.policy[f.key]) + " days"}
                      </Badge>
                    ))}
                </div>
              </>
            )}
            <details className="group rounded-md border border-border px-3 py-2">
              <summary className="cursor-pointer text-label font-medium text-foreground">
                Retention details and history
              </summary>
              <div className="mt-2 grid gap-2">
                <p className="m-0 text-body text-foreground-secondary">
                  Open evidence to preview erasure. Turning storage off only
                  affects future captures.
                </p>
                {brain.role === "admin" && (
                  <div>
                    <Button variant="ghost" size="sm" asChild>
                      <a href={`/brains/${brain.id}/activity?tab=timeline`}>
                        View policy changes in activity
                      </a>
                    </Button>
                  </div>
                )}
              </div>
            </details>
          </>
        )}
        {section === "activity" && (
          <>
            <Failure error={erasures.error} />
            {erasures.isPending ? (
              <SkeletonRows rows={3} label="Loading data removal…" />
            ) : erasures.data?.items.length === 0 ? (
              <EmptyState
                icon={Eraser}
                title="No erasure or expiry requests"
                description="Requests appear here when content is erased or expires."
                className="py-8"
              />
            ) : (
              !erasures.error &&
              erasures.data?.items.map((item, index) => {
                const selected = item.id === selectedRequest;
                const entrance = stagger(item.id, index);
                return (
                  <div
                    key={item.id}
                    ref={selected ? focused : undefined}
                    className={cn(
                      "grid gap-2.5 rounded-lg border border-border bg-card px-4 py-3",
                      selected && "border-brand bg-brand-soft/40",
                      entrance.className,
                    )}
                    style={entrance.style}
                    data-testid={selected ? "selected-erasure" : undefined}
                  >
                    <p className="m-0 flex flex-wrap items-center gap-x-2 text-body text-foreground">
                      <span className="font-medium capitalize">
                        {label(item.cause)}
                      </span>
                      <span className="text-muted-foreground">
                        · {label(item.target.kind)} ·
                      </span>
                      <span className="mono text-caption text-muted-foreground">
                        {time(item.created_at)}
                      </span>
                    </p>
                    {brain.role === "admin" ? (
                      <ErasureProgress brain={brain.id} item={item} />
                    ) : (
                      <div className="flex flex-wrap items-center gap-2">
                        <Badge>{label(item.state)}</Badge>
                        <span className="text-caption text-muted-foreground">
                          <span className="mono">{item.pending_artifacts}</span>{" "}
                          files pending.
                          {item.graph_pending
                            ? " Graph cleanup pending. "
                            : " "}
                          Offline host copies require check-in.
                        </span>
                      </div>
                    )}
                  </div>
                );
              })
            )}
            {erasures.data && erasures.data.total > 20 && (
              <div className="flex gap-2">
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={offset === 0}
                  onClick={() => setOffset(Math.max(0, offset - 20))}
                >
                  Previous requests
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={offset + 20 >= erasures.data.total}
                  onClick={() => setOffset(offset + 20)}
                >
                  Next requests
                </Button>
              </div>
            )}
          </>
        )}
      </div>
      {editing &&
        brain.role === "admin" &&
        !brain.archived &&
        policy.data &&
        !policy.error && (
          <RetentionPolicyEditor
            brain={brain.id}
            settings={policy.data}
            onClose={() => setEditing(false)}
          />
        )}
    </Card>
  );
}
