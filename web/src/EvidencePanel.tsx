import { FilterBar } from "./components/FilterBar";
import { ScopeSummary } from "./components/ScopeSummary";
import { DetailInspector } from "./components/DetailInspector";
import { ActionMenu } from "./components/ActionMenu";
import { iconSize } from "./design/tokens";
import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import { useBrainSearch } from "./app/useBrainSearch";
import { useDebouncedValue } from "@mantine/hooks";
import { EmptyState } from "./components/AsyncState";
import { StatusBadge } from "./components/StatusBadge";
import { useContentDeadline } from "./useContentDeadline";
import "./features/feature-views.css";
import { EraseAction, ExcerptAction } from "./RetentionPanel";
import { LearningAction } from "./ModelsPanel";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Divider,
  Drawer,
  FileButton,
  Group,
  Loader,
  Modal,
  MultiSelect,
  Select,
  Stack,
  Switch,
  Text,
  Textarea,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  BookOpen,
  FileText,
  FolderOpen,
  Plus,
  RefreshCw,
  Settings2,
  Search,
  SlidersHorizontal,
  Info,
  Upload,
} from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import { useKnowledgeSelection } from "./features/knowledge/selection";

type Source = components["schemas"]["SourceSummary"];
type View = components["schemas"]["EvidenceGroup"];
type Content = components["schemas"]["SourceContent"];
type Catalogue = components["schemas"]["EvidenceCatalogue"];
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
const stateLabel = (version: Source["version"]) =>
  labels[
    version.availability === "retained"
      ? version.processing
      : version.availability
  ] ?? version.processing;
const groupOptions = (views: View[]) =>
  kinds.map((kind) => ({
    group: kind.label,
    items: views
      .filter((v) => v.kind === kind.value)
      .map((v) => ({ value: v.id, label: v.name })),
  }));

export function EvidencePanel({ brain }: { brain: Brain }) {
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
  const setSelected = (source: Source | null) =>
    patchRoute({
      source: source?.id ?? null,
      version: source?.version.id ?? null,
      detail: null,
    });
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
    refetchInterval: 4000,
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
  const catalogue = useQuery({
    queryKey: ["evidence", brain.id, filters, query, offset],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          signal,
          params: {
            path: { brain: brain.id },
            query: {
              q: query || undefined,
              collection: filters.collection ?? undefined,
              area: filters.area ?? undefined,
              environment: filters.environment ?? undefined,
              offset,
            },
          },
        }),
      ),
    refetchInterval: 4000,
  });
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
  if (catalogue.error)
    return (
      <Card withBorder p="xl">
        <Alert color="red" title="Evidence could not be loaded">
          {catalogue.error.message}
          <Group mt="md">
            <Button variant="subtle" onClick={() => void catalogue.refetch()}>
              Try again
            </Button>
            <Button variant="subtle" onClick={clear}>
              Clear filters
            </Button>
          </Group>
        </Alert>
      </Card>
    );
  return (
    <section
      className="feature-view evidence-panel"
      aria-label="Knowledge sources"
    >
      <Stack gap="lg">
        <FilterBar>
          <TextInput
            className="feature-search"
            aria-label="Search source titles"
            placeholder="Search source titles…"
            leftSection={<Search size={iconSize.navigation} />}
            maxLength={200}
            value={search}
            onChange={(event) => {
              setSearch(event.currentTarget.value);
              setOffset(0);
            }}
          />
          <Button
            variant="default"
            leftSection={<SlidersHorizontal size={iconSize.small} />}
            onClick={() => setFiltering(true)}
          >
            Filters
            {Object.values(filters).some(Boolean)
              ? ` · ${Object.values(filters).filter(Boolean).length}`
              : ""}
          </Button>
          {(writable || brain.role === "admin") && (
            <Button
              variant="subtle"
              leftSection={<Settings2 size={iconSize.small} />}
              disabled={catalogue.isPending}
              onClick={() => setManage(true)}
            >
              Manage views
            </Button>
          )}
          {writable && (
            <Button
              leftSection={<Plus size={iconSize.small} />}
              disabled={catalogue.isPending}
              onClick={() => setImporting(true)}
            >
              Add source
            </Button>
          )}
        </FilterBar>
        <Drawer
          className="feature-drawer"
          opened={filtering}
          onClose={() => setFiltering(false)}
          position="right"
          title="Filter sources"
          size="md"
        >
          <Stack>
            <Text size="sm" c="dimmed">
              Collections, areas and environments organize the same evidence.
              Selected filters intersect.
            </Text>
            <Stack gap="md">
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
            </Stack>
            <Button variant="default" onClick={clear}>
              Clear source filters
            </Button>
            <Button onClick={() => setFiltering(false)}>Show sources</Button>
          </Stack>
        </Drawer>
        <Group justify="space-between">
          <Text size="xs" c="dimmed">
            {catalogue.data
              ? `${catalogue.data.total} source${catalogue.data.total === 1 ? "" : "s"}`
              : "Loading sources"}
            {query ? " · matching source titles" : ""}
          </Text>
          {Object.values(filters).some(Boolean) && (
            <Button variant="subtle" size="compact-sm" onClick={clear}>
              Clear filters
            </Button>
          )}
        </Group>
        {Object.values(filters).some(Boolean) && (
          <ScopeSummary>
            {Object.values(filters)
              .filter(Boolean)
              .map((id) => (
                <Badge variant="light" key={id}>
                  {groups.find((group) => group.id === id)?.name ??
                    "Selected view"}
                </Badge>
              ))}
          </ScopeSummary>
        )}
        {catalogue.isPending ? (
          <Loader />
        ) : !catalogue.data?.sources.length ? (
          <EmptyState
            icon={FolderOpen}
            title={
              query || Object.values(filters).some(Boolean)
                ? "No matching sources"
                : "Start with your evidence"
            }
            description={
              query || Object.values(filters).some(Boolean)
                ? "Try another title or clear the selected views."
                : "Add a text document, paste useful material, or keep a reference to a controlled source."
            }
          />
        ) : (
          <div className="feature-list">
            {catalogue.data.sources.map((source) => (
              <button
                key={source.id}
                className="source-row source-record source-record-dense"
                onClick={() => setSelected(source)}
              >
                {/* Dense row: one identifier line (title plus kind) and one
                    status line (composite mark plus updated date). Group
                    views, origin and version history move to the shared
                    lineage inspector. */}
                <span className="source-record-line">
                  <FileText size={iconSize.action} className="source-symbol" />
                  <span className="source-record-name">
                    <Text fw={600} size="sm">
                      {source.version.title}
                    </Text>
                  </span>
                  <Badge size="xs" variant="light" color="gray">
                    {source.version.availability === "reference_only"
                      ? "Reference"
                      : (
                          source.version.retention_class ?? "document"
                        ).replaceAll("_", " ")}
                  </Badge>
                </span>
                <span className="source-record-line">
                  <StatusBadge
                    state={
                      source.version.processing === "ready" &&
                      source.version.availability === "retained"
                        ? "positive"
                        : [
                              "failed",
                              "missing",
                              "unavailable",
                              "unreadable",
                            ].includes(source.version.processing) ||
                            ["missing", "unavailable", "unreadable"].includes(
                              source.version.availability,
                            )
                          ? "negative"
                          : "neutral"
                    }
                  >
                    {stateLabel(source.version)}
                  </StatusBadge>
                  <Text size="xs" c="dimmed">
                    Updated{" "}
                    {new Date(source.version.recorded_at).toLocaleDateString()}
                  </Text>
                </span>
              </button>
            ))}
          </div>
        )}
        {(offset > 0 || (catalogue.data?.total ?? 0) > 50) && (
          <Group justify="space-between">
            <Button
              size="xs"
              variant="default"
              disabled={offset === 0}
              onClick={() => setOffset(Math.max(0, offset - 50))}
            >
              Previous sources
            </Button>
            <Text size="xs">
              {offset + 1}–{Math.min(offset + 50, catalogue.data?.total ?? 0)}
            </Text>
            <Button
              size="xs"
              variant="default"
              disabled={offset + 50 >= (catalogue.data?.total ?? 0)}
              onClick={() => setOffset(offset + 50)}
            >
              Next sources
            </Button>
          </Group>
        )}
        <div className="feature-note">
          <Info size={iconSize.small} />
          <span>
            Search matches titles. Import supports text files, pasted text and
            references. Content retention is managed in Settings.
          </span>
        </div>
      </Stack>
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
        initialGroups={Object.values(filters).filter((v): v is string => !!v)}
        edit={editing}
        saved={() => {
          setImporting(false);
          setEditing(null);
          setSelected(null);
          refresh();
        }}
      />
      {route.version && !selected && !selectedSourceId && (
        <Drawer
          opened
          onClose={() => setSelected(null)}
          title="Source evidence"
          className="feature-drawer"
        >
          {versionReference.error ? (
            <Alert color="red">
              {versionReference.error.message}
              <Button
                variant="subtle"
                onClick={() => void versionReference.refetch()}
              >
                Try again
              </Button>
            </Alert>
          ) : (
            <Loader />
          )}
        </Drawer>
      )}
      {selectedSourceId && route.detail === "record" && (
        <SourceViewer
          key={selectedSourceId}
          brain={brain.id}
          sourceId={selectedSourceId}
          source={catalogue.data?.sources.find(
            (source) => source.id === selectedSourceId,
          )}
          selectedVersion={route.version}
          onVersionChange={(version) => patchRoute({ version })}
          views={groups}
          writable={writable}
          admin={brain.role === "admin"}
          close={() => setSelected(null)}
          refresh={refresh}
          edit={(content) => {
            const source = catalogue.data?.sources.find(
              (source) => source.id === selectedSourceId,
            );
            if (!source) return;
            setEditing({ source, content });
            setSelected(null);
          }}
        />
      )}
    </section>
  );
}

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
  return (
    <Modal
      opened={opened}
      onClose={close}
      title="Manage views"
      size="lg"
      centered
    >
      <Stack>
        <Text size="sm" c="dimmed">
          Collections, areas and environments organize the same sources.
          Removing a view preserves its evidence.
        </Text>
        <Select
          label="View kind"
          data={kinds}
          value={kind}
          disabled={!!editing}
          onChange={(v) => setKind(v ?? "collection")}
        />
        <form
          onSubmit={(e) => {
            e.preventDefault();
            save.mutate();
          }}
        >
          <Stack>
            <TextInput
              label="View name"
              required
              maxLength={120}
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
            />
            <Textarea
              label="View description"
              maxLength={2000}
              value={description}
              onChange={(e) => setDescription(e.currentTarget.value)}
            />
            {save.error && <Alert color="red">{save.error.message}</Alert>}
            <Group>
              <Button
                type="submit"
                loading={save.isPending}
                disabled={!editable}
              >
                {editing ? "Save view" : "Create view"}
              </Button>
              {editing && (
                <Button
                  variant="subtle"
                  onClick={() => {
                    setEditing(null);
                    setName("");
                    setDescription("");
                  }}
                >
                  Cancel edit
                </Button>
              )}
            </Group>
          </Stack>
        </form>
        <Divider />
        {views
          .filter((v) => v.kind === kind)
          .map((view) => (
            <Card withBorder key={view.id} p="sm">
              <Group justify="space-between">
                <div>
                  <Text size="sm" fw={600}>
                    {view.name}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {view.source_count} source
                    {view.source_count === 1 ? "" : "s"}
                  </Text>
                </div>
                <Group gap="xs">
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
                    size="compact-xs"
                    variant="subtle"
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
                    size="compact-xs"
                    variant="subtle"
                    color="red"
                    disabled={!editable}
                    onClick={() => {
                      remove.reset();
                      setRemoving(view);
                    }}
                  >
                    Remove view
                  </Button>
                </Group>
              </Group>
              {removing?.id === view.id && (
                <Alert mt="sm" color="yellow">
                  <Text size="sm">
                    Remove {view.name}? Its sources and history stay in the
                    Brain.
                  </Text>
                  {remove.error && <Text c="red">{remove.error.message}</Text>}
                  <Group mt="xs">
                    <Button
                      size="xs"
                      color="red"
                      loading={remove.isPending}
                      onClick={() => remove.mutate(view.id)}
                    >
                      Remove this view
                    </Button>
                    <Button
                      size="xs"
                      variant="subtle"
                      onClick={() => setRemoving(null)}
                    >
                      Keep view
                    </Button>
                  </Group>
                </Alert>
              )}
            </Card>
          ))}
        {!views.some((v) => v.kind === kind) && (
          <Text size="sm" c="dimmed">
            No views of this kind yet.
          </Text>
        )}
      </Stack>
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
      centered
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate();
        }}
      >
        <Stack>
          <TextInput
            label="Source title"
            required
            maxLength={120}
            value={title}
            onChange={(e) => setTitle(e.currentTarget.value)}
          />
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
          {!allowContent && (
            <Alert color="yellow">
              This Brain currently allows reference-only imports.
            </Alert>
          )}
          <TextInput
            label="Source reference"
            description="A document URL or controlled source location. Recollect does not fetch it."
            required={mode === "reference"}
            maxLength={2000}
            value={uri}
            onChange={(e) => setUri(e.currentTarget.value)}
          />
          {mode === "text" && (
            <>
              <Group justify="space-between">
                <Select
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
                    <Button
                      {...props}
                      variant="default"
                      leftSection={<Upload size={iconSize.small} />}
                      loading={loading}
                    >
                      Choose text file
                    </Button>
                  )}
                </FileButton>
              </Group>
              {fileError && <Alert color="red">{fileError}</Alert>}
              <Textarea
                label="Source text"
                required
                minRows={8}
                maxRows={18}
                autosize
                value={content}
                onChange={(e) => setContent(e.currentTarget.value)}
                styles={{
                  input: {
                    fontFamily: "var(--rc-font-mono)",
                    fontSize: "var(--mantine-font-size-xs)",
                  },
                }}
              />
              <Checkbox
                label="Retain this document’s text in this Brain"
                checked={consent}
                onChange={(e) => setConsent(e.currentTarget.checked)}
                required
              />
            </>
          )}
          <TextInput
            label="Observed at"
            description="Optional ISO date/time with timezone. Leave blank when unknown."
            placeholder="2026-09-14T10:00:00Z"
            value={observed}
            onChange={(e) => setObserved(e.currentTarget.value)}
          />
          {!edit && (
            <MultiSelect
              label="Organize in views"
              searchable
              data={groupOptions(views)}
              value={groups}
              onChange={setGroups}
            />
          )}
          {edit && (
            <Text c="dimmed" size="xs">
              Saving creates a new version. Previous evidence and its
              contributor history are preserved.
            </Text>
          )}
          {save.error && (
            <Alert color="red" title="Source could not be saved">
              {save.error.message}
            </Alert>
          )}
          <Group justify="flex-end">
            <Button variant="default" disabled={save.isPending} onClick={close}>
              Cancel
            </Button>
            <Button
              type="submit"
              loading={save.isPending}
              disabled={
                loading || (mode === "text" && (!consent || !allowContent))
              }
            >
              {edit ? "Save new version" : "Import source"}
            </Button>
          </Group>
        </Stack>
      </form>
    </Modal>
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
}: {
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
    refetchInterval: 4000,
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
    refetchInterval: 4000,
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
    refetchInterval: 4000,
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
  return (
    <DetailInspector
      className="feature-drawer"
      opened
      onClose={close}
      title="Source evidence"
      position="right"
      size="min(42rem, 90vw)"
    >
      <Stack>
        {error ? (
          <Alert color="red">
            {error.message}
            <Button
              variant="subtle"
              onClick={() => {
                void evidence.refetch();
                void history.refetch();
              }}
            >
              Try again
            </Button>
          </Alert>
        ) : evidence.isPending ? (
          <Loader />
        ) : (
          data && (
            <>
              <Group justify="space-between" align="flex-start">
                <div>
                  <Title order={2}>{data.version.title}</Title>
                  <Text size="xs" c="dimmed" mt="xs">
                    Contributed by {data.version.contributor} · recorded{" "}
                    {timestamp(data.version.recorded_at)}
                  </Text>
                </div>
                <Badge
                  color={
                    data.version.availability === "retained" ? "teal" : "gray"
                  }
                >
                  {stateLabel(data.version)}
                </Badge>
              </Group>
              <Text size="sm" c="dimmed">
                Retention:{" "}
                {(data.version.retention_class ?? "document").replaceAll(
                  "_",
                  " ",
                )}{" "}
                ·{" "}
                {data.version.expires_at
                  ? "expires " + timestamp(data.version.expires_at)
                  : "until erased"}
              </Text>
              {data.version.excerpt && (
                <Text size="xs" c="dimmed">
                  Supporting excerpt from source lines{" "}
                  {data.version.excerpt.first_line}–
                  {data.version.excerpt.last_line}, captured{" "}
                  {timestamp(data.version.excerpt.captured_at)}.
                </Text>
              )}
              <Text size="sm" c="dimmed">
                Observed{" "}
                {data.version.observed_at
                  ? timestamp(data.version.observed_at)
                  : "at an unknown time"}
              </Text>
              {data.version.source_uri && (
                <Text size="sm" style={{ overflowWrap: "anywhere" }}>
                  Source reference: {data.version.source_uri}
                </Text>
              )}
              <details className="feature-details">
                <summary>Version history</summary>
                <Select
                  label="Evidence version"
                  data={(history.data?.versions ?? []).map((v) => ({
                    value: v.id,
                    label: `${v.id === currentVersion ? "Current · " : "Earlier · "}${timestamp(v.recorded_at)} · ${v.contributor}`,
                  }))}
                  value={version}
                  onChange={(v) => v && onVersionChange(v)}
                />
                {(offset > 0 || (history.data?.total ?? 0) > 20) && (
                  <Group>
                    <Button
                      size="xs"
                      variant="subtle"
                      disabled={offset === 0}
                      onClick={() => setOffset(Math.max(0, offset - 20))}
                    >
                      Newer history
                    </Button>
                    <Button
                      size="xs"
                      variant="subtle"
                      disabled={offset + 20 >= (history.data?.total ?? 0)}
                      onClick={() => setOffset(offset + 20)}
                    >
                      Older history
                    </Button>
                  </Group>
                )}
              </details>
              {version !== currentVersion && (
                <Alert color="yellow">
                  You are viewing an earlier source version. Current knowledge
                  may refer to newer evidence.
                </Alert>
              )}
              {data.content != null ? (
                <>
                  <pre className="source-content" data-testid="source-content">
                    {data.content}
                  </pre>
                  <Text size="xs" c="dimmed">
                    {data.version.byte_length.toLocaleString()} retained bytes ·{" "}
                    {data.spans.length} processed support spans
                  </Text>
                </>
              ) : (
                <Alert
                  color="yellow"
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
              {writable && source && (
                <Group>
                  <Button
                    size="sm"
                    variant="default"
                    disabled={
                      version !== currentVersion ||
                      data.version.privacy_state === "erased" ||
                      data.version.retention_class === "support_excerpt"
                    }
                    onClick={() => edit(data)}
                  >
                    Edit source
                  </Button>
                  <Button
                    size="sm"
                    variant="default"
                    onClick={() => {
                      setGroups(source.group_ids);
                      setOrganizing(!organizing);
                    }}
                  >
                    Organize source
                  </Button>
                </Group>
              )}
              <Link
                to="/brains/$brainId/graph"
                params={{ brainId: brain }}
                search={{
                  kind: "knowledge",
                  center: `source_version:${version}`,
                  direction: "incoming",
                  hops: 1,
                }}
              >
                Explore memory linked to this version
              </Link>
              <Text size="xs" c="dimmed">
                Shows recorded eligible incoming evidence relationships; it does
                not imply every derived memory is currently eligible.
              </Text>
              {!source && (
                <Text size="xs" c="dimmed">
                  This exact version was opened directly. Edit its title or
                  grouping from its row in Sources.
                </Text>
              )}
              <ActionMenu label="More source actions">
                <Stack gap="md">
                  <Text size="sm" c="dimmed">
                    Reprocessing, explicit learning, supporting excerpts and
                    removal use the same Brain policy and permissions.
                  </Text>
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
                        variant="subtle"
                        leftSection={<RefreshCw size={iconSize.small} />}
                        loading={process.isPending}
                        onClick={() => process.mutate()}
                      >
                        Reprocess source
                      </Button>
                    )}
                </Stack>
              </ActionMenu>
              {process.error && (
                <Alert color="red">{process.error.message}</Alert>
              )}
              {organizing && (
                <Card withBorder>
                  <Stack>
                    <MultiSelect
                      label="Source views"
                      data={groupOptions(views)}
                      searchable
                      value={groups}
                      onChange={setGroups}
                    />
                    <Text size="xs" c="dimmed">
                      Removing an association keeps the source, history and
                      other views.
                    </Text>
                    {organize.error && (
                      <Alert color="red">{organize.error.message}</Alert>
                    )}
                    <Button
                      onClick={() => organize.mutate()}
                      loading={organize.isPending}
                    >
                      Save associations
                    </Button>
                  </Stack>
                </Card>
              )}
            </>
          )
        )}
      </Stack>
    </DetailInspector>
  );
}

export function SourcesStoragePolicy({ brain }: { brain: Brain }) {
  const cache = useQueryClient();
  const query = useQuery({
    queryKey: ["evidence", brain.id, "storage-policy"],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          signal,
          params: { path: { brain: brain.id } },
        }),
      ),
    refetchInterval: 4000,
  });
  const save = useMutation({
    mutationFn: async (allow_document_content: boolean) =>
      result(
        await client.PUT("/api/brains/{brain}/evidence/policy", {
          params: { path: { brain: brain.id } },
          body: { allow_document_content },
        }),
      ),
    onSuccess: () =>
      cache.invalidateQueries({ queryKey: ["evidence", brain.id] }),
  });
  return (
    <section className="feature-setting">
      <Stack gap="md">
        <Title order={3}>Document storage</Title>
        <Text size="sm" c="dimmed">
          Choose whether new explicit imports may retain text. Existing evidence
          follows its own retention policy.
        </Text>
        {query.error ? (
          <Alert color="red">{query.error.message}</Alert>
        ) : query.isPending ? (
          <Loader size="sm" />
        ) : (
          <Switch
            label="Allow document content retention"
            description="When off, new imports keep references only. Existing evidence is preserved."
            checked={
              save.isPending
                ? save.variables
                : (query.data?.policy.allow_document_content ?? false)
            }
            disabled={
              brain.role !== "admin" || brain.archived || save.isPending
            }
            onChange={(event) => save.mutate(event.currentTarget.checked)}
          />
        )}
        {save.error && <Alert color="red">{save.error.message}</Alert>}
      </Stack>
    </section>
  );
}
