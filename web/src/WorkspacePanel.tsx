import { iconSize } from "./design/tokens";
import { useState } from "react";
import "./features/feature-views.css";
import {
  Alert,
  Badge,
  Button,
  Card,
  Code,
  Divider,
  Group,
  Loader,
  Modal,
  MultiSelect,
  Select,
  Stack,
  Tabs,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FolderGit2, GitBranch, Plus, RefreshCw } from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { RepositoryDialog } from "./PublicationPanel";
import { McpAgentSetup } from "./McpAgentSetup";

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
    <Alert color="red" title="Request failed">
      {error.message}
    </Alert>
  ) : null;
}
function ScopeView({ scope }: { scope: Snapshot }) {
  return (
    <Stack gap="xs" className="scope-view">
      <Text size="sm">
        <strong>Repositories:</strong>{" "}
        {scope.repositories.map((v) => v.name).join(", ") ||
          "All in this Brain"}
      </Text>
      <Text size="sm">
        <strong>Areas:</strong>{" "}
        {scope.areas.map((v) => v.name).join(", ") || "All in this Brain"}
      </Text>
      <Text size="sm">
        <strong>Environment:</strong>{" "}
        {scope.environment?.name || "All in this Brain"}
      </Text>
    </Stack>
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
    <Group justify="space-between">
      <Button
        variant="subtle"
        disabled={!offset}
        onClick={() => setOffset(Math.max(0, offset - size))}
      >
        Previous
      </Button>
      <Text size="xs">
        {offset + 1}–{Math.min(offset + size, total)} of {total}
      </Text>
      <Button
        variant="subtle"
        disabled={offset + size >= total}
        onClick={() => setOffset(offset + size)}
      >
        Next
      </Button>
    </Group>
  );
}

export function WorkspacePanel({
  brain,
  section = "tasks",
}: {
  brain: Brain;
  section?: "repositories" | "tasks";
}) {
  const cache = useQueryClient();
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [tab, setTab] = useState<string | null>(
    section === "repositories" ? "repositories" : "tasks",
  );
  const [publishing, setPublishing] = useState(false);
  const [checkoutOffset, setCheckoutOffset] = useState(0);
  const [taskOffset, setTaskOffset] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const [form, setForm] = useState<{ task?: Task; parent?: Task } | null>(null);
  const [alias, setAlias] = useState<Repository | null>(null);
  const [publication, setPublication] = useState<Repository | null>(null);
  const [handoff, setHandoff] = useState(false);
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id, workspace, checkoutOffset, taskOffset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: {
            path: { brain: brain.id },
            query: {
              workspace_id: workspace ?? undefined,
              checkout_offset: checkoutOffset,
              task_offset: taskOffset,
            },
          },
        }),
      ),
    refetchInterval: 4000,
  });
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
      <Card withBorder p="xl" mt="xl">
        <Failure error={catalogue.error} />
        <Button mt="md" onClick={() => void catalogue.refetch()}>
          Retry workspace
        </Button>
      </Card>
    );
  const data = catalogue.data;
  return (
    <section className="feature-view workspace-panel">
      <div className="feature-toolbar">
        <div className="feature-search">
          <Text size="sm" c="dimmed">
            {section === "repositories"
              ? "Shared repository identities and published snapshots. Your local checkout paths remain private."
              : "Your agent manages these working scopes automatically. Inspect or adjust your own context here when needed."}
          </Text>
        </div>
        {section === "repositories" ? (
          <Button variant="default" onClick={() => setPublishing(true)}>
            Publish from companion
          </Button>
        ) : (
          <Button
            variant="default"
            leftSection={<Plus size={iconSize.small} />}
            disabled={brain.archived || !data}
            onClick={() => setForm({})}
          >
            New task scope
          </Button>
        )}
      </div>
      <Text size="xs" c="dimmed" mb="md">
        {catalogue.dataUpdatedAt
          ? `Catalogue checked ${new Date(catalogue.dataUpdatedAt).toLocaleTimeString()}`
          : "Loading catalogue…"}
        . Native workspace discovery runs on your paired device.
      </Text>
      {handoff && (
        <Alert
          mb="lg"
          withCloseButton
          onClose={() => setHandoff(false)}
          title="Fresh context required"
        >
          The new scope applies to future operations. Refresh context before
          continuing this task. Use Recall memory with the new selection.
        </Alert>
      )}
      {catalogue.isPending ? (
        <Loader size="sm" />
      ) : (
        data && (
          <Tabs value={tab} onChange={setTab} keepMounted={false}>
            <Tabs.List>
              {section === "tasks" && (
                <Tabs.Tab value="tasks">Your task scopes</Tabs.Tab>
              )}
              {section === "repositories" && (
                <Tabs.Tab value="repositories">Published repositories</Tabs.Tab>
              )}
              <Tabs.Tab value="checkouts">Your checkouts</Tabs.Tab>
            </Tabs.List>
            <Tabs.Panel value="tasks" pt="lg">
              <Stack gap="sm">
                {data.tasks.length === 0 && (
                  <Text size="sm" c="dimmed">
                    No task scopes yet. Connected agents create and maintain
                    their own scopes. Manual setup is available when you need
                    it.
                  </Text>
                )}
                {data.tasks.map((task) => (
                  <Card
                    key={task.id}
                    withBorder
                    p="md"
                    className="workspace-task-card"
                    data-testid="workspace-task"
                  >
                    <Group justify="space-between">
                      <Button
                        variant="subtle"
                        px={0}
                        onClick={() => setSelected(task.id)}
                      >
                        {task.label}
                      </Button>
                      <Badge
                        color={
                          task.closed
                            ? "gray"
                            : task.scope_valid
                              ? "teal"
                              : "orange"
                        }
                      >
                        {task.closed
                          ? "Closed"
                          : task.scope_valid
                            ? "Active"
                            : "Scope unavailable"}
                      </Badge>
                    </Group>
                    {task.parent_task_id && (
                      <Text size="xs" c="dimmed" mb="xs">
                        Independent subagent task
                      </Text>
                    )}
                    <ScopeView scope={task.scope} />
                  </Card>
                ))}
                <Pages
                  offset={taskOffset}
                  total={data.task_total}
                  size={20}
                  setOffset={setTaskOffset}
                />
              </Stack>
            </Tabs.Panel>
            <Tabs.Panel value="repositories" pt="lg">
              <Stack gap="md">
                {data.repositories.length === 0 && (
                  <Text size="sm" c="dimmed">
                    No repositories registered. Refresh a workspace from your
                    paired companion to discover its local checkouts.
                  </Text>
                )}
                {data.repositories.map((repo) => (
                  <Card key={repo.id} withBorder p="md">
                    <Group justify="space-between" align="start">
                      <div className="workspace-repository">
                        <Text fw={500} size="sm">
                          {repo.canonical_origin}
                        </Text>
                        <Text size="xs" c="dimmed" mt="xs">
                          Repository {repo.id}
                        </Text>
                        {repo.origins
                          .filter((origin) => origin !== repo.canonical_origin)
                          .map((origin) => (
                            <Text key={origin} size="xs" mt="xs">
                              Also known as {origin}
                            </Text>
                          ))}
                      </div>
                      <Group gap="xs">
                        <Button
                          size="xs"
                          variant="light"
                          onClick={() => setPublication(repo)}
                        >
                          Snapshots
                        </Button>
                        {brain.role === "admin" && !brain.archived && (
                          <Button
                            size="xs"
                            variant="light"
                            onClick={() => setAlias(repo)}
                          >
                            Add origin
                          </Button>
                        )}
                      </Group>
                    </Group>
                  </Card>
                ))}
              </Stack>
            </Tabs.Panel>
            <Tabs.Panel value="checkouts" pt="lg">
              <Stack gap="md">
                <Text size="sm" c="dimmed">
                  These are your devices’ last reported checkout observations.
                  Repository contents stay on those devices.
                </Text>
                {data.workspaces.length === 0 ? (
                  <Stack gap="xs">
                    <Text size="sm">
                      Place this selector in{" "}
                      <Code>.recollect/workspace.toml</Code> at your workspace
                      root:
                    </Text>
                    <Code block>{`brain = "${brain.id}"`}</Code>
                    <Text size="sm">Then use your paired companion:</Text>
                    <Code block>
                      recollect-agent workspace refresh /path/to/workspace
                    </Code>
                  </Stack>
                ) : (
                  <>
                    <Select
                      label="Registered workspace"
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
                        <div key={w.id}>
                          <Group mb="xs">
                            <Badge color={w.complete ? "teal" : "yellow"}>
                              {w.complete ? "Full refresh" : "Partial refresh"}
                            </Badge>
                            <Text size="xs" c="dimmed">
                              {timestamp(w.refreshed_at)}
                            </Text>
                          </Group>
                          {w.notes.map((note, index) => (
                            <Text key={index} size="xs" c="dimmed">
                              {note}
                            </Text>
                          ))}
                        </div>
                      ))}
                    {data.checkouts.length === 0 && (
                      <Text size="sm" c="dimmed">
                        No checkouts were reported for this workspace.
                      </Text>
                    )}
                    {data.checkouts.map((c) => (
                      <Card key={c.id} withBorder p="md">
                        <Stack gap="xs">
                          <Group justify="space-between">
                            <Text className="workspace-path" size="sm" fw={500}>
                              {c.observation.local_path}
                            </Text>
                            <Badge
                              color={
                                !c.present ||
                                c.observation.status === "git_unavailable"
                                  ? "yellow"
                                  : "gray"
                              }
                            >
                              {!c.present
                                ? "Not seen"
                                : c.observation.status.replaceAll("_", " ")}
                            </Badge>
                          </Group>
                          <Text size="xs">
                            {c.observation.origin ||
                              "Repository origin unavailable"}
                          </Text>
                          <Text size="xs" c="dimmed">
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
                          </Text>
                          <Text size="xs" c="dimmed">
                            Observed {timestamp(c.observed_at)}
                          </Text>
                        </Stack>
                      </Card>
                    ))}
                    <Pages
                      offset={checkoutOffset}
                      total={data.checkout_total}
                      size={100}
                      setOffset={setCheckoutOffset}
                    />
                  </>
                )}
              </Stack>
            </Tabs.Panel>
          </Tabs>
        )
      )}
      <Modal
        opened={publishing}
        onClose={() => setPublishing(false)}
        title="Publish from your companion"
        size="lg"
      >
        <Stack>
          <Text>
            Use the paired native companion from a task scoped to the
            repository. Recollect publishes the selected committed tree, not
            uncommitted working files.
          </Text>
          <Code
            block
          >{`recollect-agent repository publish ${brain.id} REPOSITORY_ID TASK_ID /path/to/checkout --revision COMMIT`}</Code>
          <Text size="sm" c="dimmed">
            Replace the identifiers with your repository and task scope. File
            text stays local unless explicitly selected with --retain-file and
            permitted in Settings. This browser cannot scan your checkout.
          </Text>
        </Stack>
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
            ? "Start subagent task"
            : "Start task"
      }
      size="lg"
      centered
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          save.mutate();
        }}
      >
        <Stack>
          {task ? (
            <Text fw={600}>{task.label}</Text>
          ) : (
            <TextInput
              required
              label="Task name"
              value={label}
              onChange={(event) => setLabel(event.currentTarget.value)}
              maxLength={120}
            />
          )}
          {parent && (
            <Text size="sm">
              Parent: {parent.label}. This child keeps its own scope.
            </Text>
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
          <Text size="xs" c="dimmed">
            Leaving a dimension empty includes the whole Brain for that
            dimension. Existing operations and other tasks keep their recorded
            scopes.
          </Text>
          <Failure error={save.error} />
          <Button type="submit" loading={save.isPending}>
            {task ? "Save scope" : "Start task"}
          </Button>
        </Stack>
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
}: {
  brain: Brain;
  id: string;
  onClose: () => void;
  onChanged: () => void;
  onEdit: (task: Task) => void;
  onFork: (task: Task) => void;
}) {
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
    refetchInterval: 4000,
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
    <Modal opened onClose={onClose} title="Task context" size="lg" centered>
      <Stack>
        <Failure error={details.error} />
        {details.error && (
          <Button variant="light" onClick={() => void details.refetch()}>
            Retry task
          </Button>
        )}
        {details.isPending && <Loader size="sm" />}
        {data && (
          <>
            <Group justify="space-between">
              <Title order={3}>{data.task.label}</Title>
              <Badge color={data.task.closed ? "gray" : "teal"}>
                {data.task.closed ? "Closed" : "Active"}
              </Badge>
            </Group>
            <Text size="xs" className="workspace-path" c="dimmed">
              Task {data.task.id}
            </Text>
            {data.task.parent_task_id && (
              <Text size="xs" className="workspace-path" c="dimmed">
                Parent {data.task.parent_task_id}
              </Text>
            )}
            {!data.task.scope_valid && (
              <Alert color="yellow" title="Scope unavailable">
                A selected view was removed. Change this task’s scope before
                starting another operation.
              </Alert>
            )}
            <ScopeView scope={data.task.scope} />
            <Text size="xs" className="workspace-path" c="dimmed">
              Current scope {data.task.scope.id}
            </Text>
            {!data.task.closed && !brain.archived && (
              <Group>
                <Button variant="default" onClick={() => onEdit(data.task)}>
                  Change scope
                </Button>
                <Button
                  variant="light"
                  leftSection={<GitBranch size={iconSize.small} />}
                  onClick={() => onFork(data.task)}
                >
                  Start subagent
                </Button>
                <Button
                  variant="subtle"
                  color="gray"
                  onClick={() => setClosing(true)}
                >
                  Close task
                </Button>
              </Group>
            )}
            {closing && (
              <Alert title="Close this task?" color="yellow">
                <Text size="sm" mb="sm">
                  New operations will stop. History and child tasks remain
                  available.
                </Text>
                <Group>
                  <Button
                    size="xs"
                    onClick={() => close.mutate()}
                    loading={close.isPending}
                  >
                    Confirm close
                  </Button>
                  <Button
                    size="xs"
                    variant="subtle"
                    onClick={() => setClosing(false)}
                  >
                    Keep open
                  </Button>
                </Group>
                <Failure error={close.error} />
              </Alert>
            )}
            <Divider />
            {!data.task.closed && !brain.archived && (
              <>
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
                <Text size="xs" c="dimmed">
                  Record a fixed scope for a later operation. A binding does not
                  execute a tool or retrieve evidence.
                </Text>
                <Button
                  variant="light"
                  disabled={!data.task.scope_valid}
                  loading={begin.isPending}
                  onClick={() => begin.mutate()}
                >
                  Bind operation
                </Button>
                <Failure error={begin.error} />
              </>
            )}
            <Text fw={600} size="sm">
              Recorded operations
            </Text>
            {data.operations.length === 0 && (
              <Text c="dimmed" size="sm">
                No operation bindings yet.
              </Text>
            )}
            {data.operations.map((op) => (
              <Card
                key={op.id}
                withBorder
                p="sm"
                data-testid="operation-binding"
              >
                <Group justify="space-between">
                  <Text size="sm" tt="capitalize">
                    {op.kind} binding
                  </Text>
                  <Button
                    size="xs"
                    variant="subtle"
                    onClick={() => setInspect(op.scope)}
                  >
                    Inspect scope
                  </Button>
                </Group>
                <Text size="xs" c="dimmed">
                  {timestamp(op.created_at)} ·{" "}
                  {op.scope.id === data.task.scope.id
                    ? "Current scope"
                    : "Earlier scope"}
                </Text>
                <Text size="xs" className="workspace-path" c="dimmed">
                  {op.id}
                </Text>
              </Card>
            ))}
            <Pages
              offset={operationOffset}
              total={data.operation_total}
              size={20}
              setOffset={setOperationOffset}
            />
            <Divider />
            <Text fw={600} size="sm">
              Scope history
            </Text>
            {data.scopes.map((scope) => (
              <Group key={scope.id} justify="space-between">
                <Text size="xs">
                  {timestamp(scope.created_at)} ·{" "}
                  {scope.id === data.task.scope.id ? "Current" : "Earlier"}
                </Text>
                <Button
                  size="xs"
                  variant="subtle"
                  onClick={() => setInspect(scope)}
                >
                  View scope
                </Button>
              </Group>
            ))}
            <Pages
              offset={scopeOffset}
              total={data.scope_total}
              size={20}
              setOffset={setScopeOffset}
            />
            {inspect && (
              <Card withBorder p="md">
                <Group justify="space-between" mb="sm">
                  <Text fw={600} size="sm">
                    Recorded scope
                  </Text>
                  <Button
                    size="xs"
                    variant="subtle"
                    onClick={() => setInspect(null)}
                  >
                    Hide
                  </Button>
                </Group>
                <ScopeView scope={inspect} />
                <Text size="xs" mt="sm" className="workspace-path" c="dimmed">
                  {inspect.id}
                </Text>
              </Card>
            )}
          </>
        )}
      </Stack>
    </Modal>
  );
}

function AliasDialog({
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
    <Modal opened onClose={onClose} title="Add repository origin" centered>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          save.mutate();
        }}
      >
        <Stack>
          <Text size="sm" className="workspace-path">
            Attach an alias or moved origin to {repository.canonical_origin}.
          </Text>
          <TextInput
            label="Repository origin"
            required
            value={origin}
            onChange={(event) => setOrigin(event.currentTarget.value)}
            maxLength={2000}
          />
          <Text size="xs" c="dimmed">
            Use this only when both origins identify the same repository. Its
            UUID and recorded scopes are preserved.
          </Text>
          <Failure error={save.error} />
          <Button type="submit" loading={save.isPending}>
            Attach origin
          </Button>
        </Stack>
      </form>
    </Modal>
  );
}
