import { useState } from "react";
import { EraseAction, ExcerptAction } from "./RetentionPanel";
import { LearningAction } from "./ModelsPanel";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Divider,
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
  Upload,
} from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";

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
  const [filters, setFilters] = useState<Record<string, string | null>>({
    collection: null,
    area: null,
    environment: null,
  });
  const [offset, setOffset] = useState(0);
  const [manage, setManage] = useState(false);
  const [importing, setImporting] = useState(false);
  const [selected, setSelected] = useState<Source | null>(null);
  const [editing, setEditing] = useState<{
    source: Source;
    content: Content;
  } | null>(null);
  const writable = brain.role !== "reader" && !brain.archived;
  const catalogue = useQuery({
    queryKey: ["evidence", brain.id, filters, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          params: {
            path: { brain: brain.id },
            query: {
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
    setOffset(0);
  };
  const policy = useMutation({
    mutationFn: async (value: boolean) =>
      result(
        await client.PUT("/api/brains/{brain}/evidence/policy", {
          params: { path: { brain: brain.id } },
          body: { allow_document_content: value },
        }),
      ),
    onSuccess: (policy) => {
      cache.setQueriesData<Catalogue>(
        { queryKey: ["evidence", brain.id] },
        (current) => (current ? { ...current, policy } : current),
      );
      refresh();
    },
  });
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
    <Card withBorder p="xl" className="evidence-panel">
      <Stack gap="lg">
        <Group justify="space-between">
          <Group gap="sm">
            <BookOpen size={21} />
            <Title order={2} fz={21}>
              Knowledge sources
            </Title>
          </Group>
          {writable && (
            <Button
              size="xs"
              leftSection={<Plus size={14} />}
              disabled={catalogue.isPending || policy.isPending}
              onClick={() => setImporting(true)}
            >
              Import source
            </Button>
          )}
        </Group>
        <Text size="sm" c="dimmed">
          Keep the original evidence, follow its history and organize it across
          your Brain.
        </Text>
        <div className="evidence-filters">
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
              size="xs"
            />
          ))}
        </div>
        <Group justify="space-between">
          <Text size="xs" c="dimmed">
            {catalogue.data?.total ?? 0} sources ·{" "}
            {Object.values(filters).some(Boolean)
              ? "Matching all selected views"
              : "All sources"}
          </Text>
          {(writable || brain.role === "admin") && (
            <Button
              size="compact-xs"
              variant="subtle"
              leftSection={<Settings2 size={13} />}
              disabled={catalogue.isPending}
              onClick={() => setManage(true)}
            >
              Manage views
            </Button>
          )}
        </Group>
        {catalogue.isPending ? (
          <Loader />
        ) : !catalogue.data?.sources.length ? (
          <div className="evidence-empty">
            <FolderOpen size={28} />
            <Text fw={600}>No sources in this view</Text>
            <Text size="sm" c="dimmed" ta="center">
              {Object.values(filters).some(Boolean)
                ? "Change a filter or organize a source into this view."
                : "Import a document or keep a reference to a controlled source."}
            </Text>
          </div>
        ) : (
          <Stack gap="sm">
            {catalogue.data.sources.map((source) => (
              <button
                key={source.id}
                className="source-row"
                onClick={() => setSelected(source)}
              >
                <FileText size={20} className="source-symbol" />
                <div className="source-row-body">
                  <Text fw={600} size="sm">
                    {source.version.title}
                  </Text>
                  <Text size="xs" c="dimmed" mt={5}>
                    {source.version.contributor} ·{" "}
                    {timestamp(source.version.recorded_at)}
                  </Text>
                  <Group gap={5} mt={7}>
                    {source.group_ids.slice(0, 5).map((id) => (
                      <Badge size="xs" variant="light" color="gray" key={id}>
                        {groups.find((g) => g.id === id)?.name ?? "View"}
                      </Badge>
                    ))}
                  </Group>
                </div>
                <Badge
                  size="xs"
                  color={
                    source.version.processing === "ready" &&
                    source.version.availability === "retained"
                      ? "teal"
                      : "gray"
                  }
                >
                  {stateLabel(source.version)}
                </Badge>
              </button>
            ))}
          </Stack>
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
        {brain.role === "admin" && (
          <>
            <Divider />
            <Switch
              label="Allow document content retention"
              description="When off, new imports keep references only. Existing evidence is preserved."
              checked={
                policy.isPending
                  ? policy.variables
                  : (catalogue.data?.policy.allow_document_content ?? true)
              }
              disabled={
                brain.archived || policy.isPending || catalogue.isPending
              }
              onChange={(event) => policy.mutate(event.currentTarget.checked)}
            />
            {policy.error && <Alert color="red">{policy.error.message}</Alert>}
          </>
        )}
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
      {selected && (
        <SourceViewer
          key={selected.id}
          brain={brain.id}
          source={
            catalogue.data?.sources.find((s) => s.id === selected.id) ??
            selected
          }
          views={groups}
          writable={writable}
          admin={brain.role === "admin"}
          close={() => setSelected(null)}
          refresh={refresh}
          edit={(content) => {
            setEditing({
              source:
                catalogue.data?.sources.find((s) => s.id === selected.id) ??
                selected,
              content,
            });
            setSelected(null);
          }}
        />
      )}
    </Card>
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
                    {view.source_count} sources
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
                      leftSection={<Upload size={15} />}
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
                styles={{ input: { fontFamily: "monospace", fontSize: 12 } }}
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
  views,
  writable,
  admin,
  close,
  refresh,
  edit,
}: {
  brain: string;
  source: Source;
  views: View[];
  writable: boolean;
  admin: boolean;
  close: () => void;
  refresh: () => void;
  edit: (content: Content) => void;
}) {
  const [version, setVersion] = useState(source.version.id);
  const [offset, setOffset] = useState(0);
  const [organizing, setOrganizing] = useState(false);
  const [groups, setGroups] = useState(source.group_ids);
  const history = useQuery({
    queryKey: ["source-history", brain, source.id, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/sources/{source}/versions", {
          params: { path: { brain, source: source.id }, query: { offset } },
        }),
      ),
    refetchInterval: 4000,
  });
  const evidence = useQuery({
    queryKey: ["source-content", brain, source.id, version],
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/sources/{source}/versions/{version}",
          { params: { path: { brain, source: source.id, version } } },
        ),
      ),
    refetchInterval: 4000,
  });
  const organize = useMutation({
    mutationFn: async () =>
      result(
        await client.PUT("/api/brains/{brain}/sources/{source}/groups", {
          params: { path: { brain, source: source.id } },
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
          params: { path: { brain, source: source.id } },
        }),
      ),
    onSuccess: refresh,
  });
  const data = evidence.data;
  const error = evidence.error ?? history.error;
  return (
    <Modal opened onClose={close} title="Source evidence" size="xl" centered>
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
                  <Title order={2} fz={24}>
                    {data.version.title}
                  </Title>
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
              <EraseAction
                brain={brain}
                permitted={admin}
                target={{ kind: "source", id: source.id }}
                name={data.version.title}
              />
              {data.version.excerpt && (
                <Text size="xs" c="dimmed">
                  Supporting excerpt from source lines{" "}
                  {data.version.excerpt.first_line}–
                  {data.version.excerpt.last_line}, captured{" "}
                  {timestamp(data.version.excerpt.captured_at)}.
                </Text>
              )}
              {writable &&
                data.version.privacy_state === "active" &&
                data.content != null && (
                  <ExcerptAction
                    brain={brain}
                    source={source.id}
                    version={data.version.id}
                    title={data.version.title}
                  />
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
              <Select
                label="Evidence version"
                data={(history.data?.versions ?? []).map((v) => ({
                  value: v.id,
                  label: `${v.id === source.version.id ? "Current · " : "Earlier · "}${timestamp(v.recorded_at)} · ${v.contributor}`,
                }))}
                value={version}
                onChange={(v) => v && setVersion(v)}
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
              {version !== source.version.id && (
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
                  <LearningAction
                    brain={brain}
                    version={version}
                    writable={
                      writable &&
                      (data.version.privacy_state ?? "active") === "active"
                    }
                  />
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
              {writable && (
                <Group>
                  <Button
                    size="sm"
                    variant="default"
                    disabled={
                      version !== source.version.id ||
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
                  {data.version.retained && version === source.version.id && (
                    <Button
                      size="sm"
                      variant="subtle"
                      leftSection={<RefreshCw size={14} />}
                      loading={process.isPending}
                      onClick={() => process.mutate()}
                    >
                      Reprocess source
                    </Button>
                  )}
                </Group>
              )}
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
    </Modal>
  );
}
