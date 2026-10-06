import { useEffect, useState } from "react";
import { useDebouncedValue } from "@mantine/hooks";
import {
  Badge,
  Button,
  Code,
  Group,
  Modal,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { FolderGit2, Search } from "lucide-react";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { useBrainSearch } from "../../app/useBrainSearch";
import { useKnowledgeSelection } from "../knowledge/selection";
import { useStableRows } from "../../components/useStableRows";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
import { RepositoryDialog } from "../../PublicationPanel";

export function RepositoryBrowser({ brain }: { brain: Brain }) {
  const [route, patch] = useBrainSearch();
  const { select } = useKnowledgeSelection();
  const [text, setText] = useState("");
  const [query] = useDebouncedValue(text.trim(), 300);
  const [offset, setOffset] = useState(0);
  const [publishing, setPublishing] = useState(false);
  const [snapshotRepo, setSnapshotRepo] = useState<
    components["schemas"]["Repository"] | null
  >(null);
  useEffect(() => {
    select(
      route.repository ? { kind: "repository", id: route.repository } : null,
    );
  }, [route.repository, select]);
  const catalogue = useQuery({
    queryKey: ["repository-catalogue", brain.id, query, offset],
    gcTime: 0,
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace/repositories", {
          params: {
            path: { brain: brain.id },
            query: { q: query || undefined, offset },
          },
          signal,
        }),
      ),
  });
  const data = catalogue.isError ? undefined : catalogue.data;
  const stable = useStableRows(
    data?.items,
    JSON.stringify([brain.id, query, offset]),
    !!route.repository,
    (row) => row.id,
  );
  return (
    <section className="feature-view repository-browser">
      <div className="feature-toolbar">
        <TextInput
          className="feature-search"
          aria-label="Search repository origins"
          placeholder="Find a repository…"
          leftSection={<Search size={18} />}
          value={text}
          maxLength={200}
          onChange={(event) => {
            setText(event.currentTarget.value);
            setOffset(0);
          }}
        />
        <Button variant="default" onClick={() => setPublishing(true)}>
          Publish from host
        </Button>
      </div>
      <ErrorState
        error={catalogue.error}
        retry={() => void catalogue.refetch()}
      />
      {catalogue.isPending && <LoadingState label="Loading repositories…" />}
      {data && (
        <>
          <Group justify="space-between" mb="sm">
            <Text size="xs" c="dimmed">
              {data.total} {query ? "matching " : ""}repositories
            </Text>
            {stable.pending > 0 && (
              <Button variant="light" size="xs" onClick={stable.reveal}>
                {stable.pending} new repositories · Show updates
              </Button>
            )}
          </Group>
          {!data.items.length && (
            <EmptyState
              icon={FolderGit2}
              title={query ? "No matching repositories" : "No repositories yet"}
              description={
                query
                  ? "Try another origin."
                  : "Your connected host discovers repositories from your workspace."
              }
            />
          )}
          <div className="feature-list">
            {stable.rows.map((repo) => (
              <article
                className={`repository-library-row ${route.repository === repo.id ? "is-selected" : ""}`}
                key={repo.id}
                data-testid="repository-card"
              >
                <button
                  type="button"
                  className="repository-library-name"
                  onClick={() => patch({ repository: repo.id })}
                >
                  <span className="library-icon">
                    <FolderGit2 size={18} />
                  </span>
                  <span>
                    <strong>{repo.canonical_origin.split("/").at(-1)}</strong>
                    <span className="feature-meta">
                      {repo.canonical_origin}
                    </span>
                  </span>
                </button>
                <Badge variant="light" color="gray">
                  {repo.origins.length} origin
                  {repo.origins.length === 1 ? "" : "s"}
                </Badge>
                <Button
                  variant="subtle"
                  size="xs"
                  onClick={() => setSnapshotRepo(repo)}
                >
                  Snapshots
                </Button>
              </article>
            ))}
          </div>
          {(offset > 0 || data.next_offset != null) && (
            <Group justify="space-between" mt="md">
              <Button
                variant="default"
                size="xs"
                disabled={!offset}
                onClick={() => setOffset(Math.max(0, offset - 50))}
              >
                Previous repositories
              </Button>
              <Text size="xs">
                {data.items.length ? offset + 1 : 0}–
                {Math.min(offset + 50, data.total)} of {data.total}
              </Text>
              <Button
                variant="default"
                size="xs"
                disabled={data.next_offset == null}
                onClick={() => setOffset(data.next_offset!)}
              >
                Next repositories
              </Button>
            </Group>
          )}
        </>
      )}
      <Modal
        opened={publishing}
        onClose={() => setPublishing(false)}
        title="Publish from your host"
      >
        <Stack>
          <Text size="sm">
            Your connected coding host publishes an exact committed tree from
            its local checkout.
          </Text>
          <Text size="sm">
            Ask your agent, from a task scoped to this repository, to publish
            the checkout through the Recollect plugin.
          </Text>
          <Text size="xs" c="dimmed">
            Working files stay local. Retaining selected file text requires the
            Brain policy and an explicit --retain-file selection.
          </Text>
        </Stack>
      </Modal>
      {snapshotRepo && (
        <RepositoryDialog
          brain={brain}
          repository={snapshotRepo}
          onClose={() => setSnapshotRepo(null)}
        />
      )}
    </section>
  );
}
