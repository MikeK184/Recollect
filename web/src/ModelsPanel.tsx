import { useState } from "react";
import "./features/feature-views.css";
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
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import { ClaimDialog } from "./ClaimsPanel";
import { SemanticPanel } from "./SemanticPanel";

type Policy = components["schemas"]["ModelPolicy"];
type Settings = components["schemas"]["ModelSettings"];
type Run = components["schemas"]["LearningRun"];
type Selection = components["schemas"]["ScopeSelection"];
const label = (s: string) => s.replaceAll("_", " ");
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
    <Modal opened onClose={onClose} title="Model policy" size="lg">
      <Stack>
        <Text size="sm">
          Choose what this Brain may send to OpenAI. Capturing a source does not
          grant permission to transmit it.
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
        <TextInput label="Text model" value={draft.text_model} readOnly />
        <TextInput
          label="Embedding model"
          value={draft.embedding_model}
          readOnly
        />
        <Text size="sm">
          {draft.embedding_dimensions.toLocaleString()} embedding dimensions ·{" "}
          {settings.installed.endpoint}
        </Text>
        {(draft.text_model !== settings.installed.text_model ||
          draft.embedding_model !== settings.installed.embedding_model ||
          draft.embedding_dimensions !==
            settings.installed.embedding_dimensions) && (
          <Button
            variant="light"
            onClick={() =>
              patch({
                provider: settings.installed.provider,
                text_model: settings.installed.text_model,
                embedding_model: settings.installed.embedding_model,
                embedding_dimensions: settings.installed.embedding_dimensions,
              })
            }
          >
            Use installed models
          </Button>
        )}
        <MultiSelect
          searchable
          label="Allowed model purposes"
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
          label="Allowed content classes"
          data={choices(classes)}
          value={draft.content_classes}
          onChange={(content_classes) => patch({ content_classes })}
        />
        <Text size="xs" c="dimmed">
          Query permission is also required for Ask, semantic search and the
          fixed synthetic connection check. Answering is a separate permission
          and does not run tools or save conversations.
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
          Index existing and newly eligible content using the approved embedding
          model and content classes. Processing runs in the background without
          per-record review.
        </Text>
        <Switch
          label="Maintain memory autonomously"
          checked={draft.autonomous_memory}
          onChange={(e) => {
            const autonomous_memory = e.currentTarget.checked;
            patch({
              autonomous_memory,
              ...(autonomous_memory
                ? {
                    purposes: [
                      ...new Set([
                        ...draft.purposes,
                        "extraction",
                        "synthesis",
                      ]),
                    ],
                    content_classes: [
                      ...new Set([...draft.content_classes, "claim", "query"]),
                    ],
                  }
                : {}),
            });
          }}
        />
        <Text size="sm">
          Automatically learn permitted sources, reconcile changes and refresh
          generated handovers. Supported results are accepted by policy;
          uncertain evidence stays labeled. Human review is optional.
        </Text>
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
              Only exact declarations such as Amber.port = 8080 can qualify. In
              explicit mode, other interpretations remain proposals. Acceptance
              records this policy and does not create a human reviewer.
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
                patch({ acceptance: { ...draft.acceptance!, source_classes } })
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
                patch({ acceptance: { ...draft.acceptance!, collection_ids } })
              }
            />
            <Failure error={groups.error} />
          </>
        )}
        <Failure error={save.error} />
        <Button loading={save.isPending} onClick={() => save.mutate()}>
          Save model policy
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
          Brain's model policy. Empty applicability means the whole Brain. Model
          output starts as proposed unless a permitted literal rule applies.
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
  const [history, setHistory] = useState(false);
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const cache = useQueryClient();
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
  const policy = settings.error ? undefined : settings.data?.current.policy;
  const installed = settings.error ? undefined : settings.data?.installed;
  return (
    <section className="feature-setting" id="models">
      <Stack>
        {section === "settings" && (
          <>
            <Group justify="space-between">
              <Title order={3}>AI &amp; automation</Title>
              <Group>
                <Button variant="subtle" onClick={() => setHistory(true)}>
                  Policy history
                </Button>
                {brain.role === "admin" && (
                  <Button
                    variant="default"
                    disabled={!settings.data}
                    onClick={() => setEditing(true)}
                  >
                    Edit model policy
                  </Button>
                )}
              </Group>
            </Group>
            <Failure error={settings.error ?? usage.error ?? check.error} />
            {settings.isPending && <Loader />}
            {installed && (
              <>
                <Text>
                  {installed.text_model} · {installed.embedding_model} ·{" "}
                  {installed.embedding_dimensions.toLocaleString()} dimensions
                </Text>
                <Text size="sm" c="dimmed">
                  {installed.credentials_present
                    ? "Provider credential present."
                    : "Provider credential missing."}{" "}
                  Connection results below are based on actual calls.
                </Text>
              </>
            )}
            {policy && (
              <>
                <Badge w="fit-content" color={policy.enabled ? "teal" : "gray"}>
                  {policy.enabled
                    ? "Transmission enabled"
                    : "Transmission disabled"}
                </Badge>
                <Text size="sm">
                  Allowed purposes:{" "}
                  {policy.purposes.map(label).join(", ") || "none"}. Allowed
                  content:{" "}
                  {policy.content_classes.map(label).join(", ") || "none"}.
                </Text>
                <Text size="sm">
                  {policy.autonomous_memory
                    ? "Autonomous memory is configured: source catch-up, evidence-based revisions and handover refresh. Human review is optional."
                    : policy.automatic_learning
                      ? "Automatic source learning is enabled."
                      : "Learning starts when requested."}{" "}
                  {!policy.autonomous_memory && (
                    <>
                      Acceptance rule:{" "}
                      {policy.acceptance?.name ?? "explicit proposals"}.
                    </>
                  )}
                </Text>
              </>
            )}
            {brain.role === "admin" && !brain.archived && (
              <Group>
                <Button
                  variant="light"
                  loading={check.isPending}
                  disabled={!policy?.enabled}
                  onClick={() => check.mutate(crypto.randomUUID())}
                >
                  Check selected models
                </Button>
                {check.isError && check.variables && (
                  <Button
                    variant="subtle"
                    disabled={check.isPending}
                    onClick={() => check.mutate(check.variables!)}
                  >
                    Repeat last connection check
                  </Button>
                )}
              </Group>
            )}
            {check.data && (
              <Alert color="teal">
                Both selected models responded successfully. Embeddings have{" "}
                {check.data.requests
                  .find((r) => r.purpose === "embedding")
                  ?.dimensions?.toLocaleString()}{" "}
                dimensions. Only fixed synthetic text was sent.
              </Alert>
            )}
            <SemanticPanel brain={brain} />
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
        {editing && settings.data && !settings.error && (
          <PolicyEditor
            brain={brain.id}
            settings={settings.data}
            onClose={() => setEditing(false)}
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
