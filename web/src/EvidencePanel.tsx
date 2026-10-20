import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { motion } from "motion/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowRight,
  Check,
  CircleAlert,
  CircleDashed,
  FileText,
  FolderOpen,
  Layers,
  Link2,
  Pencil,
  Plus,
  RefreshCw,
  SlidersHorizontal,
  Upload,
} from "lucide-react";
import { useStableRows } from "./components/useStableRows";
import { FilterBar } from "./components/FilterBar";
import { ScopeSummary } from "./components/ScopeSummary";
import {
  DetailInspector,
  InspectorSection,
  PropList,
} from "./components/Inspector";
import { ActionMenu } from "./components/ActionMenu";
import { EmptyState } from "./components/AsyncState";
import { Skeleton, SkeletonRows } from "./components/Skeleton";
import { StatusDot } from "./components/StatusDot";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Count,
  Drawer,
  Field,
  FileButton,
  Modal,
  MultiSelect,
  SearchInput,
  Select,
  Separator,
  Spinner,
  StatusBadge,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableWrap,
  Textarea,
  TextareaField,
  TextField,
  Tile,
  type StatusState,
} from "./components/ui";
import { cn } from "./lib/utils";
import { useDebouncedValue } from "./lib/hooks";
import { listItem } from "./design/motion";
import { useBrainSearch } from "./app/useBrainSearch";
import { useContentDeadline } from "./useContentDeadline";
import { EraseAction, ExcerptAction } from "./RetentionPanel";
import { LearningAction } from "./ModelsPanel";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import {
  useKnowledgeSelection,
  useLineageOverlay,
} from "./features/knowledge/selection";
import { usePollInterval } from "./usePollInterval";
import { useEvidenceList } from "./catalogueQueries";
import { useExploreCount } from "./features/knowledge/frame";

type Source = components["schemas"]["SourceSummary"];
type Version = Source["version"];
type View = components["schemas"]["EvidenceGroup"];
type Content = components["schemas"]["SourceContent"];
const kinds = [
  { value: "collection", label: "Collection" },
  { value: "area", label: "Area" },
  { value: "environment", label: "Environment" },
];
const labels: Record<string, string> = {
  queued: "Queued",
  running: "Processing",
  failed: "Processing failed",
  cancelled: "Processing cancelled",
  ready: "Processed",
  reference_only: "Reference only",
  missing: "Source missing",
  retained: "Text retained",
  unreadable: "Source unreadable",
  unavailable: "Storage unavailable",
  expired: "Expired",
  erased: "Erased",
};
const timestamp = (value: string) => new Date(value).toLocaleString();
/** Compact list time: date and minutes, full value in the title. */
const shortTime = (value: string) =>
  new Date(value).toLocaleString(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
const words = (value: string) => {
  const text = value.replaceAll("_", " ");
  return text.charAt(0).toUpperCase() + text.slice(1);
};
const stateLabel = (version: Version) =>
  labels[
    version.availability === "retained"
      ? version.processing
      : version.availability
  ] ?? version.processing;
const kindLabel = (version: Version) =>
  version.availability === "reference_only"
    ? "Reference"
    : words(version.retention_class ?? "document");
/** One composite status: colour, icon and word together. */
const statusState = (version: Version): StatusState =>
  version.processing === "ready" && version.availability === "retained"
    ? "positive"
    : ["failed", "missing", "unavailable", "unreadable"].includes(
          version.processing,
        ) ||
        ["missing", "unavailable", "unreadable"].includes(version.availability)
      ? "negative"
      : "neutral";
const groupOptions = (views: View[]) =>
  kinds.map((kind) => ({
    group: kind.label,
    items: views
      .filter((v) => v.kind === kind.value)
      .map((v) => ({ value: v.id, label: v.name })),
  }));

/** Processing state as one badge; live work shows a jade live dot. */
function SourceStatus({ version }: { version: Version }) {
  if (version.processing === "running" && version.availability === "retained")
    return (
      <Badge variant="brand">
        <StatusDot tone="accent" live size={6} />
        {stateLabel(version)}
      </Badge>
    );
  return (
    <StatusBadge state={statusState(version)}>
      {stateLabel(version)}
    </StatusBadge>
  );
}

export function EvidencePanel({ brain }: { brain: Brain }) {
  const pollNormal = usePollInterval("normal");
  const cache = useQueryClient();
  const { select } = useKnowledgeSelection();
  const [route, patchRoute] = useBrainSearch();
  const filters: Record<string, string | null> = {
    collection: route.collection ?? null,
    area: route.area ?? null,
    environment: route.environment ?? null,
  };
  const setFilters = (values: Record<string, string | null>) =>
    patchRoute(values);
  useEffect(
    () => setOffset(0),
    [route.collection, route.area, route.environment],
  );
  const [offset, setOffset] = useState(0);
  const [search, setSearch] = useState("");
  const [query] = useDebouncedValue(search.trim(), 300);
  const [filtering, setFiltering] = useState(false);
  const [manage, setManage] = useState(false);
  const [importing, setImporting] = useState(false);
  const selected = route.source ?? null;
  // Selecting a source shows what it currently supports, so a row never pins
  // the version its list read had; the inspector follows the source's current
  // version. Only an explicit version choice or deep link pins one.
  const setSelected = (source: Source | null) =>
    patchRoute({ source: source?.id ?? null, version: null, detail: null });
  const versionReference = useQuery({
    queryKey: ["source-reference", brain.id, route.version],
    enabled: !!route.version && !selected,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/claim-evidence/{kind}/{evidence}",
          {
            params: {
              path: {
                brain: brain.id,
                kind: "source_version",
                evidence: route.version!,
              },
            },
            signal,
          },
        ),
      ),
    refetchInterval: pollNormal,
  });
  const selectedSourceId =
    selected ??
    (!versionReference.error
      ? versionReference.data?.evidence.source_id
      : null);
  useEffect(() => {
    select(
      selectedSourceId
        ? { kind: "source", id: selectedSourceId, version: route.version }
        : null,
    );
  }, [selectedSourceId, route.version, select]);
  useEffect(() => () => select(null), [select]);

  const [editing, setEditing] = useState<{
    source: Source;
    content: Content;
  } | null>(null);
  const writable = brain.role !== "reader" && !brain.archived;
  const catalogue = useEvidenceList(
    brain.id,
    {
      q: query || undefined,
      collection: filters.collection ?? undefined,
      area: filters.area ?? undefined,
      environment: filters.environment ?? undefined,
      offset,
    },
    { poll: "normal" },
  );
  const refresh = () => {
    for (const key of [
      "evidence",
      "source-history",
      "source-content",
      "audit",
      "jobs",
      "processing",
    ])
      void cache.invalidateQueries({ queryKey: [key, brain.id] });
  };
  const clear = () => {
    setFilters({ collection: null, area: null, environment: null });
    setSearch("");
    setOffset(0);
  };
  const groups = catalogue.data?.groups ?? [];
  const stable = useStableRows(
    catalogue.isError ? undefined : catalogue.data?.sources,
    JSON.stringify([brain.id, filters, query, offset]),
    !!selectedSourceId,
    (row) => row.id,
  );
  // Rows stagger in on the first load only; later mounts (refetch, paging,
  // new rows) appear in place.
  const settled = useRef(false);
  const firstLoad = !settled.current;
  useEffect(() => {
    if (stable.rows.length) settled.current = true;
  });
  const active = Object.values(filters).filter(Boolean) as string[];
  const total = catalogue.data?.total ?? 0;
  // The switcher tab shows the Brain's source count, not a filtered count.
  useExploreCount(
    "sources",
    !query && !active.length && !catalogue.isError
      ? catalogue.data?.total
      : null,
  );
  if (catalogue.error)
    return (
      <Alert
        variant="destructive"
        title="Evidence could not be loaded"
        className="animate-enter"
        actions={
          <>
            <Button
              variant="destructive-ghost"
              size="sm"
              onClick={() => void catalogue.refetch()}
            >
              Try again
            </Button>
            <Button variant="ghost" size="sm" onClick={clear}>
              Clear filters
            </Button>
          </>
        }
      >
        {catalogue.error.message}
      </Alert>
    );
  return (
    <section className="flex flex-col gap-4" aria-label="Knowledge sources">
      <FilterBar>
        <div className="min-w-0 flex-[0_1_280px]">
          <SearchInput
            size="sm"
            aria-label="Search source titles"
            placeholder="Search source titles…"
            maxLength={200}
            value={search}
            onChange={(event) => {
              setSearch(event.currentTarget.value);
              setOffset(0);
            }}
          />
        </div>
        <Button variant="ghost" size="sm" onClick={() => setFiltering(true)}>
          <SlidersHorizontal aria-hidden="true" />
          Filters
          {active.length > 0 && <Count>{active.length}</Count>}
        </Button>
        {(writable || brain.role === "admin") && (
          <Button
            variant="ghost"
            size="sm"
            disabled={catalogue.isPending}
            onClick={() => setManage(true)}
          >
            <Layers aria-hidden="true" />
            Organize
          </Button>
        )}
        <span
          className="mono ml-auto shrink-0 whitespace-nowrap text-caption text-muted-foreground"
          aria-live="polite"
        >
          {catalogue.data
            ? `${catalogue.data.total} source${catalogue.data.total === 1 ? "" : "s"}`
            : "Loading sources"}
          {query ? " · matching source titles" : ""}
        </span>
        {writable && (
          <Button
            size="sm"
            disabled={catalogue.isPending}
            onClick={() => setImporting(true)}
          >
            <Plus aria-hidden="true" />
            Add source
          </Button>
        )}
      </FilterBar>
      {active.length > 0 && (
        <ScopeSummary>
          <span>Showing</span>
          {active.map((id) => {
            const group = groups.find((entry) => entry.id === id);
            return (
              <Badge variant="outline" key={id}>
                {group
                  ? `${words(group.kind)} · ${group.name}`
                  : "Selected view"}
              </Badge>
            );
          })}
          <Button variant="ghost" size="sm" onClick={clear}>
            Clear filters
          </Button>
        </ScopeSummary>
      )}
      <Drawer
        opened={filtering}
        onClose={() => setFiltering(false)}
        title="Filter sources"
        description="Collections, areas and environments organize the same evidence. Selected filters intersect."
        footer={
          <>
            <Button variant="secondary" onClick={clear}>
              Clear source filters
            </Button>
            <Button onClick={() => setFiltering(false)}>Show sources</Button>
          </>
        }
      >
        <div className="grid gap-4">
          {kinds.map((kind) => (
            <Select
              key={kind.value}
              label={kind.label}
              placeholder="All"
              clearable
              searchable
              value={filters[kind.value]}
              onChange={(value) => {
                setFilters({ ...filters, [kind.value]: value });
                setOffset(0);
              }}
              data={groups
                .filter((v) => v.kind === kind.value)
                .map((v) => ({ value: v.id, label: v.name }))}
            />
          ))}
        </div>
      </Drawer>
      {stable.pending > 0 && (
        <div>
          <Button variant="secondary" size="sm" onClick={stable.reveal}>
            <StatusDot tone="accent" size={6} />
            {stable.pending} new sources · Show updates
          </Button>
        </div>
      )}
      {catalogue.isPending ? (
        <TableWrap className="px-3">
          <SkeletonRows label="Loading sources…" rows={6} />
        </TableWrap>
      ) : !catalogue.data?.sources.length ? (
        <div className="rounded-lg border border-border bg-card">
          <EmptyState
            icon={FolderOpen}
            title={
              query || active.length
                ? "No matching sources"
                : "Start with your evidence"
            }
            description={
              query || active.length
                ? "Try another title or clear the selected views."
                : "Add a text document, paste useful material, or keep a reference to a controlled source."
            }
          />
        </div>
      ) : (
        <TableWrap className="overflow-clip">
          <Table>
            <TableHead>
              <tr>
                <TableHeader scope="col">Source</TableHeader>
                <TableHeader scope="col">Kind</TableHeader>
                <TableHeader scope="col">Status</TableHeader>
                <TableHeader scope="col">Contributor</TableHeader>
                <TableHeader scope="col" className="text-right">
                  Versions
                </TableHeader>
                <TableHeader scope="col" className="text-right">
                  Recorded
                </TableHeader>
              </tr>
            </TableHead>
            <TableBody>
              {stable.rows.map((source, index) => {
                const isSelected = selectedSourceId === source.id;
                const Icon =
                  source.version.availability === "reference_only"
                    ? Link2
                    : FileText;
                return (
                  <MotionRow
                    key={source.id}
                    selected={isSelected}
                    variants={listItem}
                    custom={index}
                    initial={firstLoad ? "initial" : false}
                    animate="animate"
                    className="source-row table-row cursor-pointer border-0 p-0"
                    onClick={(event) => {
                      // Keep focus on the row's button so closing the
                      // inspector returns focus here.
                      event.currentTarget
                        .querySelector<HTMLButtonElement>("button")
                        ?.focus({ preventScroll: true });
                      setSelected(source);
                    }}
                  >
                    {/* One identifier line (title and kind) and one status
                        line (composite status, contributor, versions, time).
                        Group views, origin and history live in the shared
                        lineage inspector. */}
                    <TableCell primary className="w-full max-w-0">
                      <span className="source-record-line flex min-w-0">
                        <span className="flex min-w-0 items-center gap-2.5">
                          <Icon
                            aria-hidden="true"
                            className="size-4 shrink-0 text-muted-foreground"
                          />
                          <button
                            type="button"
                            aria-current={isSelected ? "true" : undefined}
                            title={source.version.title}
                            className="min-w-0 truncate rounded-xs text-left font-medium text-foreground outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                          >
                            {source.version.title}
                          </button>
                        </span>
                      </span>
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <Badge variant="outline">
                        {kindLabel(source.version)}
                      </Badge>
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <span className="source-record-line flex">
                        <SourceStatus version={source.version} />
                      </span>
                    </TableCell>
                    <TableCell className="max-w-40 truncate whitespace-nowrap">
                      {source.version.contributor}
                    </TableCell>
                    <TableCell num>{source.version_count}</TableCell>
                    <TableCell
                      num
                      className="whitespace-nowrap"
                      title={timestamp(source.version.recorded_at)}
                    >
                      {shortTime(source.version.recorded_at)}
                    </TableCell>
                  </MotionRow>
                );
              })}
            </TableBody>
          </Table>
          {(offset > 0 || total > 50) && (
            <div className="flex items-center gap-2 border-t border-border px-3 py-2.5">
              <Button
                size="sm"
                variant="secondary"
                aria-label="Previous sources"
                disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - 50))}
              >
                Previous
              </Button>
              <span className="mono mx-auto text-caption text-muted-foreground">
                {offset + 1}–{Math.min(offset + 50, total)} of {total}
              </span>
              <Button
                size="sm"
                variant="secondary"
                aria-label="Next sources"
                disabled={offset + 50 >= total}
                onClick={() => setOffset(offset + 50)}
              >
                Next
              </Button>
            </div>
          )}
        </TableWrap>
      )}
      <Alert variant="info" role="note" className="max-w-[560px]">
        Search matches titles. Import supports text files, pasted text and
        references. Content retention is managed in{" "}
        <Link
          to="/brains/$brainId/settings"
          params={{ brainId: brain.id }}
          search={{ tab: "privacy" }}
          className="link-inline"
        >
          Settings
        </Link>
        .
      </Alert>
      <GroupManager
        brain={brain.id}
        admin={brain.role === "admin"}
        editable={writable}
        views={groups}
        opened={manage}
        close={() => setManage(false)}
        refresh={() => {
          clear();
          refresh();
        }}
      />
      <SourceEditor
        key={editing?.source.version.id ?? `new-${importing}`}
        brain={brain.id}
        opened={importing || !!editing}
        close={() => {
          setImporting(false);
          setEditing(null);
        }}
        allowContent={catalogue.data?.policy.allow_document_content ?? false}
        views={groups}
        initialGroups={active}
        edit={editing}
        saved={() => {
          setImporting(false);
          setEditing(null);
          setSelected(null);
          refresh();
        }}
      />
      {route.version && !selected && !selectedSourceId && (
        <DetailInspector
          opened
          onClose={() => setSelected(null)}
          title="Source evidence"
        >
          {versionReference.error ? (
            <Alert
              variant="destructive"
              actions={
                <Button
                  variant="destructive-ghost"
                  size="sm"
                  onClick={() => void versionReference.refetch()}
                >
                  Try again
                </Button>
              }
            >
              {versionReference.error.message}
            </Alert>
          ) : (
            <Spinner label="Opening the exact source version…" />
          )}
        </DetailInspector>
      )}
    </section>
  );
}

const MotionRow = motion.create(TableRow);

function GroupManager({
  brain,
  admin,
  editable,
  views,
  opened,
  close,
  refresh,
}: {
  brain: string;
  admin: boolean;
  editable: boolean;
  views: View[];
  opened: boolean;
  close: () => void;
  refresh: () => void;
}) {
  const [kind, setKind] = useState("collection");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [removing, setRemoving] = useState<View | null>(null);
  const command = useIdempotency();
  const save = useMutation({
    mutationFn: async () => {
      const body = { name, description };
      if (editing)
        return result(
          await client.PATCH("/api/brains/{brain}/evidence/groups/{group}", {
            params: { path: { brain, group: editing } },
            body,
          }),
        );
      return result(
        await client.POST("/api/brains/{brain}/evidence/groups", {
          params: { path: { brain } },
          body: { ...body, kind },
          headers: { "Idempotency-Key": command.forInput({ ...body, kind }) },
        }),
      );
    },
    onSuccess: () => {
      setName("");
      setDescription("");
      setEditing(null);
      command.reset();
      refresh();
    },
  });
  const remove = useMutation({
    mutationFn: async (id: string) =>
      result(
        await client.DELETE("/api/brains/{brain}/evidence/groups/{group}", {
          params: { path: { brain, group: id } },
        }),
      ),
    onSuccess: () => {
      setRemoving(null);
      refresh();
    },
  });
  const shown = views.filter((v) => v.kind === kind);
  return (
    <Modal
      opened={opened}
      onClose={close}
      title="Manage views"
      description="Collections, areas and environments organize the same sources. Removing a view preserves its evidence."
      size="lg"
    >
      <div className="grid gap-4">
        <Select
          label="View kind"
          data={kinds}
          value={kind}
          disabled={!!editing}
          onChange={(v) => setKind(v ?? "collection")}
        />
        <form
          className="grid gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            save.mutate();
          }}
        >
          <TextField
            label="View name"
            required
            maxLength={120}
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
          />
          <TextareaField
            label="View description"
            maxLength={2000}
            value={description}
            onChange={(e) => setDescription(e.currentTarget.value)}
          />
          {save.error && (
            <Alert variant="destructive">{save.error.message}</Alert>
          )}
          <div className="flex flex-wrap gap-2">
            <Button type="submit" loading={save.isPending} disabled={!editable}>
              {editing ? "Save view" : "Create view"}
            </Button>
            {editing && (
              <Button
                variant="ghost"
                onClick={() => {
                  setEditing(null);
                  setName("");
                  setDescription("");
                }}
              >
                Cancel edit
              </Button>
            )}
          </div>
        </form>
        <Separator />
        <div className="grid gap-2">
          <h3 className="section-label m-0">
            {words(kind)} views
            <span className="mono ml-1.5">{shown.length}</span>
          </h3>
          {shown.map((view) => (
            <Card key={view.id} className="grid gap-3 p-3">
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <p className="m-0 truncate text-body font-medium text-foreground">
                    {view.name}
                  </p>
                  <p className="mono m-0 text-caption text-muted-foreground">
                    {view.source_count} source
                    {view.source_count === 1 ? "" : "s"}
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  {view.kind === "collection" && (
                    <EraseAction
                      brain={brain}
                      permitted={admin}
                      target={{ kind: "collection", id: view.id }}
                      name={view.name}
                    />
                  )}
                  <Button
                    disabled={!editable}
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setEditing(view.id);
                      setName(view.name);
                      setDescription(view.description);
                      save.reset();
                    }}
                  >
                    Rename
                  </Button>
                  <Button
                    size="sm"
                    variant="destructive-ghost"
                    disabled={!editable}
                    onClick={() => {
                      remove.reset();
                      setRemoving(view);
                    }}
                  >
                    Remove view
                  </Button>
                </div>
              </div>
              {removing?.id === view.id && (
                <Alert
                  variant="warning"
                  actions={
                    <>
                      <Button
                        size="sm"
                        variant="destructive"
                        loading={remove.isPending}
                        onClick={() => remove.mutate(view.id)}
                      >
                        Remove this view
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => setRemoving(null)}
                      >
                        Keep view
                      </Button>
                    </>
                  }
                >
                  <p>
                    Remove {view.name}? Its sources and history stay in the
                    Brain.
                  </p>
                  {remove.error && (
                    <p className="text-destructive-text">
                      {remove.error.message}
                    </p>
                  )}
                </Alert>
              )}
            </Card>
          ))}
          {!shown.length && (
            <p className="m-0 text-body text-muted-foreground">
              No views of this kind yet.
            </p>
          )}
        </div>
      </div>
    </Modal>
  );
}

function SourceEditor({
  brain,
  opened,
  close,
  allowContent,
  views,
  initialGroups,
  edit,
  saved,
}: {
  brain: string;
  opened: boolean;
  close: () => void;
  allowContent: boolean;
  views: View[];
  initialGroups: string[];
  edit: { source: Source; content: Content } | null;
  saved: () => void;
}) {
  const formId = useId();
  const [title, setTitle] = useState(edit?.content.version.title ?? "");
  const [mode, setMode] = useState(
    edit?.content.content != null
      ? "text"
      : allowContent
        ? "text"
        : "reference",
  );
  const [content, setContent] = useState(edit?.content.content ?? "");
  const [uri, setUri] = useState(edit?.content.version.source_uri ?? "");
  const [retentionClass, setRetentionClass] = useState(
    edit?.content.version.retention_class ?? "document",
  );
  const [media, setMedia] = useState(
    edit?.content.version.media_type ?? "text/plain",
  );
  const [observed, setObserved] = useState(
    edit?.content.version.observed_at ?? "",
  );
  const [groups, setGroups] = useState(initialGroups);
  const [consent, setConsent] = useState(false);
  const [fileError, setFileError] = useState("");
  const [loading, setLoading] = useState(false);
  const command = useIdempotency();
  const save = useMutation({
    mutationFn: async () => {
      if (observed.trim() && !/T.*(Z|[+-]\d{2}:\d{2})$/i.test(observed.trim()))
        throw new Error(
          "Enter an ISO date/time with timezone, or leave observation time unknown.",
        );
      const observation = observed.trim() ? new Date(observed) : null;
      if (observation && Number.isNaN(observation.getTime()))
        throw new Error("The observation date could not be read.");
      const body = {
        title,
        media_type: media,
        source_uri: uri || null,
        observed_at: observation?.toISOString() ?? null,
        content: mode === "text" ? content : null,
        retain_content: mode === "text" && consent,
        group_ids: edit ? [] : groups,
        base_version: edit?.source.version.id ?? null,
        retention_class: retentionClass,
      };
      const headers = { "Idempotency-Key": command.forInput(body) };
      return edit
        ? result(
            await client.POST("/api/brains/{brain}/sources/{source}/versions", {
              params: { path: { brain, source: edit.source.id } },
              body,
              headers,
            }),
          )
        : result(
            await client.POST("/api/brains/{brain}/sources", {
              params: { path: { brain } },
              body,
              headers,
            }),
          );
    },
    onSuccess: () => {
      command.reset();
      saved();
    },
  });
  async function readFile(file: File | null) {
    if (!file) return;
    setFileError("");
    setLoading(true);
    try {
      if (file.size > 1024 * 1024)
        throw new Error("Choose a text file no larger than 1 MiB.");
      const extension = file.name.split(".").at(-1)?.toLowerCase();
      const formats: Record<string, string> = {
        txt: "text/plain",
        md: "text/markdown",
        markdown: "text/markdown",
        json: "application/json",
        yaml: "text/yaml",
        yml: "text/yaml",
        toml: "application/toml",
        log: "text/plain",
      };
      if (!extension || !formats[extension])
        throw new Error(
          "Choose a UTF-8 text, Markdown, JSON, YAML or TOML file.",
        );
      let value: string;
      try {
        value = new TextDecoder("utf-8", { fatal: true }).decode(
          await file.arrayBuffer(),
        );
      } catch {
        throw new Error("This file is not valid UTF-8 text.");
      }
      if (value.includes("\0"))
        throw new Error("Binary content cannot be imported as text.");
      setContent(value);
      setMedia(formats[extension]);
      if (!title) setTitle(file.name.slice(0, 120));
      setMode("text");
    } catch (error) {
      setFileError(
        error instanceof Error ? error.message : "File could not be read.",
      );
    } finally {
      setLoading(false);
    }
  }
  return (
    <Modal
      opened={opened}
      onClose={() => !save.isPending && close()}
      title={edit ? "Edit source · new version" : "Import source"}
      size="xl"
      footer={
        <>
          <Button variant="secondary" disabled={save.isPending} onClick={close}>
            Cancel
          </Button>
          <Button
            type="submit"
            form={formId}
            loading={save.isPending}
            disabled={
              loading || (mode === "text" && (!consent || !allowContent))
            }
          >
            {edit ? "Save new version" : "Import source"}
          </Button>
        </>
      }
    >
      <form
        id={formId}
        className="grid gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate();
        }}
      >
        <TextField
          label="Source title"
          required
          maxLength={120}
          value={title}
          onChange={(e) => setTitle(e.currentTarget.value)}
        />
        <div className="grid gap-4">
          <Select
            label="Import method"
            data={[
              ...(allowContent
                ? [{ value: "text", label: "Retain document text" }]
                : []),
              { value: "reference", label: "Reference only" },
            ]}
            value={mode}
            onChange={(v) => setMode(v ?? "reference")}
          />
          <Select
            label="Retention class"
            value={retentionClass}
            disabled={!!edit}
            onChange={(v) => setRetentionClass(v ?? "document")}
            data={[
              { value: "document", label: "Document" },
              { value: "raw_session", label: "Sanitized raw session" },
              { value: "tool_output", label: "Sanitized tool output" },
            ]}
            description="Raw classes expire after 30 days by default. This class stays with the source."
          />
        </div>
        {!allowContent && (
          <Alert variant="warning" role="status">
            This Brain currently allows reference-only imports.
          </Alert>
        )}
        <TextField
          label="Source reference"
          description="A document URL or controlled source location. Recollect does not fetch it."
          required={mode === "reference"}
          maxLength={2000}
          value={uri}
          onChange={(e) => setUri(e.currentTarget.value)}
        />
        {mode === "text" && (
          <>
            <div className="flex items-end justify-between gap-4">
              <Select
                className="max-w-[280px]"
                label="Text format"
                value={media}
                onChange={(v) => setMedia(v ?? "text/plain")}
                data={[
                  { value: "text/plain", label: "Plain text" },
                  { value: "text/markdown", label: "Markdown" },
                  { value: "application/json", label: "JSON" },
                  { value: "text/yaml", label: "YAML" },
                  { value: "application/toml", label: "TOML" },
                ]}
              />
              <FileButton
                onChange={readFile}
                accept=".txt,.md,.markdown,.json,.yaml,.yml,.toml,.log"
              >
                {(props) => (
                  <Button {...props} variant="secondary" loading={loading}>
                    <Upload aria-hidden="true" />
                    Choose text file
                  </Button>
                )}
              </FileButton>
            </div>
            {fileError && <Alert variant="destructive">{fileError}</Alert>}
            <Field label="Source text" required>
              <Textarea
                required
                minRows={8}
                maxRows={18}
                autosize
                value={content}
                onChange={(e) => setContent(e.currentTarget.value)}
                className="mono text-label"
              />
            </Field>
            <Checkbox
              label="Retain this document’s text in this Brain"
              checked={consent}
              onChange={(e) => setConsent(e.currentTarget.checked)}
              required
            />
          </>
        )}
        <TextField
          label="Observed at"
          description="Optional ISO date/time with timezone. Leave blank when unknown."
          placeholder="2026-09-14T10:00:00Z"
          inputClassName="mono"
          value={observed}
          onChange={(e) => setObserved(e.currentTarget.value)}
        />
        {!edit && (
          <MultiSelect
            inputClassName="relative"
            label="Organize in views"
            searchable
            data={groupOptions(views)}
            value={groups}
            onChange={setGroups}
          />
        )}
        {edit && (
          <p className="m-0 text-caption text-muted-foreground">
            Saving creates a new version. Previous evidence and its contributor
            history are preserved.
          </p>
        )}
        {save.error && (
          <Alert variant="destructive" title="Source could not be saved">
            {save.error.message}
          </Alert>
        )}
      </form>
    </Modal>
  );
}

export function SourceInspector({
  brain,
  id,
  version,
  derived,
}: {
  brain: Brain;
  id: string;
  version?: string;
  derived: (version: string) => ReactNode;
}) {
  const pollNormal = usePollInterval("normal");
  const cache = useQueryClient();
  const [, patch] = useBrainSearch();
  const [editing, setEditing] = useState<Content | null>(null);
  useLineageOverlay(!!editing);
  const identity = useQuery({
    queryKey: ["source-identity", brain.id, id],
    gcTime: 0,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/sources/{source}", {
          params: { path: { brain: brain.id, source: id } },
          signal,
        }),
      ),
  });
  const catalogue = useEvidenceList(brain.id);
  if (identity.isError)
    return (
      <Alert
        variant="destructive"
        actions={
          <Button
            variant="destructive-ghost"
            size="sm"
            onClick={() => void identity.refetch()}
          >
            Retry source
          </Button>
        }
      >
        {identity.error.message}
      </Alert>
    );
  if (!identity.data) return <InspectorSkeleton />;
  const source = identity.data;
  const refresh = () => {
    for (const key of [
      "source-identity",
      "source-history",
      "source-content",
      "evidence",
      "lineage-neighbourhood",
    ])
      void cache.invalidateQueries({ queryKey: [key, brain.id] });
  };
  return (
    <>
      <SourceViewer
        brain={brain.id}
        sourceId={id}
        source={source}
        selectedVersion={version}
        onVersionChange={(version) => patch({ source: id, version })}
        views={catalogue.isError ? [] : (catalogue.data?.groups ?? [])}
        writable={brain.role !== "reader" && !brain.archived}
        admin={brain.role === "admin"}
        close={() => patch({ source: null, version: null, detail: null })}
        refresh={refresh}
        edit={setEditing}
        embedded
        derived={derived}
      />
      {editing && (
        <SourceEditor
          brain={brain.id}
          opened
          close={() => setEditing(null)}
          allowContent={catalogue.data?.policy.allow_document_content ?? false}
          views={catalogue.data?.groups ?? []}
          initialGroups={source.group_ids}
          edit={{ source, content: editing }}
          saved={() => {
            setEditing(null);
            patch({ version: null });
            refresh();
          }}
        />
      )}
    </>
  );
}

/** Shaped placeholder while a source read is in flight. */
function InspectorSkeleton() {
  return (
    <div className="grid gap-4" role="status">
      <div className="flex items-center gap-3">
        <Skeleton width={36} height={36} className="rounded-md" />
        <div className="grid flex-1 gap-2">
          <Skeleton width="70%" height={14} />
          <Skeleton width="45%" height={10} />
        </div>
      </div>
      <Skeleton height={96} className="rounded-md" />
      <span className="sr-only">Loading source…</span>
    </div>
  );
}

/** One recorded stage: label, icon and value, never colour alone. */
function Stage({
  label,
  value,
  tone,
}: {
  label: string;
  value: ReactNode;
  tone: "done" | "live" | "failed" | "idle";
}) {
  return (
    <div className="grid min-w-0 flex-1 content-start gap-1 rounded-md border border-border bg-card px-2.5 py-2 shadow-xs">
      <span className="section-label">{label}</span>
      <span className="flex items-start gap-1 text-label font-medium text-foreground">
        {tone === "done" && (
          <Check
            aria-hidden="true"
            className="mt-0.5 size-3.5 shrink-0 text-brand-text"
          />
        )}
        {tone === "live" && (
          <StatusDot tone="accent" live size={7} className="mt-1.5" />
        )}
        {tone === "failed" && (
          <CircleAlert
            aria-hidden="true"
            className="mt-0.5 size-3.5 shrink-0 text-destructive-text"
          />
        )}
        {tone === "idle" && (
          <CircleDashed
            aria-hidden="true"
            className="mt-0.5 size-3.5 shrink-0 text-muted-foreground"
          />
        )}
        <span className="min-w-0 [overflow-wrap:anywhere]">{value}</span>
      </span>
    </div>
  );
}

function StageLink({ done }: { done: boolean }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "w-3.5 shrink-0 self-center border-t",
        done ? "border-brand/50" : "border-border-strong",
      )}
    />
  );
}

function SourceViewer({
  brain,
  source,
  sourceId,
  selectedVersion,
  onVersionChange,
  views,
  writable,
  admin,
  close,
  refresh,
  edit,
  embedded = false,
  derived,
}: {
  embedded?: boolean;
  derived?: (version: string) => ReactNode;
  brain: string;
  source?: Source;
  sourceId: string;
  selectedVersion?: string;
  onVersionChange: (version: string) => void;
  views: View[];
  writable: boolean;
  admin: boolean;
  close: () => void;
  refresh: () => void;
  edit: (content: Content) => void;
}) {
  const pollNormal = usePollInterval("normal");
  const [offset, setOffset] = useState(0);
  const [organizing, setOrganizing] = useState(false);
  const [groups, setGroups] = useState(source?.group_ids ?? []);
  const history = useQuery({
    queryKey: ["source-history", brain, sourceId, offset],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/sources/{source}/versions", {
          signal,
          params: { path: { brain, source: sourceId }, query: { offset } },
        }),
      ),
    refetchInterval: pollNormal,
  });
  const head = useQuery({
    queryKey: ["source-history", brain, sourceId, 0],
    enabled: !source,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/sources/{source}/versions", {
          params: { path: { brain, source: sourceId }, query: { offset: 0 } },
          signal,
        }),
      ),
    refetchInterval: pollNormal,
  });
  const currentVersion = source?.version.id ?? head.data?.versions[0]?.id;
  const version = selectedVersion ?? currentVersion ?? "";
  const evidence = useQuery({
    queryKey: ["source-content", brain, sourceId, version],
    enabled: !!version,
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/sources/{source}/versions/{version}",
          { params: { path: { brain, source: sourceId, version } }, signal },
        ),
      ),
    refetchInterval: pollNormal,
  });
  const organize = useMutation({
    mutationFn: async () =>
      result(
        await client.PUT("/api/brains/{brain}/sources/{source}/groups", {
          params: { path: { brain, source: sourceId } },
          body: { group_ids: groups },
        }),
      ),
    onSuccess: () => {
      setOrganizing(false);
      refresh();
    },
  });
  const process = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/brains/{brain}/sources/{source}/process", {
          params: { path: { brain, source: sourceId } },
        }),
      ),
    onSuccess: refresh,
  });
  const expired = useContentDeadline(evidence.data?.version.expires_at);
  const data = expired ? undefined : evidence.data;
  const error =
    evidence.error ??
    history.error ??
    head.error ??
    (expired
      ? new Error(
          "This source reached its retention deadline. Refresh to inspect its remaining metadata.",
        )
      : null);
  const versions = source?.version_count ?? history.data?.total;
  const body = (
    <div className="grid">
      {error ? (
        <Alert
          variant="destructive"
          actions={
            <Button
              variant="destructive-ghost"
              size="sm"
              onClick={() => {
                void evidence.refetch();
                void history.refetch();
              }}
            >
              Try again
            </Button>
          }
        >
          {error.message}
        </Alert>
      ) : evidence.isPending ? (
        <InspectorSkeleton />
      ) : (
        data && (
          <>
            <InspectorSection>
              <div className="flex items-start gap-3">
                <Tile>
                  {data.version.availability === "reference_only" ? (
                    <Link2 aria-hidden="true" />
                  ) : (
                    <FileText aria-hidden="true" />
                  )}
                </Tile>
                <div className="grid min-w-0 gap-0.5">
                  <h3 className="m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
                    {data.version.title}
                  </h3>
                  <span className="text-caption text-muted-foreground">
                    {kindLabel(data.version)} · contributed by{" "}
                    {data.version.contributor}
                  </span>
                </div>
              </div>
            </InspectorSection>
            <InspectorSection label="Details">
              <PropList
                items={[
                  {
                    term: "Status",
                    value: <SourceStatus version={data.version} />,
                  },
                  { term: "Kind", value: kindLabel(data.version) },
                  { term: "Contributor", value: data.version.contributor },
                  ...(versions != null
                    ? [
                        {
                          term: "Versions",
                          value: <span className="mono">{versions}</span>,
                        },
                      ]
                    : []),
                  {
                    term: "Recorded",
                    value: (
                      <span className="mono">
                        {timestamp(data.version.recorded_at)}
                      </span>
                    ),
                  },
                  {
                    term: "Observed",
                    value: data.version.observed_at ? (
                      <span className="mono">
                        {timestamp(data.version.observed_at)}
                      </span>
                    ) : (
                      <span className="text-muted-foreground">
                        At an unknown time
                      </span>
                    ),
                  },
                  {
                    term: "Retention",
                    value: (
                      <span>
                        {words(data.version.retention_class ?? "document")} ·{" "}
                        {data.version.expires_at ? (
                          <span className="mono">
                            expires {timestamp(data.version.expires_at)}
                          </span>
                        ) : (
                          "until erased"
                        )}
                      </span>
                    ),
                  },
                  ...(data.version.source_uri
                    ? [
                        {
                          term: "Reference",
                          value: (
                            <span className="mono text-caption">
                              {data.version.source_uri}
                            </span>
                          ),
                        },
                      ]
                    : []),
                ]}
              />
              {data.version.excerpt && (
                <p className="m-0 text-caption text-muted-foreground">
                  Supporting excerpt from source lines{" "}
                  <span className="mono">
                    {data.version.excerpt.first_line}–
                    {data.version.excerpt.last_line}
                  </span>
                  , captured{" "}
                  <span className="mono">
                    {timestamp(data.version.excerpt.captured_at)}
                  </span>
                  .
                </p>
              )}
            </InspectorSection>
            <InspectorSection label="Processing">
              <div className="flex items-stretch">
                <Stage
                  label="Contributed"
                  value={data.version.contributor}
                  tone="done"
                />
                <StageLink done />
                <Stage
                  label="Processing"
                  value={stateLabel(data.version)}
                  tone={
                    data.version.processing === "running" &&
                    data.version.availability === "retained"
                      ? "live"
                      : statusState(data.version) === "positive"
                        ? "done"
                        : statusState(data.version) === "negative"
                          ? "failed"
                          : "idle"
                  }
                />
                <StageLink done={statusState(data.version) === "positive"} />
                <Stage
                  label="Version"
                  value={
                    version === currentVersion
                      ? "Current version"
                      : "Earlier version"
                  }
                  tone="done"
                />
              </div>
              <p className="m-0 text-caption text-muted-foreground">
                Only recorded stages are shown.
              </p>
            </InspectorSection>
            {version !== currentVersion && (
              <Alert variant="warning" role="status" className="my-4">
                You are viewing an earlier source version. Current knowledge may
                refer to newer evidence.
              </Alert>
            )}
            <InspectorSection label="Content">
              {data.content != null ? (
                <>
                  <pre
                    tabIndex={0}
                    aria-label="Retained source text"
                    className="mono m-0 max-h-[360px] overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-muted p-3 text-label leading-5 text-foreground scrollbar-thin"
                    data-testid="source-content"
                  >
                    {data.content}
                  </pre>
                  <p className="mono m-0 text-caption text-muted-foreground">
                    {data.version.byte_length.toLocaleString()} retained bytes ·{" "}
                    {data.spans.length} processed support span
                    {data.spans.length === 1 ? "" : "s"}
                  </p>
                </>
              ) : (
                <Alert
                  variant="warning"
                  role="status"
                  title={
                    data.version.availability === "reference_only"
                      ? "Reference without retained text"
                      : "Source text is unavailable"
                  }
                >
                  {data.version.availability === "reference_only"
                    ? "Recollect keeps this reference but has not verified that the external source is available."
                    : ["expired", "erased"].includes(
                          data.version.privacy_state ?? "active",
                        )
                      ? "This content was " +
                        data.version.privacy_state +
                        ". Retention changes cannot restore removed content."
                      : "The recorded artifact could not be read. Restore it or import a new version; existing metadata does not recreate missing evidence."}
                </Alert>
              )}
              <ActionMenu label="Version history">
                <Select
                  label="Evidence version"
                  data={[
                    ...(data &&
                    !history.data?.versions.some((v) => v.id === version)
                      ? [data.version]
                      : []),
                    ...(history.data?.versions ?? []),
                  ].map((v) => ({
                    value: v.id,
                    label: `${v.id === currentVersion ? "Current · " : "Earlier · "}${timestamp(v.recorded_at)} · ${v.contributor}`,
                  }))}
                  value={version}
                  onChange={(v) => v && onVersionChange(v)}
                />
                {(offset > 0 || (history.data?.total ?? 0) > 20) && (
                  <div className="flex gap-1">
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={offset === 0}
                      onClick={() => setOffset(Math.max(0, offset - 20))}
                    >
                      Newer history
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={offset + 20 >= (history.data?.total ?? 0)}
                      onClick={() => setOffset(offset + 20)}
                    >
                      Older history
                    </Button>
                  </div>
                )}
              </ActionMenu>
            </InspectorSection>
            {derived && (
              <InspectorSection label="What this source supports">
                {derived(version)}
              </InspectorSection>
            )}
            <InspectorSection label="Actions">
              {writable && source && (
                <div className="flex flex-wrap gap-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    disabled={
                      version !== currentVersion ||
                      data.version.privacy_state === "erased" ||
                      data.version.retention_class === "support_excerpt"
                    }
                    onClick={() => edit(data)}
                  >
                    <Pencil aria-hidden="true" />
                    Edit source
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    aria-expanded={organizing}
                    onClick={() => {
                      setGroups(source.group_ids);
                      setOrganizing(!organizing);
                    }}
                  >
                    <Layers aria-hidden="true" />
                    Organize source
                  </Button>
                </div>
              )}
              {organizing && (
                <Card className="grid gap-3 p-3">
                  <MultiSelect
                    inputClassName="relative"
                    label="Source views"
                    data={groupOptions(views)}
                    searchable
                    value={groups}
                    onChange={setGroups}
                  />
                  <p className="m-0 text-caption text-muted-foreground">
                    Removing an association keeps the source, history and other
                    views.
                  </p>
                  {organize.error && (
                    <Alert variant="destructive">
                      {organize.error.message}
                    </Alert>
                  )}
                  <div>
                    <Button
                      size="sm"
                      onClick={() => organize.mutate()}
                      loading={organize.isPending}
                    >
                      Save associations
                    </Button>
                  </div>
                </Card>
              )}
              <div className="grid gap-1">
                <Link
                  to="/brains/$brainId/graph"
                  params={{ brainId: brain }}
                  search={{
                    kind: "knowledge",
                    center: `source_version:${version}`,
                    direction: "incoming",
                    hops: 1,
                  }}
                  className="link-inline inline-flex w-fit items-center gap-1 text-label font-medium"
                >
                  Explore memory linked to this version
                  <ArrowRight aria-hidden="true" className="size-3.5" />
                </Link>
                <p className="m-0 text-caption text-muted-foreground">
                  Shows recorded eligible incoming evidence relationships; it
                  does not imply every derived memory is currently eligible.
                </p>
              </div>
              {!source && (
                <p className="m-0 text-caption text-muted-foreground">
                  This exact version was opened directly. Edit its title or
                  grouping from its row in Sources.
                </p>
              )}
              <ActionMenu label="More source actions">
                <p className="m-0 text-label text-muted-foreground">
                  Reprocessing, explicit learning, supporting excerpts and
                  removal use the same Brain policy and permissions.
                </p>
                <div className="flex flex-wrap gap-2">
                  <EraseAction
                    brain={brain}
                    permitted={admin}
                    target={{ kind: "source", id: sourceId }}
                    name={data.version.title}
                  />
                  {writable &&
                    data.version.privacy_state === "active" &&
                    data.content != null && (
                      <ExcerptAction
                        brain={brain}
                        source={sourceId}
                        version={data.version.id}
                        title={data.version.title}
                      />
                    )}
                  <LearningAction
                    brain={brain}
                    version={version}
                    writable={
                      writable &&
                      (data.version.privacy_state ?? "active") === "active"
                    }
                  />
                  {writable &&
                    data.version.retained &&
                    version === currentVersion && (
                      <Button
                        size="sm"
                        variant="ghost"
                        loading={process.isPending}
                        onClick={() => process.mutate()}
                      >
                        <RefreshCw aria-hidden="true" />
                        Reprocess source
                      </Button>
                    )}
                </div>
              </ActionMenu>
              {process.error && (
                <Alert variant="destructive">{process.error.message}</Alert>
              )}
            </InspectorSection>
          </>
        )
      )}
    </div>
  );
  return embedded ? (
    body
  ) : (
    <DetailInspector opened onClose={close} title="Source evidence">
      {body}
    </DetailInspector>
  );
}
