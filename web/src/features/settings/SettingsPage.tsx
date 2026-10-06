import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Modal,
  Stack,
  Text,
  TextInput,
  ActionIcon,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
  Archive,
  Pencil,
  RotateCcw,
  Trash2,
  GitBranch,
  Plus,
} from "lucide-react";
import { client, result, RequestError, type Brain } from "../../api";
import type { components } from "../../api-schema";
import "./brain-deletion.css";
import "./general-settings.css";
import { useBrainSearch } from "../../app/useBrainSearch";
import { useBrain, useWorkspace } from "../../app/context";
import { useIdempotency } from "../../useIdempotency";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { SettingsSection } from "../../components/SettingsSection";
import { ErrorState } from "../../components/AsyncState";
import { BrainIcon } from "../../components/BrainIcon";
import { BrainForm } from "../../components/BrainForm";
import { AccessPanel } from "../../AccessPanel";
import { ModelsPanel } from "../../ModelsPanel";

import { PrivacyOverview } from "./PrivacyOverview";
import { deletionReceiptKey } from "./DeletionNotice";
type DeletionResult = components["schemas"]["BrainDeletionResult"];
// Carries the API error code so the confirmation flow can tell a mismatched
// name, a stale preview counter and an already-absent Brain apart.
class DeletionError extends RequestError {
  constructor(
    status: number,
    public code: string,
    message: string,
  ) {
    super(status, message);
  }
}
const tabs = [
  { value: "general", label: "General" },
  { value: "access", label: "Access" },
  { value: "privacy", label: "Privacy" },
  { value: "ai", label: "AI permissions" },
] as const;
export function SettingsPage() {
  const brain = useBrain();
  const session = useWorkspace();
  const navigate = useNavigate();
  const [search] = useBrainSearch();
  useEffect(() => {
    if (search.tab === "capture")
      void navigate({
        to: "/brains/$brainId/settings",
        params: { brainId: brain.id },
        search: { ...search, tab: "privacy" },
        replace: true,
      });
  }, [navigate, brain.id, search]);
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "general",
  );
  return (
    <div className="settings-management-page">
      <PageHeader
        title="Settings"
        description="Keep this Brain working the way you want."
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        {tab === "general" && <GeneralSettings />}
        {tab === "access" && (
          <section className="settings-members">
            {brain.role === "admin" ? (
              <AccessPanel brain={brain} actor={session.user.id} embedded />
            ) : (
              <Alert color="gray">
                A Brain administrator can manage its members and roles.
              </Alert>
            )}
          </section>
        )}
        {tab === "ai" && <ModelsPanel brain={brain} section="settings" />}
        {tab === "privacy" && <PrivacyOverview brain={brain} />}
      </FeatureTabs>
    </div>
  );
}
function GeneralSettings() {
  const brain = useBrain();
  const cache = useQueryClient();
  const command = useIdempotency();
  const [editing, setEditing] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [environmentEditing, setEnvironmentEditing] = useState(false);
  const admin = brain.role === "admin";
  useEffect(() => {
    setEditing(false);
    setConfirming(false);
  }, [brain.id, admin]);
  const archive = useMutation({
    mutationFn: async () => {
      if (!admin)
        throw new Error("Brain administration is no longer available.");
      return result(
        await client.PATCH("/api/brains/{id}", {
          params: { path: { id: brain.id } },
          body: { archived: !brain.archived },
          headers: {
            "Idempotency-Key": command.forInput({ archived: !brain.archived }),
          },
        }),
      );
    },
    onSuccess: () => {
      command.reset();
      setConfirming(false);
      void cache.invalidateQueries({ queryKey: ["brain", brain.id] });
      void cache.invalidateQueries({ queryKey: ["brains"] });
    },
  });
  return (
    <>
      <div className="general-settings-layout">
        <div className="general-settings-main">
          <section
            className="general-settings-card general-brain-details"
            aria-label="Brain details"
          >
            <header className="general-card-heading">
              <h2>Brain details</h2>
              {admin && !editing && (
                <Button
                  variant="default"
                  size="sm"
                  aria-label="Edit Brain"
                  leftSection={<Pencil size={16} />}
                  disabled={environmentEditing || archive.isPending}
                  onClick={() => setEditing(true)}
                >
                  Edit
                </Button>
              )}
            </header>
            {editing && admin ? (
              <BrainForm
                key={brain.id}
                brain={brain}
                opened
                inline
                disabled={!admin}
                close={() => setEditing(false)}
                saved={() => {
                  setEditing(false);
                  void cache.invalidateQueries({
                    queryKey: ["brain", brain.id],
                  });
                  void cache.invalidateQueries({ queryKey: ["brains"] });
                }}
              />
            ) : (
              <>
                <div className="general-brain-identity">
                  <BrainIcon
                    id={brain.id}
                    revision={brain.icon_revision}
                    size={56}
                  />
                  <strong>{brain.name}</strong>
                  <Badge
                    variant="light"
                    color={brain.archived ? "gray" : "brand"}
                  >
                    {brain.archived ? "Archived" : "Active"}
                  </Badge>
                </div>
                <dl className="general-brain-fields">
                  <div>
                    <dt>Name</dt>
                    <dd>{brain.name}</dd>
                  </div>
                  <div>
                    <dt>Description</dt>
                    <dd className={!brain.description ? "is-empty" : undefined}>
                      {brain.description || "No description yet."}
                    </dd>
                  </div>
                </dl>
              </>
            )}
            <p className="general-created">
              Created {new Date(brain.created_at).toLocaleDateString()}
            </p>
          </section>
          <EnvironmentsSettings
            key={brain.id}
            brain={brain}
            blocked={editing || archive.isPending}
            onEditingChange={setEnvironmentEditing}
          />
        </div>
        <aside
          className="general-settings-side"
          aria-label="Brain status and permanent deletion"
        >
          <section
            className="general-settings-card general-brain-status"
            aria-label="Brain status"
          >
            <h2>Brain status</h2>
            <Badge variant="light" color={brain.archived ? "gray" : "brand"}>
              {brain.archived ? "Archived" : "Active"}
            </Badge>
            {admin && (
              <Button
                fullWidth
                variant="default"
                disabled={editing || environmentEditing || archive.isPending}
                leftSection={
                  brain.archived ? (
                    <RotateCcw size={16} />
                  ) : (
                    <Archive size={16} />
                  )
                }
                onClick={() => {
                  archive.reset();
                  setConfirming(true);
                }}
              >
                {brain.archived ? "Reopen Brain" : "Archive Brain"}
              </Button>
            )}
            <p>
              {brain.archived
                ? "Content and history are preserved. Reopen to resume work."
                : "Content and history are preserved when archived."}
            </p>
          </section>
          {admin && (
            <fieldset
              className="general-deletion-card"
              disabled={editing || environmentEditing || archive.isPending}
            >
              <DeleteBrainSection brain={brain} />
            </fieldset>
          )}
        </aside>
      </div>
      <Modal
        opened={confirming && admin}
        onClose={() => !archive.isPending && setConfirming(false)}
        title={brain.archived ? "Reopen Brain" : "Archive Brain"}
      >
        <Stack>
          <Text>
            {brain.archived
              ? `Resume work in ${brain.name}?`
              : `Archive ${brain.name}? Its content remains available, while new work is paused.`}
          </Text>
          <ErrorState error={archive.error} />
          <Group justify="flex-end">
            <Button
              variant="default"
              onClick={() => setConfirming(false)}
              disabled={archive.isPending}
            >
              Cancel
            </Button>
            <Button
              loading={archive.isPending}
              onClick={() => archive.mutate()}
            >
              {brain.archived ? "Reopen Brain" : "Archive Brain"}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </>
  );
}
// Delete is a separate, irreversible command: it previews the dependent
// closure first and requires the exact Brain name before the DELETE is sent.
function DeleteBrainSection({ brain }: { brain: Brain }) {
  const cache = useQueryClient();
  const navigate = useNavigate();
  const command = useIdempotency();
  const [open, setOpen] = useState(false);
  const [typed, setTyped] = useState("");
  const leave = () => {
    // The Brain is gone: drop its cached reads and land on the Brains list.
    navigate({ to: "/" });
    void cache.removeQueries({
      predicate: (q) => q.queryKey.includes(brain.id),
    });
    void cache.invalidateQueries({ queryKey: ["brains"] });
  };
  const preview = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/brains/{brain}/deletions/preview", {
          params: { path: { brain: brain.id } },
        }),
      ),
    onError: (error) => {
      // Already absent (or no visibility): nothing left to delete here.
      if (error instanceof RequestError && error.status === 404) leave();
    },
  });
  const deletion = useMutation({
    mutationFn: async ({
      closure,
      confirmation,
    }: {
      closure: string;
      confirmation: string;
    }) => {
      const response = await client.DELETE("/api/brains/{brain}", {
        params: { path: { brain: brain.id } },
        body: { closure, confirmation },
        headers: {
          "Idempotency-Key": command.forInput({ closure, confirmation }),
        },
      });
      if (!response.response.ok)
        throw new DeletionError(
          response.response.status,
          response.error?.code ?? "",
          response.error?.message ?? "The deletion failed. Please retry.",
        );
      return result(response);
    },
    onSuccess: (receipt) => {
      cache.setQueryData(deletionReceiptKey, receipt);
      leave();
    },
    onError: (error) => {
      if (
        error instanceof DeletionError &&
        error.code === "brain_deletion_preview_changed"
      ) {
        // Stale closure counter: refresh the summary for a fresh confirmation.
        command.reset();
        void preview.mutate();
      } else if (
        error instanceof DeletionError &&
        error.code === "brain_deletion_confirmation_mismatch"
      ) {
        command.reset();
      } else if (error instanceof RequestError && error.status === 404) {
        leave();
      }
    },
  });
  const openDialog = () => {
    setTyped("");
    preview.reset();
    deletion.reset();
    command.reset();
    setOpen(true);
    void preview.mutate();
  };
  const closeDialog = () => {
    if (deletion.isPending || deletion.data) return;
    setOpen(false);
  };
  const counts = preview.data
    ? Object.entries(preview.data.counts).filter(([, n]) => n > 0)
    : [];
  const total = counts.reduce((sum, [, n]) => sum + n, 0);
  return (
    <>
      <SettingsSection
        title="Delete this Brain"
        description="Permanent removal of the Brain and everything derived from it. Archiving keeps its content; deleting erases it."
      >
        <Button
          color="red"
          variant="light"
          leftSection={<Trash2 size={16} />}
          onClick={openDialog}
        >
          Delete Brain
        </Button>
      </SettingsSection>
      <Modal
        opened={open}
        onClose={closeDialog}
        title="Delete this Brain"
        size="lg"
      >
        <Stack>
          {deletion.data ? (
            <DeletionOutcome name={brain.name} result={deletion.data} />
          ) : preview.isPending ? (
            <Group justify="center" py="xl">
              <Loader size="sm" />
              <Text c="dimmed">Preparing the deletion summary…</Text>
            </Group>
          ) : preview.error ? (
            <>
              <Alert color="red">{preview.error.message}</Alert>
              <Group justify="flex-end">
                <Button variant="default" onClick={() => void preview.mutate()}>
                  Retry preview
                </Button>
                <Button onClick={closeDialog}>Close</Button>
              </Group>
            </>
          ) : preview.data ? (
            <>
              <Alert color="red" title="This cannot be undone">
                Deleting {preview.data.name} removes the Brain and everything
                derived from it. There is no undo, recycle bin or restore.
              </Alert>
              <Text size="sm">
                {counts.length === 0
                  ? "No dependent records — this Brain is empty."
                  : `${total} dependent record${total === 1 ? "" : "s"} across ${counts.length} class${counts.length === 1 ? "" : "es"} will be removed.`}
              </Text>
              {counts.length > 0 && (
                <div className="brain-deletion-counts">
                  {counts.map(([key, value]) => (
                    <div key={key} className="brain-deletion-count">
                      <span>{countLabel(key)}</span>
                      <strong>{value}</strong>
                    </div>
                  ))}
                </div>
              )}
              <Text size="sm" c="dimmed">
                Retained copies in managed backups can remain for up to{" "}
                {preview.data.backup_days} days.
                {preview.data.pending_work > 0 &&
                  ` ${preview.data.pending_work} queued or running job${preview.data.pending_work === 1 ? "" : "s"} will be fenced.`}
              </Text>
              <TextInput
                label="Exact Brain name"
                description={`Type "${preview.data.name}" exactly to enable deletion.`}
                value={typed}
                onChange={(e) => setTyped(e.target.value)}
                disabled={deletion.isPending}
              />
              {deletion.error && (
                <Alert color="red">{deletionMessage(deletion.error)}</Alert>
              )}
              <Group justify="flex-end">
                <Button
                  variant="default"
                  onClick={closeDialog}
                  disabled={deletion.isPending}
                >
                  Cancel
                </Button>
                <Button
                  color="red"
                  loading={deletion.isPending}
                  // Stay disabled while a fresh preview is in flight so the
                  // DELETE never carries a counter the re-preview supersedes.
                  disabled={typed !== preview.data.name || preview.isPending}
                  onClick={() =>
                    deletion.mutate({
                      closure: preview.data.closure,
                      confirmation: typed,
                    })
                  }
                >
                  Delete Brain
                </Button>
              </Group>
            </>
          ) : null}
        </Stack>
      </Modal>
    </>
  );
}
function countLabel(key: string) {
  return key.replaceAll("_", " ").replace(/^\w/, (c) => c.toUpperCase());
}
function deletionMessage(error: Error) {
  if (
    error instanceof DeletionError &&
    error.code === "brain_deletion_confirmation_mismatch"
  )
    return "The confirmation did not match the Brain name. Type it exactly as shown.";
  if (
    error instanceof DeletionError &&
    error.code === "brain_deletion_preview_changed"
  )
    return "This Brain changed since the preview. A fresh summary is ready — review it and confirm again.";
  return error.message;
}
function DeletionOutcome({
  name,
  result,
}: {
  name: string;
  result: DeletionResult;
}) {
  const status = result.request;
  return (
    <Stack gap="sm">
      {status.state === "complete" ? (
        <Alert color="teal" title="Deletion complete">
          {name} is gone from this installation.
        </Alert>
      ) : status.state === "error" ? (
        <Alert color="red" title="Deletion committed — cleanup failed">
          {name} is absent from listings and recall, but physical cleanup hit an
          error. It stays pending and will not report complete until the cleanup
          succeeds.
        </Alert>
      ) : (
        <Alert color="orange" title="Deletion in progress">
          {name} is already absent from listings and recall. Physical cleanup is
          pending: {status.pending_artifacts} file
          {status.pending_artifacts === 1 ? "" : "s"} remaining, journal{" "}
          {status.journaled ? "saved" : "pending"}, graph{" "}
          {status.graph_pending ? "cleanup pending" : "cleanup complete"}.
        </Alert>
      )}
      <Text size="xs" c="dimmed">
        Retained copies in managed backups can remain for up to{" "}
        {result.backup_days} days. Offline host copies are removed on their next
        check-in.
      </Text>
    </Stack>
  );
}

function EnvironmentsSettings({
  brain,
  blocked = false,
  onEditingChange,
}: {
  brain: Brain;
  blocked?: boolean;
  onEditingChange?: (editing: boolean) => void;
}) {
  const [editing, setEditing] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [originalName, setOriginalName] = useState("");
  const cache = useQueryClient();
  const command = useIdempotency();
  const canEdit = brain.role === "admin" && !brain.archived && !blocked;
  const workspace = useQuery({
    queryKey: ["workspace", brain.id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  useEffect(() => {
    if (workspace.error || !canEdit) {
      setEditing(null);
      setName("");
    }
  }, [workspace.error, canEdit]);
  useEffect(() => {
    onEditingChange?.(editing !== null);
    return () => onEditingChange?.(false);
  }, [editing, onEditingChange]);
  const save = useMutation({
    mutationFn: async () => {
      if (!canEdit || workspace.error || !workspace.data || !editing)
        throw new Error(
          "Environment editing is no longer available. Reload settings.",
        );
      if (editing !== "new") {
        const catalogue = result(
          await client.GET("/api/brains/{brain}/evidence", {
            params: { path: { brain: brain.id } },
          }),
        );
        const current = catalogue.groups.find(
          (e) => e.id === editing && e.kind === "environment",
        );
        if (!current)
          throw new Error(
            "This environment is no longer available. Reload settings.",
          );
        return result(
          await client.PATCH("/api/brains/{brain}/evidence/groups/{group}", {
            params: { path: { brain: brain.id, group: editing! } },
            body: { name, description: current.description },
          }),
        );
      }
      const body = { kind: "environment", name, description: "" };
      return result(
        await client.POST("/api/brains/{brain}/evidence/groups", {
          params: { path: { brain: brain.id } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: async () => {
      command.reset();
      setEditing(null);
      await cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain.id),
      });
    },
  });
  const environments = workspace.error
    ? []
    : (workspace.data?.environments ?? []);
  const missing =
    !!editing &&
    editing !== "new" &&
    !!workspace.data &&
    !workspace.error &&
    !environments.some((environment) => environment.id === editing);
  const cancel = () => {
    if (save.isPending) return;
    setEditing(null);
    setName("");
    save.reset();
    command.reset();
  };
  const editor = (creating: boolean) => (
    <form
      className={`general-environment-editor${creating ? " is-new" : ""}`}
      role="region"
      aria-label={creating ? "Add environment" : "Rename environment"}
      onSubmit={(event) => {
        event.preventDefault();
        if (canEdit && !save.isPending && !missing && name.trim())
          save.mutate();
      }}
    >
      <h3>{creating ? "New environment" : `Rename ${originalName}`}</h3>
      <TextInput
        label="Environment name"
        placeholder="Staging"
        required
        maxLength={120}
        value={name}
        autoFocus
        disabled={save.isPending || missing || !canEdit}
        onChange={(event) => setName(event.currentTarget.value)}
      />
      {missing && (
        <Alert color="orange">
          This environment is no longer available. Cancel and reload settings.
        </Alert>
      )}
      <ErrorState error={save.error} />
      <Group justify="flex-end">
        <Button variant="default" disabled={save.isPending} onClick={cancel}>
          Cancel
        </Button>
        <Button
          type="submit"
          loading={save.isPending}
          disabled={!name.trim() || missing || !canEdit}
        >
          Save environment
        </Button>
      </Group>
    </form>
  );
  return (
    <section
      id="environments"
      className="general-settings-card general-environments"
      aria-label="Environments"
    >
      <header className="general-card-heading">
        <div>
          <h2>Environments</h2>
          <p>Optional scopes for this Brain.</p>
        </div>
        {canEdit && (
          <Button
            variant="filled"
            size="sm"
            leftSection={<Plus size={16} />}
            disabled={
              !!workspace.error ||
              !workspace.data ||
              !!editing ||
              save.isPending
            }
            onClick={() => {
              setName("");
              save.reset();
              command.reset();
              setOriginalName("");
              setEditing("new");
            }}
          >
            Add environment
          </Button>
        )}
      </header>
      <ErrorState
        error={workspace.error}
        retry={() => void workspace.refetch()}
      />
      {workspace.isPending && (
        <Group py="md">
          <Loader size="sm" />
          <Text size="sm" c="dimmed">
            Loading environments…
          </Text>
        </Group>
      )}
      <div className="general-environment-list">
        {environments.map((e) => (
          <div
            key={e.id}
            className={`general-environment-row${editing === e.id ? " is-editing" : ""}`}
            data-testid="general-environment-row"
          >
            <span className="general-environment-icon">
              <GitBranch size={20} />
            </span>
            <div className="general-environment-content">
              {editing === e.id && canEdit ? (
                editor(false)
              ) : (
                <span>{e.name}</span>
              )}
            </div>
            {canEdit && editing !== e.id && (
              <ActionIcon
                variant="subtle"
                size="md"
                aria-label={`Rename ${e.name}`}
                title={`Rename ${e.name}`}
                disabled={!!editing || save.isPending}
                onClick={() => {
                  setName(e.name);
                  setOriginalName(e.name);
                  save.reset();
                  command.reset();
                  setEditing(e.id);
                }}
              >
                <Pencil size={17} />
              </ActionIcon>
            )}
          </div>
        ))}
      </div>
      {!workspace.error &&
        workspace.data?.environments.length === 0 &&
        !editing && (
          <p className="general-environments-empty">
            No environments yet. Your agents can work Brain-wide.
          </p>
        )}
      {editing === "new" && canEdit && !workspace.error && editor(true)}
      {missing && canEdit && editor(false)}
    </section>
  );
}
