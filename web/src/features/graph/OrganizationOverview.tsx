import type { ReactNode } from "react";
import { motion } from "motion/react";
import { ChevronRight, FolderTree, Pin } from "lucide-react";
import { EmptyState } from "../../components/AsyncState";
import { SectionHeader } from "../../components/PageHeader";
import { Badge, Button, Card, StatusBadge } from "../../components/ui";
import { listItem } from "../../design/motion";
import { stateTone } from "./GraphStatusDrawer";
import {
  count,
  label,
  type Center,
  type OrganizationMaps,
  type View,
} from "./organizationModel";

const cardGrid = "grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4";

export function OrganizationCoverage({
  view,
  lower,
}: {
  view: View;
  lower: boolean;
}) {
  return (
    <Card className="organization-coverage grid gap-3 p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="mono m-0 text-label text-foreground">
          {count(view.coverage.qualified_sources, lower)} eligible sources ·{" "}
          {count(view.coverage.mapped_sources, lower)} mapped ·{" "}
          {count(view.coverage.unassigned_sources, lower)} unassigned in this
          read
        </p>
        <StatusBadge state={view.state === "partial" ? "attention" : "neutral"}>
          {label(view.state)}
        </StatusBadge>
      </div>
      {lower && (
        <p className="m-0 text-caption text-muted-foreground">
          Counts are lower bounds for this bounded read. Narrow the scope or
          read the next page for more eligible sources.
        </p>
      )}
      {(view.producer_status || view.community_status) && (
        <div className="grid gap-2 border-t border-border pt-3">
          {view.producer_status && (
            <div
              className="flex flex-wrap items-center gap-2"
              aria-label="Entity producer readiness"
            >
              <StatusBadge state={stateTone(view.producer_status)}>
                {label(view.producer_status)}
              </StatusBadge>
              <p className="m-0 text-label text-foreground-secondary">
                {view.producer_status === "ready"
                  ? "A qualified compatible entity producer is selected. Extraction follows this Brain’s model policy."
                  : "Entity extraction awaits a qualified compatible producer."}
              </p>
            </div>
          )}
          {view.community_status && (
            <div
              className="flex flex-wrap items-center gap-2"
              aria-label="Native community readiness"
            >
              <StatusBadge state={stateTone(view.community_status.state)}>
                {label(view.community_status.state)}
              </StatusBadge>
              <p className="m-0 text-label text-foreground-secondary">
                {view.community_status.state === "ready"
                  ? "Qualified native communities are current; only eligible memberships appear."
                  : view.community_status.state === "unconfigured"
                    ? "Native communities await a qualified recipe. Entity facets and canonical references remain separate."
                    : `Native communities are ${label(view.community_status.state)}. Entity facets and canonical references remain separate.`}
              </p>
              {view.community_status.vector_features && (
                <p className="m-0 text-caption text-muted-foreground">
                  Retained semantic features:{" "}
                  {label(view.community_status.vector_features)}.
                </p>
              )}
            </div>
          )}
        </div>
      )}
      {view.coverage.unassigned_sources > 0 && (
        <p className="m-0 text-label text-muted-foreground">
          Unassigned sources remain available as source evidence.
        </p>
      )}
    </Card>
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
    <Card className="p-0">
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
            <Button variant="secondary" onClick={() => onExpand(null)}>
              All topics
            </Button>
          ) : undefined
        }
      />
    </Card>
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
    <section aria-label="Topic overview" className="grid gap-3">
      <SectionHeader
        title={center?.kind === "topic" ? "Expanded topic" : "Topics"}
        actions={
          <span className="text-caption text-muted-foreground">
            Counts apply to this read
          </span>
        }
      />
      <div className={cardGrid}>
        {view.topics.map((topic, index) => (
          <motion.div
            key={topic.id}
            variants={listItem}
            custom={index}
            initial="initial"
            animate="animate"
            className="min-w-0"
          >
            <Card
              className="organization-card grid h-full content-start gap-2 [overflow-wrap:anywhere]"
              data-testid="organization-topic"
              data-topic-id={topic.id}
            >
              <div className="flex flex-wrap items-center gap-1.5">
                <FolderTree
                  className="mr-auto size-5 text-muted-foreground"
                  aria-hidden="true"
                />
                {topic.topic_kind === "manual_overlay" ? (
                  <Badge variant="outline">Manual group</Badge>
                ) : (
                  [...sourceMemberships, ...representativeSources].some(
                    (member) => member.topic_id === topic.id,
                  ) && <Badge variant="outline">Source group</Badge>
                )}
                {topic.pinned && (
                  <Badge>
                    <Pin aria-hidden="true" />
                    Pinned
                  </Badge>
                )}
              </div>
              <h3 className="m-0 text-title font-semibold">{topic.label}</h3>
              <p className="mono m-0 text-caption text-muted-foreground">
                {count(topic.source_count, lower)}{" "}
                {topic.source_count === 1 && !lower ? "source" : "sources"} ·{" "}
                {count(topic.entity_count, lower)}{" "}
                {topic.entity_count === 1 && !lower ? "entity" : "entities"}
              </p>
              <div className="flex flex-wrap gap-1.5 pt-1">
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => onExpand({ kind: "topic", id: topic.id })}
                  aria-label={`Expand topic ${topic.label}`}
                >
                  Explore topic
                  <ChevronRight aria-hidden="true" />
                </Button>
                {writable && topic.change_id && (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => onOrganize(topic.id)}
                    aria-label={`Organize topic ${topic.label}`}
                  >
                    Organize
                  </Button>
                )}
              </div>
            </Card>
          </motion.div>
        ))}
      </div>
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
    <section aria-label="Entity overview" className="grid gap-3">
      <SectionHeader
        title={center?.kind === "entity" ? "Expanded entity" : "Entities"}
      />
      <div className={cardGrid}>
        {view.entities.map((entity, index) => (
          <motion.div
            key={entity.id}
            variants={listItem}
            custom={index}
            initial="initial"
            animate="animate"
            className="min-w-0"
          >
            <Card
              className="organization-card grid h-full content-start gap-2 p-3 [overflow-wrap:anywhere]"
              data-testid="organization-entity"
            >
              <div className="flex items-start justify-between gap-2">
                <p className="m-0 text-body font-semibold">{entity.label}</p>
                <Badge>{label(entity.kind)}</Badge>
              </div>
              <p className="mono m-0 text-caption text-muted-foreground">
                {count(entity.source_count, lower)} supporting sources in this
                read
              </p>
              <Button
                size="sm"
                variant="ghost"
                className="justify-self-start"
                onClick={() => onExpand({ kind: "entity", id: entity.id })}
                aria-label={`Expand entity ${entity.label}`}
              >
                Explore entity
              </Button>
            </Card>
          </motion.div>
        ))}
      </div>
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
    <details className="organization-context group grid gap-3">
      <summary className="flex cursor-pointer list-none items-center gap-1.5 text-body font-medium text-foreground [&::-webkit-details-marker]:hidden">
        <ChevronRight
          aria-hidden="true"
          className="size-4 text-muted-foreground transition-transform duration-fast group-open:rotate-90"
        />
        Additional source context · {contextTopics.length} topics ·{" "}
        {contextEntities.length} entities
      </summary>
      <div className={`${cardGrid} mt-3`}>
        {[...contextTopics, ...contextEntities].map((card) => (
          <Card
            key={card.id}
            className="organization-card grid content-start gap-2 p-3 [overflow-wrap:anywhere]"
          >
            <div className="flex items-start justify-between gap-2">
              <p className="m-0 text-body font-semibold">{card.label}</p>
              <Badge variant="outline">
                {card.topic_kind === "manual_overlay"
                  ? "Manual group"
                  : card.kind === "topic"
                    ? "Topic context"
                    : label(card.kind)}
              </Badge>
            </div>
            <p className="mono m-0 text-caption text-muted-foreground">
              {count(card.source_count, lower)} supporting sources in this read
            </p>
            <div className="flex flex-wrap gap-1.5">
              <Button
                size="sm"
                variant="ghost"
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
                  size="sm"
                  variant="ghost"
                  onClick={() => onOrganize(card.id)}
                >
                  Organize context topic
                </Button>
              )}
            </div>
          </Card>
        ))}
      </div>
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
