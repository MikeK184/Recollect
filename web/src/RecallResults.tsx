import { useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  SimpleGrid,
  Stack,
  Tabs,
  Text,
} from "@mantine/core";
import type { components } from "./api-schema";

type Item = components["schemas"]["RecallItem"];
type Answer = components["schemas"]["RecallResponse"];
type Request = components["schemas"]["RecallRequest"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type Scope = components["schemas"]["ScopeSelection"];
const label = (s: string) => s.replaceAll("_", " ");
const time = (s: string) => new Date(s).toLocaleString();

export function recallEvidence(item: Item, index: number): Evidence {
  const p = item.provenance[index];
  return {
    kind: p.kind,
    id: p.id,
    label: item.label,
    source_id: p.source_id,
    repository_id: p.repository_id,
    snapshot_id: p.snapshot_id,
    revision: p.revision,
    reference: null,
    observed_at: null,
    created_at: item.recorded_at,
    availability: p.availability,
  };
}

export function RecallScope({
  selection,
  catalogue,
}: {
  selection: Scope;
  catalogue?: Catalogue;
}) {
  return (
    <Text size="xs" className="claim-wrap">
      Repositories:{" "}
      {selection.repository_ids?.length
        ? selection.repository_ids
            .map(
              (id) =>
                catalogue?.repositories.find((r) => r.id === id)
                  ?.canonical_origin ?? id,
            )
            .join(", ")
        : "All in this Brain"}{" "}
      · Areas:{" "}
      {selection.area_ids?.length
        ? selection.area_ids
            .map((id) => catalogue?.areas.find((a) => a.id === id)?.name ?? id)
            .join(", ")
        : "All in this Brain"}{" "}
      · Environment:{" "}
      {catalogue?.environments.find((e) => e.id === selection.environment_id)
        ?.name ??
        selection.environment_id ??
        "All in this Brain"}
    </Text>
  );
}

export function RecallSelectionSummary({
  answer,
  request,
  brainName,
  catalogue,
  collectionName,
}: {
  answer: Answer;
  request: Request;
  brainName: string;
  catalogue?: Catalogue;
  collectionName?: string;
}) {
  return (
    <Card
      withBorder
      bg="var(--mantine-color-gray-0)"
      data-testid="recall-selection"
    >
      <Stack gap={4}>
        <Text size="sm" fw={600}>
          {brainName} · {label(answer.context.mode)}
        </Text>
        <RecallScope selection={answer.selection} catalogue={catalogue} />
        <Text size="xs" className="claim-wrap">
          Collection:{" "}
          {collectionName ?? request.collection_id ?? "All collections"} · Exact
          manifest: {answer.manifest_revision_id ?? "None selected"}
        </Text>
        <Text size="xs">
          Knowledge recorded by {time(answer.knowledge_at)} · Fact time:{" "}
          {answer.fact_at ? time(answer.fact_at) : "Not filtered"}
        </Text>
        {(request.mode === "history" || request.knowledge_at) && (
          <Text size="sm" c="orange.9">
            Historical selection. Current retention and corrections still apply.
            Historical graph exploration is unavailable.
          </Text>
        )}
        <Text size="xs" c="dimmed">
          Retrieved evidence requires interpretation. A match does not establish
          support for an answer or authorize a procedure.
        </Text>
      </Stack>
    </Card>
  );
}

type Actions = {
  onClaim: (id: string) => void;
  onEvidence: (evidence: Evidence) => void;
  onGraph: (item: Item) => void;
};
function Match({
  item,
  request,
  catalogue,
  onClaim,
  onEvidence,
  onGraph,
}: Actions & {
  item: Item;
  request: Request;
  catalogue?: Catalogue;
}) {
  const graphAllowed =
    request.mode !== "history" &&
    !request.knowledge_at &&
    (item.kind === "claim" ||
      item.kind === "source_version" ||
      (item.kind === "repository_fact" &&
        item.provenance.some((p) => p.snapshot_id)));
  return (
    <Card withBorder data-testid="recall-result">
      <Stack gap="xs">
        <Group justify="space-between">
          <Text fw={600} className="claim-wrap">
            {item.label}
          </Text>
          <Badge>{label(item.claim?.kind ?? item.kind)}</Badge>
        </Group>
        {item.claim && (
          <>
            <Text size="sm">
              Review: {item.claim.review} · Freshness:{" "}
              {label(item.claim.freshness)} · Operational:{" "}
              {item.claim.operational}
            </Text>
            <Group gap="xs">
              {item.claim.lifecycle !== "active" && (
                <Badge color="orange">{label(item.claim.lifecycle)}</Badge>
              )}
              {item.claim.conflicting_claim_ids.length > 0 && (
                <Badge color="yellow">Unresolved disagreement</Badge>
              )}
              {item.claim.rule_ids.length > 0 && (
                <Badge color="red">Current review rule applies</Badge>
              )}
            </Group>
            <Text size="xs" c="dimmed">
              Origin: {label(item.claim.origin)}
              {item.claim.acceptance_policy
                ? ` · Accepted by Brain policy ${item.claim.acceptance_policy}`
                : item.claim.reviewer_id
                  ? " · Review recorded by an authenticated account"
                  : " · No recorded acceptance authority"}
            </Text>
            <Text size="xs">
              Fact validity:{" "}
              {item.claim.validity.kind === "unknown"
                ? "Unknown; no interval inferred"
                : item.claim.validity.kind === "point"
                  ? `${item.claim.validity.from ? time(item.claim.validity.from) : "Unknown"} (${item.claim.validity.precision} point observation)`
                  : `${item.claim.validity.from ? time(item.claim.validity.from) : "Unknown start"} → ${item.claim.validity.to ? time(item.claim.validity.to) + " (end excluded)" : "Unknown end"}`}
              {item.claim.knowledge_until &&
                ` · Knowledge replaced ${time(item.claim.knowledge_until)}`}
            </Text>
          </>
        )}
        <RecallScope selection={item.selection} catalogue={catalogue} />
        {item.text && (
          <pre className="source-content" data-testid="recall-fragment">
            {item.text}
          </pre>
        )}
        {item.qualifications.length > 0 && (
          <Text size="xs" c="dimmed">
            {item.qualifications.map(label).join(" · ")}
          </Text>
        )}
        <Text size="xs">
          Recorded {time(item.recorded_at)} ·{" "}
          {item.channels.map(label).join(" + ")}
        </Text>
        {item.semantic_similarity != null && (
          <Text size="xs" c="dimmed">
            Cosine similarity: {item.semantic_similarity.toFixed(3)} ·
            similarity is not truth
          </Text>
        )}
        {item.graph_match && (
          <Stack gap={4} data-testid="recall-graph-witness">
            <Text size="sm" fw={500}>
              Discovered through {item.graph_match.edges.length} recorded
              relationship{item.graph_match.edges.length === 1 ? "" : "s"} (
              {label(item.graph_match.direction)})
            </Text>
            <Text size="sm" className="claim-wrap">
              {item.graph_match.nodes.map((n) => n.label).join(" → ")}
            </Text>
            <Text size="xs" c="dimmed">
              {item.graph_match.edges
                .map((e) => `${label(e.family)}: ${label(e.relation)}`)
                .join(" · ")}
              . This explains discovery; the evidence below supports this
              record.
            </Text>
          </Stack>
        )}
        {item.provenance
          .filter((p) => p.revision)
          .map((p) => (
            <Text size="xs" className="claim-wrap" key={`${p.kind}:${p.id}`}>
              Repository:{" "}
              {catalogue?.repositories.find((r) => r.id === p.repository_id)
                ?.canonical_origin ?? p.repository_id}{" "}
              · Commit: {p.revision}
              {p.path
                ? ` · ${p.path}${p.line_from ? `:${p.line_from}` : ""}`
                : ""}
            </Text>
          ))}
        <Group gap="xs">
          {item.kind === "claim" && (
            <Button size="xs" variant="light" onClick={() => onClaim(item.id)}>
              Inspect claim and history
            </Button>
          )}
          {item.provenance.map((p, i) => (
            <Button
              key={`${p.kind}:${p.id}`}
              size="xs"
              variant="subtle"
              onClick={() => onEvidence(recallEvidence(item, i))}
            >
              Inspect evidence {i + 1}
              {p.path
                ? ` · ${p.path}${p.line_from ? `:${p.line_from}` : ""}`
                : ""}
            </Button>
          ))}
          <Button
            size="xs"
            variant="default"
            disabled={!graphAllowed}
            onClick={() => onGraph(item)}
            title={
              graphAllowed
                ? "Inspect current qualified relationships using this selection"
                : request.mode === "history" || request.knowledge_at
                  ? "Historical graphs are unavailable"
                  : "This record has no direct graph center"
            }
          >
            Explore relationships
          </Button>
        </Group>
      </Stack>
    </Card>
  );
}

export function RecallResults({
  answer,
  request,
  catalogue,
  ...actions
}: Actions & {
  answer: Answer;
  request: Request;
  catalogue?: Catalogue;
}) {
  const [view, setView] = useState<string | null>("matches");
  const [conflictPage, setConflictPage] = useState(0);
  const items = answer.context.items;
  const claims = new Map(
    items.filter((i) => i.kind === "claim").map((i) => [i.id, i]),
  );
  const conflicts = new Map<string, { left: Item; right?: Item; id: string }>();
  for (const item of claims.values())
    for (const id of item.claim?.conflicting_claim_ids ?? []) {
      if (id === item.id) continue;
      const key = [id, item.id].sort().join(":");
      if (!conflicts.has(key))
        conflicts.set(key, { left: item, right: claims.get(id), id });
    }
  const sources = new Map<
    string,
    { item: Item; index: number; matches: Item[]; locations: Set<string> }
  >();
  for (const item of items)
    item.provenance.forEach((p, index) => {
      const key = `${p.kind}:${p.id}`;
      const entry = sources.get(key) ?? {
        item,
        index,
        matches: [],
        locations: new Set<string>(),
      };
      if (!entry.matches.some((i) => i.kind === item.kind && i.id === item.id))
        entry.matches.push(item);
      if (p.path || p.line_from != null || p.byte_from != null)
        entry.locations.add(
          [
            p.path,
            p.line_from != null
              ? `lines ${p.line_from}–${p.line_to ?? p.line_from}`
              : null,
            p.byte_from != null
              ? `bytes ${p.byte_from}–${p.byte_to ?? p.byte_from}`
              : null,
          ]
            .filter(Boolean)
            .join(" · "),
        );
      sources.set(key, entry);
    });
  const match = (item: Item) => (
    <Match
      key={`${item.kind}:${item.id}`}
      item={item}
      request={request}
      catalogue={catalogue}
      {...actions}
    />
  );
  return (
    <Tabs value={view} onChange={setView} keepMounted={false}>
      <Tabs.List aria-label="Investigation views">
        <Tabs.Tab value="matches">Matches ({items.length})</Tabs.Tab>
        <Tabs.Tab value="disagreements">
          Disagreements ({conflicts.size})
        </Tabs.Tab>
        <Tabs.Tab value="sources">Sources ({sources.size})</Tabs.Tab>
      </Tabs.List>
      <Tabs.Panel value="matches" pt="md">
        <Stack>{items.map(match)}</Stack>
      </Tabs.Panel>
      <Tabs.Panel value="disagreements" pt="md">
        <Stack>
          <Text size="sm">
            Canonical disagreements linked from these results. Inspection and
            resolution are optional; normal learning continues under Brain
            policy.
          </Text>
          {conflicts.size === 0 && (
            <Alert>
              No unresolved disagreements are listed in this result set. This
              does not establish consistency across the Brain.
            </Alert>
          )}
          {[...conflicts.entries()]
            .slice(conflictPage, conflictPage + 10)
            .map(([key, pair]) => (
              <Card withBorder key={key} data-testid="recall-disagreement">
                <SimpleGrid cols={2}>
                  {match(pair.left)}
                  {pair.right ? (
                    match(pair.right)
                  ) : (
                    <Stack justify="center">
                      <Text size="sm">
                        A related claim is outside this bounded result set.
                        Inspect its retained history and actual applicability;
                        it is not added to copied context.
                      </Text>
                      <Button
                        variant="light"
                        onClick={() => actions.onClaim(pair.id)}
                      >
                        Inspect related claim
                      </Button>
                    </Stack>
                  )}
                </SimpleGrid>
              </Card>
            ))}
          {conflicts.size > 10 && (
            <Group>
              <Button
                variant="subtle"
                disabled={conflictPage === 0}
                onClick={() => setConflictPage((p) => Math.max(0, p - 10))}
              >
                Previous disagreements
              </Button>
              <Text size="sm">
                {conflictPage + 1}–{Math.min(conflictPage + 10, conflicts.size)}{" "}
                of {conflicts.size}
              </Text>
              <Button
                variant="subtle"
                disabled={conflictPage + 10 >= conflicts.size}
                onClick={() => setConflictPage((p) => p + 10)}
              >
                Next disagreements
              </Button>
            </Group>
          )}
        </Stack>
      </Tabs.Panel>
      <Tabs.Panel value="sources" pt="md">
        <Stack>
          <Text size="sm">
            Exact evidence cited by the returned records. Several records citing
            one source do not establish independent corroboration.
          </Text>
          {!sources.size && (
            <Alert>
              No source references were returned for this selection.
            </Alert>
          )}
          {[...sources.entries()].map(([key, entry]) => {
            const p = entry.item.provenance[entry.index];
            return (
              <Card withBorder key={key} data-testid="recall-source">
                <Stack gap="xs">
                  <Group>
                    <Badge>{label(p.kind)}</Badge>
                    <Badge variant="outline">{label(p.availability)}</Badge>
                  </Group>
                  <Text size="sm">
                    Cited by: {entry.matches.map((i) => i.label).join(" · ")}
                  </Text>
                  <Text size="xs" className="claim-wrap">
                    Exact evidence: {p.id}
                  </Text>
                  {p.repository_id && (
                    <Text size="xs" className="claim-wrap">
                      {catalogue?.repositories.find(
                        (r) => r.id === p.repository_id,
                      )?.canonical_origin ?? p.repository_id}{" "}
                      · Commit: {p.revision ?? "Unavailable"}
                    </Text>
                  )}
                  {[...entry.locations].map((location) => (
                    <Text size="xs" className="claim-wrap" key={location}>
                      {location}
                    </Text>
                  ))}
                  <Button
                    variant="light"
                    size="xs"
                    w="fit-content"
                    onClick={() =>
                      actions.onEvidence(
                        recallEvidence(entry.item, entry.index),
                      )
                    }
                  >
                    Inspect exact source
                  </Button>
                </Stack>
              </Card>
            );
          })}
        </Stack>
      </Tabs.Panel>
    </Tabs>
  );
}
