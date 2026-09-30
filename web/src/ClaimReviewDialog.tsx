import { useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Divider,
  Group,
  Loader,
  Modal,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { EvidenceDialog } from "./ClaimsPanel";
type Content = components["schemas"]["ClaimContent"];
type Revision = components["schemas"]["ClaimRevision"];
type View = components["schemas"]["ClaimView"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type EvidenceViewer = typeof EvidenceDialog;
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
            Back to history
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
          Back to history
        </Button>
      </Stack>
    </Modal>
  );
}

export function ClaimReviewDialog({
  brain,
  view,
  catalogue,
  EvidenceViewer,
  onClose,
}: {
  brain: Brain;
  view: View;
  catalogue?: Catalogue;
  EvidenceViewer: EvidenceViewer;
  onClose: () => void;
}) {
  const r = view.revision;
  const [offset, setOffset] = useState(0);
  const [inspecting, setInspecting] = useState<Revision | null>(null);
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
  if (history.isError)
    return (
      <Modal opened onClose={onClose} size="xl" title="Review history">
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
  return (
    <Modal opened onClose={onClose} size="xl" title="Review history">
      <Stack className="claims-panel">
        <Alert color="blue">
          New input is handled by autonomous learning. See Activity → Model
          usage and learning for recent learning. This view is history only.
        </Alert>
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
        {brain.archived && <Alert>This Brain is archived.</Alert>}
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
              <Scope content={c.content} catalogue={catalogue} />
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
              </Group>
            </Stack>
          </Card>
        ))}
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
