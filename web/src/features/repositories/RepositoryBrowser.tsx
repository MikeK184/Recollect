import { useEffect, useRef, useState } from "react";
import { motion } from "motion/react";
import { useQuery } from "@tanstack/react-query";
import { FolderGit2, Upload } from "lucide-react";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { useBrainSearch } from "../../app/useBrainSearch";
import { useKnowledgeSelection } from "../knowledge/selection";
import { useStableRows } from "../../components/useStableRows";
import { EmptyState, ErrorState } from "../../components/AsyncState";
import { SkeletonRows } from "../../components/Skeleton";
import { StatusDot } from "../../components/StatusDot";
import {
  Badge,
  Button,
  Code,
  Modal,
  SearchInput,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableWrap,
} from "../../components/ui";
import { useDebouncedValue } from "../../lib/hooks";
import { listItem } from "../../design/motion";
import { RepositoryDialog } from "../../PublicationPanel";
import { usePollInterval } from "../../usePollInterval";
import { Toolbar, useOpener } from "./Toolbar";
import { useExploreCount } from "../knowledge/frame";

const MotionRow = motion.create(TableRow);

/** Canonical origin with the identifying last segment always visible; only
 * the middle of a long path yields to the ellipsis. */
function Origin({ value }: { value: string }) {
  const cut = value.lastIndexOf("/") + 1;
  return (
    <span className="flex min-w-0" title={value}>
      <span className="truncate">{value.slice(0, cut)}</span>
      <span className="shrink-0">{value.slice(cut)}</span>
    </span>
  );
}

export function RepositoryBrowser({
  brain,
  toolbar,
}: {
  brain: Brain;
  /** The switch row to place search, count and actions in; inline if absent. */
  toolbar?: HTMLElement | null;
}) {
  const pollNormal = usePollInterval("normal");
  const [route, patch] = useBrainSearch();
  const { select } = useKnowledgeSelection();
  const [text, setText] = useState("");
  const [query] = useDebouncedValue(text.trim(), 300);
  const [offset, setOffset] = useState(0);
  const [publishing, setPublishing] = useState(false);
  const snapshotOpener = useOpener();
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
    refetchInterval: pollNormal,
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
  // The switcher tab shows the Brain's repository count, not a search count.
  useExploreCount("repositories", query ? null : data?.total);
  const stable = useStableRows(
    data?.items,
    JSON.stringify([brain.id, query, offset]),
    !!route.repository,
    (row) => row.id,
  );
  // Rows stagger in on the first load only, never on refetch or paging.
  const settled = useRef(false);
  const firstLoad = !settled.current;
  useEffect(() => {
    if (stable.rows.length) settled.current = true;
  });
  return (
    <section className="flex flex-col gap-4">
      <Toolbar target={toolbar}>
        <div className="min-w-0 flex-[0_1_260px]">
          <SearchInput
            aria-label="Search repository origins"
            placeholder="Find a repository…"
            value={text}
            maxLength={200}
            onChange={(event) => {
              setText(event.currentTarget.value);
              setOffset(0);
            }}
          />
        </div>
        <span
          className="mono ml-auto shrink-0 whitespace-nowrap text-caption text-muted-foreground"
          aria-live="polite"
        >
          {data
            ? `${data.total} ${query ? "matching " : ""}repositor${data.total === 1 ? "y" : "ies"}`
            : catalogue.isPending
              ? "Loading repositories"
              : ""}
        </span>
        <Button variant="secondary" onClick={() => setPublishing(true)}>
          <Upload aria-hidden="true" />
          Publish from host
        </Button>
      </Toolbar>
      <ErrorState
        error={catalogue.error}
        retry={() => void catalogue.refetch()}
      />
      {catalogue.isPending && (
        <TableWrap className="px-3">
          <SkeletonRows label="Loading repositories…" rows={6} />
        </TableWrap>
      )}
      {data && stable.pending > 0 && (
        <div>
          <Button variant="secondary" size="sm" onClick={stable.reveal}>
            <StatusDot tone="accent" size={6} />
            {stable.pending} new repositories · Show updates
          </Button>
        </div>
      )}
      {data && !data.items.length && (
        <div className="rounded-lg border border-border bg-card">
          <EmptyState
            icon={FolderGit2}
            title={query ? "No matching repositories" : "No repositories yet"}
            description={
              query
                ? "Try another origin."
                : "Your connected host discovers repositories from your workspace."
            }
          />
        </div>
      )}
      {data && stable.rows.length > 0 && (
        <TableWrap className="overflow-clip">
          <Table>
            <TableHead>
              <tr>
                <TableHeader scope="col">Repository</TableHeader>
                <TableHeader scope="col">Origin</TableHeader>
                <TableHeader scope="col">Origins</TableHeader>
                <TableHeader scope="col" aria-label="Actions" />
              </tr>
            </TableHead>
            <TableBody>
              {stable.rows.map((repo, index) => {
                const isSelected = route.repository === repo.id;
                return (
                  <MotionRow
                    key={repo.id}
                    data-testid="repository-card"
                    selected={isSelected}
                    variants={listItem}
                    custom={index}
                    initial={firstLoad ? "initial" : false}
                    animate="animate"
                    className="cursor-pointer"
                    onClick={(event) => {
                      // Focus stays on the row's name so closing the
                      // inspector returns here.
                      event.currentTarget
                        .querySelector<HTMLButtonElement>("button")
                        ?.focus({ preventScroll: true });
                      patch({ repository: repo.id });
                    }}
                  >
                    <TableCell primary className="whitespace-nowrap">
                      <span className="flex items-center gap-2.5">
                        <FolderGit2
                          aria-hidden="true"
                          className="size-4 shrink-0 text-muted-foreground"
                        />
                        <button
                          type="button"
                          aria-current={isSelected ? "true" : undefined}
                          className="rounded-xs text-left font-medium text-foreground outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                        >
                          {repo.canonical_origin.split("/").at(-1)}
                        </button>
                      </span>
                    </TableCell>
                    <TableCell className="mono w-full max-w-0 text-caption text-foreground-secondary">
                      <Origin value={repo.canonical_origin} />
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <Badge variant="outline">
                        {repo.origins.length} origin
                        {repo.origins.length === 1 ? "" : "s"}
                      </Badge>
                    </TableCell>
                    <TableCell className="whitespace-nowrap text-right">
                      <Button
                        variant="link"
                        size="sm"
                        onClick={(event) => {
                          event.stopPropagation();
                          snapshotOpener.remember();
                          setSnapshotRepo(repo);
                        }}
                      >
                        Snapshots
                      </Button>
                    </TableCell>
                  </MotionRow>
                );
              })}
            </TableBody>
          </Table>
          {(offset > 0 || data.next_offset != null) && (
            <div className="flex items-center gap-2 border-t border-border px-3 py-2.5">
              <Button
                variant="secondary"
                size="sm"
                aria-label="Previous repositories"
                disabled={!offset}
                onClick={() => setOffset(Math.max(0, offset - 50))}
              >
                Previous
              </Button>
              <span className="mono mx-auto text-caption text-muted-foreground">
                {data.items.length ? offset + 1 : 0}–
                {Math.min(offset + 50, data.total)} of {data.total}
              </span>
              <Button
                variant="secondary"
                size="sm"
                aria-label="Next repositories"
                disabled={data.next_offset == null}
                onClick={() => setOffset(data.next_offset!)}
              >
                Next
              </Button>
            </div>
          )}
        </TableWrap>
      )}
      <Modal
        opened={publishing}
        onClose={() => setPublishing(false)}
        title="Publish from your host"
        description="Your connected coding host publishes an exact committed tree from its local checkout."
      >
        <div className="grid gap-3">
          <p className="m-0 text-body text-foreground">
            Ask your agent, from a task scoped to this repository, to publish
            the checkout through the Recollect plugin.
          </p>
          <p className="m-0 text-caption text-muted-foreground">
            Working files stay local. Retaining selected file text requires the
            Brain policy and an explicit <Code>--retain-file</Code> selection.
          </p>
        </div>
      </Modal>
      {snapshotRepo && (
        <RepositoryDialog
          brain={brain}
          repository={snapshotRepo}
          onClose={() => {
            setSnapshotRepo(null);
            snapshotOpener.restore();
          }}
        />
      )}
    </section>
  );
}
