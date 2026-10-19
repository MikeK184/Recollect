import type { ReactNode } from "react";
import {
  Badge,
  Button,
  Card,
  Group,
  SimpleGrid,
  Text,
  Title,
} from "@mantine/core";
import { FolderTree, Pin } from "lucide-react";
import { EmptyState } from "../../components/AsyncState";
import { iconSize } from "../../design/tokens";
import {
  count,
  label,
  type Center,
  type OrganizationMaps,
  type View,
} from "./organizationModel";

export function OrganizationCoverage({
  view,
  lower,
}: {
  view: View;
  lower: boolean;
}) {
  return (
    <>
      <Card withBorder p="sm" className="organization-coverage">
        <Group justify="space-between">
          <Text size="sm">
            {count(view.coverage.qualified_sources, lower)} eligible sources ·{" "}
            {count(view.coverage.mapped_sources, lower)} mapped ·{" "}
            {count(view.coverage.unassigned_sources, lower)} unassigned in this
            read
          </Text>
          <Badge
            variant="light"
            color={view.state === "partial" ? "yellow" : "gray"}
          >
            {label(view.state)}
          </Badge>
        </Group>
        {lower && (
          <Text size="xs" c="dimmed" mt={4}>
            Counts are lower bounds for this bounded read. Narrow the scope or
            read the next page for more eligible sources.
          </Text>
        )}
      </Card>
      {view.producer_status && (
        <Group gap="xs" aria-label="Entity producer readiness">
          <Group gap="xs">
            <Badge variant="light">{label(view.producer_status)}</Badge>
            <Text size="xs">
              {view.producer_status === "ready"
                ? "A qualified compatible entity producer is selected. Extraction follows this Brain’s model policy."
                : "Entity extraction awaits a qualified compatible producer."}
            </Text>
          </Group>
        </Group>
      )}
      {view.community_status && (
        <Group gap="xs" aria-label="Native community readiness">
          <Group gap="xs">
            <Badge variant="light">{label(view.community_status.state)}</Badge>
            <Text size="xs">
              {view.community_status.state === "ready"
                ? "Qualified native communities are current; only eligible memberships appear."
                : view.community_status.state === "unconfigured"
                  ? "Native communities await a qualified recipe. Entity facets and canonical references remain separate."
                  : `Native communities are ${label(view.community_status.state)}. Entity facets and canonical references remain separate.`}
            </Text>
            {view.community_status.vector_features && (
              <Text size="xs" c="dimmed">
                Retained semantic features:{" "}
                {label(view.community_status.vector_features)}.
              </Text>
            )}
          </Group>
        </Group>
      )}
      {view.coverage.unassigned_sources > 0 && (
        <Text size="sm" c="dimmed">
          Unassigned sources remain available as source evidence.
        </Text>
      )}
    </>
  );
}

export function OrganizationEmpty({
  view,
  center,
  maps,
  onExpand,
}: {
  view: View;
  center: Center | null;
  maps: OrganizationMaps;
  onExpand: (next: Center | null) => void;
}) {
  if (
    !(
      view.topics.length === 0 &&
      view.entities.length === 0 &&
      maps.contextTopics.length === 0 &&
      maps.contextEntities.length === 0 &&
      maps.mentions.length === 0 &&
      view.memberships.length === 0 &&
      maps.sourceMemberships.length === 0 &&
      maps.representativeSources.length === 0 &&
      view.connections.length === 0
    )
  )
    return null;
  return (
    <EmptyState
      icon={FolderTree}
      title={
        center
          ? "No eligible support in this scope"
          : "No published topics yet"
      }
      description={
        view.producer_status && view.producer_status !== "ready"
          ? "Entity extraction awaits a qualified compatible producer. Canonical field references and native source communities keep their own readiness and evidence."
          : "Eligible sources without usable mappings stay unassigned. Add or process source evidence, then refresh organization."
      }
      action={
        center ? (
          <Button variant="light" onClick={() => onExpand(null)}>
            All topics
          </Button>
        ) : undefined
      }
    />
  );
}

export function OrganizationTopics({
  view,
  center,
  lower,
  writable,
  maps,
  onExpand,
  onOrganize,
}: {
  view: View;
  center: Center | null;
  lower: boolean;
  writable: boolean;
  maps: OrganizationMaps;
  onExpand: (next: Center | null) => void;
  onOrganize: (id: string) => void;
}) {
  const { sourceMemberships, representativeSources } = maps;
  return (
    <section aria-label="Topic overview">
      <Group justify="space-between" mb="sm">
        <Title order={3}>
          {center?.kind === "topic" ? "Expanded topic" : "Topics"}
        </Title>
        <Text size="xs" c="dimmed">
          Counts apply to this read
        </Text>
      </Group>
      <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="sm">
        {view.topics.map((topic) => (
          <Card
            key={topic.id}
            withBorder
            p="md"
            className="organization-card"
            data-testid="organization-topic"
            data-topic-id={topic.id}
          >
            <Group justify="space-between" mb="xs">
              <FolderTree size={iconSize.action} />
              {topic.topic_kind === "manual_overlay" ? (
                <Badge variant="outline">Manual group</Badge>
              ) : (
                [...sourceMemberships, ...representativeSources].some(
                  (member) => member.topic_id === topic.id,
                ) && <Badge variant="outline">Source group</Badge>
              )}
              {topic.pinned && (
                <Badge variant="light" leftSection={<Pin size={12} />}>
                  Pinned
                </Badge>
              )}
            </Group>
            <Title order={4}>{topic.label}</Title>
            <Text size="sm" c="dimmed" mt="xs">
              {count(topic.source_count, lower)}{" "}
              {topic.source_count === 1 && !lower ? "source" : "sources"} ·{" "}
              {count(topic.entity_count, lower)}{" "}
              {topic.entity_count === 1 && !lower ? "entity" : "entities"}
            </Text>
            <Group mt="md">
              <Button
                size="xs"
                variant="light"
                onClick={() => onExpand({ kind: "topic", id: topic.id })}
                aria-label={`Expand topic ${topic.label}`}
              >
                Explore topic
              </Button>
              {writable && topic.change_id && (
                <Button
                  size="xs"
                  variant="subtle"
                  onClick={() => onOrganize(topic.id)}
                  aria-label={`Organize topic ${topic.label}`}
                >
                  Organize
                </Button>
              )}
            </Group>
          </Card>
        ))}
      </SimpleGrid>
    </section>
  );
}

export function OrganizationEntities({
  view,
  center,
  lower,
  onExpand,
}: {
  view: View;
  center: Center | null;
  lower: boolean;
  onExpand: (next: Center | null) => void;
}) {
  return (
    <section aria-label="Entity overview">
      <Title order={3} mb="sm">
        {center?.kind === "entity" ? "Expanded entity" : "Entities"}
      </Title>
      <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="sm">
        {view.entities.map((entity) => (
          <Card
            key={entity.id}
            withBorder
            p="sm"
            className="organization-card"
            data-testid="organization-entity"
          >
            <Group justify="space-between">
              <Text fw={600}>{entity.label}</Text>
              <Badge variant="light">{label(entity.kind)}</Badge>
            </Group>
            <Text size="xs" c="dimmed" mt="xs">
              {count(entity.source_count, lower)} supporting sources in this
              read
            </Text>
            <Button
              mt="sm"
              size="compact-xs"
              variant="subtle"
              onClick={() => onExpand({ kind: "entity", id: entity.id })}
              aria-label={`Expand entity ${entity.label}`}
            >
              Explore entity
            </Button>
          </Card>
        ))}
      </SimpleGrid>
    </section>
  );
}

export function OrganizationContext({
  lower,
  writable,
  maps,
  onExpand,
  onOrganize,
}: {
  lower: boolean;
  writable: boolean;
  maps: OrganizationMaps;
  onExpand: (next: Center | null) => void;
  onOrganize: (id: string) => void;
}) {
  const { contextTopics, contextEntities } = maps;
  return (
    <details className="organization-context">
      <summary>
        Additional source context · {contextTopics.length} topics ·{" "}
        {contextEntities.length} entities
      </summary>
      <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} mt="sm" spacing="sm">
        {[...contextTopics, ...contextEntities].map((card) => (
          <Card
            withBorder
            key={card.id}
            p="sm"
            className="organization-card"
          >
            <Group justify="space-between">
              <Text fw={600}>{card.label}</Text>
              <Badge variant="outline">
                {card.topic_kind === "manual_overlay"
                  ? "Manual group"
                  : card.kind === "topic"
                    ? "Topic context"
                    : label(card.kind)}
              </Badge>
            </Group>
            <Text size="xs" c="dimmed" mt="xs">
              {count(card.source_count, lower)} supporting sources in this read
            </Text>
            <Group mt="sm">
              <Button
                size="xs"
                variant="subtle"
                onClick={() =>
                  onExpand({
                    kind: card.kind === "topic" ? "topic" : "entity",
                    id: card.id,
                  })
                }
              >
                Explore source context
              </Button>
              {card.kind === "topic" && writable && card.change_id && (
                <Button
                  size="xs"
                  variant="subtle"
                  onClick={() => onOrganize(card.id)}
                >
                  Organize context topic
                </Button>
              )}
            </Group>
          </Card>
        ))}
      </SimpleGrid>
    </details>
  );
}

export function OrganizationOverviewSection({
  view,
  center,
  lower,
  writable,
  maps,
  fieldPanel,
  onExpand,
  onOrganize,
}: {
  view: View;
  center: Center | null;
  lower: boolean;
  writable: boolean;
  maps: OrganizationMaps;
  fieldPanel: ReactNode;
  onExpand: (next: Center | null) => void;
  onOrganize: (id: string) => void;
}) {
  const noTopicsOrEntities =
    view.topics.length === 0 && view.entities.length === 0;
  return (
    <>
      <OrganizationCoverage view={view} lower={lower} />
      {noTopicsOrEntities && fieldPanel}
      <OrganizationEmpty
        view={view}
        center={center}
        maps={maps}
        onExpand={onExpand}
      />
      {view.topics.length > 0 && (
        <OrganizationTopics
          view={view}
          center={center}
          lower={lower}
          writable={writable}
          maps={maps}
          onExpand={onExpand}
          onOrganize={onOrganize}
        />
      )}
      {view.entities.length > 0 && (
        <OrganizationEntities
          view={view}
          center={center}
          lower={lower}
          onExpand={onExpand}
        />
      )}
      {(view.topics.length > 0 || view.entities.length > 0) && fieldPanel}
      {(maps.contextTopics.length > 0 || maps.contextEntities.length > 0) && (
        <OrganizationContext
          lower={lower}
          writable={writable}
          maps={maps}
          onExpand={onExpand}
          onOrganize={onOrganize}
        />
      )}
    </>
  );
}
