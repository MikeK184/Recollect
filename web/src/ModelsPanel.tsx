import {
  Check,
  Minus,
  ShieldCheck,
  MessageSquare,
  Search,
  Sparkles,
  RefreshCw,
  BookOpen,
  List,
  Database,
  History,
  FileText,
  ChevronRight,
  Pencil,
  Info,
} from "lucide-react";
import { HostIcon } from "./components/HostIcon";
import { SemanticPanel } from "./SemanticPanel";
import { useEffect, useState } from "react";
import "./features/feature-views.css";
import "./features/settings/ai-settings.css";
import {
  Alert,
  ActionIcon,
  Badge,
  Button,
  Card,
  Checkbox,
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
  Title,
  Tooltip,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import { ClaimDialog } from "./ClaimsPanel";

type Policy = components["schemas"]["ModelPolicy"];
type Settings = components["schemas"]["ModelSettings"];
type Run = components["schemas"]["LearningRun"];
type Selection = components["schemas"]["ScopeSelection"];
const label = (s: string) =>
  (
    ({
      extraction: "Source learning",
      synthesis: "Memory maintenance",
      embedding: "Semantic search",
      reranking: "Result ordering",
      answering: "Ask answers",
      document: "Documents",
      raw_session: "Captured sessions",
      tool_output: "Tool results",
      support_excerpt: "Supporting excerpts",
      repository: "Repository text",
      claim: "Memory",
      query: "Questions",
      provider_incomplete: "The model reached its output limit",
      provider_shape: "The model returned an invalid structured result",
      model_input_too_large: "The source exceeds the input byte limit",
      model_input_sensitive: "The source contains sensitive input",
      model_budget_exhausted: "The request exceeds the remaining daily token allowance",
    }) as Record<string, string>
  )[s] ?? s.replaceAll("_", " ");
const time = (s: string) => new Date(s).toLocaleString();
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
function Failure({ error }: { error: Error | null }) {
  return error ? <Alert color="red">{error.message}</Alert> : null;
}
function Pages({
  offset,
  total,
  change,
}: {
  offset: number;
  total: number;
  change: (v: number) => void;
}) {
  return total > 20 ? (
    <Group>
      <Button
        variant="subtle"
        disabled={!offset}
        onClick={() => change(Math.max(0, offset - 20))}
      >
        Previous
      </Button>
      <Text size="xs">
        {offset + 1}–{Math.min(total, offset + 20)} of {total}
      </Text>
      <Button
        variant="subtle"
        disabled={offset + 20 >= total}
        onClick={() => change(offset + 20)}
      >
        Next
      </Button>
    </Group>
  ) : null;
}
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
  const groups = useQuery({
    queryKey: ["model-collections", brain],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          params: { path: { brain } },
        }),
      ),
  });
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
            <TextInput label="Text model" value={draft.text_model} readOnly />
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
              {p.policy.text_model} · {p.policy.embedding_model}
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
  const catalogue = useQuery({
    queryKey: ["workspace", brain],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain } },
        }),
      ),
  });
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
    onSuccess: () =>
      cache.invalidateQueries({ predicate: (q) => q.queryKey.includes(brain) }),
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
    refetchInterval: 2000,
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
    refetchInterval: 4000,
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
    refetchInterval: 3000,
  });
  const modelCatalogue = useQuery({
    queryKey: ["model-catalogue", brain.id],
    enabled: section === "settings",
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/models/catalogue", {
          params: { path: { brain: brain.id } },
        }),
      ),
    retry: false,
  });
  const refreshModels = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/brains/{brain}/models/catalogue", {
          params: { path: { brain: brain.id } },
        }),
      ),
    onSuccess: (data) =>
      cache.setQueryData(["model-catalogue", brain.id], data),
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
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id],
    enabled: !!selected,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
        }),
      ),
  });
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
  const installed = settings.error ? undefined : settings.data?.installed;
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
    (draft.embedding_model !== settings.data.current.policy.embedding_model ||
      draft.embedding_dimensions !==
        settings.data.current.policy.embedding_dimensions);
  const modelChoices = (kind: string, selected: string) => {
    const rows =
      modelCatalogue.data?.models.filter(
        (m) => m.kind === kind && (m.available || m.id === selected),
      ) ?? [];
    const options = rows.map((m) => ({
      value: m.id,
      label: m.id,
      disabled: !m.selectable && m.id !== selected,
    }));
    if (!options.some((m) => m.value === selected))
      options.unshift({ value: selected, label: selected, disabled: false });
    return options;
  };
  const price = (id: string) => {
    const entry = modelCatalogue.data?.models.find((m) => m.id === id);
    if (!entry)
      return <p className="management-muted ai-model-price">Price unknown</p>;
    const amount = (value: number | null | undefined) =>
      value == null ? "Unknown" : `$${value.toFixed(2)}`;
    return (
      <p className="management-muted ai-model-price">
        {amount(entry.input_usd_per_million)} input
        {entry.kind === "text"
          ? ` · ${amount(entry.cached_input_usd_per_million)} cached · ${amount(entry.output_usd_per_million)} output`
          : ""}{" "}
        / 1M tokens
        <a
          href={entry.source_url}
          target="_blank"
          rel="noreferrer"
          title="Standard short-context rates; batch, regional and long-context pricing can differ."
        >
          Standard · {entry.checked_on}
          {entry.pricing_stale ? " · stale" : ""}
        </a>
      </p>
    );
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
                  <section className="management-surface">
                    <div className="management-section-heading">
                      <div>
                        <h2 className="management-title ai-policy-title">
                          AI permissions
                        </h2>
                        <p className="management-muted">
                          Choose what this Brain may send to your installed AI
                          provider.
                        </p>
                      </div>
                      {brain.role === "admin" && !editing && (
                        <Button
                          variant="default"
                          leftSection={<Pencil size={17} />}
                          disabled={brain.archived}
                          onClick={() => {
                            if (settings.data) {
                              setDraft(
                                structuredClone(settings.data.current.policy),
                              );
                              setBase(settings.data.current.change_id);
                              setEditing(true);
                              save.reset();
                              if (
                                !modelCatalogue.data ||
                                modelCatalogue.data.stale ||
                                !modelCatalogue.data.observed_at ||
                                Date.now() -
                                  Date.parse(modelCatalogue.data.observed_at) >=
                                  300_000
                              )
                                refreshModels.mutate();
                            }
                          }}
                        >
                          Edit AI permissions
                        </Button>
                      )}
                      {editing && (
                        <Group gap="xs" className="ai-inline-actions">
                          <Button
                            variant="default"
                            disabled={save.isPending}
                            onClick={() => {
                              setDraft(null);
                              setBase(null);
                              setEditing(false);
                              save.reset();
                              command.reset();
                            }}
                          >
                            Cancel
                          </Button>
                          <Button
                            loading={save.isPending}
                            onClick={() => save.mutate()}
                          >
                            {changedEmbedding
                              ? "Save and rebuild"
                              : "Save AI permissions"}
                          </Button>
                        </Group>
                      )}
                    </div>
                    <Failure error={save.error} />
                    <div className="ai-installed-provider">
                      <strong>Installed provider</strong>
                      <span className="ai-provider-identity">
                        <HostIcon host={installed.provider} size={26} />
                        <span>
                          {installed.provider === "openai"
                            ? "OpenAI"
                            : installed.provider}
                        </span>
                      </span>
                      <Switch
                        label="Allow AI processing"
                        checked={policy.enabled}
                        disabled={!editable}
                        onChange={(event) =>
                          updateDraft({
                            enabled: event.currentTarget.checked,
                            ...(!event.currentTarget.checked
                              ? {
                                  automatic_learning: false,
                                  automatic_embedding: false,
                                }
                              : {}),
                          })
                        }
                      />
                    </div>
                    <div className="ai-automatic-memory">
                      <RefreshCw size={37} />
                      <div>
                        <strong>Automatic memory</strong>
                        <p className="management-muted">
                          Learning and maintenance follow this Brain’s standing
                          policy.
                        </p>
                      </div>
                      <Switch
                        label="Automatic memory"
                        checked={policy.autonomous_memory && policy.enabled}
                        disabled={
                          !editable ||
                          !policy.enabled ||
                          !["extraction", "synthesis"].every((p) =>
                            policy.purposes.includes(p),
                          ) ||
                          !policy.content_classes.includes("claim")
                        }
                        onChange={(event) =>
                          updateDraft({
                            autonomous_memory: event.currentTarget.checked,
                            automatic_learning: event.currentTarget.checked,
                          })
                        }
                      />
                    </div>
                    <h3 className="ai-processing-title">Allowed processing</h3>
                    {[
                      {
                        purpose: "answering",
                        title: "Answer questions",
                        description:
                          "Use this Brain’s evidence to answer with citations.",
                        icon: MessageSquare,
                      },
                      {
                        purpose: "embedding",
                        title: "Find related knowledge",
                        description:
                          "Index approved content for semantic search.",
                        icon: Search,
                      },
                      {
                        purpose: "extraction",
                        title: "Learn from sources",
                        description:
                          "Extract evidence-backed memory into this Brain.",
                        icon: BookOpen,
                      },
                      {
                        purpose: "reranking",
                        title: "Order search results",
                        description:
                          "Use the provider to rank and order retrieved results.",
                        icon: List,
                      },
                      {
                        purpose: "synthesis",
                        title: "Maintain memory",
                        description:
                          "Summarize, organize and maintain this Brain’s memory.",
                        icon: Database,
                      },
                    ].map((row) => (
                      <div key={row.purpose} className="management-row">
                        <row.icon size={20} />
                        <div className="management-row-main">
                          <strong>{row.title}</strong>
                          <p className="management-muted">{row.description}</p>
                        </div>
                        <Checkbox
                          label={
                            policy.purposes.includes(row.purpose)
                              ? "Allowed"
                              : "Disallowed"
                          }
                          aria-label={row.title}
                          checked={policy.purposes.includes(row.purpose)}
                          disabled={!editable}
                          onChange={(event) =>
                            togglePurpose(
                              row.purpose,
                              event.currentTarget.checked,
                            )
                          }
                        />
                      </div>
                    ))}
                    <div className="management-content-policy">
                      <h3 className="management-title">Allowed content</h3>
                      <div className="management-chips">
                        {[
                          "claim",
                          "repository",
                          "document",
                          "support_excerpt",
                          "query",
                          "tool_output",
                          "raw_session",
                        ].map((value) => {
                          const allowed =
                            policy.enabled &&
                            policy.content_classes.includes(value);
                          return (
                            <label
                              key={value}
                              className={
                                "management-chip" + (allowed ? "" : " denied")
                              }
                            >
                              <Checkbox
                                aria-label={`Allow ${label(value)}`}
                                checked={policy.content_classes.includes(value)}
                                disabled={!editable}
                                onChange={(event) =>
                                  updateDraft({
                                    content_classes: event.currentTarget.checked
                                      ? [...policy.content_classes, value]
                                      : policy.content_classes.filter(
                                          (v) => v !== value,
                                        ),
                                    ...(value === "claim" &&
                                    !event.currentTarget.checked
                                      ? { autonomous_memory: false }
                                      : {}),
                                  })
                                }
                              />
                              {label(value)}
                            </label>
                          );
                        })}
                      </div>
                    </div>
                    <section
                      className="ai-policy-detail"
                      aria-label="Automatic processing"
                    >
                      <h3 className="ai-processing-title">
                        Automatic processing
                      </h3>
                      <Group mt="sm">
                        <Checkbox
                          label="Automatic source learning"
                          checked={policy.automatic_learning}
                          disabled={
                            !editable ||
                            !policy.enabled ||
                            !policy.purposes.includes("extraction")
                          }
                          onChange={(event) =>
                            updateDraft({
                              automatic_learning: event.currentTarget.checked,
                            })
                          }
                        />
                        <Checkbox
                          label="Automatic search indexing"
                          checked={policy.automatic_embedding}
                          disabled={
                            !editable ||
                            !policy.enabled ||
                            !policy.purposes.includes("embedding")
                          }
                          onChange={(event) =>
                            updateDraft({
                              automatic_embedding: event.currentTarget.checked,
                            })
                          }
                        />
                      </Group>
                    </section>
                    <section
                      className="ai-policy-detail ai-processing-limits"
                      aria-label="Processing limits"
                    >
                      <Group gap="xs">
                        <h3 className="ai-processing-title">
                          Processing limits
                        </h3>
                        <Tooltip
                          label="Per model call and per Brain. The daily allowance resets at midnight UTC."
                          multiline
                          w={260}
                          events={{ hover: true, focus: true, touch: true }}
                        >
                          <ActionIcon
                            type="button"
                            variant="subtle"
                            size="sm"
                            aria-label="About processing limits"
                          >
                            <Info size={16} />
                          </ActionIcon>
                        </Tooltip>
                      </Group>
                      <div className="ai-limits-grid">
                        <NumberInput
                          label="Maximum output tokens"
                          value={policy.max_output_tokens}
                          min={128}
                          max={4096}
                          step={256}
                          allowDecimal={false}
                          disabled={!editable}
                          onChange={(value) =>
                            updateDraft({ max_output_tokens: Number(value) })
                          }
                        />
                        <NumberInput
                          label="Maximum input bytes"
                          value={policy.max_input_bytes}
                          min={256}
                          max={32768}
                          step={1024}
                          allowDecimal={false}
                          disabled={!editable}
                          onChange={(value) =>
                            updateDraft({ max_input_bytes: Number(value) })
                          }
                        />
                        <NumberInput
                          label="Daily token allowance"
                          value={policy.daily_token_limit}
                          min={1000}
                          max={10000000}
                          step={1000}
                          allowDecimal={false}
                          disabled={!editable}
                          onChange={(value) =>
                            updateDraft({ daily_token_limit: Number(value) })
                          }
                        />
                        <NumberInput
                          label="Concurrent model calls"
                          value={policy.max_concurrent}
                          min={1}
                          max={4}
                          allowDecimal={false}
                          disabled={!editable}
                          onChange={(value) =>
                            updateDraft({ max_concurrent: Number(value) })
                          }
                        />
                      </div>
                    </section>
                  </section>
                </div>
                <div className="management-side">
                  <section className="management-surface ai-installed-models">
                    <h3 className="management-title">Selected models</h3>
                    <Select
                      label="Text model"
                      value={policy.text_model}
                      data={modelChoices("text", policy.text_model)}
                      disabled={!editable}
                      allowDeselect={false}
                      onChange={(value) =>
                        value && updateDraft({ text_model: value })
                      }
                    />
                    {price(policy.text_model)}
                    <Select
                      label="Embedding model"
                      value={policy.embedding_model}
                      data={modelChoices("embedding", policy.embedding_model)}
                      disabled={!editable}
                      allowDeselect={false}
                      onChange={(value) =>
                        value &&
                        updateDraft({
                          embedding_model: value,
                          embedding_dimensions:
                            modelCatalogue.data?.models.find(
                              (m) => m.id === value,
                            )?.max_dimensions ?? policy.embedding_dimensions,
                        })
                      }
                    />
                    {price(policy.embedding_model)}
                    <Select
                      label="Embedding dimensions"
                      value={String(policy.embedding_dimensions)}
                      disabled={!editable}
                      allowDeselect={false}
                      data={[
                        ...new Set([
                          256,
                          512,
                          1024,
                          1536,
                          3072,
                          policy.embedding_dimensions,
                        ]),
                      ]
                        .filter(
                          (n) =>
                            n <=
                            (modelCatalogue.data?.models.find(
                              (m) => m.id === policy.embedding_model,
                            )?.max_dimensions ?? policy.embedding_dimensions),
                        )
                        .sort((a, b) => a - b)
                        .map((n) => ({
                          value: String(n),
                          label: n.toLocaleString(),
                        }))}
                      onChange={(value) =>
                        value &&
                        updateDraft({ embedding_dimensions: Number(value) })
                      }
                    />
                    {brain.role === "admin" && !brain.archived && (
                      <Button
                        mt="sm"
                        size="compact-sm"
                        variant="subtle"
                        loading={refreshModels.isPending}
                        onClick={() => refreshModels.mutate()}
                      >
                        Refresh available models
                      </Button>
                    )}
                    <Failure
                      error={modelCatalogue.error ?? refreshModels.error}
                    />
                    {modelCatalogue.data?.error_code && (
                      <Text size="xs" c="red">
                        Model list unavailable. Current selection is retained.
                      </Text>
                    )}
                    <p className="management-muted ai-catalogue-state">
                      {modelCatalogue.data?.observed_at
                        ? `Availability checked ${time(modelCatalogue.data.observed_at)}${modelCatalogue.data.stale ? " · stale" : ""}`
                        : "Refresh to load models available to this account."}
                    </p>
                    {changedEmbedding && (
                      <Alert color="yellow" mt="sm">
                        Saving rebuilds search embeddings and may incur provider
                        usage. Exact and text search stay available.
                        {!policy.enabled ||
                        !policy.automatic_embedding ||
                        !policy.purposes.includes("embedding")
                          ? " Rebuilding waits until embedding permissions are enabled."
                          : ""}
                      </Alert>
                    )}
                  </section>
                  <section className="management-surface ai-more-options">
                    <h3 className="management-title">More options</h3>
                    <button onClick={() => setDiagnostics(true)}>
                      <FileText size={22} />
                      <span>Diagnostics</span>
                      <ChevronRight size={18} />
                    </button>
                    {brain.role === "admin" && (
                      <button onClick={() => setHistory(true)}>
                        <History size={22} />
                        <span>Policy history</span>
                        <ChevronRight size={18} />
                      </button>
                    )}
                    {brain.role === "admin" && !brain.archived && !editing && (
                      <button onClick={() => setAdvancedEditing(true)}>
                        <ShieldCheck size={22} />
                        <span>Advanced policy</span>
                        <ChevronRight size={18} />
                      </button>
                    )}
                    <button onClick={() => setDiagnostics(true)}>
                      <Database size={22} />
                      <span>Recent batches</span>
                      <ChevronRight size={18} />
                    </button>
                  </section>
                </div>
              </div>
            )}
            <Modal
              opened={diagnostics && !settings.error}
              onClose={() => setDiagnostics(false)}
              title="Models & search diagnostics"
              size="xl"
            >
              <Stack>
                {installed && (
                  <Text size="sm">
                    {policy?.text_model} · {policy?.embedding_model} ·{" "}
                    {policy?.embedding_dimensions.toLocaleString()} dimensions
                  </Text>
                )}
                <Group>
                  <Button variant="default" onClick={() => setHistory(true)}>
                    Policy history
                  </Button>
                  {brain.role === "admin" && !brain.archived && (
                    <Button
                      variant="default"
                      loading={check.isPending}
                      disabled={!policy?.enabled}
                      onClick={() => check.mutate(crypto.randomUUID())}
                    >
                      Check selected models
                    </Button>
                  )}
                </Group>
                <Text size="xs" c="dimmed">
                  A check sends fixed synthetic text to the provider and may
                  incur usage. It does not change permissions.
                </Text>
                <Failure error={check.error} />
                {check.data && (
                  <Alert color="brand">
                    Both selected models responded successfully to the requested
                    check.
                  </Alert>
                )}
                {diagnostics && <SemanticPanel brain={brain} />}
              </Stack>
            </Modal>
          </>
        )}
        {section === "activity" && (
          <>
            <Title order={3}>Model usage and learning</Title>
            <Failure error={usage.error} />
            {usage.isPending && <Loader size="sm" />}
            {!usage.error && usage.data && (
              <>
                <Text size="sm">
                  {usage.data.charged_tokens.toLocaleString()} /{" "}
                  {usage.data.daily_limit.toLocaleString()} tokens accounted for{" "}
                  {usage.data.day} UTC ·{" "}
                  {usage.data.remaining_tokens.toLocaleString()} remaining ·{" "}
                  {usage.data.in_flight} calls in flight.
                </Text>
                <Text size="xs" c="dimmed">
                  Uncertain calls retain their reserved allowance. Starting a
                  new attempt can incur another provider charge.
                </Text>
              </>
            )}
            <Divider label="Learning results" />
            <LearningRows brain={brain} onInspect={setSelected} />
            <Divider label="Model request history" />
            {usage.data?.total === 0 && (
              <Text size="sm">No model calls have been admitted.</Text>
            )}
            {!usage.error &&
              usage.data?.requests.map((request) => (
                <Card key={request.id} withBorder>
                  <Group justify="space-between">
                    <Text size="sm" fw={600}>
                      {label(request.purpose)} ·{" "}
                      {request.detail_expired
                        ? "Request detail expired"
                        : (request.returned_model ?? request.model)}
                    </Text>
                    <Badge
                      color={
                        request.suppressed
                          ? "yellow"
                          : request.state === "succeeded"
                            ? "teal"
                            : "gray"
                      }
                    >
                      {request.suppressed
                        ? "Output discarded"
                        : label(request.state)}
                    </Badge>
                  </Group>
                  <Text size="xs" c="dimmed">
                    {time(request.created_at)} ·{" "}
                    {request.charged_tokens.toLocaleString()} tokens{" "}
                    {request.detail_expired
                      ? "accounted; detailed history expired"
                      : request.total_tokens === null
                        ? "reserved"
                        : "used"}
                  </Text>
                  {request.error_code && (
                    <Text size="sm" c="red">
                      {label(request.error_code)}
                    </Text>
                  )}
                </Card>
              ))}
            <Pages
              offset={offset}
              total={usage.data?.total ?? 0}
              change={setOffset}
            />
          </>
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
