import { useEffect, useState, type ReactNode } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
  Archive,
  CircleCheck,
  GitBranch,
  Layers,
  Pencil,
  Plus,
  RotateCcw,
  Trash2,
} from "lucide-react";
import { client, result, RequestError, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { useBrainSearch } from "../../app/useBrainSearch";
import { useBrain, useWorkspace } from "../../app/context";
import { useIdempotency } from "../../useIdempotency";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { ErrorState } from "../../components/AsyncState";
import { Page } from "../../components/Page";
import { SkeletonRows } from "../../components/Skeleton";
import {
  Alert,
  Badge,
  Button,
  IconButton,
  Modal,
  Spinner,
  StatusBadge,
  TextField,
  Tile,
} from "../../components/ui";
import { cn } from "../../lib/utils";
import { BrainIcon } from "../../components/BrainIcon";
import { BrainForm } from "../../components/BrainForm";
import { AccessPanel } from "../../AccessPanel";
import { ModelsPanel } from "../../ModelsPanel";

import { PrivacyOverview } from "./PrivacyOverview";
import { deletionReceiptKey } from "./DeletionNotice";
import { useWorkspaceCatalogue } from "../../catalogueQueries";
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
    <Page>
      <PageHeader
        title="Settings"
        description="Keep this Brain working the way you want."
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        {tab === "general" && <GeneralSettings />}
        {tab === "access" &&
          (brain.role === "admin" ? (
            <AccessPanel brain={brain} actor={session.user.id} embedded />
          ) : (
            <Alert role="status">
              A Brain administrator can manage its members and roles.
            </Alert>
          ))}
        {tab === "ai" && <ModelsPanel brain={brain} section="settings" />}
        {tab === "privacy" && <PrivacyOverview brain={brain} />}
      </FeatureTabs>
    </Page>
  );
}

/**
 * One General settings group (board: Settings · General): title and
 * description in a 240px column, its value and actions beside them.
 */
function SettingsCard({
  title,
  description,
  label,
  id,
  danger = false,
  children,
}: {
  title: string;
  description: string;
  label?: string;
  id?: string;
  danger?: boolean;
  children: ReactNode;
}) {
  const heading = `settings-${title.toLowerCase().replaceAll(" ", "-")}`;
  return (
    <section
      id={id}
      aria-label={label}
      aria-labelledby={label ? undefined : heading}
      className={cn(
        "grid scroll-mt-6 grid-cols-[240px_minmax(0,1fr)] gap-8 rounded-lg border bg-card px-6 py-5",
        danger ? "border-destructive/45" : "border-border",
      )}
    >
      <div className="min-w-0">
        <h2 id={heading} className="m-0 text-title font-semibold text-foreground">
          {title}
        </h2>
        <p className="m-0 mt-1 text-body text-muted-foreground">
          {description}
        </p>
      </div>
      <div className="min-w-0">{children}</div>
    </section>
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
  const busy = editing || environmentEditing || archive.isPending;
  return (
    <div className="grid gap-4">
      <SettingsCard
        title="Brain details"
        label="Brain details"
        description="How this Brain is named and described for your team."
      >
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
          <div className="grid gap-4">
            <div className="flex items-start gap-3.5">
              <BrainIcon
                id={brain.id}
                revision={brain.icon_revision}
                size={40}
              />
              <dl className="m-0 grid min-w-0 flex-1 gap-3">
                <div className="grid gap-0.5">
                  <dt className="text-caption text-muted-foreground">Name</dt>
                  <dd className="m-0 truncate text-body font-medium text-foreground">
                    {brain.name}
                  </dd>
                </div>
                <div className="grid gap-0.5">
                  <dt className="text-caption text-muted-foreground">
                    Description
                  </dt>
                  <dd
                    className={cn(
                      "m-0 max-w-[68ch] text-body",
                      brain.description
                        ? "text-foreground-secondary"
                        : "text-muted-foreground",
                    )}
                  >
                    {brain.description || "No description yet."}
                  </dd>
                </div>
              </dl>
              {admin && (
                <Button
                  variant="secondary"
                  aria-label="Edit Brain"
                  disabled={environmentEditing || archive.isPending}
                  onClick={() => setEditing(true)}
                >
                  <Pencil aria-hidden="true" />
                  Edit
                </Button>
              )}
            </div>
            <p className="m-0 flex items-center gap-2 border-t border-border pt-3 text-caption text-muted-foreground">
              Created
              <span className="mono text-foreground-secondary">
                {new Date(brain.created_at).toLocaleDateString()}
              </span>
            </p>
          </div>
        )}
      </SettingsCard>
      <EnvironmentsSettings
        key={brain.id}
        brain={brain}
        blocked={editing || archive.isPending}
        onEditingChange={setEnvironmentEditing}
      />
      <SettingsCard
        title="Status"
        label="Brain status"
        description="Whether this Brain is active or archived."
      >
        <div className="flex min-h-8 items-center gap-3">
          {brain.archived ? (
            <StatusBadge state="neutral">Archived</StatusBadge>
          ) : (
            <Badge variant="brand">
              <CircleCheck aria-hidden="true" />
              Active
            </Badge>
          )}
          <span className="min-w-0 flex-1 text-caption text-muted-foreground">
            {brain.archived
              ? "Content and history are preserved. Reopen to resume work."
              : "Content and history are preserved when archived."}
          </span>
          {admin && (
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => {
                archive.reset();
                setConfirming(true);
              }}
            >
              {brain.archived ? (
                <RotateCcw aria-hidden="true" />
              ) : (
                <Archive aria-hidden="true" />
              )}
              {brain.archived ? "Reopen Brain" : "Archive Brain"}
            </Button>
          )}
        </div>
      </SettingsCard>
      {admin && (
        <SettingsCard
          title="Danger zone"
          label="Permanent deletion"
          description="These actions can't be undone."
          danger
        >
          <fieldset disabled={busy} className="m-0 min-w-0 border-0 p-0">
            <DeleteBrainSection brain={brain} />
          </fieldset>
        </SettingsCard>
      )}
      <Modal
        opened={confirming && admin}
        onClose={() => !archive.isPending && setConfirming(false)}
        title={brain.archived ? "Reopen Brain" : "Archive Brain"}
        size="sm"
        footer={
          <>
            <Button
              variant="secondary"
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
          </>
        }
      >
        <div className="grid gap-3">
          <p className="m-0 text-body text-foreground-secondary">
            {brain.archived
              ? `Resume work in ${brain.name}?`
              : `Archive ${brain.name}? Its content remains available, while new work is paused.`}
          </p>
          <ErrorState error={archive.error} />
        </div>
      </Modal>
    </div>
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
  const ready = !deletion.data && !preview.isPending && !preview.error && !!preview.data;
  return (
    <>
      <div className="flex items-center gap-6">
        <div className="min-w-0 flex-1">
          <h3 className="m-0 text-body font-semibold text-foreground">
            Delete this Brain
          </h3>
          <p className="m-0 mt-0.5 text-body text-muted-foreground">
            Permanent removal of the Brain and everything derived from it.
            Archiving keeps its content; deleting erases it.
          </p>
        </div>
        <Button variant="destructive-outline" onClick={openDialog}>
          <Trash2 aria-hidden="true" />
          Delete Brain
        </Button>
      </div>
      <Modal
        opened={open}
        onClose={closeDialog}
        title="Delete this Brain"
        size="lg"
        footer={
          deletion.data ? undefined : preview.error ? (
            <>
              <Button variant="secondary" onClick={() => void preview.mutate()}>
                Retry preview
              </Button>
              <Button onClick={closeDialog}>Close</Button>
            </>
          ) : ready ? (
            <>
              <Button
                variant="secondary"
                onClick={closeDialog}
                disabled={deletion.isPending}
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                loading={deletion.isPending}
                // Stay disabled while a fresh preview is in flight so the
                // DELETE never carries a counter the re-preview supersedes.
                disabled={typed !== preview.data!.name || preview.isPending}
                onClick={() =>
                  deletion.mutate({
                    closure: preview.data!.closure,
                    confirmation: typed,
                  })
                }
              >
                <Trash2 aria-hidden="true" />
                Delete Brain
              </Button>
            </>
          ) : undefined
        }
      >
        <div className="grid gap-4">
          {deletion.data ? (
            <DeletionOutcome name={brain.name} result={deletion.data} />
          ) : preview.isPending ? (
            <div className="flex items-center justify-center gap-2 py-8">
              <Spinner />
              <span className="text-body text-muted-foreground">
                Preparing the deletion summary…
              </span>
            </div>
          ) : preview.error ? (
            <Alert variant="destructive">{preview.error.message}</Alert>
          ) : preview.data ? (
            <>
              <Alert variant="destructive" title="This cannot be undone">
                Deleting {preview.data.name} removes the Brain and everything
                derived from it. There is no undo, recycle bin or restore.
              </Alert>
              <p className="m-0 text-body text-foreground">
                {counts.length === 0
                  ? "No dependent records — this Brain is empty."
                  : `${total} dependent record${total === 1 ? "" : "s"} across ${counts.length} class${counts.length === 1 ? "" : "es"} will be removed.`}
              </p>
              {counts.length > 0 && (
                <div className="grid grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] gap-2">
                  {counts.map(([key, value]) => (
                    <div
                      key={key}
                      className="flex items-baseline justify-between gap-3 rounded-md border border-border bg-muted/50 px-2.5 py-1.5"
                    >
                      <span className="text-label text-muted-foreground">
                        {countLabel(key)}
                      </span>
                      <strong className="mono text-label font-semibold text-foreground">
                        {value}
                      </strong>
                    </div>
                  ))}
                </div>
              )}
              <p className="m-0 text-caption text-muted-foreground">
                Retained copies in managed backups can remain for up to{" "}
                {preview.data.backup_days} days.
                {preview.data.pending_work > 0 &&
                  ` ${preview.data.pending_work} queued or running job${preview.data.pending_work === 1 ? "" : "s"} will be fenced.`}
              </p>
              <TextField
                label="Exact Brain name"
                description={`Type "${preview.data.name}" exactly to enable deletion.`}
                value={typed}
                onChange={(e) => setTyped(e.target.value)}
                disabled={deletion.isPending}
                autoComplete="off"
              />
              {deletion.error && (
                <Alert variant="destructive">
                  {deletionMessage(deletion.error)}
                </Alert>
              )}
            </>
          ) : null}
        </div>
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
    <div className="grid gap-2">
      {status.state === "complete" ? (
        <Alert variant="brand" title="Deletion complete" role="status">
          {name} is gone from this installation.
        </Alert>
      ) : status.state === "error" ? (
        <Alert variant="destructive" title="Deletion committed — cleanup failed">
          {name} is absent from listings and recall, but physical cleanup hit an
          error. It stays pending and will not report complete until the cleanup
          succeeds.
        </Alert>
      ) : (
        <Alert variant="warning" title="Deletion in progress" role="status">
          {name} is already absent from listings and recall. Physical cleanup is
          pending: {status.pending_artifacts} file
          {status.pending_artifacts === 1 ? "" : "s"} remaining, journal{" "}
          {status.journaled ? "saved" : "pending"}, graph{" "}
          {status.graph_pending ? "cleanup pending" : "cleanup complete"}.
        </Alert>
      )}
      <p className="m-0 text-caption text-muted-foreground">
        Retained copies in managed backups can remain for up to{" "}
        {result.backup_days} days. Offline host copies are removed on their next
        check-in.
      </p>
    </div>
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
  const workspace = useWorkspaceCatalogue(brain.id, undefined, { poll: "normal" });
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
      className={cn(
        "grid animate-enter gap-3 rounded-lg border border-border-strong bg-card p-3.5 shadow-xs",
        creating && "mt-3",
      )}
      role="region"
      aria-label={creating ? "Add environment" : "Rename environment"}
      onSubmit={(event) => {
        event.preventDefault();
        if (canEdit && !save.isPending && !missing && name.trim())
          save.mutate();
      }}
    >
      <h3 className="m-0 text-body font-semibold text-foreground">
        {creating ? "New environment" : `Rename ${originalName}`}
      </h3>
      <TextField
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
        <Alert variant="warning">
          This environment is no longer available. Cancel and reload settings.
        </Alert>
      )}
      <ErrorState error={save.error} />
      <div className="flex justify-end gap-2">
        <Button variant="secondary" disabled={save.isPending} onClick={cancel}>
          Cancel
        </Button>
        <Button
          type="submit"
          loading={save.isPending}
          disabled={!name.trim() || missing || !canEdit}
        >
          Save environment
        </Button>
      </div>
    </form>
  );
  const addButton = canEdit && (
    <Button
      variant="secondary"
      disabled={
        !!workspace.error || !workspace.data || !!editing || save.isPending
      }
      onClick={() => {
        setName("");
        save.reset();
        command.reset();
        setOriginalName("");
        setEditing("new");
      }}
    >
      <Plus aria-hidden="true" />
      Add environment
    </Button>
  );
  const empty =
    !workspace.error && workspace.data?.environments.length === 0 && !editing;
  return (
    <SettingsCard
      id="environments"
      title="Environments"
      label="Environments"
      description="Optional scopes for this Brain."
    >
      <div className="grid gap-3">
        <ErrorState
          error={workspace.error}
          retry={() => void workspace.refetch()}
        />
        {workspace.isPending && (
          <SkeletonRows rows={2} label="Loading environments…" />
        )}
        {empty && (
          <div className="flex items-center gap-3.5">
            <Tile size="md" className="size-10">
              <Layers aria-hidden="true" />
            </Tile>
            <div className="min-w-0 flex-1">
              <p className="m-0 text-body font-semibold text-foreground">
                No environments yet
              </p>
              <p className="m-0 text-body text-muted-foreground">
                Your agents can work Brain-wide.
              </p>
            </div>
            {addButton}
          </div>
        )}
        {environments.length > 0 && (
          <div className="overflow-hidden rounded-lg border border-border">
            {environments.map((e) => (
              <div
                key={e.id}
                className={cn(
                  "flex min-h-12 items-center gap-3 border-b border-border px-3 py-2 last:border-b-0",
                  editing === e.id && "bg-muted/40",
                )}
                data-testid="general-environment-row"
              >
                <Tile size="sm">
                  <GitBranch aria-hidden="true" />
                </Tile>
                <div className="min-w-0 flex-1">
                  {editing === e.id && canEdit ? (
                    editor(false)
                  ) : (
                    <span className="text-body font-medium text-foreground">
                      {e.name}
                    </span>
                  )}
                </div>
                {canEdit && editing !== e.id && (
                  <IconButton
                    label={`Rename ${e.name}`}
                    disabled={!!editing || save.isPending}
                    onClick={() => {
                      setName(e.name);
                      setOriginalName(e.name);
                      save.reset();
                      command.reset();
                      setEditing(e.id);
                    }}
                  >
                    <Pencil aria-hidden="true" />
                  </IconButton>
                )}
              </div>
            ))}
          </div>
        )}
        {!empty && environments.length > 0 && !editing && (
          <div className="flex justify-end">{addButton}</div>
        )}
        {editing === "new" && canEdit && !workspace.error && editor(true)}
        {missing && canEdit && editor(false)}
      </div>
    </SettingsCard>
  );
}
