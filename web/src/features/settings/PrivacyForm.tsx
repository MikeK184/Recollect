import { Fragment, useEffect, useState, type ReactNode } from "react";
import { useMutation } from "@tanstack/react-query";
import { Pencil, Shield, TriangleAlert } from "lucide-react";
import { client, result, type Brain } from "../../api";
import { useIdempotency } from "../../useIdempotency";
import { PolicyRow } from "./PolicyRow";
import {
  privacyChanges, privacyDraft, privacyIsCurrent, privacyLabels, savePrivacyDraft,
  type PrivacyDomain, type PrivacyDraft, type PrivacySnapshot,
} from "./privacyDraft";
import {
  Alert, Badge, Button, NumberInput, SegmentedControl, Switch, Tile,
} from "../../components/ui";
import { cn } from "../../lib/utils";

type Duration = Exclude<keyof PrivacyDraft["retention"], "allow_support_excerpts">;
export function PrivacyForm({ brain, snapshot, refresh, acknowledge }: {
  brain: Brain;
  snapshot: PrivacySnapshot;
  refresh: () => Promise<PrivacySnapshot>;
  acknowledge: (value: PrivacySnapshot, domain: PrivacyDomain) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [base, setBase] = useState(snapshot);
  const [draft, setDraft] = useState(() => privacyDraft(snapshot));
  const [savedDomains, setSavedDomains] = useState<PrivacyDomain[]>([]);
  const captureCommand = useIdempotency();
  const retentionCommand = useIdempotency();
  const canEdit = brain.role === "admin" && !brain.archived;
  useEffect(() => { if (!canEdit) setEditing(false); }, [canEdit]);
  const input = draft;
  const changes = privacyChanges(base, input);
  const stale = editing && !privacyIsCurrent(base, snapshot);
  const save = useMutation({
    mutationFn: async () => {
      if (!canEdit) throw new Error("Privacy editing is unavailable.");
      const current = await refresh();
      return savePrivacyDraft(base, current, input, {
        capture: async (base_change, policy) => {
          const body = { base_change, policy };
          return result(await client.PUT("/api/brains/{brain}/capture/policy", {
            params: { path: { brain: brain.id } }, body,
            headers: { "Idempotency-Key": captureCommand.forInput(body) },
          }));
        },
        retention: async (base_change, policy) => {
          const body = { base_change, policy };
          return result(await client.PUT("/api/brains/{brain}/retention", {
            params: { path: { brain: brain.id } }, body,
            headers: { "Idempotency-Key": retentionCommand.forInput(body) },
          }));
        },
        documents: async (allow_document_content) => {
          result(await client.PUT("/api/brains/{brain}/evidence/policy", {
            params: { path: { brain: brain.id } }, body: { allow_document_content },
          }));
          return allow_document_content;
        },
        repository: async (allow_file_content) => {
          result(await client.PUT("/api/brains/{brain}/repositories/policy", {
            params: { path: { brain: brain.id } }, body: { allow_file_content },
          }));
          return allow_file_content;
        },
      }, (value, domain) => {
        setBase(value);
        setSavedDomains((previous) => [...new Set([...previous, domain])]);
        acknowledge(value, domain);
      });
    },
    onSuccess: () => { setEditing(false); setSavedDomains([]); },
    onSettled: () => { void refresh().catch(() => undefined); },
  });
  const reset = () => {
    setBase(snapshot);
    setDraft(privacyDraft(snapshot));
    setSavedDomains([]);
    save.reset();
  };
  const readOnly = !editing || !canEdit || save.isPending;
  const value = editing ? input : privacyDraft(snapshot);
  const updateCapture = (patch: Partial<PrivacyDraft["capture"]>) =>
    setDraft((previous) => ({ ...previous, capture: { ...previous.capture, ...patch } }));
  const updateRetention = (patch: Partial<PrivacyDraft["retention"]>) =>
    setDraft((previous) => ({ ...previous, retention: { ...previous.retention, ...patch } }));
  // View mode shows values in the same column the inline controls occupy, so
  // entering Edit never moves a row.
  // Room for the unit suffix beside the value, with or without the steppers.
  const valueInput = cn(
    "text-right text-label",
    readOnly
      ? "border-transparent bg-transparent pr-11 shadow-none read-only:hover:border-transparent"
      : "pr-[66px]",
  );
  const duration = (key: Duration, title: string, optional = false) => (
    <PolicyRow key={key} title={title}>
      {readOnly && value.retention[key] == null ? (
        <span className="flex h-8 items-center pr-11 text-label text-foreground">Until erased</span>
      ) : (
        <div className="w-48">
          <NumberInput aria-label={title + " · days"} min={1}
            max={key === "backup_days" ? 365 : 3650} allowDecimal={false}
            value={value.retention[key] ?? ""} placeholder={optional ? "Until erased" : undefined}
            readOnly={readOnly} hideControls={readOnly} suffix="days"
            className={valueInput}
            onChange={(days) => updateRetention({ [key]: typeof days === "number" ? days : optional ? null : 0 })}
          />
        </div>
      )}
    </PolicyRow>
  );
  const toggle = (title: string, label: string, checked: boolean, update: (value: boolean) => void, description?: string) => (
    <PolicyRow title={title} description={description}><Switch aria-label={label} checked={checked} disabled={readOnly}
      className={cn(readOnly && "cursor-default opacity-100")}
      onChange={(event) => update(event.currentTarget.checked)} /></PolicyRow>
  );
  const section = (title: string, children: ReactNode) => (
    <section className="-mx-3 grid gap-1 border-t border-border pt-4 first-of-type:border-t-0 first-of-type:pt-0" aria-label={title}>
      <h3 className="m-0 px-3 pb-1 text-body font-semibold text-foreground">{title}</h3>
      <div className="grid">{children}</div>
    </section>
  );
  return (
    <form className="grid gap-5 rounded-lg border border-border bg-card p-5" aria-label="Privacy settings" onSubmit={(event) => {
      event.preventDefault();
      if (editing && canEdit && changes.length && !stale && !save.isPending) save.mutate();
    }}>
      <header className="flex min-h-9 items-center gap-3">
        <Tile size="sm" tone="brand" className="size-8"><Shield aria-hidden="true" /></Tile>
        <div className="grid min-w-0 flex-1 gap-0.5">
          <div className="flex items-center gap-2">
            <h2 className="m-0 text-title font-semibold text-foreground">Privacy</h2>
            <Badge variant="brand">Automatic</Badge>
          </div>
          <p className="m-0 text-caption text-muted-foreground">
            Capture, retention and storage for this Brain. Secret redaction always applies.
          </p>
        </div>
        {canEdit && <div className="flex shrink-0 gap-2">
          {editing ? <>
            <Button type="button" variant="secondary" disabled={save.isPending}
              onClick={() => { reset(); setEditing(false); }}>Cancel</Button>
            <Button type="submit" loading={save.isPending} disabled={!changes.length || stale}>Save changes</Button>
          </> : <Button type="button" variant="secondary" aria-label="Edit privacy settings"
            onClick={() => { reset(); setEditing(true); }}><Pencil aria-hidden="true" />Edit</Button>}
        </div>}
      </header>
      {stale && !save.isPending && <Alert variant="warning">Privacy settings changed elsewhere. Cancel and reopen to edit them.</Alert>}
      {save.error && <Alert variant="destructive">
        {savedDomains.length > 0 && <p>Already saved: {savedDomains.map((domain) => privacyLabels[domain]).join(", ")}. Remaining changes are still in this draft.</p>}
        {save.error.message}
      </Alert>}
      {editing && changes.includes("retention") && <p className="m-0 flex animate-enter items-center gap-2 rounded-md bg-warning-soft px-3 py-2 text-label text-warning-text">
        <TriangleAlert className="size-4 shrink-0" aria-hidden="true" />
        Shortening retention can expire existing content immediately. Extending it does not recover removed content.
      </p>}
      {section("Raw data retention", <>
        {duration("raw_session_days", "Raw sessions")}
        {duration("tool_output_days", "Raw tool output")}
      </>)}
      {section("Capture", <>
        {toggle("Automatic session capture", "Enable automatic session capture", value.capture.enabled, (enabled) => updateCapture({ enabled }))}
        <PolicyRow title="Questions & replies">
          <div className="flex justify-end gap-5">{["prompt", "reply"].map((kind) => (
            <Switch key={kind} label={kind === "prompt" ? "Questions" : "Replies"}
              aria-label={kind === "prompt" ? "Capture questions" : "Capture replies"}
              className={cn("text-label", readOnly && "cursor-default opacity-100")}
              checked={value.capture.kinds.includes(kind)} disabled={readOnly}
              onChange={(event) => updateCapture({ kinds: event.currentTarget.checked
                ? [...value.capture.kinds, kind] : value.capture.kinds.filter((item) => item !== kind) })} />
          ))}</div>
        </PolicyRow>
        {[{ kind: "tool_result", title: "Tool results" }, { kind: "lifecycle", title: "Session events" }].map(({ kind, title }) =>
          <Fragment key={kind}>{toggle(title, kind === "tool_result" ? "Capture tool result" : "Capture session events", value.capture.kinds.includes(kind), (checked) => updateCapture({
            kinds: checked ? [...value.capture.kinds, kind] : value.capture.kinds.filter((item) => item !== kind),
          }))}</Fragment>)}
        {toggle("Managed tool results", "Capture managed tool results", value.capture.managed_tools ?? false, (managed_tools) => updateCapture({ managed_tools }),
          "Publishes sanitized, permitted results of managed tool calls as Brain evidence, readable by this Brain’s knowledge readers.")}
        <PolicyRow title="Maximum text per event">
          <div className="w-48">
            <NumberInput aria-label="Maximum text per event · KiB" min={1} max={64}
              value={value.capture.max_event_bytes / 1024} readOnly={readOnly} hideControls={readOnly}
              suffix="KiB" className={valueInput}
              onChange={(bytes) => updateCapture({ max_event_bytes: typeof bytes === "number" ? bytes * 1024 : 0 })} />
          </div>
        </PolicyRow>
      </>)}
      {section("Evidence & memory", <>
        {duration("document_days", "Documents", true)}
        {duration("support_excerpt_days", "Supporting excerpts", true)}
        {duration("repository_days", "Repository snapshots", true)}
        {duration("claim_days", "Memory", true)}
        {toggle("Supporting excerpt capture", "Allow explicitly retained supporting excerpts", value.retention.allow_support_excerpts, (allow_support_excerpts) => updateRetention({ allow_support_excerpts }))}
        {toggle("Document content retention", "Allow document content retention", value.documents, (documents) => setDraft((previous) => ({ ...previous, documents })))}
      </>)}
      {section("Repository file text", <PolicyRow title="Repository content">
        <SegmentedControl label="Repository content capture" size="sm"
          value={value.repository ? "text" : "references"} disabled={readOnly}
          className={cn(readOnly && "opacity-100 [&_label]:cursor-default")}
          data={[{ value: "references", label: "References only" }, { value: "text", label: "Include file text" }]}
          onChange={(selected) => setDraft((previous) => ({ ...previous, repository: selected === "text" }))} />
      </PolicyRow>)}
      {section("Activity & backup", <>
        {duration("audit_days", "Activity detail")}
        {duration("backup_days", "Managed backup window")}
      </>)}
    </form>
  );
}
