import { useState } from "react";
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
} from "@mantine/core";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Archive, Pencil, RotateCcw, Trash2 } from "lucide-react";
import { client, result, RequestError, type Brain } from "../../api";
import type { components } from "../../api-schema";
import "./brain-deletion.css";
import { useBrain, useWorkspace } from "../../app/context";
import { useIdempotency } from "../../useIdempotency";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { SettingsSection } from "../../components/SettingsSection";
import { ErrorState } from "../../components/AsyncState";
import { BrainForm } from "../../components/BrainForm";
import { AccessPanel } from "../../AccessPanel";
import { ModelsPanel } from "../../ModelsPanel";
import { CapturePanel } from "../../CapturePanel";
import { RetentionPanel } from "../../RetentionPanel";
import { SourcesStoragePolicy } from "../../EvidencePanel";
import { RepositoryStoragePolicy } from "../../PublicationPanel";
import { ManagedMemoryPanel } from "./ManagedMemoryPanel";
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
  { value: "ai", label: "AI & automation" },
  { value: "capture", label: "Capture" },
  { value: "privacy", label: "Retention & privacy" },
] as const;
export function SettingsPage() {
  const brain = useBrain();
  const session = useWorkspace();
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "general",
  );
  return (
    <>
      <PageHeader
        title="Settings"
        description="Connect your agents once. Memory learns and maintains itself."
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        {tab === "general" && <GeneralSettings />}
        {tab === "access" && (
          <SettingsSection
            title="Who can use this Brain"
            description="Brain access is separate from permission to use connected tools."
          >
            <Text size="sm" mb="md">
              Your role: <strong>{brain.role}</strong>
            </Text>
            {brain.role === "admin" ? (
              <AccessPanel brain={brain} actor={session.user.id} embedded />
            ) : (
              <Alert color="gray">
                A Brain administrator can manage its members and roles.
              </Alert>
            )}
          </SettingsSection>
        )}
        {tab === "ai" && (
          <Stack gap="xl">
            <ManagedMemoryPanel brain={brain} />
            <AdvancedSettings label="Advanced model controls">
              <ModelsPanel brain={brain} section="settings" />
            </AdvancedSettings>
          </Stack>
        )}
        {tab === "capture" && (
          <Stack gap="xl">
            <CapturePanel brain={brain} section="settings" simple />
            <AdvancedSettings label="Advanced capture controls">
              <CapturePanel brain={brain} section="settings" />
            </AdvancedSettings>
          </Stack>
        )}
        {tab === "privacy" && (
          <Stack gap="xl">
            <RetentionPanel brain={brain} section="settings" simple />
            {/* Content retention is a primary privacy control, not an
                advanced one: it stays visible without expanding anything. */}
            <SourcesStoragePolicy brain={brain} />
            <AdvancedSettings label="Advanced retention and storage controls">
              <Stack gap="xl">
                <RepositoryStoragePolicy brain={brain} />
                <RetentionPanel brain={brain} section="settings" />
              </Stack>
            </AdvancedSettings>
          </Stack>
        )}
      </FeatureTabs>
    </>
  );
}
function AdvancedSettings({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(false);
  return (
    <details
      className="feature-advanced"
      onToggle={(e) => setOpen(e.currentTarget.open)}
    >
      <summary>{label}</summary>
      {open && <div className="feature-advanced-content">{children}</div>}
    </details>
  );
}
function GeneralSettings() {
  const brain = useBrain();
  const cache = useQueryClient();
  const command = useIdempotency();
  const [editing, setEditing] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const archive = useMutation({
    mutationFn: async () =>
      result(
        await client.PATCH("/api/brains/{id}", {
          params: { path: { id: brain.id } },
          body: { archived: !brain.archived },
          headers: {
            "Idempotency-Key": command.forInput({ archived: !brain.archived }),
          },
        }),
      ),
    onSuccess: () => {
      command.reset();
      setConfirming(false);
      void cache.invalidateQueries({ queryKey: ["brain", brain.id] });
      void cache.invalidateQueries({ queryKey: ["brains"] });
    },
  });
  return (
    <>
      <SettingsSection
        title="Brain details"
        description="Make this space easy to recognize."
      >
        <Stack gap="md">
          <Text fw={600}>{brain.name}</Text>
          <Text c="dimmed">{brain.description || "No description yet."}</Text>
          <Group>
            <Badge color={brain.archived ? "gray" : "brand"}>
              {brain.archived ? "Archived" : "Active"}
            </Badge>
            <Text size="xs" c="dimmed">
              Created {new Date(brain.created_at).toLocaleDateString()}
            </Text>
          </Group>
          {brain.role === "admin" && (
            <Button
              variant="default"
              w="fit-content"
              leftSection={<Pencil size={16} />}
              onClick={() => setEditing(true)}
            >
              Edit Brain
            </Button>
          )}
        </Stack>
      </SettingsSection>
      {brain.role === "admin" && (
        <SettingsSection
          title={brain.archived ? "Reopen this Brain" : "Archive this Brain"}
          description="Archiving preserves its content and history. It can be reopened later."
        >
          <Button
            variant="default"
            leftSection={
              brain.archived ? <RotateCcw size={16} /> : <Archive size={16} />
            }
            onClick={() => setConfirming(true)}
          >
            {brain.archived ? "Reopen Brain" : "Archive Brain"}
          </Button>
        </SettingsSection>
      )}
      {brain.role === "admin" && <DeleteBrainSection brain={brain} />}
      <BrainForm
        key={brain.updated_at}
        brain={brain}
        opened={editing}
        close={() => setEditing(false)}
        saved={() => {
          setEditing(false);
          void cache.invalidateQueries({ queryKey: ["brain", brain.id] });
          void cache.invalidateQueries({ queryKey: ["brains"] });
        }}
      />
      <Modal
        opened={confirming}
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
        {result.backup_days} days. Offline host copies are removed on their
        next check-in.
      </Text>
    </Stack>
  );
}
