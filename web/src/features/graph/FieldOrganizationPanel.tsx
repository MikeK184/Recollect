import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Loader,
  SimpleGrid,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { ClaimDialog, EvidenceDialog } from "../../ClaimsPanel";
import { GraphRenderer } from "../../GraphRenderer";
import type { GraphChoice } from "../../GraphCanvas";
import { useContentDeadline } from "../../useContentDeadline";
import { useQualifiedRead } from "./useQualifiedRead";

type Scope = components["schemas"]["GraphSelection"];
type View = components["schemas"]["FieldOrganizationView"];
type Cursor = components["schemas"]["FieldOrganizationCursor"];
type Node = components["schemas"]["GraphNode"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
const emptyEdges: never[] = [];
const emptyKeys: string[] = [];
const words = (s: string) => s.replaceAll("_", " ");

function evidenceChoice(node: Node): Evidence | null {
  if (node.evidence.kind === "claim") return null;
  const p = node.evidence.provenance[0];
  if (!p) return null;
  return {
    kind: p.kind,
    id: p.id,
    label: node.evidence.label,
    source_id: p.source_id,
    repository_id: p.repository_id,
    snapshot_id: p.snapshot_id,
    revision: p.revision,
    reference: p.path,
    observed_at: null,
    created_at: node.evidence.recorded_at,
    availability: p.availability,
  };
}

export function FieldOrganizationPanel({
  brain,
  scope,
  epoch,
  qualificationKey,
  onFailure,
}: {
  brain: Brain;
  scope: Scope;
  epoch: number;
  qualificationKey: string;
  onFailure: (error: Error) => void;
}) {
  const [entityId, setEntityId] = useState<string | null>(null);
  const [cursor, setCursor] = useState<Cursor | null>(null);
  const [history, setHistory] = useState<(Cursor | null)[]>([]);
  const [choice, setChoice] = useState<GraphChoice | null>(null);
  const [opened, setOpened] = useState<Node | null>(null);
  const failure = useRef(onFailure);
  failure.current = onFailure;
  const requestKey = JSON.stringify([qualificationKey, entityId, cursor]);
  const active = useRef(requestKey);
  active.current = requestKey;
  const [read] = useQualifiedRead<View>({
    key: requestKey,
    fallback: "Canonical fields could not be read.",
    read: async (signal) => {
      const next = result(
        await client.POST("/api/brains/{brain}/organization/fields/view", {
          params: { path: { brain: brain.id } },
          body: { scope, entity_id: entityId, cursor },
          signal,
        }),
      );
      if (next.brain_id !== brain.id || next.epoch !== epoch)
        throw new Error(
          "Canonical fields changed during this read. Read current organization again.",
        );
      return next;
    },
    onError: (error) => failure.current(error),
  });
  const expires = useContentDeadline(
    read.key === requestKey ? read.view?.expires_at : null,
  );
  const view = read.key === requestKey && !expires ? read.view : undefined;
  useEffect(() => {
    setChoice(null);
    setOpened(null);
  }, [requestKey]);
  useEffect(() => {
    if (expires)
      failure.current(
        new Error(
          "Canonical fields reached their retention deadline. Read current organization again.",
        ),
      );
  }, [expires]);
  function expand(id: string | null) {
    setEntityId(id);
    setCursor(null);
    setHistory([]);
    setChoice(null);
    setOpened(null);
  }
  const selected =
    choice?.kind === "node"
      ? view?.evidence.find((n) => n.key === choice.id)
      : undefined;
  const lower = view?.coverage.counts_are_lower_bounds;
  const number = (n: number) => `${lower ? "≥" : ""}${n}`;
  const inspector = (
    <aside
      className="organization-inspector"
      aria-label="Canonical field inspector"
    >
      <Stack gap="sm">
        <Title order={4}>{selected?.evidence.label ?? "Field inspector"}</Title>
        {selected ? (
          <>
            <Badge variant="light" w="fit-content">
              {words(selected.evidence.kind)}
            </Badge>
            {view?.origins
              .filter((o) => o.evidence_key === selected.key)
              .map((origin) => (
                <Card key={origin.id} withBorder p="sm">
                  <Text fw={600} size="sm">
                    {
                      view.entities.find((e) => e.id === origin.entity_id)
                        ?.label
                    }
                  </Text>
                  <Text size="xs" c="dimmed">
                    {words(origin.origin_kind)} · exact field
                  </Text>
                  <code>{origin.field_pointer}</code>
                  <Text size="xs" mt="xs">
                    Canonical name field:{" "}
                    {
                      view.entities.find((e) => e.id === origin.entity_id)
                        ?.label_pointer
                    }
                  </Text>
                  <Text size="xs" mt="xs">
                    Original reference ID
                  </Text>
                  <code className="organization-reference-id">
                    {origin.value}
                  </code>
                  <Text size="xs" mt="xs">
                    Immutable input revision
                  </Text>
                  <code className="organization-reference-id">
                    {origin.revision_id}
                  </code>
                </Card>
              ))}
            <Button onClick={() => setOpened(selected)}>
              Open canonical evidence
            </Button>
          </>
        ) : (
          <Text size="sm" c="dimmed">
            Select evidence to inspect its declared canonical field and original
            identity.
          </Text>
        )}
      </Stack>
    </aside>
  );
  const evidence = opened ? evidenceChoice(opened) : null;
  return (
    <section
      aria-label="Canonical reference organization"
      data-testid="organization-fields"
    >
      <Group justify="space-between" mt="lg" mb="sm">
        <Title order={3}>Canonical references</Title>
        {entityId && (
          <Button variant="subtle" size="xs" onClick={() => expand(null)}>
            All canonical references
          </Button>
        )}
      </Group>
      <Text size="sm" c="dimmed" mb="sm">
        Repository and environment IDs come from declared canonical fields.
        Untyped claim text remains unassigned; code records keep their
        structural meaning.
      </Text>
      {!view ? (
        <Group role="status">
          <Loader size="xs" />
          <Text size="sm">Reading qualified canonical fields…</Text>
        </Group>
      ) : (
        <>
          <Card withBorder p="sm" mb="sm">
            <Group justify="space-between">
              <Text size="sm">
                {number(view.coverage.qualified_inputs)} qualified inputs ·{" "}
                {number(view.coverage.assigned_inputs)} with typed references ·{" "}
                {number(view.coverage.unassigned_inputs)} without typed
                references in this read
              </Text>
              <Badge variant="light">
                {words(view.coverage.refresh_state)}
              </Badge>
            </Group>
            {lower && (
              <Text size="xs" c="dimmed" mt={4}>
                Counts are lower bounds while this bounded read or automatic
                refresh is incomplete.
              </Text>
            )}
          </Card>
          {view.coverage.refresh_state === "blocked" && (
            <Alert color="yellow" mb="sm">
              Canonical field refresh awaits an authorized originating writer.
            </Alert>
          )}
          {view.entities.length === 0 && (
            <Text size="sm" c="dimmed">
              No qualified typed references in this page.
            </Text>
          )}
          <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="sm">
            {view.entities.map((entity) => (
              <Card
                key={entity.id}
                withBorder
                p="sm"
                data-testid="organization-field-entity"
              >
                <Group justify="space-between">
                  <Text fw={600}>{entity.label}</Text>
                  <Badge variant="light">{entity.kind}</Badge>
                </Group>
                <Text size="xs" c="dimmed" mt="xs">
                  {number(entity.contributor_count)} canonical contributors in
                  this read
                </Text>
                <Button
                  variant="subtle"
                  size="compact-xs"
                  mt="sm"
                  onClick={() => expand(entity.id)}
                  aria-label={`Explore canonical reference ${entity.label}`}
                >
                  Explore reference
                </Button>
              </Card>
            ))}
          </SimpleGrid>
          {entityId && view.evidence.length > 0 && (
            <section
              className="organization-neighborhood"
              aria-label="Canonical field neighborhood"
            >
              <Title order={3}>Canonical evidence neighborhood</Title>
              <Text c="dimmed" size="sm" mb="sm">
                One node per qualified evidence contributor. Field references
                preserve the original canonical identities.
              </Text>
              <div className="organization-graph">
                <GraphRenderer
                  key={requestKey}
                  nodes={view.evidence}
                  edges={emptyEdges}
                  choice={choice}
                  onChoose={setChoice}
                  pathNodes={emptyKeys}
                  pathEdges={emptyKeys}
                  inspector={inspector}
                />
              </div>
              <Stack mt="sm" gap="xs">
                {view.evidence.map((node) => (
                  <Button
                    key={node.key}
                    variant="subtle"
                    justify="flex-start"
                    onClick={() => setChoice({ kind: "node", id: node.key })}
                  >
                    Inspect canonical evidence {node.evidence.label}
                  </Button>
                ))}
              </Stack>
            </section>
          )}
          <Group mt="sm">
            <Button
              size="xs"
              variant="default"
              disabled={!history.length}
              onClick={() => {
                const previous = [...history];
                const next = previous.pop() ?? null;
                setHistory(previous);
                setCursor(next);
              }}
            >
              Previous reference page
            </Button>
            <Button
              size="xs"
              variant="default"
              disabled={!view.next_cursor}
              onClick={() => {
                setHistory((old) => [...old, cursor]);
                setCursor(view.next_cursor ?? null);
              }}
            >
              Next reference page
            </Button>
          </Group>
        </>
      )}
      {opened?.evidence.kind === "claim" && (
        <ClaimDialog
          key={requestKey}
          brain={brain}
          id={opened.evidence.id}
          revisionId={opened.evidence.revision_id ?? undefined}
          onClose={() => setOpened(null)}
          onSaved={() => {
            setOpened(null);
            failure.current(
              new Error(
                "Canonical evidence changed. Read current organization again.",
              ),
            );
          }}
        />
      )}
      {evidence && (
        <EvidenceDialog
          key={requestKey}
          brain={brain}
          evidence={evidence}
          returnLabel="Back to canonical fields"
          onClose={() => setOpened(null)}
        />
      )}
    </section>
  );
}
