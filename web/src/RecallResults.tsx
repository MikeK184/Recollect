import { useState, type ReactNode } from "react";
import { ChevronRight } from "lucide-react";
import type { components } from "./api-schema";
import { Markdown } from "./components/Markdown";
import { staggerStyle } from "./components/Motion";
import {
  Alert,
  Badge,
  Button,
  Card,
  KindChip,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "./components/ui";
import type { Kind } from "./design/tokens";
import { cn } from "./lib/utils";

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

const memoryKinds = new Set<string>(["claim", "decision", "procedure", "handover"]);
/** The design-system kind for a recalled record, when it has one. */
export function recallKind(item: Pick<Item, "kind" | "claim">): Kind | null {
  const kind = item.claim?.kind ?? item.kind;
  if (memoryKinds.has(kind)) return kind as Kind;
  if (item.kind === "source_version") return "source";
  if (item.kind === "repository_fact") return "entity";
  return null;
}

/** Kind chip, or a neutral outline badge for kinds without a colour role. */
export function RecallKindChip({ item }: { item: Pick<Item, "kind" | "claim"> }) {
  const kind = recallKind(item);
  const text = label(item.claim?.kind ?? item.kind);
  return kind ? (
    <KindChip kind={kind} className="capitalize">
      {text}
    </KindChip>
  ) : (
    <Badge variant="outline" className="capitalize">
      {text}
    </Badge>
  );
}

/** Small progressive-disclosure group used throughout recall results. */
export function RecallDetails({
  summary,
  children,
  className,
}: {
  summary: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <details className={cn("group/details text-caption text-muted-foreground", className)}>
      <summary className="flex w-fit cursor-pointer list-none items-center gap-1 rounded-sm font-medium text-foreground-secondary hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
        <ChevronRight
          className="size-3.5 transition-transform duration-fast group-open/details:rotate-90"
          aria-hidden="true"
        />
        {summary}
      </summary>
      <div className="mt-2 grid gap-1.5 pl-[18px]">{children}</div>
    </details>
  );
}

export function RecallScope({
  selection,
  catalogue,
}: {
  selection: Scope;
  catalogue?: Catalogue;
}) {
  return (
    <p className="m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
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
    </p>
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
    <div
      className="grid gap-1 rounded-md bg-muted px-3 py-2.5"
      data-testid="recall-selection"
    >
      <p className="m-0 text-label font-semibold text-foreground">
        {brainName} · {label(answer.context.mode)}
      </p>
      <RecallScope selection={answer.selection} catalogue={catalogue} />
      <p className="m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
        Collection:{" "}
        {collectionName ?? request.collection_id ?? "All collections"} · Exact
        manifest: <span className="mono">{answer.manifest_revision_id ?? "None selected"}</span>
      </p>
      <p className="m-0 text-caption text-muted-foreground">
        Knowledge recorded by <span className="mono">{time(answer.knowledge_at)}</span> · Fact
        time:{" "}
        <span className="mono">{answer.fact_at ? time(answer.fact_at) : "Not filtered"}</span>
      </p>
      {(request.mode === "history" || request.knowledge_at) && (
        <p className="m-0 text-label text-warning-text">
          Historical selection. Current retention and corrections still apply.
          Historical graph exploration is unavailable.
        </p>
      )}
      <p className="m-0 text-caption text-muted-foreground">
        Retrieved evidence requires interpretation. A match does not establish
        support for an answer or authorize a procedure.
      </p>
    </div>
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
  index,
  onClaim,
  onEvidence,
  onGraph,
}: Actions & {
  item: Item;
  request: Request;
  catalogue?: Catalogue;
  index?: number;
}) {
  const graphAllowed =
    request.mode !== "history" &&
    !request.knowledge_at &&
    (item.kind === "claim" ||
      item.kind === "source_version" ||
      (item.kind === "repository_fact" &&
        item.provenance.some((p) => p.snapshot_id)));
  return (
    <Card
      data-testid="recall-result"
      className={cn("grid min-w-0 gap-3", index !== undefined && "animate-enter")}
      style={index !== undefined ? staggerStyle(index) : undefined}
    >
      <div className="flex items-start gap-3">
        <p className="m-0 min-w-0 flex-1 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
          {item.label}
        </p>
        <RecallKindChip item={item} />
      </div>
      {item.claim && (
        <>
          <p className="m-0 text-label text-foreground-secondary">
            Review: {item.claim.review} · Freshness:{" "}
            {label(item.claim.freshness)} · Operational:{" "}
            {item.claim.operational}
          </p>
          {(item.claim.lifecycle !== "active" ||
            item.claim.conflicting_claim_ids.length > 0 ||
            item.claim.rule_ids.length > 0) && (
            <div className="flex flex-wrap gap-1.5">
              {item.claim.lifecycle !== "active" && (
                <Badge variant="warning" className="capitalize">
                  {label(item.claim.lifecycle)}
                </Badge>
              )}
              {item.claim.conflicting_claim_ids.length > 0 && (
                <Badge variant="warning">Unresolved disagreement</Badge>
              )}
              {item.claim.rule_ids.length > 0 && (
                <Badge variant="destructive">Current review rule applies</Badge>
              )}
            </div>
          )}
          <RecallDetails summary="Record scope and history">
            <p className="m-0">
              Origin: {label(item.claim.origin)}
              {item.claim.acceptance_policy
                ? ` · Accepted by Brain policy ${item.claim.acceptance_policy}`
                : item.claim.reviewer_id
                  ? " · Review recorded by an authenticated account"
                  : " · No recorded acceptance authority"}
            </p>
            <p className="m-0">
              Fact validity:{" "}
              {item.claim.validity.kind === "unknown"
                ? "Unknown; no interval inferred"
                : item.claim.validity.kind === "point"
                  ? `${item.claim.validity.from ? time(item.claim.validity.from) : "Unknown"} (${item.claim.validity.precision} point observation)`
                  : `${item.claim.validity.from ? time(item.claim.validity.from) : "Unknown start"} → ${item.claim.validity.to ? time(item.claim.validity.to) + " (end excluded)" : "Unknown end"}`}
              {item.claim.knowledge_until &&
                ` · Knowledge replaced ${time(item.claim.knowledge_until)}`}
            </p>
            <RecallScope selection={item.selection} catalogue={catalogue} />
          </RecallDetails>
        </>
      )}
      {!item.claim && (
        <RecallDetails summary="Recorded scope">
          <RecallScope selection={item.selection} catalogue={catalogue} />
        </RecallDetails>
      )}
      {item.text && (
        <div
          className="max-h-[360px] overflow-auto rounded-md border border-border bg-background px-3.5 py-3 scrollbar-thin"
          data-testid="recall-fragment"
        >
          <Markdown
            text={item.text}
            className={item.kind === "claim" ? "markdown-preserve-lines" : ""}
            raw
          />
        </div>
      )}
      {item.qualifications.length > 0 && (
        <p className="m-0 text-caption text-muted-foreground">
          {item.qualifications.map(label).join(" · ")}
        </p>
      )}
      <RecallDetails summary="Provenance and retrieval">
        <p className="m-0">
          Recorded <span className="mono">{time(item.recorded_at)}</span> ·{" "}
          {item.channels.map(label).join(" + ")}
        </p>
        {item.semantic_similarity != null && (
          <p className="m-0">
            Cosine similarity:{" "}
            <span className="mono">{item.semantic_similarity.toFixed(3)}</span> ·
            similarity is not truth
          </p>
        )}
        {item.graph_match && (
          <div className="grid gap-1" data-testid="recall-graph-witness">
            <p className="m-0 text-label font-medium text-foreground">
              Discovered through {item.graph_match.edges.length} recorded
              relationship{item.graph_match.edges.length === 1 ? "" : "s"} (
              {label(item.graph_match.direction)})
            </p>
            <p className="m-0 text-label text-foreground [overflow-wrap:anywhere]">
              {item.graph_match.nodes.map((n) => n.label).join(" → ")}
            </p>
            <p className="m-0">
              {item.graph_match.edges
                .map((e) => `${label(e.family)}: ${label(e.relation)}`)
                .join(" · ")}
              . This explains discovery; the evidence below supports this
              record.
            </p>
          </div>
        )}
        {item.provenance
          .filter((p) => p.revision)
          .map((p) => (
            <p className="m-0 [overflow-wrap:anywhere]" key={`${p.kind}:${p.id}`}>
              Repository:{" "}
              {catalogue?.repositories.find((r) => r.id === p.repository_id)
                ?.canonical_origin ?? p.repository_id}{" "}
              · Commit: <span className="mono">{p.revision}</span>
              {p.path
                ? ` · ${p.path}${p.line_from ? `:${p.line_from}` : ""}`
                : ""}
            </p>
          ))}
      </RecallDetails>
      <div className="flex flex-wrap gap-1.5 border-t border-border pt-3">
        {item.kind === "claim" && (
          <Button size="sm" variant="secondary" onClick={() => onClaim(item.id)}>
            Inspect claim and history
          </Button>
        )}
        {item.provenance.map((p, i) => (
          <Button
            key={`${p.kind}:${p.id}`}
            size="sm"
            variant="ghost"
            onClick={() => onEvidence(recallEvidence(item, i))}
          >
            Inspect evidence {i + 1}
            {p.path
              ? ` · ${p.path}${p.line_from ? `:${p.line_from}` : ""}`
              : ""}
          </Button>
        ))}
        <Button
          size="sm"
          variant="ghost"
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
      </div>
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
  const [view, setView] = useState("matches");
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
  const match = (item: Item, index?: number) => (
    <Match
      key={`${item.kind}:${item.id}:${item.provenance.map((p) => `${p.byte_from}-${p.byte_to}`).join(":")}`}
      item={item}
      request={request}
      catalogue={catalogue}
      index={index}
      {...actions}
    />
  );
  const count = (n: number) => (
    <span className="mono text-caption text-muted-foreground">({n})</span>
  );
  return (
    <Tabs value={view} onValueChange={setView} className="gap-4">
      <TabsList aria-label="Investigation views">
        <TabsTrigger value="matches">
          Matches {count(items.length)}
        </TabsTrigger>
        <TabsTrigger value="disagreements">
          Disagreements {count(conflicts.size)}
        </TabsTrigger>
        <TabsTrigger value="sources">
          Sources {count(sources.size)}
        </TabsTrigger>
      </TabsList>
      <TabsContent value="matches" className="grid gap-3">
        {items.map((item, index) => match(item, index))}
      </TabsContent>
      <TabsContent value="disagreements" className="grid gap-3">
        <p className="m-0 text-body text-muted-foreground">
          Canonical disagreements linked from these results. Inspection and
          resolution are optional; normal learning continues under Brain
          policy.
        </p>
        {conflicts.size === 0 && (
          <Alert role="status">
            No unresolved disagreements are listed in this result set. This
            does not establish consistency across the Brain.
          </Alert>
        )}
        {[...conflicts.entries()]
          .slice(conflictPage, conflictPage + 10)
          .map(([key, pair]) => (
            <div
              key={key}
              data-testid="recall-disagreement"
              className="grid grid-cols-2 gap-3 rounded-lg border border-warning/40 bg-warning-soft/40 p-3"
            >
              {match(pair.left)}
              {pair.right ? (
                match(pair.right)
              ) : (
                <div className="grid content-center gap-3 rounded-lg border border-dashed border-border-strong bg-card p-4">
                  <p className="m-0 text-body text-foreground-secondary">
                    A related claim is outside this bounded result set.
                    Inspect its retained history and actual applicability;
                    it is not added to copied context.
                  </p>
                  <Button
                    variant="secondary"
                    className="w-fit"
                    onClick={() => actions.onClaim(pair.id)}
                  >
                    Inspect related claim
                  </Button>
                </div>
              )}
            </div>
          ))}
        {conflicts.size > 10 && (
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              disabled={conflictPage === 0}
              onClick={() => setConflictPage((p) => Math.max(0, p - 10))}
            >
              Previous disagreements
            </Button>
            <span className="mono text-caption text-muted-foreground">
              {conflictPage + 1}–{Math.min(conflictPage + 10, conflicts.size)}{" "}
              of {conflicts.size}
            </span>
            <Button
              variant="ghost"
              size="sm"
              disabled={conflictPage + 10 >= conflicts.size}
              onClick={() => setConflictPage((p) => p + 10)}
            >
              Next disagreements
            </Button>
          </div>
        )}
      </TabsContent>
      <TabsContent value="sources" className="grid gap-3">
        <p className="m-0 text-body text-muted-foreground">
          Exact evidence cited by the returned records. Several records citing
          one source do not establish independent corroboration.
        </p>
        {!sources.size && (
          <Alert role="status">
            No source references were returned for this selection.
          </Alert>
        )}
        {[...sources.entries()].map(([key, entry], index) => {
          const p = entry.item.provenance[entry.index];
          return (
            <Card
              key={key}
              data-testid="recall-source"
              className="grid gap-2 animate-enter"
              style={staggerStyle(index)}
            >
              <div className="flex flex-wrap gap-1.5">
                <Badge className="capitalize">{label(p.kind)}</Badge>
                <Badge variant="outline" className="capitalize">
                  {label(p.availability)}
                </Badge>
              </div>
              <p className="m-0 text-body text-foreground">
                Cited by: {entry.matches.map((i) => i.label).join(" · ")}
              </p>
              <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
                Exact evidence: {p.id}
              </p>
              {p.repository_id && (
                <p className="m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
                  {catalogue?.repositories.find(
                    (r) => r.id === p.repository_id,
                  )?.canonical_origin ?? p.repository_id}{" "}
                  · Commit: <span className="mono">{p.revision ?? "Unavailable"}</span>
                </p>
              )}
              {[...entry.locations].map((location) => (
                <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]" key={location}>
                  {location}
                </p>
              ))}
              <Button
                variant="secondary"
                size="sm"
                className="w-fit"
                onClick={() =>
                  actions.onEvidence(recallEvidence(entry.item, entry.index))
                }
              >
                Inspect exact source
              </Button>
            </Card>
          );
        })}
      </TabsContent>
    </Tabs>
  );
}
