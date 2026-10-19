import { useMemo } from "react";
import type { components } from "../../api-schema";
import type { GraphDisplayGroup } from "../../GraphRenderer";

export type Scope = components["schemas"]["GraphSelection"];
export type View = components["schemas"]["OrganizationView"];
export type Center = components["schemas"]["OrganizationCenter"];
export type Cursor = components["schemas"]["OrganizationCursor"];
export type SemanticCard = components["schemas"]["OrganizationCard"];
export type TopicChange = components["schemas"]["TopicOverride"];
export type EvidenceChoice = components["schemas"]["ClaimEvidenceChoice"];
export type EvidenceNode = components["schemas"]["GraphNode"];
type SourcePage = components["schemas"]["OrganizationSourcesView"];

export const emptyKeys: string[] = [];
export const emptyEdges: never[] = [];
export const label = (value: string) => value.replaceAll("_", " ");
export const count = (value: number, lower: boolean) =>
  `${lower ? "≥" : ""}${value}`;

export function sourceChoice(node: EvidenceNode): EvidenceChoice | null {
  const provenance = node.evidence.provenance.find(
    (item) => item.kind === "source_version",
  );
  if (!provenance) return null;
  return {
    kind: provenance.kind,
    id: provenance.id,
    label: node.evidence.label,
    source_id: provenance.source_id,
    repository_id: provenance.repository_id,
    snapshot_id: provenance.snapshot_id,
    revision: provenance.revision,
    reference: provenance.path,
    observed_at: null,
    created_at: node.evidence.recorded_at,
    availability: provenance.availability,
  };
}

/** Lookup maps and merged lists derived from one organization read plus its source page. */
export function useOrganizationMaps(
  view: View | undefined,
  sourcePage: SourcePage | undefined,
) {
  const sourceMap = useMemo(
    () =>
      new Map(
        [...(view?.evidence ?? []), ...(sourcePage?.evidence ?? [])].map(
          (node) => [node.key, node],
        ),
      ),
    [view, sourcePage],
  );
  const entityMap = useMemo(
    () =>
      new Map(
        [...(view?.context_cards ?? []), ...(view?.entities ?? [])]
          .filter((card) => card.kind !== "topic")
          .map((card) => [card.id, card]),
      ),
    [view],
  );
  const topicMap = useMemo(
    () =>
      new Map(
        [
          ...(view?.context_cards ?? []),
          ...(view?.topics ?? []),
          ...(sourcePage?.topics ?? []),
        ]
          .filter((card) => card.kind === "topic")
          .map((card) => [card.id, card]),
      ),
    [view, sourcePage],
  );
  const sourceMemberships = [
    ...(view?.source_memberships ?? []),
    ...(sourcePage?.source_memberships ?? []),
  ];
  const representativeSources = [
    ...(view?.representative_sources ?? []),
    ...(sourcePage?.representative_sources ?? []),
  ];
  const evidenceNodes = [...sourceMap.values()];
  const mentions = useMemo(
    () => [
      ...new Map(
        [
          ...(view?.representative_mentions ?? []),
          ...(view?.mentions ?? []),
        ].map((mention) => [mention.id, mention]),
      ).values(),
    ],
    [view],
  );
  const contextEntities = [...entityMap.values()].filter(
    (card) => !view?.entities.some((entity) => entity.id === card.id),
  );
  const contextTopics = [...topicMap.values()].filter(
    (card) => !view?.topics.some((topic) => topic.id === card.id),
  );
  const displayGroups = useMemo<GraphDisplayGroup[]>(
    () =>
      [...topicMap.values()]
        .map((topic) => ({
          id: topic.id,
          label: topic.label,
          nodeKeys: [
            ...new Set(
              [
                ...(view?.memberships ?? []),
                ...sourceMemberships,
                ...representativeSources,
              ]
                .filter(
                  (member) =>
                    member.topic_id === topic.id &&
                    sourceMap.has(member.source_key),
                )
                .map((member) => member.source_key),
            ),
          ],
        }))
        .filter((group) => group.nodeKeys.length > 0) ?? [],
    [view, sourcePage, sourceMap, topicMap],
  );
  return {
    sourceMap,
    entityMap,
    topicMap,
    sourceMemberships,
    representativeSources,
    evidenceNodes,
    mentions,
    contextEntities,
    contextTopics,
    displayGroups,
  };
}

export type OrganizationMaps = ReturnType<typeof useOrganizationMaps>;
