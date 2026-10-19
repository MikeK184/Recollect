import { useEffect, useState } from "react";
import "./features/feature-views.css";
import "./features/settings/ai-settings.css";
import {
  Alert,
  Badge,
  Button,
  Card,
  Divider,
  Group,
  Loader,
  Modal,
  MultiSelect,
  NumberInput,
  Select,
  Stack,
  Switch,
  Text,
  TextInput,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import { ClaimDialog } from "./ClaimsPanel";
import { usePollInterval } from "./usePollInterval";
import { useWorkspaceCatalogue, useEvidenceList } from "./catalogueQueries";
import { Failure, Pages, label, time } from "./ModelsShared";
import {
  AiPermissionsCard,
  DiagnosticsModal,
  MoreOptionsCard,
  SelectedModelsCard,
  UsageSection,
} from "./ModelsSections";

type Policy = components["schemas"]["ModelPolicy"];
type Settings = components["schemas"]["ModelSettings"];
type Run = components["schemas"]["LearningRun"];
type Selection = components["schemas"]["ScopeSelection"];
const classes = [
  "document",
  "raw_session",
  "tool_output",
  "support_excerpt",
  "repository",
  "claim",
  "query",
];
const sourceClasses = classes.slice(0, 4);
const choices = (values: string[]) =>
  values.map((value) => ({ value, label: label(value) }));
function PolicyEditor({
  brain,
  settings,
  onClose,
}: {
  brain: string;
  settings: Settings;
  onClose: () => void;
}) {
  const [draft, setDraft] = useState<Policy>(
    structuredClone(settings.current.policy),
  );
  const [base] = useState(settings.current.change_id);
  const [properties, setProperties] = useState(
    settings.current.policy.acceptance?.properties.join(", ") ?? "",
  );
  const command = useIdempotency();
  const cache = useQueryClient();
  const groups = useEvidenceList(brain);
  const patch = (value: Partial<Policy>) => setDraft({ ...draft, ...value });
  const save = useMutation({
    mutationFn: async () => {
      const body = { base_change: base, policy: draft };
      return result(
        await client.PUT("/api/brains/{brain}/models/policy", {
          params: { path: { brain } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: async () => {
      await cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain),
      });
      onClose();
    },
  });
  const limits: {
    key:
      | "daily_token_limit"
      | "max_concurrent"
      | "max_input_bytes"
      | "max_output_tokens";
    name: string;
    min: number;
    max: number;
  }[] = [
    {
      key: "daily_token_limit",
      name: "Daily token allowance",
      min: 1000,
      max: 10000000,
    },
    { key: "max_concurrent", name: "Concurrent model calls", min: 1, max: 4 },
    {
      key: "max_input_bytes",
      name: "Maximum input bytes",
      min: 256,
      max: 32768,
    },
    {
      key: "max_output_tokens",
      name: "Maximum output tokens",
      min: 128,
      max: 4096,
    },
  ];
  return (
    <Modal opened onClose={onClose} title="AI permissions" size="lg">
      <Stack>
        <Text size="sm">
          Choose what this Brain may send to the installed provider. Capture
          permission is separate.
        </Text>
        <Switch
          label="Allow model transmission"
          description={
            draft.text_model ? undefined : "Choose a text model first."
          }
          disabled={!draft.text_model}
          checked={draft.enabled}
          onChange={(e) =>
            patch({
              enabled: e.currentTarget.checked,
              ...(!e.currentTarget.checked
                ? { automatic_learning: false, automatic_embedding: false }
                : {}),
            })
          }
        />
        <MultiSelect
          searchable
          label="Permitted processing"
          data={[
            ...choices(["extraction", "synthesis", "embedding", "reranking"]),
            { value: "answering", label: "Answering — evidence-backed Ask" },
          ]}
          value={draft.purposes}
          onChange={(purposes) =>
            patch({
              purposes,
              ...(!purposes.includes("extraction")
                ? { automatic_learning: false }
                : {}),
              ...(!purposes.includes("embedding")
                ? { automatic_embedding: false }
                : {}),
            })
          }
        />
        <MultiSelect
          searchable
          label="Allowed content"
          data={choices(classes)}
          value={draft.content_classes}
          onChange={(content_classes) => patch({ content_classes })}
        />
        <Text size="xs" c="dimmed">
          Query permission is also required for Ask, semantic search and the
          fixed synthetic connection check. Answering is a separate permission
          and does not run tools or save conversations.
        </Text>
        <details className="feature-advanced">
          <summary>Resource limits and automatic processing</summary>
          <Stack mt="md">
            {" "}
            <TextInput
              label="Text model"
              placeholder="Not selected"
              value={draft.text_model}
              readOnly
            />
            <TextInput
              label="Embedding model"
              value={draft.embedding_model}
              readOnly
            />
            <Text size="sm">
              {draft.embedding_dimensions.toLocaleString()} embedding dimensions
              · {settings.installed.endpoint}
            </Text>
            {limits.map((field) => (
              <NumberInput
                key={field.key}
                label={field.name}
                value={draft[field.key]}
                min={field.min}
                max={field.max}
                allowDecimal={false}
                onChange={(value) => patch({ [field.key]: Number(value) })}
              />
            ))}
            <Switch
              label="Build semantic search automatically"
              checked={draft.automatic_embedding ?? false}
              disabled={!draft.enabled || !draft.purposes.includes("embedding")}
              onChange={(e) =>
                patch({ automatic_embedding: e.currentTarget.checked })
              }
            />
            <Text size="sm">
              Index existing and newly eligible content using the approved
              embedding model and content classes. Processing runs in the
              background without per-record review.
            </Text>
            <Switch
              label="Maintain memory autonomously"
              checked={draft.autonomous_memory}
              onChange={(e) => {
                const autonomous_memory = e.currentTarget.checked;
                patch({ autonomous_memory });
              }}
            />
            <Text size="sm">
              Automatically learn permitted sources, reconcile changes and
              refresh generated handovers. Supported results are accepted by
              policy; uncertain evidence stays labeled. Human review is
              optional.
            </Text>
            {!draft.autonomous_memory && (
              <>
                <Switch
                  label="Learn from newly processed sources automatically"
                  checked={draft.automatic_learning}
                  disabled={
                    draft.autonomous_memory ||
                    !draft.enabled ||
                    !draft.purposes.includes("extraction")
                  }
                  onChange={(e) =>
                    patch({ automatic_learning: e.currentTarget.checked })
                  }
                />
                <Divider label="Automatic acceptance" />
                <Switch
                  label="Accept permitted literal configuration facts"
                  checked={!!draft.acceptance}
                  onChange={(e) =>
                    patch({
                      acceptance: e.currentTarget.checked
                        ? {
                            name: "literal-config",
                            source_classes: ["document"],
                            collection_ids: [],
                            properties: [],
                          }
                        : null,
                    })
                  }
                />
                {draft.acceptance && (
                  <>
                    <Text size="sm">
                      Only exact declarations such as Amber.port = 8080 can
                      qualify. In explicit mode, other interpretations remain
                      proposals. Acceptance records this policy and does not
                      create a human reviewer.
                    </Text>
                    <TextInput
                      label="Acceptance rule name"
                      value={draft.acceptance.name}
                      onChange={(e) =>
                        patch({
                          acceptance: {
                            ...draft.acceptance!,
                            name: e.currentTarget.value,
                          },
                        })
                      }
                    />
                    <TextInput
                      label="Allowed literal properties"
                      description="Comma-separated property names, for example port, protocol"
                      value={properties}
                      onChange={(e) => {
                        setProperties(e.currentTarget.value);
                        patch({
                          acceptance: {
                            ...draft.acceptance!,
                            properties: e.currentTarget.value
                              .split(",")
                              .map((v) => v.trim())
                              .filter(Boolean),
                          },
                        });
                      }}
                    />
                    <MultiSelect
                      searchable
                      label="Literal source classes"
                      data={choices(sourceClasses)}
                      value={draft.acceptance.source_classes}
                      onChange={(source_classes) =>
                        patch({
                          acceptance: { ...draft.acceptance!, source_classes },
                        })
                      }
                    />
                    <MultiSelect
                      searchable
                      label="Literal source collections"
                      description="Empty means all collections in this Brain."
                      data={
                        groups.data?.groups
                          .filter((g) => g.kind === "collection")
                          .map((g) => ({ value: g.id, label: g.name })) ?? []
                      }
                      value={draft.acceptance.collection_ids}
                      onChange={(collection_ids) =>
                        patch({
                          acceptance: { ...draft.acceptance!, collection_ids },
                        })
                      }
                    />
                    <Failure error={groups.error} />
                  </>
                )}
              </>
            )}
          </Stack>
        </details>
        <Failure error={save.error} />
        <Button loading={save.isPending} onClick={() => save.mutate()}>
          Save AI permissions
        </Button>
      </Stack>
    </Modal>
  );
}
function PolicyHistory({
  brain,
  onClose,
}: {
  brain: string;
  onClose: () => void;
}) {
  const [offset, setOffset] = useState(0);
  const history = useQuery({
    queryKey: ["model-policy-history", brain, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/models/policy/history", {
          params: { path: { brain }, query: { offset } },
        }),
      ),
  });
  return (
    <Modal opened onClose={onClose} title="Model policy history" size="lg">
      <Stack>
        <Failure error={history.error} />
        {history.isPending && <Loader />}
        {history.data?.length === 0 && <Text>No saved policy revisions.</Text>}
        {history.data?.map((p) => (
          <Card key={p.change_id} withBorder>
            <Text fw={600}>
              {p.created_at ? time(p.created_at) : "Initial policy"} ·{" "}
              {p.policy.enabled ? "Enabled" : "Disabled"}
            </Text>
            <Text size="sm">
              {p.policy.text_model || "No text model selected"} ·{" "}
              {p.policy.embedding_model}
            </Text>
            <Text size="sm">
              Purposes: {p.policy.purposes.join(", ") || "none"}. Content:{" "}
              {p.policy.content_classes.map(label).join(", ") || "none"}.
            </Text>
            <Text size="xs">
              Acceptance:{" "}
              {p.policy.autonomous_memory
                ? "Autonomous evidence policy"
                : (p.policy.acceptance?.name ?? "Explicit proposals")}
            </Text>
          </Card>
        ))}
        <Group>
          <Button
            variant="subtle"
            disabled={!offset}
            onClick={() => setOffset(Math.max(0, offset - 20))}
          >
            Newer policies
          </Button>
          <Button
            variant="subtle"
            disabled={(history.data?.length ?? 0) < 20}
            onClick={() => setOffset(offset + 20)}
          >
            Older policies
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
export function LearningAction({
  brain,
  version,
  writable,
}: {
  brain: string;
  version: string;
  writable: boolean;
}) {
  const [opened, setOpened] = useState(false);
  return writable ? (
    <>
      <Button variant="light" onClick={() => setOpened(true)}>
        Learn from this source
      </Button>
      {opened && (
        <LearningEditor
          brain={brain}
          version={version}
          onClose={() => setOpened(false)}
        />
      )}
    </>
  ) : null;
}
function LearningEditor({
  brain,
  version,
  onClose,
}: {
  brain: string;
  version: string;
  onClose: () => void;
}) {
  const [selection, setSelection] = useState<Selection>({
    repository_ids: [],
    area_ids: [],
    environment_id: null,
  });
  const [manifest, setManifest] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const cache = useQueryClient();
  const command = useIdempotency();
  const catalogue = useWorkspaceCatalogue(brain);
  const manifests = useQuery({
    queryKey: ["learning-manifests", brain, selection.environment_id, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/revision-manifests", {
          params: {
            path: { brain },
            query: {
              environment_id: selection.environment_id ?? undefined,
              offset,
            },
          },
        }),
      ),
  });
  const save = useMutation({
    mutationFn: async () => {
      const body = {
        source_version_id: version,
        selection,
        manifest_revision_id: manifest,
        operation_id: null,
      };
      return result(
        await client.POST("/api/brains/{brain}/learning", {
          params: { path: { brain } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: () => {
      // A completed run is spent; a deliberate second run is a new command.
      command.reset();
      return cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain),
      });
    },
  });
  return (
    <Modal opened onClose={onClose} title="Learn from source" size="lg">
      <Stack>
        <Text size="sm">
          Extract supported claims from this exact retained version using the
          Brain's model policy. Empty applicability means the whole Brain.
          Supported results are accepted under autonomous policy; uncertainty
          remains labeled. Legacy explicit mode uses its configured acceptance
          rules.
        </Text>
        <MultiSelect
          searchable
          label="Learning repositories"
          data={
            catalogue.data?.repositories.map((r) => ({
              value: r.id,
              label: r.canonical_origin,
            })) ?? []
          }
          value={selection.repository_ids ?? []}
          onChange={(repository_ids) =>
            setSelection({ ...selection, repository_ids })
          }
        />
        <MultiSelect
          searchable
          label="Learning areas"
          data={
            catalogue.data?.areas.map((r) => ({
              value: r.id,
              label: r.name,
            })) ?? []
          }
          value={selection.area_ids ?? []}
          onChange={(area_ids) => setSelection({ ...selection, area_ids })}
        />
        <Select
          label="Learning environment"
          clearable
          data={
            catalogue.data?.environments.map((r) => ({
              value: r.id,
              label: r.name,
            })) ?? []
          }
          value={selection.environment_id}
          onChange={(environment_id) => {
            setSelection({ ...selection, environment_id });
            setManifest(null);
            setOffset(0);
          }}
        />
        <Select
          label="Learning manifest revision"
          clearable
          data={
            manifests.data?.items.map((m) => ({
              value: m.id,
              label: m.name + " · " + time(m.created_at),
            })) ?? []
          }
          value={manifest}
          onChange={(value) => {
            setManifest(value);
            const entry = manifests.data?.items.find((m) => m.id === value);
            if (entry)
              setSelection({
                ...selection,
                environment_id: entry.environment_id,
              });
          }}
        />
        <Pages
          offset={offset}
          total={manifests.data?.total ?? 0}
          change={setOffset}
        />
        <Failure error={catalogue.error ?? manifests.error ?? save.error} />
        {save.data ? (
          <Alert color="teal">
            Learning queued. Results and review links appear in the Model
            learning panel.
          </Alert>
        ) : (
          <Button loading={save.isPending} onClick={() => save.mutate()}>
            Queue learning
          </Button>
        )}
      </Stack>
    </Modal>
  );
}
function LearningRows({
  brain,
  onInspect,
}: {
  brain: Brain;
  onInspect: (id: string) => void;
}) {
  const pollFast = usePollInterval("fast");
  const [offset, setOffset] = useState(0);
  const cache = useQueryClient();
  const command = useIdempotency();
  const query = useQuery({
    queryKey: ["learning", brain.id, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/learning", {
          params: { path: { brain: brain.id }, query: { offset } },
        }),
      ),
    refetchInterval: pollFast,
  });
  const retry = useMutation({
    mutationFn: async (run: Run) => {
      const body = { operation_id: null };
      return result(
        await client.POST("/api/brains/{brain}/learning/{run}/retry", {
          params: { path: { brain: brain.id, run: run.id } },
          body,
          headers: {
            "Idempotency-Key": command.forInput({ run: run.id, ...body }),
          },
        }),
      );
    },
    onSuccess: async () => {
      command.reset();
      await cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain.id),
      });
    },
  });
  return (
    <Stack>
      <Failure error={query.error ?? retry.error} />
      {query.isPending && <Loader />}
      {query.data?.total === 0 && (
        <Text size="sm">
          No learning runs. Open retained source evidence to start one.
        </Text>
      )}
      {!query.error &&
        query.data?.items.map((run) => (
          <Card key={run.id} withBorder>
            <Stack gap="xs">
              <Group justify="space-between">
                <Text fw={600}>
                  {run.automatic ? "Automatic learning" : "Source learning"}
                </Text>
                <Badge
                  color={
                    run.state === "succeeded"
                      ? "teal"
                      : run.state === "failed"
                        ? "red"
                        : "gray"
                  }
                >
                  {label(run.state)}
                </Badge>
              </Group>
              <Text size="xs" c="dimmed">
                {time(run.created_at)}
              </Text>
              {run.error_code && (
                <Text size="sm" c="red">
                  {label(run.error_code)}.{" "}
                  {run.state === "removed"
                    ? "The input cannot be restored by retrying."
                    : run.error_code === "model_budget_exhausted"
                      ? "The allowance resets at midnight UTC. Adjust it in AI permissions → Processing limits."
                      : run.error_code === "model_input_too_large"
                        ? "Use a smaller retained excerpt or adjust Maximum input bytes in AI permissions."
                        : "Inspect the policy and source before starting another attempt."}
                </Text>
              )}
              {run.state === "succeeded" && (
                <>
                  <Text size="sm">
                    {run.accepted} accepted by policy · {run.proposed} proposed
                    · {run.blocked} blocked · {run.conflicting} conflicting ·{" "}
                    {run.reused} existing claims reused · {run.revised ?? 0}{" "}
                    revised · {run.retired ?? 0} retired
                  </Text>
                  {run.claim_ids.length === 0 && (
                    <Text size="sm">The model found no supported claims.</Text>
                  )}
                </>
              )}
              <Group>
                {run.claim_ids.map((id, index) => (
                  <Button
                    key={id}
                    variant="subtle"
                    size="compact-sm"
                    onClick={() => onInspect(id)}
                  >
                    Inspect learned claim {index + 1}
                  </Button>
                ))}
              </Group>
              {["failed", "cancelled"].includes(run.state) &&
                !brain.archived &&
                ["admin", "writer"].includes(brain.role) && (
                  <Button
                    variant="light"
                    loading={retry.isPending && retry.variables?.id === run.id}
                    onClick={() => retry.mutate(run)}
                  >
                    Start new learning attempt
                  </Button>
                )}
            </Stack>
          </Card>
        ))}
      <Pages
        offset={offset}
        total={query.data?.total ?? 0}
        change={setOffset}
      />
    </Stack>
  );
}
export function ModelsPanel({
  brain,
  section = "settings",
}: {
  brain: Brain;
  section?: "settings" | "activity";
}) {
  const pollSlow = usePollInterval("slow");
  const pollNormal = usePollInterval("normal");
  const [editing, setEditing] = useState(false);
  const [advancedEditing, setAdvancedEditing] = useState(false);
  const [draft, setDraft] = useState<Policy | null>(null);
  const [base, setBase] = useState<string | null>(null);
  const [history, setHistory] = useState(false);
  const [diagnostics, setDiagnostics] = useState(false);
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const cache = useQueryClient();
  const command = useIdempotency();
  const settings = useQuery({
    queryKey: ["models-policy", brain.id],
    enabled: section === "settings",
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/models/policy", {
          params: { path: { brain: brain.id } },
        }),
      ),
    refetchInterval: pollSlow,
  });
  const usage = useQuery({
    queryKey: ["models-usage", brain.id, offset],
    enabled: section === "activity",
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/models/usage", {
          params: { path: { brain: brain.id }, query: { offset } },
        }),
      ),
    refetchInterval: pollNormal,
  });
  const catalogueProvider =
    draft?.provider ?? settings.data?.current.policy.provider ?? "openai";
  const modelCatalogue = useQuery({
    queryKey: ["model-catalogue", brain.id, catalogueProvider],
    enabled: section === "settings",
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/models/catalogue", {
          params: {
            path: { brain: brain.id },
            query: { provider: catalogueProvider },
          },
        }),
      ),
    retry: false,
  });
  const refreshModels = useMutation({
    mutationFn: async (provider: string) =>
      result(
        await client.POST("/api/brains/{brain}/models/catalogue", {
          params: { path: { brain: brain.id }, query: { provider } },
        }),
      ),
    onSuccess: (data, provider) =>
      cache.setQueryData(["model-catalogue", brain.id, provider], data),
  });
  const save = useMutation({
    mutationFn: async () => {
      if (!draft || !base || !settings.data || settings.isError)
        throw new Error("Reload AI permissions before saving.");
      const previous = settings.data.current.policy;
      const body = {
        base_change: base,
        policy: draft,
        rebuild_embeddings:
          previous.provider !== draft.provider ||
          previous.embedding_model !== draft.embedding_model ||
          previous.embedding_dimensions !== draft.embedding_dimensions,
      };
      return result(
        await client.PUT("/api/brains/{brain}/models/policy", {
          params: { path: { brain: brain.id } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: async () => {
      command.reset();
      setDraft(null);
      setBase(null);
      setEditing(false);
      await cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain.id),
      });
    },
  });
  const catalogue = useWorkspaceCatalogue(brain.id, undefined, { enabled: !!selected });
  const check = useMutation({
    mutationFn: async (operation_id: string) =>
      result(
        await client.POST("/api/brains/{brain}/models/check", {
          params: { path: { brain: brain.id } },
          body: { operation_id },
        }),
      ),
    onSettled: () =>
      cache.invalidateQueries({ queryKey: ["models-usage", brain.id] }),
  });
  useEffect(() => {
    setEditing(false);
    setAdvancedEditing(false);
    setDraft(null);
    setBase(null);
    setHistory(false);
    setDiagnostics(false);
    setSelected(null);
    setOffset(0);
  }, [brain.id]);
  useEffect(() => {
    if (brain.role !== "admin" || brain.archived || settings.isError) {
      setEditing(false);
      setDraft(null);
      setBase(null);
      setAdvancedEditing(false);
      setHistory(false);
      setDiagnostics(false);
    }
  }, [brain.role, brain.archived, settings.isError]);
  const policy = settings.error
    ? undefined
    : (draft ?? settings.data?.current.policy);
  const installed = settings.error
    ? undefined
    : (settings.data?.providers?.find((p) => p.provider === policy?.provider) ??
      settings.data?.installed);
  const editable =
    editing && brain.role === "admin" && !brain.archived && !save.isPending;
  const updateDraft = (patch: Partial<Policy>) => {
    save.reset();
    setDraft((value) => (value ? { ...value, ...patch } : value));
  };
  const togglePurpose = (purpose: string, allowed: boolean) => {
    if (!policy) return;
    const purposes = allowed
      ? [...policy.purposes, purpose]
      : policy.purposes.filter((v) => v !== purpose);
    updateDraft({
      purposes,
      ...(!purposes.includes("extraction")
        ? { automatic_learning: false }
        : {}),
      ...(!purposes.includes("embedding")
        ? { automatic_embedding: false }
        : {}),
      ...(!["extraction", "synthesis"].every((v) => purposes.includes(v))
        ? { autonomous_memory: false }
        : {}),
    });
  };
  const changedEmbedding =
    !!draft &&
    !!settings.data &&
    (draft.provider !== settings.data.current.policy.provider ||
      draft.embedding_model !== settings.data.current.policy.embedding_model ||
      draft.embedding_dimensions !==
        settings.data.current.policy.embedding_dimensions);

  const startEditing = () => {
    if (settings.data) {
      setDraft(structuredClone(settings.data.current.policy));
      setBase(settings.data.current.change_id);
      setEditing(true);
      save.reset();
      if (
        !modelCatalogue.data ||
        modelCatalogue.data.stale ||
        !modelCatalogue.data.observed_at ||
        Date.now() - Date.parse(modelCatalogue.data.observed_at) >= 300_000
      )
        refreshModels.mutate(catalogueProvider);
    }
  };
  const cancelEditing = () => {
    setDraft(null);
    setBase(null);
    setEditing(false);
    save.reset();
    command.reset();
  };
  return (
    <section
      className={
        section === "settings" ? "models-management" : "feature-setting"
      }
      id="models"
    >
      <Stack>
        {section === "settings" && (
          <>
            <Failure error={settings.error} />
            {settings.isPending && <Loader size="sm" />}
            {policy && installed && (
              <div className="management-split">
                <div className="management-side">
                  <AiPermissionsCard
                    brain={brain}
                    policy={policy}
                    installed={installed}
                    editing={editing}
                    editable={editable}
                    saving={save.isPending}
                    saveError={save.error}
                    changedEmbedding={changedEmbedding}
                    onEdit={startEditing}
                    onCancel={cancelEditing}
                    onSave={() => save.mutate()}
                    updateDraft={updateDraft}
                    togglePurpose={togglePurpose}
                  />
                </div>
                <div className="management-side">
                  <SelectedModelsCard
                    brain={brain}
                    policy={policy}
                    installed={installed}
                    providers={settings.data?.providers}
                    modelCatalogue={modelCatalogue.data}
                    catalogueError={modelCatalogue.error ?? refreshModels.error}
                    catalogueProvider={catalogueProvider}
                    refreshing={refreshModels.isPending}
                    editable={editable}
                    changedEmbedding={changedEmbedding}
                    updateDraft={updateDraft}
                    refreshModels={(provider) => refreshModels.mutate(provider)}
                  />
                  <MoreOptionsCard
                    brain={brain}
                    editing={editing}
                    onDiagnostics={() => setDiagnostics(true)}
                    onHistory={() => setHistory(true)}
                    onAdvanced={() => setAdvancedEditing(true)}
                  />
                </div>
              </div>
            )}
            <DiagnosticsModal
              brain={brain}
              diagnostics={diagnostics}
              failed={!!settings.error}
              installed={installed}
              policy={policy}
              checkPending={check.isPending}
              checkError={check.error}
              checkPassed={!!check.data}
              onClose={() => setDiagnostics(false)}
              onHistory={() => setHistory(true)}
              onCheck={() => check.mutate(crypto.randomUUID())}
            />
          </>
        )}
        {section === "activity" && (
          <UsageSection
            usage={usage.data}
            usageError={usage.error}
            usagePending={usage.isPending}
            offset={offset}
            onOffset={setOffset}
            learning={<LearningRows brain={brain} onInspect={setSelected} />}
          />
        )}
        {advancedEditing &&
          brain.role === "admin" &&
          !brain.archived &&
          settings.data &&
          !settings.error && (
            <PolicyEditor
              brain={brain.id}
              settings={settings.data}
              onClose={() => setAdvancedEditing(false)}
            />
          )}
        {history && (
          <PolicyHistory brain={brain.id} onClose={() => setHistory(false)} />
        )}
        {selected && (
          <ClaimDialog
            brain={brain}
            id={selected}
            catalogue={catalogue.data}
            onClose={() => setSelected(null)}
            onSaved={() => {
              void cache.invalidateQueries({
                predicate: (q) => q.queryKey.includes(brain.id),
              });
            }}
          />
        )}
      </Stack>
    </section>
  );
}
