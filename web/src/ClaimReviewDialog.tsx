import { useState, type ComponentType } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Divider,
  Group,
  Loader,
  Modal,
  Select,
  Stack,
  Text,
  Textarea,
  Title,
} from "@mantine/core";
import { useMutation, useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { ClaimEditor, EvidenceDialog } from "./ClaimsPanel";
import { useIdempotency } from "./useIdempotency";
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
    <Text size="xs" className="claim-wrap">
      {environment} · {repos.join(", ") || "All repositories"} ·{" "}
      {content.validity.kind === "unknown"
        ? "Unknown fact time"
        : `${content.validity.from ?? "Unknown start"} → ${content.validity.to ?? (content.validity.kind === "point" ? `${content.validity.precision} precision` : "Unknown end")}`}
    </Text>
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
  const [selected, setSelected] = useState<Evidence | null>(null);
  const query = useQuery({
    queryKey: ["review-evidence", brain.id, claim.id],
    retry: false,
    gcTime: 0,
    refetchInterval: 4000,
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
      <Modal opened onClose={onClose} title="Conflict evidence" size="xl">
        <Stack>
          {query.isPending ? (
            <Loader />
          ) : (
            <Alert color="yellow">
              {query.error?.message ??
                "This exact conflict revision is unavailable. Its prior content has been cleared."}
            </Alert>
          )}
          {query.isError && (
            <Button variant="light" onClick={() => void query.refetch()}>
              Retry conflict inspection
            </Button>
          )}
          <Button variant="default" onClick={onClose}>
            Back to review
          </Button>
        </Stack>
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
    <Modal opened onClose={onClose} title="Conflict evidence" size="xl">
      <Stack>
        <Title order={3}>{claim.content.subject}</Title>
        <Text fw={600}>
          {claim.content.predicate}: {claim.content.value}
        </Text>
        <Scope content={claim.content} catalogue={catalogue} />
        <Text size="sm">
          {label(claim.origin)} by {claim.actor_name} · {label(claim.review)} ·{" "}
          {label(claim.content.operational)}
        </Text>
        <Text size="sm">{claim.content.rationale}</Text>
        {query.data?.selected?.evidence.map((e) => (
          <Button
            key={e.id}
            variant="default"
            h="auto"
            py="sm"
            className="claim-evidence-button"
            onClick={() => setSelected(e)}
          >
            {e.label} · {label(e.availability)}
          </Button>
        ))}
        <Button variant="default" onClick={onClose}>
          Back to review
        </Button>
      </Stack>
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
    refetchInterval: 4000,
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
        <Stack>
          <Alert color="red">{history.error.message}</Alert>
          <Button variant="light" onClick={() => void history.refetch()}>
            Retry history
          </Button>
        </Stack>
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
    <Modal opened onClose={onClose} size="xl" title="Review and corrections">
      <Stack className="claims-panel">
        <Text size="sm" c="dimmed">
          Recollect learns and maintains memory automatically. These optional
          controls let you correct a record or its review when needed.
        </Text>
        {outcome && (
          <Text size="xs" c="dimmed">
            Input reviewed in this decision
          </Text>
        )}
        <Title order={3}>{r.content.subject}</Title>
        <Text fw={600}>
          {r.content.predicate}: {r.content.value}
        </Text>
        <Scope content={r.content} catalogue={catalogue} />
        <Group>
          <Badge>{label(r.review)}</Badge>
          <Badge variant="outline">{label(r.lifecycle ?? "active")}</Badge>
        </Group>
        <Button variant="default" onClick={() => setInspecting(r)}>
          Inspect evidence for this claim
        </Button>
        {!current && !outcome && (
          <Alert color="yellow">
            {openedCurrent
              ? "This claim changed while review was open. Retry an interrupted request, or return to the claim to inspect its latest revision."
              : "This is a historical revision. Return to latest knowledge before making a review decision."}
          </Alert>
        )}
        {brain.role === "reader" && (
          <Text c="dimmed">
            Read-only access. A writer or administrator can review claims.
          </Text>
        )}
        {brain.archived && <Alert>This Brain is archived.</Alert>}
        {outcome ? (
          <Card withBorder>
            <Stack>
              <Title order={4}>Review recorded</Title>
              <Text size="sm">
                {label(outcome.decision.action)} by{" "}
                {outcome.decision.actor_name} ·{" "}
                {time(outcome.decision.recorded_at)}
              </Text>
              <Text size="sm" className="claim-wrap">
                {outcome.decision.reason}
              </Text>
              {outcome.claims.map((v) => (
                <Card withBorder key={v.revision.id}>
                  <Text fw={600}>{v.revision.content.value}</Text>
                  <Text size="sm">
                    {label(v.revision.review)} ·{" "}
                    {label(v.revision.lifecycle ?? "active")}
                  </Text>
                  <Alert
                    color={v.eligibility.strict_accepted ? "teal" : "yellow"}
                    mt="xs"
                  >
                    <Text size="sm">
                      {v.eligibility.strict_accepted
                        ? "Eligible for strict accepted context"
                        : "Does not qualify for strict accepted context"}
                    </Text>
                    <Text size="xs">
                      {v.eligibility.reasons.map(label).join(" · ")}
                    </Text>
                  </Alert>
                </Card>
              ))}
              <Button onClick={onClose}>Return to claim</Button>
            </Stack>
          </Card>
        ) : (
          writable && (
            <>
              <Divider label="Review this claim" />
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
                <Text size="sm">
                  Acceptance records your review. Evidence availability,
                  freshness, conflicting claims and operational verification
                  still determine eligibility.
                </Text>
              )}
              {action === "reject" && (
                <Alert color="red">
                  Rejecting records a rule against this value in its applicable
                  scope and time. A new source or claim ID cannot silently
                  restore it.
                </Alert>
              )}
              {action === "withdraw" && (
                <Text size="sm">
                  Withdrawal removes the assertion from ordinary context and
                  blocks re-entry. It does not declare the value false or erase
                  its evidence.
                </Text>
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
                  <Button
                    variant="default"
                    onClick={() => setEditing({ kind: "single", revision: r })}
                  >
                    Edit reviewed content
                  </Button>
                  {content && (
                    <Card withBorder>
                      <Text size="sm" fw={600}>
                        Prepared value: {content.value}
                      </Text>
                      <Scope content={content} catalogue={catalogue} />
                    </Card>
                  )}
                </>
              )}
              <Textarea
                label="Review reason"
                required
                minRows={2}
                autosize
                maxLength={2000}
                value={reason}
                onChange={(e) => setReason(e.target.value)}
              />
              {save.error && (
                <Alert color="red" title="Review failed">
                  {save.error.message}
                </Alert>
              )}
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
            </>
          )
        )}
        <Divider label="Current conflicting claims" />
        {history.isPending && <Loader />}
        {history.data?.conflicts.length === 0 && (
          <Text size="sm" c="dimmed">
            No current overlapping conflicts.
          </Text>
        )}
        {(history.data?.conflicts ?? []).map((c) => (
          <Card withBorder key={c.claim_id}>
            <Stack gap="xs">
              <Text fw={600} className="claim-wrap">
                {c.content.value}
              </Text>
              <Scope
                content={drafts[c.claim_id] ?? c.content}
                catalogue={catalogue}
              />
              <Text size="xs">
                {label(c.review)} · {label(c.content.operational)} ·{" "}
                {c.actor_name}
              </Text>
              <Group>
                <Button
                  size="xs"
                  variant="default"
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
              </Group>
            </Stack>
          </Card>
        ))}
        {writable && !outcome && !!history.data?.conflicts.length && (
          <Card withBorder>
            <Stack>
              <Title order={4}>Resolve selected conflicts</Title>
              <Text size="sm">
                This claim is included. Select the other claims explicitly.
                Unselected conflicts remain open.
              </Text>
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
                  <Card withBorder key={p.claim_id}>
                    <Text size="sm" fw={600}>
                      {p.content.value}
                    </Text>
                    <Scope
                      content={drafts[p.claim_id] ?? p.content}
                      catalogue={catalogue}
                    />
                    <Button
                      variant="default"
                      size="xs"
                      mt="xs"
                      onClick={() =>
                        setEditing({ kind: "participant", revision: p })
                      }
                    >
                      Adjust applicability for {p.content.value.slice(0, 60)}
                    </Button>
                  </Card>
                ))}
              {disposition === "replace" && (
                <>
                  <Button
                    variant="default"
                    onClick={() =>
                      setEditing({ kind: "replacement", revision: r })
                    }
                  >
                    Prepare replacement
                  </Button>
                  {replacement && (
                    <Text size="sm">Replacement: {replacement.value}</Text>
                  )}
                </>
              )}
              <Text size="sm">
                The review reason above applies to this resolution. Every
                selected claim must still have the revision inspected here.
              </Text>
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
            </Stack>
          </Card>
        )}
        <Divider label="Applicable review rules" />
        {history.data?.rules.length === 0 && (
          <Text size="sm" c="dimmed">
            No matching review rules.
          </Text>
        )}
        {history.data?.rules.map((rule) => (
          <Card withBorder key={rule.id}>
            <Stack gap="xs">
              <Group>
                <Badge color="orange">{label(rule.kind)}</Badge>
                {history.data.exempted_rule_ids.includes(rule.id) && (
                  <Badge color="teal">Exception for the current revision</Badge>
                )}
              </Group>
              <Text size="sm" className="claim-wrap">
                {rule.content.subject} · {rule.content.predicate}:{" "}
                {rule.content.value}
              </Text>
              <Scope content={rule.content} catalogue={catalogue} />
              <Text size="xs">Recorded {time(rule.created_at)}</Text>
              <Text size="xs" className="claim-wrap">
                Original revision: {rule.revision_id}
              </Text>
            </Stack>
          </Card>
        ))}
        <Divider label="Review decision history" />
        {history.data?.total === 0 && (
          <Text size="sm" c="dimmed">
            No review decisions yet.
          </Text>
        )}
        {history.data?.decisions.map((d) => (
          <Card withBorder key={d.id}>
            <Stack gap="xs">
              <Text fw={600}>
                {label(d.action)} · {d.actor_name}
              </Text>
              <Text size="xs">
                {time(d.recorded_at)}
                {d.revalidation_basis
                  ? ` · ${label(d.revalidation_basis)}`
                  : ""}
              </Text>
              <Text size="sm" className="claim-wrap">
                {d.reason}
              </Text>
              {d.transitions.map((t) => (
                <Text size="xs" className="claim-wrap" key={t.claim_id}>
                  {t.before_revision ?? "New claim"} → {t.after_revision}
                </Text>
              ))}
            </Stack>
          </Card>
        ))}
        {!!history.data && history.data.total > 20 && (
          <Group justify="space-between">
            <Button
              disabled={!offset}
              variant="subtle"
              onClick={() => setOffset((o) => Math.max(0, o - 20))}
            >
              Previous decisions
            </Button>
            <Text size="xs">
              {offset + 1}–{Math.min(offset + 20, history.data.total)} of{" "}
              {history.data.total}
            </Text>
            <Button
              disabled={offset + 20 >= history.data.total}
              variant="subtle"
              onClick={() => setOffset((o) => o + 20)}
            >
              Next decisions
            </Button>
          </Group>
        )}
        <Button variant="default" onClick={onClose}>
          Back to claim
        </Button>
      </Stack>
    </Modal>
  );
}
