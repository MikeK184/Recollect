import type { ReactNode } from "react";
import { ChevronLeft, ChevronRight, FileText } from "lucide-react";
import { SectionHeader } from "../../components/PageHeader";
import { InspectorSection } from "../../components/Inspector";
import { Badge, Button, Card } from "../../components/ui";
import { GraphRenderer } from "../../GraphRenderer";
import type { GraphChoice } from "../../GraphCanvas";
import type { components } from "../../api-schema";
import type { useOrganizationSources } from "./useOrganizationSources";
import { GraphInspector } from "./GraphStage";
import {
  emptyEdges,
  emptyKeys,
  label,
  sourceChoice,
  type Center,
  type EvidenceNode,
  type OrganizationMaps,
  type View,
} from "./organizationModel";

type SourcePage = components["schemas"]["OrganizationSourcesView"];

/** Exact mention text, kept verbatim (mono, wrapped). */
export function OrganizationQuote({ children }: { children: ReactNode }) {
  return (
    <pre className="organization-quote mono m-0 whitespace-pre-wrap rounded-md bg-muted px-2.5 py-2 text-label text-foreground [overflow-wrap:anywhere]">
      {children}
    </pre>
  );
}

export function SourceInspector({
  selectedSource,
  maps,
  onInspect,
  onClose,
}: {
  selectedSource: EvidenceNode | undefined;
  maps: OrganizationMaps;
  onInspect: (node: EvidenceNode) => void;
  onClose: () => void;
}) {
  const {
    mentions,
    entityMap,
    sourceMemberships,
    representativeSources,
    topicMap,
  } = maps;
  const sourceMentions = selectedSource
    ? mentions
        .filter((mention) => mention.source_key === selectedSource.key)
        .slice(0, 20)
    : [];
  const topics = selectedSource
    ? [
        ...new Set(
          [...sourceMemberships, ...representativeSources]
            .filter((member) => member.source_key === selectedSource.key)
            .map((member) => member.topic_id),
        ),
      ]
    : [];
  return (
    <GraphInspector
      opened={!!selectedSource}
      onClose={onClose}
      title="Source evidence"
      label="Selected organization evidence"
      closeLabel="Close source inspector"
    >
      {selectedSource && (
        <>
          <div className="grid gap-1.5">
            <h3 className="m-0 text-section font-semibold">
              {selectedSource.evidence.label}
            </h3>
            <p className="mono m-0 text-caption text-muted-foreground">
              {selectedSource.key}
            </p>
          </div>
          {sourceMentions.length > 0 && (
            <InspectorSection label={`Exact mentions · ${sourceMentions.length}`}>
              {sourceMentions.map((mention) => (
                <div key={mention.id} className="grid gap-1.5">
                  <p className="m-0 text-body font-semibold">
                    {entityMap.get(mention.entity_id)?.label ??
                      "Entity mention"}
                  </p>
                  <p className="m-0 text-caption text-muted-foreground">
                    Exact UTF-8 bytes{" "}
                    <span className="mono">
                      {mention.byte_start}–{mention.byte_end}
                    </span>
                  </p>
                  <OrganizationQuote>{mention.quote}</OrganizationQuote>
                </div>
              ))}
            </InspectorSection>
          )}
          {topics.length > 0 && (
            <InspectorSection label="Topics">
              {topics.map((id) => (
                <p className="m-0 text-body" key={id}>
                  Source member of {topicMap.get(id)?.label ?? "eligible topic"}
                </p>
              ))}
            </InspectorSection>
          )}
          <Button
            className="justify-self-start"
            disabled={!sourceChoice(selectedSource)}
            onClick={() => onInspect(selectedSource)}
          >
            <FileText aria-hidden="true" />
            Open exact source
          </Button>
        </>
      )}
    </GraphInspector>
  );
}

function Pager({
  label: groupLabel,
  previous,
  next,
  status,
  previousDisabled,
  nextDisabled,
  onPrevious,
  onNext,
}: {
  label?: string;
  previous: string;
  next: string;
  status: ReactNode;
  previousDisabled: boolean;
  nextDisabled: boolean;
  onPrevious: () => void;
  onNext: () => void;
}) {
  return (
    <div
      role={groupLabel ? "group" : undefined}
      aria-label={groupLabel}
      className="flex flex-wrap items-center justify-between gap-2"
    >
      <Button
        variant="ghost"
        size="sm"
        disabled={previousDisabled}
        onClick={onPrevious}
      >
        <ChevronLeft aria-hidden="true" />
        {previous}
      </Button>
      {status}
      <Button variant="ghost" size="sm" disabled={nextDisabled} onClick={onNext}>
        {next}
        <ChevronRight aria-hidden="true" />
      </Button>
    </div>
  );
}

export function OrganizationPager({
  historyLength,
  hasNext,
  onPrevious,
  onNext,
}: {
  historyLength: number;
  hasNext: boolean;
  onPrevious: () => void;
  onNext: () => void;
}) {
  return (
    <Pager
      previous="Previous organization page"
      next="Next organization page"
      previousDisabled={!historyLength}
      nextDisabled={!hasNext}
      onPrevious={onPrevious}
      onNext={onNext}
      status={
        <p className="m-0 text-caption text-muted-foreground">
          Page <span className="mono">{historyLength + 1}</span> · bounded
          cards and source details
        </p>
      }
    />
  );
}

export function SourceMembershipPager({
  stream,
}: {
  stream: ReturnType<typeof useOrganizationSources>;
}) {
  return (
    <Pager
      label="Source membership pages"
      previous="Previous source membership page"
      next="Next source membership page"
      previousDisabled={!stream.hasPrevious || stream.pending}
      nextDisabled={!stream.hasNext || stream.pending}
      onPrevious={stream.previous}
      onNext={stream.next}
      status={
        <p className="m-0 text-caption text-muted-foreground" role="status">
          {stream.pending
            ? "Reading current source memberships…"
            : `Source membership page ${stream.pageNumber} · bounded source contributors`}
        </p>
      }
    />
  );
}

export function SourceNeighborhood({
  receipt,
  maps,
  selected,
  inspector,
  onChoose,
}: {
  receipt: string | null;
  maps: OrganizationMaps;
  selected: GraphChoice | null;
  inspector: ReactNode;
  onChoose: (value: GraphChoice) => void;
}) {
  return (
    <section aria-label="Expanded topic sources" className="grid gap-3">
      <SectionHeader
        title="Source neighborhood"
        description="One identity per eligible source, including overlapping topics. Positions express organization; source-described entity links are listed below. Evidence view provides canonical knowledge paths."
      />
      <div className="organization-graph flex h-[46rem] min-h-[42rem] min-w-0 overflow-hidden rounded-lg border border-border shadow-sm">
        <GraphRenderer
          key={receipt}
          nodes={maps.evidenceNodes}
          edges={emptyEdges}
          displayGroups={maps.displayGroups}
          choice={selected}
          onChoose={onChoose}
          pathNodes={emptyKeys}
          pathEdges={emptyKeys}
          inspector={inspector}
        />
      </div>
    </section>
  );
}

export function SourceEvidenceList({
  maps,
  limit,
  centered,
  onMore,
  onSelect,
  onInspect,
}: {
  maps: OrganizationMaps;
  limit: number;
  centered: boolean;
  onMore: () => void;
  onSelect: (node: EvidenceNode) => void;
  onInspect: (node: EvidenceNode) => void;
}) {
  const { evidenceNodes, mentions } = maps;
  return (
    <section aria-label="Organization source evidence" className="grid gap-3">
      <SectionHeader title="Sources in this read" />
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
        {evidenceNodes.slice(0, limit).map((node) => (
          <Card
            key={node.key}
            className="organization-card grid content-start gap-2 p-3 [overflow-wrap:anywhere]"
          >
            <p className="m-0 text-body font-semibold">{node.evidence.label}</p>
            <p className="mono m-0 text-caption text-muted-foreground">
              {
                mentions.filter((mention) => mention.source_key === node.key)
                  .length
              }{" "}
              exact mentions in this read
            </p>
            <div className="flex flex-wrap gap-1.5">
              <Button
                size="sm"
                variant="secondary"
                onClick={() => {
                  onSelect(node);
                  if (!centered) onInspect(node);
                }}
              >
                Inspect source
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={!sourceChoice(node)}
                onClick={() => onInspect(node)}
              >
                Open exact source
              </Button>
            </div>
          </Card>
        ))}
      </div>
      {limit < evidenceNodes.length && (
        <Button className="justify-self-start" variant="ghost" onClick={onMore}>
          More loaded sources
        </Button>
      )}
    </section>
  );
}

export function ConnectionList({
  view,
  maps,
  limit,
  onMore,
  onInspect,
}: {
  view: View;
  maps: OrganizationMaps;
  limit: number;
  onMore: () => void;
  onInspect: (node: EvidenceNode) => void;
}) {
  const { entityMap, sourceMap } = maps;
  return (
    <section
      aria-label="Source-described entity relationships"
      className="grid gap-3"
    >
      <SectionHeader
        title="Source-described connections"
        description="A source describes these connections. They do not establish accepted operational truth or a canonical knowledge path."
      />
      <div className="grid gap-3 xl:grid-cols-2">
        {view.connections.slice(0, limit).map((connection) => (
          <Card
            key={connection.id}
            className="grid content-start gap-2 p-3 [overflow-wrap:anywhere]"
          >
            <div className="flex items-start justify-between gap-2">
              <p className="m-0 text-body font-semibold">
                {entityMap.get(connection.from)?.label ?? "Entity"} →{" "}
                {entityMap.get(connection.to)?.label ?? "Entity"}
              </p>
              <Badge>
                {connection.interpretation === "supported_alias"
                  ? "Supported alias"
                  : "Source-described link"}
              </Badge>
            </div>
            <p className="m-0 text-label text-foreground-secondary">
              {label(connection.relation)}
            </p>
            <OrganizationQuote>{connection.quote}</OrganizationQuote>
            <p className="m-0 text-caption text-muted-foreground">
              Exact UTF-8 bytes{" "}
              <span className="mono">
                {connection.byte_start}–{connection.byte_end}
              </span>
            </p>
            <Button
              size="sm"
              variant="ghost"
              className="justify-self-start"
              disabled={!sourceMap.has(connection.source_key)}
              onClick={() => {
                const node = sourceMap.get(connection.source_key);
                if (node) onInspect(node);
              }}
            >
              Inspect connection source
            </Button>
          </Card>
        ))}
      </div>
      {limit < view.connections.length && (
        <Button className="justify-self-start" variant="ghost" onClick={onMore}>
          More loaded connections
        </Button>
      )}
    </section>
  );
}

export function OrganizationSourcesSection({
  view,
  center,
  receipt,
  maps,
  history,
  sourceStream,
  sourcePage,
  selected,
  selectedSource,
  sourceLimit,
  connectionLimit,
  onPrevious,
  onNext,
  onChoose,
  onSelect,
  onInspect,
  onMoreSources,
  onMoreConnections,
}: {
  view: View;
  center: Center | null;
  receipt: string | null;
  maps: OrganizationMaps;
  history: number;
  sourceStream: ReturnType<typeof useOrganizationSources>;
  sourcePage: SourcePage | undefined;
  selected: GraphChoice | null;
  selectedSource: EvidenceNode | undefined;
  sourceLimit: number;
  connectionLimit: number;
  onPrevious: () => void;
  onNext: () => void;
  onChoose: (value: GraphChoice) => void;
  onSelect: (node: EvidenceNode) => void;
  onInspect: (node: EvidenceNode) => void;
  onMoreSources: () => void;
  onMoreConnections: () => void;
}) {
  const { evidenceNodes } = maps;
  return (
    <>
      <OrganizationPager
        historyLength={history}
        hasNext={!!view.next_cursor}
        onPrevious={onPrevious}
        onNext={onNext}
      />
      {center?.kind !== "entity" &&
        (sourceStream.hasNext ||
          sourceStream.hasPrevious ||
          sourceStream.pending) && (
          <SourceMembershipPager stream={sourceStream} />
        )}
      {sourcePage?.coverage.counts_are_lower_bounds && (
        <p className="m-0 text-caption text-muted-foreground">
          Source membership coverage is partial. Narrow the scope or continue
          through its independent pages.
        </p>
      )}
      {center && evidenceNodes.length > 0 && (
        <SourceNeighborhood
          receipt={receipt}
          maps={maps}
          selected={selected}
          onChoose={onChoose}
          inspector={
            <SourceInspector
              selectedSource={selectedSource}
              maps={maps}
              onInspect={onInspect}
              onClose={() => onChoose(null)}
            />
          }
        />
      )}
      {evidenceNodes.length > 0 && (
        <SourceEvidenceList
          maps={maps}
          limit={sourceLimit}
          centered={!!center}
          onMore={onMoreSources}
          onSelect={onSelect}
          onInspect={onInspect}
        />
      )}
      {view.connections.length > 0 && (
        <ConnectionList
          view={view}
          maps={maps}
          limit={connectionLimit}
          onMore={onMoreConnections}
          onInspect={onInspect}
        />
      )}
    </>
  );
}
