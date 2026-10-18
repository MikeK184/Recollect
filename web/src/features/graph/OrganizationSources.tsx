import type { ReactNode } from "react";
import {
  Badge,
  Button,
  Card,
  Group,
  SimpleGrid,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { GraphRenderer } from "../../GraphRenderer";
import type { GraphChoice } from "../../GraphCanvas";
import type { components } from "../../api-schema";
import type { useOrganizationSources } from "./useOrganizationSources";
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

export function SourceInspector({
  selectedSource,
  maps,
  onInspect,
}: {
  selectedSource: EvidenceNode | undefined;
  maps: OrganizationMaps;
  onInspect: (node: EvidenceNode) => void;
}) {
  const {
    mentions,
    entityMap,
    sourceMemberships,
    representativeSources,
    topicMap,
  } = maps;
  return (
    <aside
      className="organization-inspector"
      aria-label="Selected organization evidence"
    >
      <Stack gap="sm">
        {selectedSource ? (
          <>
            <Title order={4}>{selectedSource.evidence.label}</Title>
            <Badge variant="light" w="fit-content">
              Source evidence
            </Badge>
            <Text size="xs" c="dimmed">
              {selectedSource.key}
            </Text>
            {mentions
              .filter((mention) => mention.source_key === selectedSource.key)
              .slice(0, 20)
              .map((mention) => (
                <Card key={mention.id} withBorder p="sm">
                  <Text size="sm" fw={600}>
                    {entityMap.get(mention.entity_id)?.label ??
                      "Entity mention"}
                  </Text>
                  <Text size="xs" c="dimmed">
                    Exact UTF-8 bytes {mention.byte_start}–{mention.byte_end}
                  </Text>
                  <pre className="organization-quote">{mention.quote}</pre>
                </Card>
              ))}
            {[
              ...new Set(
                [...sourceMemberships, ...representativeSources]
                  .filter((member) => member.source_key === selectedSource.key)
                  .map((member) => member.topic_id),
              ),
            ].map((id) => (
              <Text size="sm" key={id}>
                Source member of {topicMap.get(id)?.label ?? "eligible topic"}
              </Text>
            ))}
            <Button
              disabled={!sourceChoice(selectedSource)}
              onClick={() => onInspect(selectedSource)}
            >
              Open exact source
            </Button>
          </>
        ) : (
          <>
            <Title order={4}>Source inspector</Title>
            <Text size="sm" c="dimmed">
              Select a source in the graph or the list to inspect exact mentions
              and supporting text.
            </Text>
          </>
        )}
      </Stack>
    </aside>
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
    <Group justify="space-between">
      <Button
        variant="subtle"
        disabled={!historyLength}
        onClick={onPrevious}
      >
        Previous organization page
      </Button>
      <Text size="xs" c="dimmed">
        Page {historyLength + 1} · bounded cards and source details
      </Text>
      <Button variant="subtle" disabled={!hasNext} onClick={onNext}>
        Next organization page
      </Button>
    </Group>
  );
}

export function SourceMembershipPager({
  stream,
}: {
  stream: ReturnType<typeof useOrganizationSources>;
}) {
  return (
    <Group justify="space-between" aria-label="Source membership pages">
      <Button
        variant="subtle"
        disabled={!stream.hasPrevious || stream.pending}
        onClick={stream.previous}
      >
        Previous source membership page
      </Button>
      <Text size="xs" c="dimmed" role="status">
        {stream.pending
          ? "Reading current source memberships…"
          : `Source membership page ${stream.pageNumber} · bounded source contributors`}
      </Text>
      <Button
        variant="subtle"
        disabled={!stream.hasNext || stream.pending}
        onClick={stream.next}
      >
        Next source membership page
      </Button>
    </Group>
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
    <section aria-label="Expanded topic sources">
      <Title order={3} mb="xs">
        Source neighborhood
      </Title>
      <Text size="sm" c="dimmed" mb="sm">
        One identity per eligible source, including overlapping topics.
        Positions express organization; source-described entity links are listed
        below. Evidence view provides canonical knowledge paths.
      </Text>
      <div className="organization-graph">
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
    <section aria-label="Organization source evidence">
      <Title order={3} mb="sm">
        Sources in this read
      </Title>
      <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="sm">
        {evidenceNodes.slice(0, limit).map((node) => (
          <Card
            key={node.key}
            withBorder
            p="sm"
            className="organization-card"
          >
            <Text fw={600}>{node.evidence.label}</Text>
            <Text size="xs" c="dimmed" mt="xs">
              {
                mentions.filter((mention) => mention.source_key === node.key)
                  .length
              }{" "}
              exact mentions in this read
            </Text>
            <Group mt="sm">
              <Button
                size="xs"
                variant="light"
                onClick={() => {
                  onSelect(node);
                  if (!centered) onInspect(node);
                }}
              >
                Inspect source
              </Button>
              <Button
                size="xs"
                variant="subtle"
                disabled={!sourceChoice(node)}
                onClick={() => onInspect(node)}
              >
                Open exact source
              </Button>
            </Group>
          </Card>
        ))}
      </SimpleGrid>
      {limit < evidenceNodes.length && (
        <Button mt="sm" variant="subtle" onClick={onMore}>
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
    <section aria-label="Source-described entity relationships">
      <Title order={3} mb="xs">
        Source-described connections
      </Title>
      <Text size="sm" c="dimmed" mb="sm">
        A source describes these connections. They do not establish accepted
        operational truth or a canonical knowledge path.
      </Text>
      <Stack gap="sm">
        {view.connections.slice(0, limit).map((connection) => (
          <Card key={connection.id} withBorder p="sm">
            <Group justify="space-between">
              <Text fw={600}>
                {entityMap.get(connection.from)?.label ?? "Entity"} →{" "}
                {entityMap.get(connection.to)?.label ?? "Entity"}
              </Text>
              <Badge variant="light">
                {connection.interpretation === "supported_alias"
                  ? "Supported alias"
                  : "Source-described link"}
              </Badge>
            </Group>
            <Text size="sm" mt="xs">
              {label(connection.relation)}
            </Text>
            <pre className="organization-quote">{connection.quote}</pre>
            <Text size="xs" c="dimmed">
              Exact UTF-8 bytes {connection.byte_start}–{connection.byte_end}
            </Text>
            <Button
              mt="xs"
              size="xs"
              variant="subtle"
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
      </Stack>
      {limit < view.connections.length && (
        <Button mt="sm" variant="subtle" onClick={onMore}>
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
          sourceStream.pending) && <SourceMembershipPager stream={sourceStream} />}
      {sourcePage?.coverage.counts_are_lower_bounds && (
        <Text size="xs" c="dimmed">
          Source membership coverage is partial. Narrow the scope or continue
          through its independent pages.
        </Text>
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
