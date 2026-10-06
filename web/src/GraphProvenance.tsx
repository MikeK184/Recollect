import { useEffect, useMemo, useState } from "react";
import { useDebouncedValue } from "@mantine/hooks";
import {
  Alert,
  Badge,
  Button,
  Group,
  Select,
  Text,
  TextInput,
} from "@mantine/core";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { useBrainSearch } from "./app/useBrainSearch";
import { useContentDeadline } from "./useContentDeadline";
import { useKnowledgeSelection } from "./features/knowledge/selection";
import { EmptyState, LoadingState } from "./components/AsyncState";
import { Network, Search } from "lucide-react";
import { GraphRenderer } from "./GraphRenderer";
import { GraphSelectionPanel } from "./GraphSelectionPanel";
import type {
  GraphChoice,
  GraphRenderNode,
  GraphRenderEdge,
} from "./GraphCanvas";
import { SnapshotDialog } from "./PublicationPanel";
import { origin } from "./features/activity/pipelineModel";

type Observation = {
  title: string;
  description: string;
  source?: { id: string; version?: string };
  snapshot?: string;
  repository?: string;
};

function localDeadline(observed: string, until: string, started: number) {
  const remaining =
    Date.parse(until) - Date.parse(observed) - (performance.now() - started);
  return new Date(Date.now() + Math.max(0, remaining)).toISOString();
}

export function GraphProvenance({ brain }: { brain: Brain }) {
  const [route, patch] = useBrainSearch();
  const { select } = useKnowledgeSelection();
  const [choice, setGraphChoice] = useState<GraphChoice>(null);
  const [focusRequest, setFocusRequest] = useState(0);
  const setChoice = (next: GraphChoice) => {
    setGraphChoice(next);
  };
  const [text, setText] = useState("");
  const [query] = useDebouncedValue(text.trim(), 300);
  const [repositoryOffset, setRepositoryOffset] = useState(0);
  const [snapshotOffset, setSnapshotOffset] = useState(0);
  const [contributorOffset, setContributorOffset] = useState(0);
  const [details, setDetails] = useState<string | null>(null);
  const feedQuery = useQuery({
    queryKey: ["graph-provenance", brain.id],
    retry: false,
    gcTime: 0,
    refetchInterval: 2000,
    queryFn: async ({ signal }) => {
      const started = performance.now();
      const feed = result(
        await client.GET("/api/brains/{brain}/pipeline", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      );
      return {
        feed,
        deadline: localDeadline(feed.observed_at, feed.valid_until, started),
      };
    },
  });
  const expired = useContentDeadline(feedQuery.data?.deadline);
  const feed =
    !feedQuery.isError && !expired ? feedQuery.data?.feed : undefined;
  const repositories = useQuery({
    queryKey: ["provenance-repositories", brain.id, query, repositoryOffset],
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace/repositories", {
          params: {
            path: { brain: brain.id },
            query: { q: query || undefined, offset: repositoryOffset },
          },
          signal,
        }),
      ),
  });
  const selectedRepository = useQuery({
    queryKey: ["provenance-repository-identity", brain.id, route.repository],
    enabled: !!route.repository,
    retry: false,
    gcTime: 0,
    refetchInterval: 2000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/workspace/repositories/{repository}",
          {
            params: {
              path: { brain: brain.id, repository: route.repository! },
            },
            signal,
          },
        ),
      ),
  });
  const repositoryOptions = (
    repositories.isError ? [] : (repositories.data?.items ?? [])
  ).filter(
    (repository) =>
      !(selectedRepository.isError && repository.id === route.repository),
  );
  if (
    !selectedRepository.isError &&
    selectedRepository.data &&
    selectedRepository.data.id === route.repository &&
    !repositoryOptions.some((repository) => repository.id === route.repository)
  )
    repositoryOptions.push(selectedRepository.data);
  const snapshots = useQuery({
    queryKey: [
      "provenance-snapshots",
      brain.id,
      route.repository,
      snapshotOffset,
    ],
    enabled: !!route.repository,
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/repositories/{repository}/snapshots",
          {
            params: {
              path: { brain: brain.id, repository: route.repository! },
              query: { offset: snapshotOffset },
            },
            signal,
          },
        ),
      ),
  });
  const publication = useQuery({
    queryKey: [
      "provenance-publication",
      brain.id,
      route.snapshot,
      contributorOffset,
    ],
    enabled: !!feed && !!route.snapshot,
    retry: false,
    gcTime: 0,
    refetchInterval: 2000,
    placeholderData: keepPreviousData,
    queryFn: async ({ signal }) => {
      const started = performance.now();
      const detail = result(
        await client.GET(
          "/api/brains/{brain}/repository-snapshots/{snapshot}",
          {
            params: {
              path: { brain: brain.id, snapshot: route.snapshot! },
              query: { offset: contributorOffset },
            },
            signal,
          },
        ),
      );
      return {
        detail,
        deadline: localDeadline(
          detail.observed_at,
          detail.valid_until,
          started,
        ),
      };
    },
  });
  const publicationExpired = useContentDeadline(publication.data?.deadline);
  const published =
    feed &&
    !!route.snapshot &&
    publication.data &&
    !publication.isError &&
    !publicationExpired &&
    publication.data?.detail.snapshot.id === route.snapshot &&
    publication.data.detail.snapshot.repository_id === route.repository &&
    publication.data.detail.contributor_offset === contributorOffset
      ? publication.data?.detail
      : undefined;
  const publicationMismatch =
    feed &&
    !!route.snapshot &&
    publication.data &&
    !publication.isError &&
    !publicationExpired &&
    publication.data.detail.snapshot.id === route.snapshot &&
    publication.data.detail.snapshot.repository_id !== route.repository;
  const graph = useMemo(() => {
    const nodes = new Map<string, GraphRenderNode>();
    const edges: GraphRenderEdge[] = [];
    const observations = new Map<string, Observation>();
    const node = (
      key: string,
      title: string,
      kind: string,
      description: string,
      extra?: Partial<Observation>,
    ) => {
      nodes.set(key, { key, evidence: { label: title, kind } });
      observations.set(key, { title, description, ...extra });
    };
    const edge = (
      id: string,
      from: string,
      to: string,
      relation: string,
      description: string,
    ) => {
      edges.push({ id, from, to, relation, family: "provenance" });
      observations.set(id, { title: relation, description });
    };
    if (feed)
      for (const item of feed.items) {
        if (item.expires_at && Date.parse(item.expires_at) <= Date.now())
          continue;
        const actor = `contributor:${item.actor_id}`;
        node(
          actor,
          item.contributor,
          "contributor",
          "Recorded contributor; this does not indicate current presence.",
        );
        const input = item.source_version_id
          ? `source_version:${item.source_version_id}`
          : `capture:${item.id}`;
        node(
          input,
          item.title,
          item.source_version_id ? "source_version" : "capture",
          `Received ${new Date(item.received_at).toLocaleString()} · ${item.processing.replaceAll("_", " ")}`,
          item.source_id
            ? {
                source: {
                  id: item.source_id,
                  version: item.source_version_id ?? undefined,
                },
              }
            : undefined,
        );
        edge(
          `contribution:${item.id}`,
          actor,
          input,
          "Contributed",
          "This permitted input records this contributor. No authorship of later memory is inferred.",
        );
        if (item.device_id) {
          const device = `device:${item.device_id}`;
          node(
            device,
            origin(item),
            "device",
            "Observed agent credential on these inputs; credential identity does not indicate that an agent is online.",
          );
          edge(
            `submitted:${item.id}`,
            device,
            input,
            "Submitted via",
            "The input records this exact device identity. No relationship was inferred from names or timestamps.",
          );
        }
      }
    if (published) {
      const snapshot = published.snapshot;
      const repository = `repository:${snapshot.repository_id}`;
      const snapshotKey = `snapshot:${snapshot.id}`;
      const known = !repositories.isError
        ? repositories.data?.items.find(
            (repo) => repo.id === snapshot.repository_id,
          )
        : undefined;
      const originName =
        known?.canonical_origin ??
        published.contributors[0]?.origin ??
        "Published repository";
      node(
        repository,
        originName,
        "repository",
        "Canonical repository identity recorded by the exact snapshot.",
        { repository: snapshot.repository_id },
      );
      node(
        snapshotKey,
        `Commit ${snapshot.revision.slice(0, 12)}`,
        "snapshot",
        `Exact commit ${snapshot.revision} · ${snapshot.processing}`,
        { snapshot: snapshot.id },
      );
      edge(
        `snapshot-of:${snapshot.id}`,
        snapshotKey,
        repository,
        "Snapshot of",
        "This exact snapshot belongs to the recorded repository, not its current working tree.",
      );
      for (const contribution of published.contributors) {
        const device = `device:${contribution.device_id}`;
        if (!nodes.has(device))
          node(
            device,
            "Publishing agent",
            "device",
            "Exact device identity recorded on this publication. Host name is not reported here.",
          );
        const actor = `contributor:${contribution.actor_id}`;
        node(
          actor,
          contribution.actor_name ?? "Recorded contributor",
          "contributor",
          "Recorded publication contributor; no private task scope or local path is displayed.",
        );
        edge(
          `published:${contribution.id}`,
          device,
          snapshotKey,
          "Published via",
          `Accepted ${new Date(contribution.accepted_at).toLocaleString()} from this exact device.`,
        );
        edge(
          `publication-contributor:${contribution.id}`,
          actor,
          snapshotKey,
          "Contributed publication",
          "This actor contributed this snapshot publication; this does not assert authorship of repository files.",
        );
      }
    }
    return { nodes: [...nodes.values()], edges, observations };
  }, [feed, published, repositories.data, repositories.isError]);
  const graphKey = useMemo(
    () =>
      new Set([
        ...graph.nodes.map((node) => node.key),
        ...graph.edges.map((edge) => edge.id),
      ]),
    [graph],
  );
  useEffect(() => {
    if (choice && !graphKey.has(choice.id)) setChoice(null);
    if (!feed) {
      setChoice(null);
      select(null);
      setDetails(null);
    }
  }, [graphKey, feed, choice, select]);
  useEffect(() => {
    if (details && !published) {
      setDetails(null);
      select(null);
    }
  }, [details, published, select]);
  useEffect(() => () => select(null), [select]);
  const selected = choice ? graph.observations.get(choice.id) : undefined;
  return (
    <section
      className="graph-provenance"
      aria-label="Recorded contribution graph"
    >
      <div className="feature-toolbar graph-chrome">
        <TextInput
          aria-label="Find repository for publication graph"
          placeholder="Find repository…"
          value={text}
          maxLength={200}
          leftSection={<Search size={16} />}
          onChange={(event) => {
            setText(event.currentTarget.value);
            setRepositoryOffset(0);
          }}
        />
        <Select
          aria-label="Published repository"
          placeholder={
            selectedRepository.isError
              ? "Selected repository unavailable"
              : "Add exact repository publication…"
          }
          clearable
          searchable
          value={route.repository ?? null}
          data={repositoryOptions.map((repo) => ({
            value: repo.id,
            label: repo.canonical_origin,
          }))}
          onChange={(repository) => {
            patch({ repository, snapshot: null });
            setSnapshotOffset(0);
            setContributorOffset(0);
            setChoice(null);
          }}
        />
        {route.repository && (
          <Select
            aria-label="Exact publication snapshot"
            placeholder={
              snapshots.isPending
                ? "Loading snapshots…"
                : snapshots.isError
                  ? "Snapshots unavailable"
                  : snapshots.data.total === 0
                    ? "No published snapshots"
                    : snapshots.data.items.length === 0
                      ? "No snapshots on this page"
                      : "Choose an exact snapshot"
            }
            disabled={
              snapshots.isPending ||
              snapshots.isError ||
              !snapshots.data?.items.length
            }
            clearable
            value={route.snapshot ?? null}
            data={
              !snapshots.isError
                ? (snapshots.data?.items ?? []).map((snapshot) => ({
                    value: snapshot.id,
                    label: `${snapshot.revision.slice(0, 12)} · ${new Date(snapshot.created_at).toLocaleDateString()}`,
                  }))
                : []
            }
            onChange={(snapshot) => {
              patch({ snapshot });
              setContributorOffset(0);
              setChoice(null);
            }}
          />
        )}
        {repositoryOffset > 0 && (
          <Button
            variant="subtle"
            size="xs"
            onClick={() =>
              setRepositoryOffset(Math.max(0, repositoryOffset - 50))
            }
          >
            Previous repositories
          </Button>
        )}
        {repositories.data?.next_offset != null && !repositories.isError && (
          <Button
            variant="subtle"
            size="xs"
            onClick={() => setRepositoryOffset(repositories.data!.next_offset!)}
          >
            More repositories
          </Button>
        )}
        {snapshotOffset > 0 && (
          <Button
            variant="subtle"
            size="xs"
            onClick={() => setSnapshotOffset(Math.max(0, snapshotOffset - 20))}
          >
            Previous snapshots
          </Button>
        )}
        {snapshots.data &&
          !snapshots.isError &&
          snapshotOffset + 20 < snapshots.data.total && (
            <Button
              variant="subtle"
              size="xs"
              onClick={() => setSnapshotOffset(snapshotOffset + 20)}
            >
              More snapshots
            </Button>
          )}
      </div>
      {(feedQuery.isError || expired) && (
        <Alert color="gray" title="Recorded activity unavailable">
          {feedQuery.isError
            ? feedQuery.error.message
            : "The last read expired."}
          <Button
            variant="subtle"
            size="xs"
            onClick={() => void feedQuery.refetch()}
          >
            Refresh
          </Button>
        </Alert>
      )}
      {(repositories.isError ||
        selectedRepository.isError ||
        snapshots.isError ||
        publication.isError) && (
        <Alert color="gray">
          {repositories.error?.message ??
            selectedRepository.error?.message ??
            snapshots.error?.message ??
            publication.error?.message}
        </Alert>
      )}
      {publicationMismatch && (
        <Alert
          color="gray"
          title="Snapshot does not match the selected repository"
        >
          Choose the repository and an exact snapshot that belong together.
        </Alert>
      )}
      {feedQuery.isPending && (
        <LoadingState label="Reading recorded contributions…" />
      )}
      {feed && (
        <>
          <Group gap="xs">
            <Badge variant="light">Recent recorded activity</Badge>
            <Text size="xs" c="dimmed">
              {feed.items.length} inputs
              {feed.has_more ? " · More activity exists" : ""}
            </Text>
            {published && (
              <Text size="xs" c="dimmed">
                {published.contributors.length} of {published.contributor_total}{" "}
                publications for this snapshot
              </Text>
            )}
            {published && contributorOffset > 0 && (
              <Button
                variant="subtle"
                size="xs"
                onClick={() =>
                  setContributorOffset(Math.max(0, contributorOffset - 20))
                }
              >
                Previous publications
              </Button>
            )}
            {published &&
              contributorOffset + 20 < published.contributor_total && (
                <Button
                  variant="subtle"
                  size="xs"
                  onClick={() => setContributorOffset(contributorOffset + 20)}
                >
                  More publications
                </Button>
              )}
          </Group>
          <Text size="xs" c="dimmed">
            Recorded contributions only; agent presence and later memory
            authorship are not inferred.
          </Text>
          {graph.nodes.length ? (
            <div className="graph-stage-area">
              <GraphRenderer
                key={`${brain.id}:${route.repository ?? "none"}:${route.snapshot ?? "recent"}:${contributorOffset}`}
                inspector={
                  choice && selected ? (
                    <GraphSelectionPanel
                      nodes={graph.nodes}
                      edges={graph.edges}
                      choice={choice}
                      onChoose={setChoice}
                      onFocus={() => setFocusRequest((value) => value + 1)}
                      description={
                        <Text size="xs" c="dimmed">
                          {selected.description}
                        </Text>
                      }
                      actions={
                        <>
                          {selected.source && (
                            <Button
                              size="xs"
                              onClick={() => {
                                select({
                                  kind: "source",
                                  id: selected.source!.id,
                                  version: selected.source!.version,
                                });
                              }}
                            >
                              Open evidence
                            </Button>
                          )}
                          {selected.repository && (
                            <Button
                              size="xs"
                              onClick={() => {
                                select({
                                  kind: "repository",
                                  id: selected.repository!,
                                });
                              }}
                            >
                              Open repository
                            </Button>
                          )}
                          {selected.snapshot && (
                            <Button
                              size="xs"
                              onClick={() => setDetails(selected.snapshot!)}
                            >
                              Open snapshot
                            </Button>
                          )}
                        </>
                      }
                      details={
                        <Text size="xs" className="graph-exact-identity">
                          Recorded identity: {choice.id}
                        </Text>
                      }
                    />
                  ) : null
                }
                nodes={graph.nodes}
                edges={graph.edges}
                choice={choice}
                focusRequest={focusRequest}
                onChoose={setChoice}
                pathNodes={[]}
                pathEdges={[]}
              />
            </div>
          ) : (
            <EmptyState
              icon={Network}
              title="No recorded contributions"
              description="Permitted source activity and explicitly selected publications appear here."
            />
          )}
        </>
      )}
      {details && feed && (
        <SnapshotDialog
          brain={brain}
          id={details}
          onClose={() => setDetails(null)}
        />
      )}
    </section>
  );
}
