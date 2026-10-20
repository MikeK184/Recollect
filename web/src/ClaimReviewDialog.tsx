import { useState, type ComponentType, type ReactNode } from "react";
import { FileText, PencilLine } from "lucide-react";
import {
  Alert,
  Badge,
  Button,
  Checkbox,
  Modal,
  Select,
  StatusBadge,
  TextareaField,
} from "./components/ui";
import { useMutation, useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { ClaimEditor, EvidenceDialog } from "./ClaimsPanel";
import { useIdempotency } from "./useIdempotency";
import { usePollInterval } from "./usePollInterval";
type Content = components["schemas"]["ClaimContent"];
type Revision = components["schemas"]["ClaimRevision"];
type View = components["schemas"]["ClaimView"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type Outcome = components["schemas"]["ReviewOutcome"];
type Editor = ComponentType<Parameters<typeof ClaimEditor>[0]>;
type EvidenceViewer = ComponentType<Parameters<typeof EvidenceDialog>[0]>;
const label = (s: string) => s.replaceAll("_", " ");
const time = (s: string) => new Date(s).toLocaleString();

function Scope({
  content,
  catalogue,
}: {
  content: Content;
  catalogue?: Catalogue;
}) {
  const environment =
    catalogue?.environments.find(
      (e) => e.id === content.selection.environment_id,
    )?.name ??
    content.selection.environment_id ??
    "All environments";
  const repos = (content.selection.repository_ids ?? []).map(
    (id) =>
      catalogue?.repositories.find((r) => r.id === id)?.canonical_origin ?? id,
  );
  return (
    <p className="m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
      {environment} · {repos.join(", ") || "All repositories"} ·{" "}
      <span className="mono">
        {content.validity.kind === "unknown"
          ? "Unknown fact time"
          : `${content.validity.from ?? "Unknown start"} → ${content.validity.to ?? (content.validity.kind === "point" ? `${content.validity.precision} precision` : "Unknown end")}`}
      </span>
    </p>
  );
}

/** A hairline group heading inside the review dialog. */
function Group({
  label: text,
  children,
}: {
  label: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="grid min-w-0 gap-3 border-t border-border pt-4">
      <h3 className="section-label m-0">{text}</h3>
      {children}
    </section>
  );
}
const muted = "m-0 text-label text-muted-foreground";
const card = "grid gap-2 rounded-lg border border-border bg-card p-3";
function Pending({ text }: { text: string }) {
  return (
    <p
      role="status"
      className="m-0 flex items-center gap-2 text-label text-muted-foreground"
    >
      <span
        aria-hidden="true"
        className="size-3.5 animate-spin rounded-full border-2 border-border-strong border-t-transparent"
      />
      {text}
    </p>
  );
}
function InspectConflict({
  brain,
  claim,
  catalogue,
  EvidenceViewer,
  onClose,
}: {
  brain: Brain;
  claim: Revision;
  catalogue?: Catalogue;
  EvidenceViewer: EvidenceViewer;
  onClose: () => void;
}) {
  const pollNormal = usePollInterval("normal");
  const [selected, setSelected] = useState<Evidence | null>(null);
  const query = useQuery({
    queryKey: ["review-evidence", brain.id, claim.id],
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/claims/{claim}", {
          signal,
          params: {
            path: { brain: brain.id, claim: claim.claim_id },
            query: { knowledge_at: claim.recorded_at },
          },
        }),
      ),
  });
  if (
    query.isError ||
    !query.data?.selected ||
    query.data.selected.revision.id !== claim.id
  )
    return (
      <Modal
        opened
        onClose={onClose}
        title="Conflict evidence"
        size="xl"
        footer={
          <Button variant="secondary" onClick={onClose}>
            Back to review
          </Button>
        }
      >
        <div className="grid gap-3">
          {query.isPending ? (
            <Pending text="Loading conflict evidence…" />
          ) : (
            <Alert variant="warning">
              {query.error?.message ??
                "This exact conflict revision is unavailable. Its prior content has been cleared."}
            </Alert>
          )}
          {query.isError && (
            <Button
              variant="secondary"
              size="sm"
              className="w-fit"
              onClick={() => void query.refetch()}
            >
              Retry conflict inspection
            </Button>
          )}
        </div>
      </Modal>
    );
  if (selected)
    return (
      <EvidenceViewer
        brain={brain}
        evidence={selected}
        onClose={() => setSelected(null)}
      />
    );
  return (
    <Modal
      opened
      onClose={onClose}
      title="Conflict evidence"
      size="xl"
      footer={
        <Button variant="secondary" onClick={onClose}>
          Back to review
        </Button>
      }
    >
      <div className="grid gap-3">
        <h3 className="m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
          {claim.content.subject}
        </h3>
        <p className="m-0 text-body font-medium text-foreground [overflow-wrap:anywhere]">
          {claim.content.predicate}: {claim.content.value}
        </p>
        <Scope content={claim.content} catalogue={catalogue} />
        <p className="m-0 text-label text-foreground-secondary">
          {label(claim.origin)} by {claim.actor_name} · {label(claim.review)} ·{" "}
          {label(claim.content.operational)}
        </p>
        {claim.content.rationale && (
          <p className="m-0 text-body text-foreground-secondary [overflow-wrap:anywhere]">
            {claim.content.rationale}
          </p>
        )}
        <div className="grid gap-2">
          {query.data?.selected?.evidence.map((e) => (
            <Button
              key={e.id}
              variant="secondary"
              className="claim-evidence-button h-auto justify-start whitespace-normal py-2 text-left [overflow-wrap:anywhere]"
              onClick={() => setSelected(e)}
            >
              <FileText aria-hidden="true" />
              {e.label} · {label(e.availability)}
            </Button>
          ))}
        </div>
      </div>
    </Modal>
  );
}

export function ClaimReviewDialog({
  brain,
  view,
  catalogue,
  current,
  Editor,
  EvidenceViewer,
  onClose,
  onSaved,
}: {
  brain: Brain;
  view: View;
  catalogue?: Catalogue;
  current: boolean;
  Editor: Editor;
  EvidenceViewer: EvidenceViewer;
  onClose: () => void;
  onSaved: () => void;
}) {
  const pollNormal = usePollInterval("normal");
  const r = view.revision;
  const [openedCurrent] = useState(current);
  const writable = brain.role !== "reader" && !brain.archived && openedCurrent;
  const [action, setAction] = useState<string | null>(
    r.lifecycle === "withdrawn"
      ? "restore"
      : r.review === "proposed"
        ? "accept"
        : "correct",
  );
  const [reason, setReason] = useState("");
  const [basis, setBasis] = useState<string | null>(null);
  const [content, setContent] = useState<Content | null>(null);
  const [offset, setOffset] = useState(0);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const [disposition, setDisposition] = useState<string | null>(
    "keep_selected",
  );
  const [participants, setParticipants] = useState<Revision[]>([r]);
  const [selectedId, setSelectedId] = useState<string | null>(r.claim_id);
  const [drafts, setDrafts] = useState<Record<string, Content>>({});
  const [replacement, setReplacement] = useState<Content | null>(null);
  const [editing, setEditing] = useState<{
    kind: "single" | "participant" | "replacement";
    revision: Revision;
  } | null>(null);
  const [inspecting, setInspecting] = useState<Revision | null>(null);
  const command = useIdempotency();
  const history = useQuery({
    queryKey: ["claim-review", brain.id, r.claim_id, offset],
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/claims/{claim}/review", {
          signal,
          params: {
            path: { brain: brain.id, claim: r.claim_id },
            query: { offset },
          },
        }),
      ),
  });
  const save = useMutation({
    mutationFn: async (kind: "single" | "conflict") => {
      if (kind === "single") {
        const body = {
          base_revision: r.id,
          action: action!,
          reason,
          content:
            action === "correct" || action === "revalidate" ? content : null,
          revalidation_basis: action === "revalidate" ? basis : null,
        };
        return result(
          await client.POST("/api/brains/{brain}/claims/{claim}/review", {
            params: { path: { brain: brain.id, claim: r.claim_id } },
            body,
            headers: { "Idempotency-Key": command.forInput(body) },
          }),
        );
      }
      const body = {
        disposition: disposition!,
        participants: participants.map((p) => ({
          claim_id: p.claim_id,
          base_revision: p.id,
          content:
            disposition === "keep_both"
              ? (drafts[p.claim_id] ?? p.content)
              : null,
        })),
        selected_id: disposition === "keep_selected" ? selectedId : null,
        replacement: disposition === "replace" ? replacement : null,
        reason,
      };
      return result(
        await client.POST("/api/brains/{brain}/claim-conflicts/resolve", {
          params: { path: { brain: brain.id } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: (reply) => {
      setOutcome(reply);
      onSaved();
      void history.refetch();
    },
  });
  if (history.isError)
    return (
      <Modal opened onClose={onClose} size="xl" title="Review and corrections">
        <div className="grid gap-3">
          <Alert variant="destructive">{history.error.message}</Alert>
          <Button
            variant="secondary"
            size="sm"
            className="w-fit"
            onClick={() => void history.refetch()}
          >
            Retry history
          </Button>
        </div>
      </Modal>
    );
  if (inspecting)
    return (
      <InspectConflict
        brain={brain}
        claim={inspecting}
        catalogue={catalogue}
        EvidenceViewer={EvidenceViewer}
        onClose={() => setInspecting(null)}
      />
    );
  if (editing) {
    const edited = editing.revision;
    const draft =
      editing.kind === "single"
        ? (content ?? edited.content)
        : editing.kind === "participant"
          ? (drafts[edited.claim_id] ?? edited.content)
          : (replacement ?? { ...edited.content, value: "" });
    return (
      <Editor
        brain={brain}
        catalogue={catalogue}
        revision={edited}
        draft={draft}
        evidence={edited.id === r.id ? view.evidence : []}
        heading={
          editing.kind === "participant"
            ? "Adjust conflict applicability"
            : editing.kind === "replacement"
              ? "Draft conflict replacement"
              : "Edit reviewed content"
        }
        notice="Prepare the complete content and exact support for this review. The review is recorded after you provide a reason and confirm the decision."
        submitLabel="Use this content"
        onClose={() => setEditing(null)}
        onSaved={() => setEditing(null)}
        onCommit={async (value) => {
          if (editing.kind === "single") setContent(value);
          else if (editing.kind === "participant")
            setDrafts((d) => ({ ...d, [edited.claim_id]: value }));
          else setReplacement(value);
          return edited.claim_id;
        }}
      />
    );
  }
  return (
    <Modal
      opened
      onClose={onClose}
      size="xl"
      title="Review and corrections"
      description="Recollect learns and maintains memory automatically. These optional controls let you correct a record or its review when needed."
      footer={
        <Button variant="secondary" onClick={onClose}>
          Back to claim
        </Button>
      }
    >
      <div className="grid gap-4">
        <div className="grid gap-2">
          {outcome && (
            <p className="m-0 text-caption text-muted-foreground">
              Input reviewed in this decision
            </p>
          )}
          <h3 className="m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
            {r.content.subject}
          </h3>
          <p className="m-0 text-body font-medium text-foreground [overflow-wrap:anywhere]">
            {r.content.predicate}: {r.content.value}
          </p>
          <Scope content={r.content} catalogue={catalogue} />
          <div className="flex flex-wrap items-center gap-1.5">
            <StatusBadge
              state={
                r.review === "accepted"
                  ? "positive"
                  : r.review === "rejected"
                    ? "negative"
                    : "neutral"
              }
            >
              {label(r.review)}
            </StatusBadge>
            <Badge variant="outline">{label(r.lifecycle ?? "active")}</Badge>
          </div>
          <div>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setInspecting(r)}
            >
              <FileText aria-hidden="true" />
              Inspect evidence for this claim
            </Button>
          </div>
        </div>
        {!current && !outcome && (
          <Alert variant="warning">
            {openedCurrent
              ? "This claim changed while review was open. Retry an interrupted request, or return to the claim to inspect its latest revision."
              : "This is a historical revision. Return to latest knowledge before making a review decision."}
          </Alert>
        )}
        {brain.role === "reader" && (
          <p className={muted}>
            Read-only access. A writer or administrator can review claims.
          </p>
        )}
        {brain.archived && (
          <Alert variant="default" role="status">
            This Brain is archived.
          </Alert>
        )}
        {outcome ? (
          <div className="grid gap-3 rounded-lg border border-border bg-card p-4">
            <h4 className="m-0 text-title font-semibold text-foreground">
              Review recorded
            </h4>
            <p className="m-0 text-label text-foreground-secondary">
              {label(outcome.decision.action)} by {outcome.decision.actor_name}{" "}
              ·{" "}
              <span className="mono text-caption">
                {time(outcome.decision.recorded_at)}
              </span>
            </p>
            <p className="m-0 text-body text-foreground [overflow-wrap:anywhere]">
              {outcome.decision.reason}
            </p>
            {outcome.claims.map((v) => (
              <div key={v.revision.id} className={card}>
                <p className="m-0 text-body font-medium text-foreground [overflow-wrap:anywhere]">
                  {v.revision.content.value}
                </p>
                <p className="m-0 text-label text-foreground-secondary">
                  {label(v.revision.review)} ·{" "}
                  {label(v.revision.lifecycle ?? "active")}
                </p>
                <Alert
                  variant={v.eligibility.strict_accepted ? "brand" : "warning"}
                  role="status"
                >
                  <p>
                    {v.eligibility.strict_accepted
                      ? "Eligible for strict accepted context"
                      : "Does not qualify for strict accepted context"}
                  </p>
                  <p className="!mt-1 text-caption text-muted-foreground">
                    {v.eligibility.reasons.map(label).join(" · ")}
                  </p>
                </Alert>
              </div>
            ))}
            <div>
              <Button onClick={onClose}>Return to claim</Button>
            </div>
          </div>
        ) : (
          writable && (
            <Group label="Review this claim">
              <Select
                label="Review action"
                value={action}
                onChange={(v) => {
                  setAction(v);
                  setContent(null);
                  save.reset();
                }}
                data={[
                  {
                    value: "accept",
                    label: "Accept",
                    disabled:
                      r.review !== "proposed" || r.lifecycle === "withdrawn",
                  },
                  {
                    value: "reject",
                    label: "Reject value",
                    disabled:
                      r.review === "rejected" || r.lifecycle === "withdrawn",
                  },
                  {
                    value: "withdraw",
                    label: "Withdraw",
                    disabled: r.lifecycle === "withdrawn",
                  },
                  {
                    value: "correct",
                    label: "Correct value",
                    disabled: r.lifecycle === "withdrawn",
                  },
                  { value: "revalidate", label: "Explicitly revalidate" },
                  {
                    value: "restore",
                    label: "Restore withdrawn claim",
                    disabled: r.lifecycle !== "withdrawn",
                  },
                ]}
              />
              {action === "accept" && (
                <p className="m-0 text-body text-foreground-secondary">
                  Acceptance records your review. Evidence availability,
                  freshness, conflicting claims and operational verification
                  still determine eligibility.
                </p>
              )}
              {action === "reject" && (
                <Alert variant="destructive" role="status">
                  Rejecting records a rule against this value in its applicable
                  scope and time. A new source or claim ID cannot silently
                  restore it.
                </Alert>
              )}
              {action === "withdraw" && (
                <p className="m-0 text-body text-foreground-secondary">
                  Withdrawal removes the assertion from ordinary context and
                  blocks re-entry. It does not declare the value false or erase
                  its evidence.
                </p>
              )}
              {action === "revalidate" && (
                <Select
                  label="Revalidation basis"
                  placeholder="Select the reason for reconsideration"
                  value={basis}
                  onChange={setBasis}
                  data={[
                    {
                      value: "changed_evidence",
                      label: "Changed supporting evidence",
                    },
                    {
                      value: "changed_applicability",
                      label: "Changed applicability",
                    },
                    {
                      value: "review_correction",
                      label: "Correct an earlier review decision",
                    },
                  ]}
                />
              )}
              {(action === "correct" || action === "revalidate") && (
                <>
                  <div>
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() =>
                        setEditing({ kind: "single", revision: r })
                      }
                    >
                      <PencilLine aria-hidden="true" />
                      Edit reviewed content
                    </Button>
                  </div>
                  {content && (
                    <div className={card}>
                      <p className="m-0 text-label font-medium text-foreground [overflow-wrap:anywhere]">
                        Prepared value: {content.value}
                      </p>
                      <Scope content={content} catalogue={catalogue} />
                    </div>
                  )}
                </>
              )}
              <TextareaField
                label="Review reason"
                required
                minRows={2}
                autosize
                maxLength={2000}
                value={reason}
                onChange={(e) => setReason(e.target.value)}
              />
              {save.error && (
                <Alert variant="destructive" title="Review failed">
                  {save.error.message}
                </Alert>
              )}
              <div>
                <Button
                  loading={save.isPending}
                  disabled={
                    !action ||
                    !reason.trim() ||
                    (action === "correct" && !content) ||
                    (action === "revalidate" && !basis)
                  }
                  onClick={() => save.mutate("single")}
                >
                  Confirm review
                </Button>
              </div>
            </Group>
          )
        )}
        <Group label="Current conflicting claims">
          {history.isPending && <Pending text="Loading review history…" />}
          {history.data?.conflicts.length === 0 && (
            <p className={muted}>No current overlapping conflicts.</p>
          )}
          {(history.data?.conflicts ?? []).map((c) => (
            <div key={c.claim_id} className={card}>
              <p className="m-0 text-body font-medium text-foreground [overflow-wrap:anywhere]">
                {c.content.value}
              </p>
              <Scope
                content={drafts[c.claim_id] ?? c.content}
                catalogue={catalogue}
              />
              <p className="m-0 text-caption text-foreground-secondary">
                {label(c.review)} · {label(c.content.operational)} ·{" "}
                {c.actor_name}
              </p>
              <div className="flex flex-wrap items-center gap-3">
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => setInspecting(c)}
                >
                  Inspect conflicting evidence
                </Button>
                {writable && !outcome && (
                  <Checkbox
                    label={`Include ${c.content.value.slice(0, 80)} in resolution`}
                    checked={participants.some(
                      (p) => p.claim_id === c.claim_id,
                    )}
                    disabled={
                      participants.length >= 20 &&
                      !participants.some((p) => p.claim_id === c.claim_id)
                    }
                    onChange={(e) => {
                      const checked = e.currentTarget.checked;
                      setParticipants((p) =>
                        checked
                          ? [...p, c]
                          : p.filter((r) => r.claim_id !== c.claim_id),
                      );
                    }}
                  />
                )}
              </div>
            </div>
          ))}
          {writable && !outcome && !!history.data?.conflicts.length && (
            <div className="grid gap-3 rounded-lg border border-border bg-muted p-4">
              <h4 className="m-0 text-title font-semibold text-foreground">
                Resolve selected conflicts
              </h4>
              <p className="m-0 text-body text-foreground-secondary">
                This claim is included. Select the other claims explicitly.
                Unselected conflicts remain open.
              </p>
              <Select
                label="Conflict resolution"
                value={disposition}
                onChange={setDisposition}
                data={[
                  { value: "keep_selected", label: "Keep one selected value" },
                  {
                    value: "keep_both",
                    label: "Keep both with disjoint applicability",
                  },
                  { value: "retract", label: "Withdraw all selected claims" },
                  {
                    value: "replace",
                    label: "Replace with a new reviewed value",
                  },
                ]}
              />
              {disposition === "keep_selected" && (
                <Select
                  label="Value to keep"
                  value={selectedId}
                  onChange={setSelectedId}
                  data={participants.map((p) => ({
                    value: p.claim_id,
                    label: p.content.value.slice(0, 100),
                  }))}
                />
              )}
              {disposition === "keep_both" &&
                participants.map((p) => (
                  <div key={p.claim_id} className={card}>
                    <p className="m-0 text-label font-medium text-foreground [overflow-wrap:anywhere]">
                      {p.content.value}
                    </p>
                    <Scope
                      content={drafts[p.claim_id] ?? p.content}
                      catalogue={catalogue}
                    />
                    <div>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() =>
                          setEditing({ kind: "participant", revision: p })
                        }
                      >
                        Adjust applicability for {p.content.value.slice(0, 60)}
                      </Button>
                    </div>
                  </div>
                ))}
              {disposition === "replace" && (
                <>
                  <div>
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() =>
                        setEditing({ kind: "replacement", revision: r })
                      }
                    >
                      Prepare replacement
                    </Button>
                  </div>
                  {replacement && (
                    <p className="m-0 text-body text-foreground [overflow-wrap:anywhere]">
                      Replacement: {replacement.value}
                    </p>
                  )}
                </>
              )}
              <p className="m-0 text-label text-muted-foreground">
                The review reason above applies to this resolution. Every
                selected claim must still have the revision inspected here.
              </p>
              <div>
                <Button
                  loading={save.isPending}
                  disabled={
                    participants.length < 2 ||
                    !reason.trim() ||
                    (disposition === "replace" && !replacement) ||
                    (disposition === "keep_selected" &&
                      !participants.some((p) => p.claim_id === selectedId))
                  }
                  onClick={() => save.mutate("conflict")}
                >
                  Confirm conflict resolution
                </Button>
              </div>
            </div>
          )}
        </Group>
        <Group label="Applicable review rules">
          {history.data?.rules.length === 0 && (
            <p className={muted}>No matching review rules.</p>
          )}
          {history.data?.rules.map((rule) => (
            <div key={rule.id} className={card}>
              <div className="flex flex-wrap gap-1.5">
                <StatusBadge state="attention">{label(rule.kind)}</StatusBadge>
                {history.data.exempted_rule_ids.includes(rule.id) && (
                  <StatusBadge state="positive">
                    Exception for the current revision
                  </StatusBadge>
                )}
              </div>
              <p className="m-0 text-label text-foreground [overflow-wrap:anywhere]">
                {rule.content.subject} · {rule.content.predicate}:{" "}
                {rule.content.value}
              </p>
              <Scope content={rule.content} catalogue={catalogue} />
              <p className="m-0 text-caption text-muted-foreground">
                Recorded <span className="mono">{time(rule.created_at)}</span>
              </p>
              <p className="m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
                Original revision:{" "}
                <span className="mono">{rule.revision_id}</span>
              </p>
            </div>
          ))}
        </Group>
        <Group label="Review decision history">
          {history.data?.total === 0 && (
            <p className={muted}>No review decisions yet.</p>
          )}
          {history.data?.decisions.map((d) => (
            <div key={d.id} className={card}>
              <p className="m-0 text-body font-medium text-foreground">
                {label(d.action)} · {d.actor_name}
              </p>
              <p className="m-0 text-caption text-muted-foreground">
                <span className="mono">{time(d.recorded_at)}</span>
                {d.revalidation_basis
                  ? ` · ${label(d.revalidation_basis)}`
                  : ""}
              </p>
              <p className="m-0 text-body text-foreground-secondary [overflow-wrap:anywhere]">
                {d.reason}
              </p>
              {d.transitions.map((t) => (
                <p
                  className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]"
                  key={t.claim_id}
                >
                  {t.before_revision ?? "New claim"} → {t.after_revision}
                </p>
              ))}
            </div>
          ))}
          {!!history.data && history.data.total > 20 && (
            <div className="flex items-center justify-between gap-3">
              <Button
                disabled={!offset}
                variant="secondary"
                size="sm"
                onClick={() => setOffset((o) => Math.max(0, o - 20))}
              >
                Previous decisions
              </Button>
              <span className="mono text-caption text-muted-foreground">
                {offset + 1}–{Math.min(offset + 20, history.data.total)} of{" "}
                {history.data.total}
              </span>
              <Button
                disabled={offset + 20 >= history.data.total}
                variant="secondary"
                size="sm"
                onClick={() => setOffset((o) => o + 20)}
              >
                Next decisions
              </Button>
            </div>
          )}
        </Group>
      </div>
    </Modal>
  );
}
