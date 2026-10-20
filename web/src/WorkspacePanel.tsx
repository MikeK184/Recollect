import { useEffect, useId, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Check,
  ChevronDown,
  Copy,
  FolderGit2,
  GitBranch,
  Plus,
} from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { RepositoryDialog } from "./PublicationPanel";
import { usePollInterval } from "./usePollInterval";
import { useWorkspaceCatalogue } from "./catalogueQueries";
import { PropList } from "./components/Inspector";
import { Skeleton } from "./components/Skeleton";
import {
  Alert,
  Badge,
  Button,
  Card,
  Code,
  CopyButton,
  Modal,
  MultiSelect,
  Select,
  Separator,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  TextField,
  Tile,
} from "./components/ui";

type Catalogue = components["schemas"]["WorkspaceCatalogue"];
type Task = components["schemas"]["WorkspaceTask"];
type Snapshot = components["schemas"]["ScopeSnapshot"];
type Selection = components["schemas"]["ScopeSelection"];
type Repository = components["schemas"]["Repository"];
const timestamp = (value: string) => new Date(value).toLocaleString();
const empty: Selection = {
  repository_ids: [],
  area_ids: [],
  environment_id: null,
};
function Failure({ error }: { error: Error | null }) {
  return error ? (
    <Alert variant="destructive" title="Request failed">
      {error.message}
    </Alert>
  ) : null;
}
function Loading() {
  return (
    <div className="grid gap-2" role="status">
      <Skeleton height={14} width="40%" />
      <Skeleton height={44} className="rounded-md" />
      <span className="sr-only">Loading…</span>
    </div>
  );
}
function ScopeView({ scope }: { scope: Snapshot }) {
  return (
    <PropList
      items={[
        {
          term: "Repositories",
          value:
            scope.repositories.map((v) => v.name).join(", ") ||
            "All in this Brain",
        },
        {
          term: "Areas",
          value:
            scope.areas.map((v) => v.name).join(", ") || "All in this Brain",
        },
        {
          term: "Environment",
          value: scope.environment?.name || "All in this Brain",
        },
      ]}
    />
  );
}
function Pages({
  offset,
  total,
  size,
  setOffset,
}: {
  offset: number;
  total: number;
  size: number;
  setOffset: (n: number) => void;
}) {
  if (total <= size) return null;
  return (
    <div className="flex items-center gap-2">
      <Button
        variant="secondary"
        size="sm"
        disabled={!offset}
        onClick={() => setOffset(Math.max(0, offset - size))}
      >
        Previous
      </Button>
      <span className="mono mx-auto text-caption text-muted-foreground">
        {offset + 1}–{Math.min(offset + size, total)} of {total}
      </span>
      <Button
        variant="secondary"
        size="sm"
        disabled={offset + size >= total}
        onClick={() => setOffset(offset + size)}
      >
        Next
      </Button>
    </div>
  );
}
/** One compact row: icon tile, name and one detail line, trailing status. */
const rowClass =
  "flex w-full items-center gap-3 rounded-lg border border-border bg-card px-3 py-2.5 text-left transition-[border-color,background-color] duration-fast ease-standard hover:border-border-strong hover:bg-accent focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring";

export function WorkspacePanel({
  brain,
  section = "tasks",
  checkoutOnly = false,
  diagnosticsOnly = false,
  onInspectorChange,
  selectedId,
  onSelectedIdChange,
}: {
  brain: Brain;
  section?: "repositories" | "tasks";
  checkoutOnly?: boolean;
  diagnosticsOnly?: boolean;
  onInspectorChange?: (open: boolean) => void;
  /** Optional shared selection bridge for the Knowledge surface's lineage
   * inspector. Other hosts (Agents) leave it unset. */
  selectedId?: string | null;
  onSelectedIdChange?: (id: string | null) => void;
}) {
  const cache = useQueryClient();
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [tab, setTab] = useState<string>(
    checkoutOnly
      ? "checkouts"
      : section === "repositories"
        ? "repositories"
        : "tasks",
  );
  const [publishing, setPublishing] = useState(false);
  const [checkoutOffset, setCheckoutOffset] = useState(0);
  const [taskOffset, setTaskOffset] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const [form, setForm] = useState<{ task?: Task; parent?: Task } | null>(null);
  const [alias, setAlias] = useState<Repository | null>(null);
  const [publication, setPublication] = useState<Repository | null>(null);
  const [handoff, setHandoff] = useState(false);
  useEffect(() => {
    onInspectorChange?.(
      !!(selected || form || alias || publication || publishing || handoff),
    );
    return () => onInspectorChange?.(false);
  }, [
    selected,
    form,
    alias,
    publication,
    publishing,
    handoff,
    onInspectorChange,
  ]);
  const catalogue = useWorkspaceCatalogue(
    brain.id,
    {
      workspace_id: workspace ?? undefined,
      checkout_offset: checkoutOffset,
      task_offset: taskOffset,
      include_repositories: checkoutOnly ? false : undefined,
    },
    { poll: "normal" },
  );
  const refresh = () => {
    for (const key of [
      "workspace",
      "workspace-task",
      "audit",
      "jobs",
      "processing",
    ])
      void cache.invalidateQueries({ queryKey: [key, brain.id] });
  };
  if (catalogue.error)
    return (
      <Card className="mt-6 grid gap-3 p-6">
        <Failure error={catalogue.error} />
        <div>
          <Button onClick={() => void catalogue.refetch()}>
            Retry workspace
          </Button>
        </div>
      </Card>
    );
  const data = catalogue.data;
  return (
    <section className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <p className="m-0 min-w-60 flex-1 text-body text-muted-foreground">
          {section === "repositories"
            ? "Shared repository identities and published snapshots. Your local checkout paths remain private."
            : "Your agent manages these contexts automatically."}
        </p>
        {section === "repositories" ? (
          <Button variant="secondary" onClick={() => setPublishing(true)}>
            Publish from your host
          </Button>
        ) : !diagnosticsOnly ? (
          <Button
            variant="secondary"
            disabled={brain.archived || !data}
            onClick={() => setForm({})}
          >
            <Plus aria-hidden="true" />
            New task scope
          </Button>
        ) : null}
      </div>
      {!checkoutOnly && (
        <p className="mono m-0 text-caption text-muted-foreground">
          {catalogue.dataUpdatedAt
            ? `Catalogue checked ${new Date(catalogue.dataUpdatedAt).toLocaleTimeString()}`
            : "Loading catalogue…"}
          . Native workspace discovery runs on your connected devices.
        </p>
      )}
      {handoff && (
        <Alert
          role="status"
          title="Fresh context required"
          actions={
            <Button variant="ghost" size="sm" onClick={() => setHandoff(false)}>
              Dismiss
            </Button>
          }
        >
          The new scope applies to future operations. Refresh context before
          continuing this task. Use Recall memory with the new selection.
        </Alert>
      )}
      {catalogue.isPending ? (
        <Loading />
      ) : (
        data && (
          <Tabs value={tab} onValueChange={setTab}>
            {!checkoutOnly && (
              <TabsList>
                {section === "tasks" && (
                  <TabsTrigger value="tasks">Work contexts</TabsTrigger>
                )}
                {section === "repositories" && (
                  <TabsTrigger value="repositories">
                    Published repositories
                  </TabsTrigger>
                )}
                {diagnosticsOnly && (
                  <TabsTrigger value="checkouts">Local folders</TabsTrigger>
                )}
              </TabsList>
            )}
            <TabsContent value="tasks">
              <div className="grid gap-2">
                {data.tasks.length === 0 && (
                  <p className="m-0 text-body text-muted-foreground">
                    No recorded task scopes. Your connected agents manage them
                    automatically.
                  </p>
                )}
                {data.tasks.map((task) => (
                  <button
                    type="button"
                    key={task.id}
                    className={rowClass}
                    data-testid="workspace-task"
                    onClick={() => setSelected(task.id)}
                  >
                    <Tile size="sm">
                      <GitBranch aria-hidden="true" />
                    </Tile>
                    <span className="grid min-w-0 flex-1 gap-0.5">
                      <span className="truncate text-body font-medium text-foreground">
                        {task.label}
                      </span>
                      <span className="truncate text-caption text-muted-foreground">
                        {task.parent_task_id ? "Child context · " : ""}
                        {[
                          task.scope.repositories.length
                            ? task.scope.repositories
                                .map((v) => v.name)
                                .join(", ")
                            : "",
                          task.scope.areas.length
                            ? task.scope.areas.map((v) => v.name).join(", ")
                            : "",
                          task.scope.environment?.name,
                        ]
                          .filter(Boolean)
                          .join(" · ") || "Whole Brain"}
                      </span>
                    </span>
                    <Badge
                      variant={
                        task.closed
                          ? "default"
                          : task.scope_valid
                            ? "brand"
                            : "warning"
                      }
                    >
                      {task.closed
                        ? "Closed"
                        : task.scope_valid
                          ? "Open"
                          : "Scope unavailable"}
                    </Badge>
                  </button>
                ))}
                <Pages
                  offset={taskOffset}
                  total={data.task_total}
                  size={20}
                  setOffset={setTaskOffset}
                />
              </div>
            </TabsContent>
            <TabsContent value="repositories">
              <div className="grid gap-3">
                {data.repositories.length === 0 && (
                  <p className="m-0 text-body text-muted-foreground">
                    No repositories registered. Start your coding host in the
                    workspace; the Recollect plugin discovers its local
                    checkouts.
                  </p>
                )}
                {data.repositories.map((repo) => {
                  const extraOrigins = repo.origins.filter(
                    (origin) => origin !== repo.canonical_origin,
                  );
                  return (
                    <Card
                      key={repo.id}
                      className="grid gap-2 p-4"
                      data-testid="repository-card"
                      data-selected={selectedId === repo.id || undefined}
                    >
                      {/* Dense card: the canonical origin is the primary
                          identifier; extra origins overflow into an explicit
                          expansion and the UUID moves to a copyable field. */}
                      <div className="flex items-start justify-between gap-3">
                        <div className="grid min-w-0 gap-1">
                          <button
                            type="button"
                            className="mono w-fit text-left text-label font-medium text-foreground [overflow-wrap:anywhere] hover:text-brand-text focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                            onClick={() => onSelectedIdChange?.(repo.id)}
                          >
                            {repo.canonical_origin}
                          </button>
                          {extraOrigins.length > 0 && (
                            <details className="group/origins">
                              <summary className="inline-flex cursor-pointer list-none items-center gap-1 text-caption text-muted-foreground hover:text-foreground [&::-webkit-details-marker]:hidden">
                                {extraOrigins.length} more origin
                                {extraOrigins.length === 1 ? "" : "s"}
                                <ChevronDown
                                  aria-hidden="true"
                                  className="size-3 transition-transform duration-fast group-open/origins:rotate-180"
                                />
                              </summary>
                              {extraOrigins.map((origin) => (
                                <p
                                  key={origin}
                                  className="mono m-0 mt-1 text-caption text-muted-foreground [overflow-wrap:anywhere]"
                                >
                                  Also known as {origin}
                                </p>
                              ))}
                            </details>
                          )}
                        </div>
                        <div className="flex shrink-0 gap-1.5">
                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={() => setPublication(repo)}
                          >
                            Snapshots
                          </Button>
                          {brain.role === "admin" && !brain.archived && (
                            <Button
                              size="sm"
                              variant="secondary"
                              onClick={() => setAlias(repo)}
                            >
                              Add origin
                            </Button>
                          )}
                        </div>
                      </div>
                      {/* Machine identifiers stay out of the row; the
                          copyable field offers them instead. */}
                      <details className="group/identity">
                        <summary className="inline-flex cursor-pointer list-none items-center gap-1 text-caption text-muted-foreground hover:text-foreground [&::-webkit-details-marker]:hidden">
                          Repository identifier
                          <ChevronDown
                            aria-hidden="true"
                            className="size-3 transition-transform duration-fast group-open/identity:rotate-180"
                          />
                        </summary>
                        <div className="pt-2">
                          <CopyButton value={repo.id}>
                            {({ copied, copy }) => (
                              <Button
                                size="sm"
                                variant="secondary"
                                onClick={copy}
                              >
                                {copied ? (
                                  <Check
                                    aria-hidden="true"
                                    className="text-brand-text"
                                  />
                                ) : (
                                  <Copy aria-hidden="true" />
                                )}
                                {copied ? "Copied" : "Copy repository ID"}
                              </Button>
                            )}
                          </CopyButton>
                        </div>
                      </details>
                    </Card>
                  );
                })}
              </div>
            </TabsContent>
            <TabsContent value="checkouts">
              <div className="grid gap-3">
                <p className="m-0 text-body text-muted-foreground">
                  Repositories hold shared identities and published evidence.
                  These folders are private observations from your devices;
                  their contents stay local.
                </p>
                {data.workspaces.length === 0 ? (
                  <div className="grid gap-2">
                    <p className="m-0 text-body text-foreground">
                      Place this selector in{" "}
                      <Code>.recollect/workspace.toml</Code> at your workspace
                      root:
                    </p>
                    <Code block>{`brain = "${brain.id}"`}</Code>
                    <p className="m-0 text-body text-foreground">
                      Then start your coding host there — the Recollect plugin
                      reports the workspace automatically.
                    </p>
                  </div>
                ) : (
                  <>
                    <Select
                      label="Reported workspace"
                      value={data.selected_workspace ?? null}
                      searchable
                      data={data.workspaces.map((w) => ({
                        value: w.id,
                        label: `${w.root} · ${w.device_id.slice(0, 8)}`,
                      }))}
                      onChange={(value) => {
                        setWorkspace(value);
                        setCheckoutOffset(0);
                      }}
                    />
                    {data.workspaces
                      .filter((w) => w.id === data.selected_workspace)
                      .map((w) => (
                        <div key={w.id} className="grid gap-1">
                          <div className="flex items-center gap-2">
                            <Badge variant={w.complete ? "brand" : "warning"}>
                              {w.complete ? "Full refresh" : "Partial refresh"}
                            </Badge>
                            <span className="mono text-caption text-muted-foreground">
                              {timestamp(w.refreshed_at)}
                            </span>
                          </div>
                          {w.notes.map((note, index) => (
                            <p
                              key={index}
                              className="m-0 text-caption text-muted-foreground"
                            >
                              {note}
                            </p>
                          ))}
                        </div>
                      ))}
                    {data.checkouts.length === 0 && (
                      <p className="m-0 text-body text-muted-foreground">
                        No checkouts were reported for this workspace.
                      </p>
                    )}
                    {data.checkouts.map((c) => (
                      <details
                        key={c.id}
                        className="group/checkout rounded-lg border border-border bg-card"
                      >
                        <summary className="flex cursor-pointer list-none items-center gap-3 px-3 py-2.5 [&::-webkit-details-marker]:hidden">
                          <Tile size="sm">
                            <FolderGit2 aria-hidden="true" />
                          </Tile>
                          <span className="grid min-w-0 flex-1 gap-0.5">
                            <span className="truncate text-body font-medium text-foreground">
                              {c.observation.local_path
                                .split(/[\\/]/)
                                .filter(Boolean)
                                .at(-1) || c.observation.local_path}
                            </span>
                            <span className="mono truncate text-caption text-muted-foreground">
                              {c.observation.origin || "Origin not reported"}
                            </span>
                          </span>
                          <span className="grid shrink-0 justify-items-end gap-0.5">
                            <Badge
                              variant={
                                !c.present ||
                                c.observation.status === "git_unavailable"
                                  ? "warning"
                                  : "default"
                              }
                            >
                              {!c.present
                                ? "Not seen"
                                : c.observation.status.replaceAll("_", " ")}
                            </Badge>
                            <span className="mono text-caption text-muted-foreground">
                              Observed {timestamp(c.observed_at)}
                            </span>
                          </span>
                        </summary>
                        <div className="grid gap-2 border-t border-border p-3">
                          <Code block>{c.observation.local_path}</Code>
                          <p className="m-0 text-caption text-muted-foreground">
                            {c.observation.branch || "No branch reported"} ·{" "}
                            {c.observation.head
                              ? `HEAD ${c.observation.head.slice(0, 12)}`
                              : "No committed revision reported"}{" "}
                            ·{" "}
                            {c.observation.dirty === true
                              ? "Uncommitted changes"
                              : c.observation.dirty === false
                                ? "Clean when observed"
                                : "Working tree state unknown"}
                          </p>
                        </div>
                      </details>
                    ))}
                    <Pages
                      offset={checkoutOffset}
                      total={data.checkout_total}
                      size={100}
                      setOffset={setCheckoutOffset}
                    />
                  </>
                )}
              </div>
            </TabsContent>
          </Tabs>
        )
      )}
      <Modal
        opened={publishing}
        onClose={() => setPublishing(false)}
        title="Publish from your host"
        size="lg"
      >
        <div className="grid gap-3">
          <p className="m-0 text-body text-foreground">
            Ask your agent, from a task scoped to this repository, to publish
            the checkout. The Recollect plugin runs the publication for it.
            Recollect publishes the selected committed tree, not uncommitted
            working files.
          </p>
          <p className="m-0 text-label text-muted-foreground">
            Replace the identifiers with your repository and task scope. File
            text stays local unless explicitly selected with --retain-file and
            permitted in Settings. This browser cannot scan your checkout.
          </p>
        </div>
      </Modal>
      {data && form && (
        <TaskForm
          brain={brain}
          catalogue={data}
          task={form.task}
          parent={form.parent}
          onClose={() => setForm(null)}
          onSaved={(id) => {
            setSelected(id);
            setForm(null);
            setHandoff(true);
            refresh();
          }}
        />
      )}
      {data && selected && !form && (
        <TaskDialog
          brain={brain}
          id={selected}
          onClose={() => setSelected(null)}
          onChanged={refresh}
          onEdit={(task) => setForm({ task })}
          diagnosticsOnly={diagnosticsOnly}
          onFork={(parent) => setForm({ parent })}
        />
      )}
      {publication && (
        <RepositoryDialog
          brain={brain}
          repository={publication}
          onClose={() => setPublication(null)}
        />
      )}
      {alias && (
        <AliasDialog
          brain={brain}
          repository={alias}
          onClose={() => setAlias(null)}
          onSaved={() => {
            setAlias(null);
            refresh();
          }}
        />
      )}
    </section>
  );
}

function TaskForm({
  brain,
  catalogue,
  task,
  parent,
  onClose,
  onSaved,
}: {
  brain: Brain;
  catalogue: Catalogue;
  task?: Task;
  parent?: Task;
  onClose: () => void;
  onSaved: (id: string) => void;
}) {
  const formId = useId();
  const scope = task?.scope ?? parent?.scope;
  const initial = scope?.selection ?? empty;
  const [label, setLabel] = useState("");
  const [repositories, setRepositories] = useState(
    initial.repository_ids ?? [],
  );
  const [areas, setAreas] = useState(initial.area_ids ?? []);
  const [environment, setEnvironment] = useState<string | null>(
    initial.environment_id ?? null,
  );
  const [workspace, setWorkspace] = useState<string | null>(
    parent?.workspace_id ?? null,
  );
  const choices = (
    available: { id: string; name: string }[],
    previous: { id: string; name: string }[] = [],
  ) =>
    [
      ...available,
      ...previous
        .filter((p) => !available.some((a) => a.id === p.id))
        .map((p) => ({ id: p.id, name: `${p.name} (unavailable)` })),
    ].map((v) => ({ value: v.id, label: v.name }));
  const save = useMutation({
    mutationFn: async () => {
      const selection = {
        repository_ids: repositories,
        area_ids: areas,
        environment_id: environment,
      };
      if (task)
        return result(
          await client.PUT("/api/brains/{brain}/workspace/tasks/{task}/scope", {
            params: { path: { brain: brain.id, task: task.id } },
            body: { base_scope: task.scope.id, selection },
          }),
        );
      return result(
        await client.POST("/api/brains/{brain}/workspace/tasks", {
          params: { path: { brain: brain.id } },
          body: {
            label,
            parent_task_id: parent?.id,
            workspace_id: workspace,
            selection,
          },
        }),
      );
    },
    onSuccess: (value) => onSaved(value.task.id),
  });
  return (
    <Modal
      opened
      onClose={onClose}
      title={
        task
          ? "Change task scope"
          : parent
            ? "Create child scope"
            : "Start task"
      }
      size="lg"
      footer={
        <Button type="submit" form={formId} loading={save.isPending}>
          {task ? "Save scope" : "Start task"}
        </Button>
      }
    >
      <form
        id={formId}
        className="grid gap-4"
        onSubmit={(event) => {
          event.preventDefault();
          save.mutate();
        }}
      >
        {task ? (
          <p className="m-0 text-title font-semibold text-foreground">
            {task.label}
          </p>
        ) : (
          <TextField
            required
            label="Task name"
            value={label}
            onChange={(event) => setLabel(event.currentTarget.value)}
            maxLength={120}
          />
        )}
        {parent && (
          <p className="m-0 text-body text-foreground-secondary">
            Parent: {parent.label}. This child keeps its own scope.
          </p>
        )}
        {!task && (
          <Select
            label="Workspace registration (optional)"
            data={catalogue.workspaces.map((w) => ({
              value: w.id,
              label: `${w.root} · ${w.device_id.slice(0, 8)}`,
            }))}
            value={workspace}
            onChange={setWorkspace}
            clearable
            searchable
          />
        )}
        <MultiSelect
          inputClassName="relative"
          label="Repositories"
          placeholder="All repositories in this Brain"
          searchable
          clearable
          data={choices(
            catalogue.repositories.map((r) => ({
              id: r.id,
              name: r.canonical_origin,
            })),
            scope?.repositories,
          )}
          value={repositories}
          onChange={setRepositories}
        />
        <MultiSelect
          inputClassName="relative"
          label="Areas"
          placeholder="All areas in this Brain"
          searchable
          clearable
          data={choices(catalogue.areas, scope?.areas)}
          value={areas}
          onChange={setAreas}
        />
        <Select
          label="Environment"
          placeholder="All environments in this Brain"
          searchable
          clearable
          data={choices(
            catalogue.environments,
            scope?.environment ? [scope.environment] : [],
          )}
          value={environment}
          onChange={setEnvironment}
        />
        <p className="m-0 text-caption text-muted-foreground">
          Leaving a dimension empty includes the whole Brain for that dimension.
          Existing operations and other tasks keep their recorded scopes.
        </p>
        <Failure error={save.error} />
      </form>
    </Modal>
  );
}

function TaskDialog({
  brain,
  id,
  onClose,
  onChanged,
  onEdit,
  onFork,
  diagnosticsOnly = false,
}: {
  brain: Brain;
  id: string;
  onClose: () => void;
  onChanged: () => void;
  onEdit: (task: Task) => void;
  onFork: (task: Task) => void;
  diagnosticsOnly?: boolean;
}) {
  const pollNormal = usePollInterval("normal");
  const [scopeOffset, setScopeOffset] = useState(0);
  const [operationOffset, setOperationOffset] = useState(0);
  const [purpose, setPurpose] = useState<string | null>("context");
  const [closing, setClosing] = useState(false);
  const [inspect, setInspect] = useState<Snapshot | null>(null);
  const details = useQuery({
    queryKey: ["workspace-task", brain.id, id, scopeOffset, operationOffset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/workspace/tasks/{task}", {
          params: {
            path: { brain: brain.id, task: id },
            query: {
              scope_offset: scopeOffset,
              operation_offset: operationOffset,
            },
          },
        }),
      ),
    refetchInterval: pollNormal,
  });
  const begin = useMutation({
    mutationFn: async () =>
      result(
        await client.POST(
          "/api/brains/{brain}/workspace/tasks/{task}/operations",
          {
            params: { path: { brain: brain.id, task: id } },
            body: { kind: purpose ?? "context" },
          },
        ),
      ),
    onSuccess: () => {
      setOperationOffset(0);
      onChanged();
    },
  });
  const close = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/brains/{brain}/workspace/tasks/{task}/close", {
          params: { path: { brain: brain.id, task: id } },
        }),
      ),
    onSuccess: () => {
      setClosing(false);
      onChanged();
    },
  });
  const data = details.error ? undefined : details.data;
  return (
    <Modal opened onClose={onClose} title="Task context" size="lg">
      <div className="grid gap-4">
        <Failure error={details.error} />
        {details.error && (
          <div>
            <Button variant="secondary" onClick={() => void details.refetch()}>
              Retry task
            </Button>
          </div>
        )}
        {details.isPending && <Loading />}
        {data && (
          <>
            <div className="flex items-center justify-between gap-3">
              <h3 className="m-0 min-w-0 text-section font-semibold text-foreground [overflow-wrap:anywhere]">
                {data.task.label}
              </h3>
              <Badge variant={data.task.closed ? "default" : "brand"}>
                {data.task.closed ? "Closed" : "Open"}
              </Badge>
            </div>
            <div className="mono grid gap-0.5 text-caption text-muted-foreground [overflow-wrap:anywhere]">
              <span>Task {data.task.id}</span>
              {data.task.parent_task_id && (
                <span>Parent {data.task.parent_task_id}</span>
              )}
            </div>
            {!data.task.scope_valid && (
              <Alert variant="warning" title="Scope unavailable">
                A selected view was removed. Change this task’s scope before
                starting another operation.
              </Alert>
            )}
            <ScopeView scope={data.task.scope} />
            <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
              Current scope {data.task.scope.id}
            </p>
            {!data.task.closed && !brain.archived && (
              <div className="flex flex-wrap gap-2">
                <Button variant="secondary" onClick={() => onEdit(data.task)}>
                  Change selection
                </Button>
                {!diagnosticsOnly && (
                  <Button variant="secondary" onClick={() => onFork(data.task)}>
                    <GitBranch aria-hidden="true" />
                    Create child scope
                  </Button>
                )}
                <Button variant="ghost" onClick={() => setClosing(true)}>
                  Close task
                </Button>
              </div>
            )}
            {closing && (
              <Alert
                variant="warning"
                title="Close this task?"
                actions={
                  <>
                    <Button
                      size="sm"
                      onClick={() => close.mutate()}
                      loading={close.isPending}
                    >
                      Confirm close
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => setClosing(false)}
                    >
                      Keep open
                    </Button>
                  </>
                }
              >
                <p>
                  New operations will stop. History and child tasks remain
                  available.
                </p>
                <Failure error={close.error} />
              </Alert>
            )}
            <Separator />
            {!diagnosticsOnly && !data.task.closed && !brain.archived && (
              <div className="grid gap-2">
                <Select
                  label="Operation purpose"
                  value={purpose}
                  onChange={setPurpose}
                  data={[
                    { value: "context", label: "Context" },
                    { value: "retrieval", label: "Retrieval" },
                    ...(brain.role !== "reader"
                      ? [
                          { value: "write", label: "Write" },
                          { value: "capture", label: "Capture" },
                        ]
                      : []),
                    { value: "tool", label: "Tool" },
                  ]}
                />
                <p className="m-0 text-caption text-muted-foreground">
                  Record a fixed scope for a later operation. A binding does not
                  execute a tool or retrieve evidence.
                </p>
                <div>
                  <Button
                    variant="secondary"
                    disabled={!data.task.scope_valid}
                    loading={begin.isPending}
                    onClick={() => begin.mutate()}
                  >
                    Bind operation
                  </Button>
                </div>
                <Failure error={begin.error} />
              </div>
            )}
            <div className="grid gap-2">
              <h4 className="section-label m-0">Recorded operations</h4>
              {data.operations.length === 0 && (
                <p className="m-0 text-body text-muted-foreground">
                  No operation bindings yet.
                </p>
              )}
              {data.operations.map((op) => (
                <Card
                  key={op.id}
                  className="grid gap-1 p-3"
                  data-testid="operation-binding"
                >
                  <div className="flex items-center justify-between gap-3">
                    <p className="m-0 text-body font-medium capitalize text-foreground">
                      {op.kind} binding
                    </p>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => setInspect(op.scope)}
                    >
                      Inspect scope
                    </Button>
                  </div>
                  <p className="mono m-0 text-caption text-muted-foreground">
                    {timestamp(op.created_at)} ·{" "}
                    {op.scope.id === data.task.scope.id
                      ? "Current scope"
                      : "Earlier scope"}
                  </p>
                  <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
                    {op.id}
                  </p>
                </Card>
              ))}
              <Pages
                offset={operationOffset}
                total={data.operation_total}
                size={20}
                setOffset={setOperationOffset}
              />
            </div>
            <Separator />
            <div className="grid gap-2">
              <h4 className="section-label m-0">Scope history</h4>
              {data.scopes.map((scope) => (
                <div
                  key={scope.id}
                  className="flex items-center justify-between gap-3"
                >
                  <span className="mono text-caption text-foreground-secondary">
                    {timestamp(scope.created_at)} ·{" "}
                    {scope.id === data.task.scope.id ? "Current" : "Earlier"}
                  </span>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setInspect(scope)}
                  >
                    View scope
                  </Button>
                </div>
              ))}
              <Pages
                offset={scopeOffset}
                total={data.scope_total}
                size={20}
                setOffset={setScopeOffset}
              />
            </div>
            {inspect && (
              <Card className="grid gap-3">
                <div className="flex items-center justify-between gap-3">
                  <p className="m-0 text-body font-semibold text-foreground">
                    Recorded scope
                  </p>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setInspect(null)}
                  >
                    Hide
                  </Button>
                </div>
                <ScopeView scope={inspect} />
                <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
                  {inspect.id}
                </p>
              </Card>
            )}
          </>
        )}
      </div>
    </Modal>
  );
}

export function AliasDialog({
  brain,
  repository,
  onClose,
  onSaved,
}: {
  brain: Brain;
  repository: Repository;
  onClose: () => void;
  onSaved: () => void;
}) {
  const formId = useId();
  const [origin, setOrigin] = useState("");
  const save = useMutation({
    mutationFn: async () =>
      result(
        await client.POST(
          "/api/brains/{brain}/workspace/repositories/{repository}/origins",
          {
            params: { path: { brain: brain.id, repository: repository.id } },
            body: { origin },
          },
        ),
      ),
    onSuccess: onSaved,
  });
  return (
    <Modal
      opened
      onClose={onClose}
      title="Add repository origin"
      footer={
        <Button type="submit" form={formId} loading={save.isPending}>
          Attach origin
        </Button>
      }
    >
      <form
        id={formId}
        className="grid gap-4"
        onSubmit={(event) => {
          event.preventDefault();
          save.mutate();
        }}
      >
        <p className="m-0 text-body text-foreground-secondary [overflow-wrap:anywhere]">
          Attach an alias or moved origin to{" "}
          <span className="mono text-label text-foreground">
            {repository.canonical_origin}
          </span>
          .
        </p>
        <TextField
          label="Repository origin"
          required
          inputClassName="mono"
          value={origin}
          onChange={(event) => setOrigin(event.currentTarget.value)}
          maxLength={2000}
        />
        <p className="m-0 text-caption text-muted-foreground">
          Use this only when both origins identify the same repository. Its UUID
          and recorded scopes are preserved.
        </p>
        <Failure error={save.error} />
      </form>
    </Modal>
  );
}
