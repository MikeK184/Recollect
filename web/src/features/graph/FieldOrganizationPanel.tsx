import { useEffect, useRef, useState } from "react";
import { SectionHeader } from "../../components/PageHeader";
import { InspectorSection } from "../../components/Inspector";
import {
  Alert,
  Badge,
  Button,
  Card,
  Code,
  Spinner,
  StatusBadge,
} from "../../components/ui";
import { GraphInspector } from "./GraphStage";
import { stateTone } from "./GraphStatusDrawer";
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
  const origins = selected
    ? (view?.origins.filter((o) => o.evidence_key === selected.key) ?? [])
    : [];
  const inspector = (
    <GraphInspector
      opened={!!selected}
      onClose={() => setChoice(null)}
      title={selected ? words(selected.evidence.kind) : "Field inspector"}
      label="Canonical field inspector"
      closeLabel="Close field inspector"
    >
      {selected && (
        <>
          <h3 className="m-0 text-section font-semibold">
            {selected.evidence.label}
          </h3>
          {origins.length > 0 && (
            <InspectorSection label={`Declared fields · ${origins.length}`}>
              {origins.map((origin) => {
                const entity = view?.entities.find(
                  (e) => e.id === origin.entity_id,
                );
                return (
                  <div key={origin.id} className="grid gap-1.5">
                    <p className="m-0 text-body font-semibold">
                      {entity?.label}
                    </p>
                    <p className="m-0 text-caption text-muted-foreground">
                      {words(origin.origin_kind)} · exact field
                    </p>
                    <Code className="justify-self-start">
                      {origin.field_pointer}
                    </Code>
                    <p className="m-0 text-caption text-muted-foreground">
                      Canonical name field:{" "}
                      <span className="mono">{entity?.label_pointer}</span>
                    </p>
                    <p className="m-0 text-caption text-muted-foreground">
                      Original reference ID
                    </p>
                    <Code className="organization-reference-id justify-self-start [overflow-wrap:anywhere] whitespace-normal">
                      {origin.value}
                    </Code>
                    <p className="m-0 text-caption text-muted-foreground">
                      Immutable input revision
                    </p>
                    <Code className="organization-reference-id justify-self-start [overflow-wrap:anywhere] whitespace-normal">
                      {origin.revision_id}
                    </Code>
                  </div>
                );
              })}
            </InspectorSection>
          )}
          <Button
            className="justify-self-start"
            onClick={() => setOpened(selected)}
          >
            Open canonical evidence
          </Button>
        </>
      )}
    </GraphInspector>
  );
  const evidence = opened ? evidenceChoice(opened) : null;
  return (
    <section
      aria-label="Canonical reference organization"
      data-testid="organization-fields"
      className="grid gap-3"
    >
      <SectionHeader
        title="Canonical references"
        description="Repository and environment IDs come from declared canonical fields. Untyped claim text remains unassigned; code records keep their structural meaning."
        actions={
          entityId ? (
            <Button variant="ghost" size="sm" onClick={() => expand(null)}>
              All canonical references
            </Button>
          ) : undefined
        }
      />
      {!view ? (
        <p
          role="status"
          className="m-0 flex items-center gap-2 text-body text-foreground-secondary"
        >
          <Spinner />
          Reading qualified canonical fields…
        </p>
      ) : (
        <>
          <Card className="grid gap-1.5 p-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="mono m-0 text-label">
                {number(view.coverage.qualified_inputs)} qualified inputs ·{" "}
                {number(view.coverage.assigned_inputs)} with typed references ·{" "}
                {number(view.coverage.unassigned_inputs)} without typed
                references in this read
              </p>
              <StatusBadge state={stateTone(view.coverage.refresh_state)}>
                {words(view.coverage.refresh_state)}
              </StatusBadge>
            </div>
            {lower && (
              <p className="m-0 text-caption text-muted-foreground">
                Counts are lower bounds while this bounded read or automatic
                refresh is incomplete.
              </p>
            )}
          </Card>
          {view.coverage.refresh_state === "blocked" && (
            <Alert variant="warning" role="status">
              Canonical field refresh awaits an authorized originating writer.
            </Alert>
          )}
          {view.entities.length === 0 && (
            <p className="m-0 text-body text-muted-foreground">
              No qualified typed references in this page.
            </p>
          )}
          {view.entities.length > 0 && (
            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
              {view.entities.map((entity) => (
                <Card
                  key={entity.id}
                  className="grid content-start gap-2 p-3 [overflow-wrap:anywhere]"
                  data-testid="organization-field-entity"
                >
                  <div className="flex items-start justify-between gap-2">
                    <p className="m-0 text-body font-semibold">
                      {entity.label}
                    </p>
                    <Badge>{entity.kind}</Badge>
                  </div>
                  <p className="mono m-0 text-caption text-muted-foreground">
                    {number(entity.contributor_count)} canonical contributors
                    in this read
                  </p>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-self-start"
                    onClick={() => expand(entity.id)}
                    aria-label={`Explore canonical reference ${entity.label}`}
                  >
                    Explore reference
                  </Button>
                </Card>
              ))}
            </div>
          )}
          {entityId && view.evidence.length > 0 && (
            <section
              className="organization-neighborhood grid gap-3"
              aria-label="Canonical field neighborhood"
            >
              <SectionHeader
                as="h3"
                title="Canonical evidence neighborhood"
                description="One node per qualified evidence contributor. Field references preserve the original canonical identities."
              />
              <div className="organization-graph flex h-[46rem] min-h-[42rem] min-w-0 overflow-hidden rounded-lg border border-border shadow-sm">
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
              <ul className="m-0 grid list-none gap-0.5 p-0">
                {view.evidence.map((node) => (
                  <li key={node.key}>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="w-full justify-start"
                      onClick={() => setChoice({ kind: "node", id: node.key })}
                    >
                      Inspect canonical evidence {node.evidence.label}
                    </Button>
                  </li>
                ))}
              </ul>
            </section>
          )}
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="secondary"
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
              size="sm"
              variant="secondary"
              disabled={!view.next_cursor}
              onClick={() => {
                setHistory((old) => [...old, cursor]);
                setCursor(view.next_cursor ?? null);
              }}
            >
              Next reference page
            </Button>
          </div>
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
