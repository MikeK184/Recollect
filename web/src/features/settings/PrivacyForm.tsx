import { useEffect, useState, type ReactNode } from "react";
import {
  Alert, Badge, Button, Group, NumberInput, SegmentedControl, Switch,
} from "@mantine/core";
import { useMutation } from "@tanstack/react-query";
import { Shield, TriangleAlert } from "lucide-react";
import { client, result, type Brain } from "../../api";
import { useIdempotency } from "../../useIdempotency";
import { PolicyRow } from "./PolicyRow";
import {
  privacyChanges, privacyDraft, privacyIsCurrent, privacyLabels, savePrivacyDraft,
  type PrivacyDomain, type PrivacyDraft, type PrivacySnapshot,
} from "./privacyDraft";
import "./privacy-form.css";

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
  const duration = (key: Duration, title: string, optional = false) => (
    <PolicyRow key={key} title={title}>
      {readOnly && value.retention[key] == null ? <span>Until erased</span> : (
        <NumberInput aria-label={title + " · days"} min={1}
          max={key === "backup_days" ? 365 : 3650} allowDecimal={false}
          value={value.retention[key] ?? ""} placeholder={optional ? "Until erased" : undefined}
          readOnly={readOnly} hideControls={readOnly} rightSection={<span className="privacy-form-unit">days</span>}
          rightSectionWidth={48}
          onChange={(days) => updateRetention({ [key]: typeof days === "number" ? days : optional ? null : 0 })}
        />
      )}
    </PolicyRow>
  );
  const toggle = (title: string, label: string, checked: boolean, update: (value: boolean) => void) => (
    <PolicyRow title={title}><Switch aria-label={label} checked={checked} disabled={readOnly}
      onChange={(event) => update(event.currentTarget.checked)} /></PolicyRow>
  );
  const section = (title: string, children: ReactNode) => (
    <section className="privacy-form-section" aria-label={title}>
      <h3>{title}</h3>{children}
    </section>
  );
  return (
    <form className="privacy-form" aria-label="Privacy settings" onSubmit={(event) => {
      event.preventDefault();
      if (editing && canEdit && changes.length && !stale && !save.isPending) save.mutate();
    }}>
      <header className="privacy-form-header">
        <Shield size={22} aria-hidden="true" />
        <h2>Privacy</h2><Badge color="brand">Automatic</Badge>
        {canEdit && <Group className="privacy-form-actions" gap="xs">
          {editing ? <>
            <Button type="button" variant="default" disabled={save.isPending}
              onClick={() => { reset(); setEditing(false); }}>Cancel</Button>
            <Button type="submit" loading={save.isPending} disabled={!changes.length || stale}>Save changes</Button>
          </> : <Button type="button" variant="subtle" aria-label="Edit privacy settings"
            onClick={() => { reset(); setEditing(true); }}>Edit</Button>}
        </Group>}
      </header>
      {stale && !save.isPending && <Alert color="yellow">Privacy settings changed elsewhere. Cancel and reopen to edit them.</Alert>}
      {save.error && <Alert color="red">
        {savedDomains.length > 0 && <p>Already saved: {savedDomains.map((domain) => privacyLabels[domain]).join(", ")}. Remaining changes are still in this draft.</p>}
        {save.error.message}
      </Alert>}
      {editing && changes.includes("retention") && <div className="privacy-form-warning">
        <TriangleAlert size={16} aria-hidden="true" />
        Shortening retention can expire existing content immediately. Extending it does not recover removed content.
      </div>}
      {section("Raw data retention", <>
        {duration("raw_session_days", "Raw sessions")}
        {duration("tool_output_days", "Raw tool output")}
      </>)}
      {section("Capture", <>
        {toggle("Automatic session capture", "Enable automatic session capture", value.capture.enabled, (enabled) => updateCapture({ enabled }))}
        <PolicyRow title="Questions & replies">
          <div className="privacy-form-switches">{["prompt", "reply"].map((kind) => (
            <Switch key={kind} label={kind === "prompt" ? "Questions" : "Replies"}
              aria-label={kind === "prompt" ? "Capture questions" : "Capture replies"}
              checked={value.capture.kinds.includes(kind)} disabled={readOnly}
              onChange={(event) => updateCapture({ kinds: event.currentTarget.checked
                ? [...value.capture.kinds, kind] : value.capture.kinds.filter((item) => item !== kind) })} />
          ))}</div>
        </PolicyRow>
        {[{ kind: "tool_result", title: "Tool results" }, { kind: "lifecycle", title: "Session events" }].map(({ kind, title }) =>
          <div key={kind}>{toggle(title, kind === "tool_result" ? "Capture tool result" : "Capture session events", value.capture.kinds.includes(kind), (checked) => updateCapture({
            kinds: checked ? [...value.capture.kinds, kind] : value.capture.kinds.filter((item) => item !== kind),
          }))}</div>)}
        {toggle("Managed tool results", "Capture managed tool results", value.capture.managed_tools ?? false, (managed_tools) => updateCapture({ managed_tools }))}
        <PolicyRow title="Maximum text per event">
          <NumberInput aria-label="Maximum text per event · KiB" min={1} max={64} decimalScale={10}
            value={value.capture.max_event_bytes / 1024} readOnly={readOnly} hideControls={readOnly}
            rightSection={<span className="privacy-form-unit">KiB</span>} rightSectionWidth={48}
            onChange={(bytes) => updateCapture({ max_event_bytes: typeof bytes === "number" ? bytes * 1024 : 0 })} />
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
        <SegmentedControl className="vision-segment" aria-label="Repository content capture"
          value={value.repository ? "text" : "references"} disabled={readOnly}
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
