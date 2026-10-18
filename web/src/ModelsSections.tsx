import {
  BookOpen,
  ChevronRight,
  Database,
  FileText,
  History,
  Info,
  List,
  MessageSquare,
  Pencil,
  RefreshCw,
  Search,
  ShieldCheck,
} from "lucide-react";
import type { ReactNode } from "react";
import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Divider,
  Group,
  Loader,
  Modal,
  NumberInput,
  Select,
  Stack,
  Switch,
  Text,
  Title,
  Tooltip,
} from "@mantine/core";
import { HostIcon } from "./components/HostIcon";
import { SemanticPanel } from "./SemanticPanel";
import type { Brain } from "./api";
import type { components } from "./api-schema";
import { Failure, Pages, label, time } from "./ModelsShared";

type Policy = components["schemas"]["ModelPolicy"];
type Settings = components["schemas"]["ModelSettings"];
type Installed = components["schemas"]["InstalledModels"];
type Catalogue = components["schemas"]["ModelCatalogue"];
type Usage = components["schemas"]["ModelUsage"];
type UpdateDraft = (patch: Partial<Policy>) => void;
export function PolicyDetails({
  policy,
  editable,
  updateDraft,
}: {
  policy: Policy;
  editable: boolean;
  updateDraft: UpdateDraft;
}) {
  return (
    <>
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
    </>
  );
}
export function AiPermissionsCard({
  brain,
  policy,
  installed,
  editing,
  editable,
  saving,
  saveError,
  changedEmbedding,
  onEdit,
  onCancel,
  onSave,
  updateDraft,
  togglePurpose,
}: {
  brain: Brain;
  policy: Policy;
  installed: Installed;
  editing: boolean;
  editable: boolean;
  saving: boolean;
  saveError: Error | null;
  changedEmbedding: boolean;
  onEdit: () => void;
  onCancel: () => void;
  onSave: () => void;
  updateDraft: UpdateDraft;
  togglePurpose: (purpose: string, allowed: boolean) => void;
}) {
  return (
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
            onClick={onEdit}
          >
            Edit AI permissions
          </Button>
        )}
        {editing && (
          <Group gap="xs" className="ai-inline-actions">
            <Button
              variant="default"
              disabled={saving}
              onClick={onCancel}
            >
              Cancel
            </Button>
            <Button loading={saving} onClick={onSave}>
              {changedEmbedding
                ? "Save and rebuild"
                : "Save AI permissions"}
            </Button>
          </Group>
        )}
      </div>
      <Failure error={saveError} />
      <div className="ai-installed-provider">
        <strong>Selected provider</strong>
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
      <PolicyDetails
        policy={policy}
        editable={editable}
        updateDraft={updateDraft}
      />
    </section>
  );
}
export function SelectedModelsCard({
  brain,
  policy,
  installed,
  providers,
  modelCatalogue,
  catalogueError,
  catalogueProvider,
  refreshing,
  editable,
  changedEmbedding,
  updateDraft,
  refreshModels,
}: {
  brain: Brain;
  policy: Policy;
  installed: Installed;
  providers: Settings["providers"];
  modelCatalogue: Catalogue | undefined;
  catalogueError: Error | null;
  catalogueProvider: string;
  refreshing: boolean;
  editable: boolean;
  changedEmbedding: boolean;
  updateDraft: UpdateDraft;
  refreshModels: (provider: string) => void;
}) {
  const modelChoices = (kind: string, selected: string) => {
    const rows =
      modelCatalogue?.models.filter(
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
    const entry = modelCatalogue?.models.find((m) => m.id === id);
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
          title={
            policy?.provider === "openrouter"
              ? "Reference rates; OpenRouter routes can charge different prices. Request history shows actual billed cost."
              : "Standard short-context rates; batch, regional and long-context pricing can differ."
          }
        >
          {policy?.provider === "openrouter" ? "Reference" : "Standard"} ·{" "}
          {entry.checked_on}
          {entry.pricing_stale ? " · stale" : ""}
        </a>
      </p>
    );
  };
  return (
    <section className="management-surface ai-installed-models">
      <h3 className="management-title">Selected models</h3>
      <Select
        label="AI provider"
        value={policy.provider}
        data={(providers ?? [installed]).map(
          (p) => ({
            value: p.provider,
            label:
              p.provider === "openrouter"
                ? "OpenRouter"
                : "OpenAI",
          }),
        )}
        disabled={!editable}
        allowDeselect={false}
        onChange={(value) => {
          const next = providers?.find(
            (p) => p.provider === value,
          );
          if (next && next.provider !== policy.provider) {
            updateDraft({
              provider: next.provider,
              text_model: next.text_model,
              embedding_model: next.embedding_model,
              embedding_dimensions: next.embedding_dimensions,
            });
            refreshModels(next.provider);
          }
        }}
      />
      {!installed.credentials_present && (
        <Alert color="yellow" mt="sm">
          The selected provider credential is missing on this
          installation.
        </Alert>
      )}
      {policy.text_model ===
        "meta/muse-spark-1.3-contributor" && (
        <Alert color="yellow" mt="sm">
          Meta may use prompts and outputs to improve its products
          for this Contributor model.
        </Alert>
      )}
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
              modelCatalogue?.models.find(
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
              (modelCatalogue?.models.find(
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
          loading={refreshing}
          onClick={() => refreshModels(catalogueProvider)}
        >
          Refresh available models
        </Button>
      )}
      <Failure
        error={catalogueError}
      />
      {modelCatalogue?.error_code && (
        <Text size="xs" c="red">
          Model list unavailable. Current selection is retained.
        </Text>
      )}
      <p className="management-muted ai-catalogue-state">
        {modelCatalogue?.observed_at
          ? `Availability checked ${time(modelCatalogue.observed_at)}${modelCatalogue.stale ? " · stale" : ""}`
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
  );
}
export function MoreOptionsCard({
  brain,
  editing,
  onDiagnostics,
  onHistory,
  onAdvanced,
}: {
  brain: Brain;
  editing: boolean;
  onDiagnostics: () => void;
  onHistory: () => void;
  onAdvanced: () => void;
}) {
  return (
    <section className="management-surface ai-more-options">
      <h3 className="management-title">More options</h3>
      <button onClick={onDiagnostics}>
        <FileText size={22} />
        <span>Diagnostics</span>
        <ChevronRight size={18} />
      </button>
      {brain.role === "admin" && (
        <button onClick={onHistory}>
          <History size={22} />
          <span>Policy history</span>
          <ChevronRight size={18} />
        </button>
      )}
      {brain.role === "admin" && !brain.archived && !editing && (
        <button onClick={onAdvanced}>
          <ShieldCheck size={22} />
          <span>Advanced policy</span>
          <ChevronRight size={18} />
        </button>
      )}
      <button onClick={onDiagnostics}>
        <Database size={22} />
        <span>Recent batches</span>
        <ChevronRight size={18} />
      </button>
    </section>
  );
}
export function DiagnosticsModal({
  brain,
  diagnostics,
  failed,
  installed,
  policy,
  checkPending,
  checkError,
  checkPassed,
  onClose,
  onHistory,
  onCheck,
}: {
  brain: Brain;
  diagnostics: boolean;
  failed: boolean;
  installed: Installed | undefined;
  policy: Policy | undefined;
  checkPending: boolean;
  checkError: Error | null;
  checkPassed: boolean;
  onClose: () => void;
  onHistory: () => void;
  onCheck: () => void;
}) {
  return (
    <Modal
      opened={diagnostics && !failed}
      onClose={onClose}
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
          <Button variant="default" onClick={onHistory}>
            Policy history
          </Button>
          {brain.role === "admin" && !brain.archived && (
            <Button
              variant="default"
              loading={checkPending}
              disabled={!policy?.enabled}
              onClick={onCheck}
            >
              Check selected models
            </Button>
          )}
        </Group>
        <Text size="xs" c="dimmed">
          A check sends fixed synthetic text to the provider and may
          incur usage. It does not change permissions.
        </Text>
        <Failure error={checkError} />
        {checkPassed && (
          <Alert color="brand">
            Both selected models responded successfully to the requested
            check.
          </Alert>
        )}
        {diagnostics && <SemanticPanel brain={brain} />}
      </Stack>
    </Modal>
  );
}
export function UsageSection({
  usage,
  usageError,
  usagePending,
  offset,
  onOffset,
  learning,
}: {
  usage: Usage | undefined;
  usageError: Error | null;
  usagePending: boolean;
  offset: number;
  onOffset: (offset: number) => void;
  learning: ReactNode;
}) {
  return (
    <>
      <Title order={3}>Model usage and learning</Title>
      <Failure error={usageError} />
      {usagePending && <Loader size="sm" />}
      {!usageError && usage && (
        <>
          <Text size="sm">
            {usage.charged_tokens.toLocaleString()} /{" "}
            {usage.daily_limit.toLocaleString()} tokens accounted for{" "}
            {usage.day} UTC ·{" "}
            {usage.remaining_tokens.toLocaleString()} remaining ·{" "}
            {usage.in_flight} calls in flight.
          </Text>
          <Text size="xs" c="dimmed">
            Uncertain calls retain their reserved allowance. Starting a
            new attempt can incur another provider charge.
          </Text>
        </>
      )}
      <Divider label="Learning results" />
      {learning}
      <Divider label="Model request history" />
      {usage?.total === 0 && (
        <Text size="sm">No model calls have been admitted.</Text>
      )}
      {!usageError &&
        usage?.requests.map((request) => (
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
            {request.provider === "openrouter" && (
              <Text size="xs" c="dimmed">
                OpenRouter ·{" "}
                {request.cost_usd == null
                  ? "Billed cost unavailable"
                  : request.cost_usd > 0 && request.cost_usd < 0.000001
                    ? "< $0.000001 billed"
                    : `$${request.cost_usd.toFixed(6)} billed`}
              </Text>
            )}
            {request.error_code && (
              <Text size="sm" c="red">
                {label(request.error_code)}
              </Text>
            )}
          </Card>
        ))}
      <Pages
        offset={offset}
        total={usage?.total ?? 0}
        change={onOffset}
      />
    </>
  );
}
