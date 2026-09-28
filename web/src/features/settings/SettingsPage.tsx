import { useState } from "react";
import { Alert, Badge, Button, Group, Modal, Stack, Text } from "@mantine/core";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Archive, Pencil, RotateCcw } from "lucide-react";
import { client, result } from "../../api";
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
            <AdvancedSettings label="Advanced retention and storage controls">
              <Stack gap="xl">
                <SourcesStoragePolicy brain={brain} />
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
