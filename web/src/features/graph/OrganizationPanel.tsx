import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Drawer,
  Group,
  Loader,
  Modal,
  MultiSelect,
  Select,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import {
  FolderTree,
  Pin,
  SlidersHorizontal,
  ArrowLeft,
  RefreshCw,
} from "lucide-react";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { EvidenceDialog } from "../../ClaimsPanel";
import { GraphRenderer, type GraphDisplayGroup } from "../../GraphRenderer";
import type { GraphChoice } from "../../GraphCanvas";
import { useContentDeadline } from "../../useContentDeadline";
import { useIdempotency } from "../../useIdempotency";
import { EmptyState } from "../../components/AsyncState";
import { iconSize } from "../../design/tokens";
import { AliasReviewPanel } from "./AliasReviewPanel";
import { FieldOrganizationPanel } from "./FieldOrganizationPanel";
import { useOrganizationSources } from "./useOrganizationSources";
import "./organization.css";

type Scope = components["schemas"]["GraphSelection"];
type View = components["schemas"]["OrganizationView"];
type Center = components["schemas"]["OrganizationCenter"];
type Cursor = components["schemas"]["OrganizationCursor"];
type SemanticCard = components["schemas"]["OrganizationCard"];
type TopicChange = components["schemas"]["TopicOverride"];
type EvidenceChoice = components["schemas"]["ClaimEvidenceChoice"];
type EvidenceNode = components["schemas"]["GraphNode"];
const emptyKeys: string[] = [];
const emptyEdges: never[] = [];
const label = (value: string) => value.replaceAll("_", " ");
const count = (value: number, lower: boolean) => `${lower ? "≥" : ""}${value}`;

function sourceChoice(node: EvidenceNode): EvidenceChoice | null {
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

function TopicEditor({
  topic,
  view,
  busy,
  onChange,
}: {
  topic: SemanticCard;
  view: View;
  busy: boolean;
  onChange: (change: TopicChange) => void;
}) {
  const [name, setName] = useState(topic.label),
    [selected, setSelected] = useState<string[]>([]);
  const sources = new Map(view.evidence.map((node) => [node.key, node]));
  const native = [
    ...(view.source_memberships ?? []),
    ...(view.representative_sources ?? []),
  ].filter((member) => member.topic_id === topic.id);
  const sourceTopic = native.length > 0;
  const inputs = [
    ...new Map(
      (sourceTopic ? native : view.memberships)
        .filter((member) => member.topic_id === topic.id)
        .map((member) => [
          member.input_id,
          { id: member.input_id, node: sources.get(member.source_key) },
        ]),
    ).values(),
  ].filter((item) => item.node);
  const titleCounts = new Map<string, number>();
  for (const item of inputs) {
    const title = item.node!.evidence.label;
    titleCounts.set(title, (titleCounts.get(title) ?? 0) + 1);
  }
  const inputLabel = (item: (typeof inputs)[number]) => {
    const node = item.node!,
      title = node.evidence.label;
    return titleCounts.get(title)! > 1
      ? `${title} · version ${node.key.replace(/^source_version:/, "").slice(0, 8)}`
      : title;
  };
  const [basis, setBasis] = useState<string | null>(inputs[0]?.id ?? null);
  const maximumNameBytes = sourceTopic ? 240 : 256;
  const validName =
    name.trim().length > 0 &&
    new TextEncoder().encode(name.trim()).length <= maximumNameBytes &&
    !/[\u0000-\u001f\u007f]/.test(name);
  return (
    <Stack gap="md">
      <Text size="sm">
        Changes apply to this{" "}
        {topic.topic_kind === "manual_overlay"
          ? "manual group"
          : "automatic topic"}
        . Source evidence and manual collections keep their own identities.
      </Text>
      <TextInput
        label="Topic name"
        value={name}
        onChange={(event) => setName(event.currentTarget.value)}
        maxLength={maximumNameBytes}
        disabled={busy}
      />
      <Select
        label="Name supported by"
        value={basis}
        onChange={setBasis}
        allowDeselect={false}
        data={inputs.map((item) => ({
          value: item.id,
          label: inputLabel(item),
        }))}
        disabled={busy}
      />
      <Group>
        <Button
          disabled={busy || !basis || !validName}
          onClick={() =>
            basis &&
            onChange({
              action: sourceTopic ? "rename_source" : "rename",
              label: name.trim(),
              basis_input: basis,
            })
          }
        >
          Save topic name
        </Button>
        <Button
          variant="default"
          disabled={busy}
          onClick={() => onChange({ action: "pin", pinned: !topic.pinned })}
        >
          {topic.pinned ? "Unpin topic" : "Pin topic"}
        </Button>
      </Group>
      <Text fw={600} size="sm">
        Source memberships in this read
      </Text>
      {inputs.length === 0 && (
        <Text size="sm" c="dimmed">
          This page has no editable membership rows. Explore the topic and read
          the next organization page to load its eligible source memberships.
        </Text>
      )}
      <Checkbox.Group value={selected} onChange={setSelected}>
        <Stack gap="xs">
          {inputs.map((item) => (
            <Checkbox
              key={item.id}
              value={item.id}
              label={inputLabel(item)}
              disabled={busy}
            />
          ))}
        </Stack>
      </Checkbox.Group>
      {view.coverage.counts_are_lower_bounds && (
        <Text size="xs" c="dimmed">
          This read is partial. Only the selected, currently eligible source
          memberships change.
        </Text>
      )}
      <Group>
        <Button
          variant="default"
          disabled={busy || !selected.length}
          onClick={() =>
            onChange({
              action: sourceTopic ? "exclude_sources" : "exclude",
              input_ids: selected,
              excluded: true,
            })
          }
        >
          Exclude selected sources
        </Button>
        <Button
          variant="light"
          disabled={busy || !selected.length}
          onClick={() =>
            onChange({
              action: sourceTopic ? "split_sources" : "split",
              input_ids: selected,
            })
          }
        >
          Split selected into topic
        </Button>
      </Group>
      <Text size="xs" c="dimmed">
        Exclusion removes membership from this group. Splitting moves the
        selected memberships into a separate topic; neither action erases source
        evidence.
      </Text>
    </Stack>
  );
}

export function OrganizationPanel({
  brain,
  scope,
  onScopeChange,
}: {
  brain: Brain;
  scope: Scope;
  onScopeChange: (scope: Scope) => void;
}) {
  const cache = useQueryClient();
  const scopeKey = JSON.stringify(scope);
  const [center, setCenter] = useState<Center | null>(null),
    [cursor, setCursor] = useState<Cursor | null>(null),
    [history, setHistory] = useState<(Cursor | null)[]>([]);
  const [nonce, setNonce] = useState(0),
    [filters, setFilters] = useState(false),
    [aliases, setAliases] = useState(false),
    [draft, setDraft] = useState(scope);
  const [choice, setChoice] = useState<{
    key: string;
    value: GraphChoice;
  } | null>(null);
  const [openedSource, setOpenedSource] = useState<{
    key: string;
    evidence: EvidenceChoice;
  } | null>(null);
  const [editing, setEditing] = useState<{ key: string; id: string } | null>(
    null,
  );
  const [commandError, setCommandError] = useState<Error | null>(null),
    [commandBusy, setCommandBusy] = useState(false),
    [saved, setSaved] = useState(false);
  const [sourceLimit, setSourceLimit] = useState(20),
    [connectionLimit, setConnectionLimit] = useState(20);
  const idem = useIdempotency();
  const status = useQuery({
    queryKey: ["graph", brain.id, 0],
    retry: false,
    refetchInterval: 3000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/graph", {
          params: { path: { brain: brain.id }, query: { offset: 0 } },
          signal,
        }),
      ),
  });
  const requestKey = JSON.stringify([
    brain.id,
    scope,
    center,
    cursor,
    nonce,
    status.data?.memory_epoch,
    status.data?.link_epoch,
    status.data?.organization_epoch,
  ]);
  const activeRequest = useRef(requestKey);
  activeRequest.current = requestKey;
  const [read, setRead] = useState<{
    key: string;
    view?: View;
    error?: Error;
    pending: boolean;
  }>({ key: "", pending: true });
  const expires = useContentDeadline(
    read.key === requestKey ? read.view?.expires_at : null,
  );
  const view =
    read.key === requestKey &&
    !read.pending &&
    !read.error &&
    !status.isError &&
    !expires &&
    !commandError &&
    read.view?.brain_id === brain.id
      ? read.view
      : undefined;
  const receipt = view ? `${requestKey}:${view.epoch}` : null;
  const sourceStream = useOrganizationSources({
    brain: brain.id,
    center,
    view,
    qualificationKey: receipt,
    onClear: () => {
      setChoice(null);
      setOpenedSource(null);
      setEditing(null);
    },
    onFailure: failQualification,
  });
  const sourcePage = view ? sourceStream.page : undefined;

  useEffect(() => {
    const controller = new AbortController();
    setRead({ key: requestKey, pending: true });
    if (!status.data || status.isError) return () => controller.abort();
    void client
      .POST("/api/brains/{brain}/organization/view", {
        params: { path: { brain: brain.id } },
        body: { scope, center, cursor, limit: 25 },
        signal: controller.signal,
      })
      .then((response) => {
        const next = result(response);
        if (!controller.signal.aborted && activeRequest.current === requestKey)
          setRead({ key: requestKey, view: next, pending: false });
      })
      .catch((error) => {
        if (!controller.signal.aborted && activeRequest.current === requestKey)
          setRead({
            key: requestKey,
            error:
              error instanceof Error
                ? error
                : new Error("Organization could not be read."),
            pending: false,
          });
      });
    return () => controller.abort();
  }, [requestKey, status.isError, !!status.data]);
  useEffect(() => {
    setCenter(null);
    setCursor(null);
    setHistory([]);
    setChoice(null);
    setOpenedSource(null);
    setEditing(null);
    setAliases(false);
    setCommandError(null);
    setSaved(false);
    setDraft(scope);
  }, [brain.id, scopeKey]);
  useEffect(
    () =>
      cache.getQueryCache().subscribe((event) => {
        if (
          event.query.queryKey[1] === brain.id &&
          event.type === "updated" &&
          event.action.type === "invalidate"
        ) {
          setNonce((value) => value + 1);
          setChoice(null);
          setOpenedSource(null);
          setEditing(null);
        }
      }),
    [brain.id, cache],
  );
  useEffect(() => {
    setSourceLimit(20);
    setConnectionLimit(20);
  }, [requestKey]);
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id, "organization"],
    enabled: filters,
    retry: false,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
  });
  const manual = useQuery({
    queryKey: ["evidence", brain.id, "organization"],
    enabled: filters,
    retry: false,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
  });
  const manualGroups = !manual.isError ? (manual.data?.groups ?? []) : [];
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
  const selected = receipt && choice?.key === receipt ? choice.value : null;
  const selectedSource =
    selected?.kind === "node" ? sourceMap.get(selected.id) : undefined;
  const editTopic =
    receipt && editing?.key === receipt ? topicMap.get(editing.id) : undefined;
  const writable = ["admin", "writer"].includes(brain.role);
  function refresh() {
    setCommandError(null);
    setSaved(false);
    setChoice(null);
    setOpenedSource(null);
    setEditing(null);
    setNonce((value) => value + 1);
    void status.refetch();
  }
  function failQualification(error: Error) {
    setChoice(null);
    setOpenedSource(null);
    setEditing(null);
    setAliases(false);
    setCommandError(error);
  }
  function expand(next: Center | null) {
    setCenter(next);
    setCursor(null);
    setHistory([]);
    setChoice(null);
    setOpenedSource(null);
    setEditing(null);
    setSaved(false);
  }
  function inspect(node: EvidenceNode) {
    const evidence = sourceChoice(node);
    if (evidence && receipt) setOpenedSource({ key: receipt, evidence });
  }
  async function changeTopic(topic: SemanticCard, change: TopicChange) {
    if (!topic.change_id || !receipt) return;
    const key = receipt;
    const body = { base_change_id: topic.change_id, change };
    setCommandBusy(true);
    setCommandError(null);
    setSaved(false);
    try {
      const response = result(
        await client.PUT("/api/brains/{brain}/organization/topics/{id}", {
          params: { path: { brain: brain.id, id: topic.id } },
          body,
          headers: {
            "Idempotency-Key": idem.forInput({
              brain: brain.id,
              id: topic.id,
              body,
            }),
          },
        }),
      );
      if (activeRequest.current !== requestKey) return;
      idem.reset();
      setEditing(null);
      setChoice(null);
      setOpenedSource(null);
      setSaved(true);
      if (response.split_topic_id)
        setCenter({ kind: "topic", id: response.split_topic_id });
      setCursor(null);
      setHistory([]);
      setNonce((value) => value + 1);
      void cache.invalidateQueries({ queryKey: ["graph", brain.id] });
    } catch (error) {
      if (activeRequest.current === requestKey)
        setCommandError(
          error instanceof Error ? error : new Error("Topic change failed."),
        );
    } finally {
      if (activeRequest.current === requestKey || key === receipt)
        setCommandBusy(false);
    }
  }
  const inspector = (
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
              onClick={() => inspect(selectedSource)}
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
  const failure =
    commandError ??
    (read.key === requestKey ? read.error : undefined) ??
    status.error;
  const lower = view?.coverage.counts_are_lower_bounds ?? false;
  return (
    <section
      className="organization-panel"
      aria-label="Automatic knowledge organization"
    >
      <Group justify="space-between" className="organization-toolbar">
        <Group>
          <Badge variant="light">Automatic organization</Badge>
          {center && (
            <Button
              size="xs"
              variant="subtle"
              leftSection={<ArrowLeft size={iconSize.small} />}
              onClick={() => expand(null)}
            >
              All topics
            </Button>
          )}
        </Group>
        <Group>
          <Button
            variant="subtle"
            disabled={!view}
            onClick={() => setAliases(true)}
          >
            Review aliases
          </Button>
          <Button
            variant="default"
            leftSection={<SlidersHorizontal size={iconSize.small} />}
            onClick={() => {
              setDraft(scope);
              setFilters(true);
            }}
          >
            Scope
          </Button>
          <Button
            variant="subtle"
            leftSection={<RefreshCw size={iconSize.small} />}
            onClick={refresh}
          >
            Refresh organization
          </Button>
        </Group>
      </Group>
      <Group gap="xs" aria-label="Active organization scope">
        <Badge variant="outline">
          {(scope.selection.repository_ids?.length ?? 0)
            ? `${scope.selection.repository_ids?.length ?? 0} repositories`
            : "All repositories"}
        </Badge>
        {(scope.selection.area_ids?.length ?? 0) > 0 && (
          <Badge variant="outline">
            {scope.selection.area_ids?.length ?? 0} areas
          </Badge>
        )}
        {scope.selection.environment_id && (
          <Badge variant="outline">Selected environment</Badge>
        )}
        {scope.collection_id && (
          <Badge variant="outline">Selected collection</Badge>
        )}
        {scope.manifest_revision_id && (
          <Badge variant="outline">Exact manifest</Badge>
        )}
        {scope.fact_at && (
          <Badge variant="outline">
            Fact time {new Date(scope.fact_at).toLocaleString()}
          </Badge>
        )}
      </Group>
      <Text size="sm" c="dimmed">
        Topics organize eligible sources and source-backed entities across
        sessions. A source can belong to several topics; mentions and
        source-described links keep their evidence.
      </Text>
      {failure && (
        <Alert color="red" title="Organization unavailable">
          {failure.message}
          <Button size="xs" variant="subtle" onClick={refresh}>
            Read current organization
          </Button>
        </Alert>
      )}
      {expires && (
        <Alert color="yellow">
          This organization reached its retention deadline. Its names, counts
          and graph have been cleared.
          <Button size="xs" variant="subtle" onClick={refresh}>
            Read current organization
          </Button>
        </Alert>
      )}
      {saved && !failure && (
        <Alert variant="light" color="teal" role="status">
          Topic change saved.
        </Alert>
      )}
      {!view && !failure && !expires && (
        <Group role="status" py="xl">
          <Loader size="sm" />
          <Text>Reading current topics and source evidence…</Text>
        </Group>
      )}
      {view && (
        <>
          <Card withBorder p="sm" className="organization-coverage">
            <Group justify="space-between">
              <Text size="sm">
                {count(view.coverage.qualified_sources, lower)} eligible sources
                · {count(view.coverage.mapped_sources, lower)} mapped ·{" "}
                {count(view.coverage.unassigned_sources, lower)} unassigned in
                this read
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
                Counts are lower bounds for this bounded read. Narrow the scope
                or read the next page for more eligible sources.
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
                <Badge variant="light">
                  {label(view.community_status.state)}
                </Badge>
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
          {view.topics.length === 0 &&
            view.entities.length === 0 &&
            receipt && (
              <FieldOrganizationPanel
                key={receipt}
                brain={brain}
                scope={scope}
                epoch={view.epoch}
                qualificationKey={receipt}
                onFailure={failQualification}
              />
            )}
          {view.topics.length === 0 &&
            view.entities.length === 0 &&
            contextTopics.length === 0 &&
            contextEntities.length === 0 &&
            mentions.length === 0 &&
            view.memberships.length === 0 &&
            sourceMemberships.length === 0 &&
            representativeSources.length === 0 &&
            view.connections.length === 0 && (
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
                    <Button variant="light" onClick={() => expand(null)}>
                      All topics
                    </Button>
                  ) : undefined
                }
              />
            )}
          {view.topics.length > 0 && (
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
                      {topic.source_count === 1 && !lower
                        ? "source"
                        : "sources"}{" "}
                      · {count(topic.entity_count, lower)}{" "}
                      {topic.entity_count === 1 && !lower
                        ? "entity"
                        : "entities"}
                    </Text>
                    <Group mt="md">
                      <Button
                        size="xs"
                        variant="light"
                        onClick={() => expand({ kind: "topic", id: topic.id })}
                        aria-label={`Expand topic ${topic.label}`}
                      >
                        Explore topic
                      </Button>
                      {writable && topic.change_id && (
                        <Button
                          size="xs"
                          variant="subtle"
                          onClick={() =>
                            receipt &&
                            setEditing({ key: receipt, id: topic.id })
                          }
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
          )}
          {view.entities.length > 0 && (
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
                      {count(entity.source_count, lower)} supporting sources in
                      this read
                    </Text>
                    <Button
                      mt="sm"
                      size="compact-xs"
                      variant="subtle"
                      onClick={() => expand({ kind: "entity", id: entity.id })}
                      aria-label={`Expand entity ${entity.label}`}
                    >
                      Explore entity
                    </Button>
                  </Card>
                ))}
              </SimpleGrid>
            </section>
          )}
          {(view.topics.length > 0 || view.entities.length > 0) && receipt && (
            <FieldOrganizationPanel
              key={receipt}
              brain={brain}
              scope={scope}
              epoch={view.epoch}
              qualificationKey={receipt}
              onFailure={failQualification}
            />
          )}
          {(contextTopics.length > 0 || contextEntities.length > 0) && (
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
                      {count(card.source_count, lower)} supporting sources in
                      this read
                    </Text>
                    <Group mt="sm">
                      <Button
                        size="xs"
                        variant="subtle"
                        onClick={() =>
                          expand({
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
                          onClick={() =>
                            receipt && setEditing({ key: receipt, id: card.id })
                          }
                        >
                          Organize context topic
                        </Button>
                      )}
                    </Group>
                  </Card>
                ))}
              </SimpleGrid>
            </details>
          )}
          <Group justify="space-between">
            <Button
              variant="subtle"
              disabled={!history.length}
              onClick={() => {
                setCursor(history[history.length - 1]);
                setHistory((old) => old.slice(0, -1));
              }}
            >
              Previous organization page
            </Button>
            <Text size="xs" c="dimmed">
              Page {history.length + 1} · bounded cards and source details
            </Text>
            <Button
              variant="subtle"
              disabled={!view.next_cursor}
              onClick={() => {
                setHistory((old) => [...old, cursor]);
                setCursor(view.next_cursor ?? null);
              }}
            >
              Next organization page
            </Button>
          </Group>
          {center?.kind !== "entity" &&
            (sourceStream.hasNext ||
              sourceStream.hasPrevious ||
              sourceStream.pending) && (
              <Group
                justify="space-between"
                aria-label="Source membership pages"
              >
                <Button
                  variant="subtle"
                  disabled={!sourceStream.hasPrevious || sourceStream.pending}
                  onClick={sourceStream.previous}
                >
                  Previous source membership page
                </Button>
                <Text size="xs" c="dimmed" role="status">
                  {sourceStream.pending
                    ? "Reading current source memberships…"
                    : `Source membership page ${sourceStream.pageNumber} · bounded source contributors`}
                </Text>
                <Button
                  variant="subtle"
                  disabled={!sourceStream.hasNext || sourceStream.pending}
                  onClick={sourceStream.next}
                >
                  Next source membership page
                </Button>
              </Group>
            )}
          {sourcePage?.coverage.counts_are_lower_bounds && (
            <Text size="xs" c="dimmed">
              Source membership coverage is partial. Narrow the scope or
              continue through its independent pages.
            </Text>
          )}
          {center && evidenceNodes.length > 0 && (
            <section aria-label="Expanded topic sources">
              <Title order={3} mb="xs">
                Source neighborhood
              </Title>
              <Text size="sm" c="dimmed" mb="sm">
                One identity per eligible source, including overlapping topics.
                Positions express organization; source-described entity links
                are listed below. Evidence view provides canonical knowledge
                paths.
              </Text>
              <div className="organization-graph">
                <GraphRenderer
                  key={receipt}
                  nodes={evidenceNodes}
                  edges={emptyEdges}
                  displayGroups={displayGroups}
                  choice={selected}
                  onChoose={(value) =>
                    receipt && setChoice({ key: receipt, value })
                  }
                  pathNodes={emptyKeys}
                  pathEdges={emptyKeys}
                  inspector={inspector}
                />
              </div>
            </section>
          )}
          {evidenceNodes.length > 0 && (
            <section aria-label="Organization source evidence">
              <Title order={3} mb="sm">
                Sources in this read
              </Title>
              <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="sm">
                {evidenceNodes.slice(0, sourceLimit).map((node) => (
                  <Card
                    key={node.key}
                    withBorder
                    p="sm"
                    className="organization-card"
                  >
                    <Text fw={600}>{node.evidence.label}</Text>
                    <Text size="xs" c="dimmed" mt="xs">
                      {
                        mentions.filter(
                          (mention) => mention.source_key === node.key,
                        ).length
                      }{" "}
                      exact mentions in this read
                    </Text>
                    <Group mt="sm">
                      <Button
                        size="xs"
                        variant="light"
                        onClick={() => {
                          if (receipt)
                            setChoice({
                              key: receipt,
                              value: { kind: "node", id: node.key },
                            });
                          if (!center) inspect(node);
                        }}
                      >
                        Inspect source
                      </Button>
                      <Button
                        size="xs"
                        variant="subtle"
                        disabled={!sourceChoice(node)}
                        onClick={() => inspect(node)}
                      >
                        Open exact source
                      </Button>
                    </Group>
                  </Card>
                ))}
              </SimpleGrid>
              {sourceLimit < evidenceNodes.length && (
                <Button
                  mt="sm"
                  variant="subtle"
                  onClick={() => setSourceLimit((value) => value + 20)}
                >
                  More loaded sources
                </Button>
              )}
            </section>
          )}
          {view.connections.length > 0 && (
            <section aria-label="Source-described entity relationships">
              <Title order={3} mb="xs">
                Source-described connections
              </Title>
              <Text size="sm" c="dimmed" mb="sm">
                A source describes these connections. They do not establish
                accepted operational truth or a canonical knowledge path.
              </Text>
              <Stack gap="sm">
                {view.connections
                  .slice(0, connectionLimit)
                  .map((connection) => (
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
                      <pre className="organization-quote">
                        {connection.quote}
                      </pre>
                      <Text size="xs" c="dimmed">
                        Exact UTF-8 bytes {connection.byte_start}–
                        {connection.byte_end}
                      </Text>
                      <Button
                        mt="xs"
                        size="xs"
                        variant="subtle"
                        disabled={!sourceMap.has(connection.source_key)}
                        onClick={() => {
                          const node = sourceMap.get(connection.source_key);
                          if (node) inspect(node);
                        }}
                      >
                        Inspect connection source
                      </Button>
                    </Card>
                  ))}
              </Stack>
              {connectionLimit < view.connections.length && (
                <Button
                  mt="sm"
                  variant="subtle"
                  onClick={() => setConnectionLimit((value) => value + 20)}
                >
                  More loaded connections
                </Button>
              )}
            </section>
          )}
        </>
      )}
      <Drawer
        opened={aliases && !!view}
        onClose={() => setAliases(false)}
        title="Review alias evidence"
        size="xl"
        position="right"
      >
        {aliases && view && receipt && (
          <AliasReviewPanel
            key={receipt}
            brain={brain}
            scope={scope}
            epoch={view.epoch}
            qualification={receipt}
            onInspect={inspect}
            onChanged={() =>
              void cache.invalidateQueries({ queryKey: ["graph", brain.id] })
            }
            onFailure={failQualification}
          />
        )}
      </Drawer>
      <Drawer
        opened={filters}
        onClose={() => setFilters(false)}
        title="Organization scope"
        position="right"
        size="md"
      >
        <Stack>
          <Text size="sm" c="dimmed">
            Choose which source evidence may contribute names, memberships and
            counts.
          </Text>
          {(catalogue.error || manual.error) && (
            <Alert color="red">
              Scope choices could not be refreshed. Existing semantic results do
              not supply scope labels.
            </Alert>
          )}
          <MultiSelect
            label="Organization repositories"
            searchable
            value={draft.selection.repository_ids}
            data={(!catalogue.isError
              ? (catalogue.data?.repositories ?? [])
              : []
            ).map((repository) => ({
              value: repository.id,
              label: repository.canonical_origin,
            }))}
            onChange={(repository_ids) =>
              setDraft((old) => ({
                ...old,
                selection: { ...old.selection, repository_ids },
              }))
            }
          />
          <MultiSelect
            label="Organization areas"
            searchable
            value={draft.selection.area_ids}
            data={manualGroups
              .filter((group) => group.kind === "area")
              .map((group) => ({ value: group.id, label: group.name }))}
            onChange={(area_ids) =>
              setDraft((old) => ({
                ...old,
                selection: { ...old.selection, area_ids },
              }))
            }
          />
          <Select
            label="Organization environment"
            clearable
            value={draft.selection.environment_id}
            data={manualGroups
              .filter((group) => group.kind === "environment")
              .map((group) => ({ value: group.id, label: group.name }))}
            onChange={(environment_id) =>
              setDraft((old) => ({
                ...old,
                selection: { ...old.selection, environment_id },
              }))
            }
          />
          <Select
            label="Organization collection"
            clearable
            value={draft.collection_id}
            data={manualGroups
              .filter((group) => group.kind === "collection")
              .map((group) => ({ value: group.id, label: group.name }))}
            onChange={(collection_id) =>
              setDraft((old) => ({ ...old, collection_id }))
            }
          />
          <Group>
            <Button
              onClick={() => {
                setChoice(null);
                setOpenedSource(null);
                setEditing(null);
                setCenter(null);
                setCursor(null);
                setHistory([]);
                setNonce((value) => value + 1);
                setFilters(false);
                onScopeChange(draft);
              }}
            >
              Apply organization scope
            </Button>
            <Button
              variant="subtle"
              onClick={() =>
                setDraft({
                  ...scope,
                  collection_id: null,
                  manifest_revision_id: null,
                  fact_at: null,
                  selection: {
                    repository_ids: [],
                    area_ids: [],
                    environment_id: null,
                  },
                })
              }
            >
              Clear scope
            </Button>
          </Group>
        </Stack>
      </Drawer>
      {view && editTopic && (
        <Modal
          opened
          onClose={() => {
            if (!commandBusy) setEditing(null);
          }}
          title={
            editTopic.topic_kind === "manual_overlay"
              ? "Organize manual group"
              : "Organize automatic topic"
          }
          size="lg"
        >
          <TopicEditor
            key={editTopic.change_id}
            topic={editTopic}
            view={{
              ...view,
              evidence: evidenceNodes,
              source_memberships: sourceMemberships,
              representative_sources: representativeSources,
            }}
            busy={commandBusy}
            onChange={(change) => void changeTopic(editTopic, change)}
          />
        </Modal>
      )}
      {view && receipt && openedSource?.key === receipt && (
        <EvidenceDialog
          brain={brain}
          evidence={openedSource.evidence}
          onClose={() => setOpenedSource(null)}
          returnLabel="Back to organization"
        />
      )}
    </section>
  );
}
