import {
  BookOpen,
  ChevronRight,
  Database,
  FileText,
  History,
  Info,
  ListOrdered,
  MessageSquare,
  Pencil,
  RefreshCw,
  Search,
  ShieldCheck,
  Sparkles,
  type LucideIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import { HostIcon } from "./components/HostIcon";
import { SemanticPanel } from "./SemanticPanel";
import type { Brain } from "./api";
import type { components } from "./api-schema";
import { Failure, Pages, label, time } from "./ModelsShared";
import { ChooseModelNotice, needsModel } from "./features/settings/ChooseModel";
import { EmptyState } from "./components/AsyncState";
import { SkeletonRows } from "./components/Skeleton";
import {
  Alert,
  Badge,
  Button,
  Card,
  CardBar,
  Checkbox,
  IconButton,
  Modal,
  NumberField,
  Progress,
  Select,
  StatusBadge,
  Switch,
  Tile,
  Tooltip,
} from "./components/ui";
import { cn } from "./lib/utils";

type Policy = components["schemas"]["ModelPolicy"];
type Settings = components["schemas"]["ModelSettings"];
type Installed = components["schemas"]["InstalledModels"];
type Catalogue = components["schemas"]["ModelCatalogue"];
type Usage = components["schemas"]["ModelUsage"];
type UpdateDraft = (patch: Partial<Policy>) => void;

// Read-only controls keep their full colour so the saved policy reads clearly;
// editing is entered explicitly with Edit AI permissions.
const settled = "disabled:opacity-100 disabled:cursor-default";
const settledToggle = "cursor-default opacity-100";

function SubHeading({ children, id }: { children: ReactNode; id?: string }) {
  return (
    <h3 id={id} className="m-0 text-body font-semibold text-foreground">
      {children}
    </h3>
  );
}

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
      <div className="grid gap-2.5">
        <SubHeading>Allowed content</SubHeading>
        <div className="flex flex-wrap gap-2">
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
              policy.enabled && policy.content_classes.includes(value);
            return (
              <Checkbox
                key={value}
                aria-label={`Allow ${label(value)}`}
                label={label(value)}
                checked={policy.content_classes.includes(value)}
                disabled={!editable}
                className={cn(
                  "h-[30px] items-center gap-2 rounded-full border border-border pl-2 pr-3 text-label transition-colors duration-fast [&>span:first-child]:mt-0",
                  editable && "hover:border-border-strong hover:bg-accent",
                  !editable && settledToggle,
                  !allowed && "text-muted-foreground",
                )}
                onChange={(event) =>
                  updateDraft({
                    content_classes: event.currentTarget.checked
                      ? [...policy.content_classes, value]
                      : policy.content_classes.filter((v) => v !== value),
                    ...(value === "claim" && !event.currentTarget.checked
                      ? { autonomous_memory: false }
                      : {}),
                  })
                }
              />
            );
          })}
        </div>
      </div>
      <section className="grid gap-2.5" aria-label="Automatic processing">
        <SubHeading>Automatic processing</SubHeading>
        <div className="flex flex-wrap gap-x-8 gap-y-2">
          <Switch
            label="Automatic source learning"
            checked={policy.automatic_learning}
            className={cn(!editable && settledToggle)}
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
          <Switch
            label="Automatic search indexing"
            checked={policy.automatic_embedding}
            className={cn(!editable && settledToggle)}
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
        </div>
      </section>
      <section className="grid gap-2.5" aria-label="Processing limits">
        <div className="flex items-center gap-1">
          <SubHeading>Processing limits</SubHeading>
          <Tooltip label="Per model call and per Brain. The daily allowance resets at midnight UTC.">
            <IconButton
              type="button"
              size="icon-sm"
              label="About processing limits"
            >
              <Info aria-hidden="true" />
            </IconButton>
          </Tooltip>
        </div>
        <div className="grid grid-cols-2 gap-x-4 gap-y-3">
          {(
            [
              ["Maximum output tokens", "max_output_tokens", 128, 4096, 256],
              ["Maximum input bytes", "max_input_bytes", 256, 32768, 1024],
              ["Daily token allowance", "daily_token_limit", 1000, 10000000, 1000],
              ["Concurrent model calls", "max_concurrent", 1, 4, 1],
            ] as const
          ).map(([name, key, min, max, step]) => (
            <NumberField
              key={key}
              label={name}
              value={policy[key]}
              min={min}
              max={max}
              step={step}
              allowDecimal={false}
              disabled={!editable}
              hideControls={!editable}
              inputClassName={settled}
              onChange={(value) => updateDraft({ [key]: Number(value) })}
            />
          ))}
        </div>
      </section>
    </>
  );
}

const purposeRows: {
  purpose: string;
  title: string;
  description: string;
  icon: LucideIcon;
}[] = [
  {
    purpose: "answering",
    title: "Answer questions",
    description: "Use this Brain’s evidence to answer with citations.",
    icon: MessageSquare,
  },
  {
    purpose: "embedding",
    title: "Find related knowledge",
    description: "Index approved content for semantic search.",
    icon: Search,
  },
  {
    purpose: "extraction",
    title: "Learn from sources",
    description: "Extract evidence-backed memory into this Brain.",
    icon: BookOpen,
  },
  {
    purpose: "reranking",
    title: "Order search results",
    description: "Use the provider to rank and order retrieved results.",
    icon: ListOrdered,
  },
  {
    purpose: "synthesis",
    title: "Maintain memory",
    description: "Summarize, organize and maintain this Brain’s memory.",
    icon: Database,
  },
];

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
  const automatic = policy.autonomous_memory && policy.enabled;
  return (
    <Card
      className={cn(
        "grid gap-5 p-5 transition-[border-color,box-shadow] duration-fast",
        editing && "border-border-strong shadow-sm",
      )}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h2 className="m-0 text-title font-semibold text-foreground">
            AI permissions
          </h2>
          <p className="m-0 mt-1 text-body text-muted-foreground">
            Choose what this Brain may send to your installed AI provider.
          </p>
        </div>
        {brain.role === "admin" && !editing && (
          <Button disabled={brain.archived} onClick={onEdit}>
            <Pencil aria-hidden="true" />
            Edit AI permissions
          </Button>
        )}
        {editing && (
          <div className="flex shrink-0 gap-2">
            <Button variant="secondary" disabled={saving} onClick={onCancel}>
              Cancel
            </Button>
            <Button loading={saving} onClick={onSave}>
              {changedEmbedding ? "Save and rebuild" : "Save AI permissions"}
            </Button>
          </div>
        )}
      </div>
      <Failure error={saveError} />
      <div className="grid gap-2">
        <div className="flex items-center gap-3 rounded-lg border border-border py-2.5 pl-2.5 pr-3.5">
          <Tile size="sm" className="size-8">
            <HostIcon host={installed.provider} size={18} />
          </Tile>
          <div className="min-w-0 flex-1">
            <div className="text-caption text-muted-foreground">
              Selected provider
            </div>
            <div className="text-body font-medium text-foreground">
              {installed.provider === "openai" ? "OpenAI" : installed.provider}
            </div>
          </div>
          <Switch
            label="Allow AI processing"
            description={
              needsModel(policy) ? "Choose a text model first." : undefined
            }
            checked={policy.enabled}
            className={cn("items-center", !editable && settledToggle)}
            disabled={!editable || needsModel(policy)}
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
        <div className="flex items-center gap-3 rounded-lg border border-border py-2.5 pl-2.5 pr-3.5">
          <Tile size="sm" tone={automatic ? "brand" : "muted"} className="size-8">
            {automatic ? (
              <Sparkles aria-hidden="true" />
            ) : (
              <RefreshCw aria-hidden="true" />
            )}
          </Tile>
          <div className="min-w-0 flex-1">
            <div className="text-body font-medium text-foreground">
              Automatic memory
            </div>
            <div className="text-caption text-muted-foreground">
              Learning and maintenance follow this Brain’s standing policy.
            </div>
          </div>
          <Switch
            aria-label="Automatic memory"
            label={automatic ? "On" : "Off"}
            checked={automatic}
            className={cn("items-center", !editable && settledToggle)}
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
      </div>
      <div className="grid gap-2.5">
        <SubHeading>Allowed processing</SubHeading>
        <ul className="m-0 grid list-none overflow-hidden rounded-lg border border-border p-0">
          {purposeRows.map((row) => {
            const allowed = policy.purposes.includes(row.purpose);
            return (
              <li
                key={row.purpose}
                className="flex items-center gap-3 border-b border-border py-2 pl-2.5 pr-3.5 last:border-b-0"
              >
                <Tile size="sm" className="size-8">
                  <row.icon aria-hidden="true" />
                </Tile>
                <div className="min-w-0 flex-1">
                  <div className="text-body font-medium text-foreground">
                    {row.title}
                  </div>
                  <div className="text-caption text-muted-foreground">
                    {row.description}
                  </div>
                </div>
                <Checkbox
                  label={allowed ? "Allowed" : "Disallowed"}
                  aria-label={row.title}
                  checked={allowed}
                  disabled={!editable}
                  className={cn(
                    "w-28 items-center text-label",
                    !editable && settledToggle,
                    !allowed && "text-muted-foreground",
                  )}
                  onChange={(event) =>
                    togglePurpose(row.purpose, event.currentTarget.checked)
                  }
                />
              </li>
            );
          })}
        </ul>
      </div>
      <PolicyDetails
        policy={policy}
        editable={editable}
        updateDraft={updateDraft}
      />
    </Card>
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
    if (selected && !options.some((m) => m.value === selected))
      options.unshift({ value: selected, label: selected, disabled: false });
    return options;
  };
  const price = (id: string) => {
    if (!id) return null;
    const entry = modelCatalogue?.models.find((m) => m.id === id);
    if (!entry)
      return (
        <p className="m-0 text-caption text-muted-foreground">Price unknown</p>
      );
    const amount = (value: number | null | undefined) =>
      value == null ? "Unknown" : `$${value.toFixed(2)}`;
    return (
      <div className="grid gap-0.5">
        <p className="m-0 mono text-caption text-muted-foreground">
          {amount(entry.input_usd_per_million)} input
          {entry.kind === "text"
            ? ` · ${amount(entry.cached_input_usd_per_million)} cached · ${amount(entry.output_usd_per_million)} output`
            : ""}{" "}
          / 1M tokens
        </p>
        <a
          href={entry.source_url}
          target="_blank"
          rel="noreferrer"
          className="w-fit text-caption text-muted-foreground underline-offset-2 hover:text-foreground hover:underline"
          title={
            policy?.provider === "openrouter"
              ? "Reference rates; OpenRouter routes can charge different prices. Request history shows actual billed cost."
              : "Standard short-context rates; batch, regional and long-context pricing can differ."
          }
        >
          {policy?.provider === "openrouter" ? "Reference" : "Standard"} ·{" "}
          <span className="mono">{entry.checked_on}</span>
          {entry.pricing_stale ? " · stale" : ""}
        </a>
      </div>
    );
  };
  // View mode reads as values; edit mode shows the full controls.
  const view = !editable;
  const selectClass = cn(view && "[&_input~svg]:hidden");
  const inputClass = cn(
    view &&
      "h-6 border-transparent bg-transparent px-0 shadow-none hover:border-transparent disabled:cursor-default disabled:opacity-100",
  );
  return (
    <Card className="grid gap-4 p-5">
      <h2 className="m-0 text-title font-semibold text-foreground">
        Selected models
      </h2>
      <div className="grid gap-4">
        <Select
          label="AI provider"
          value={policy.provider}
          data={(providers ?? [installed]).map((p) => ({
            value: p.provider,
            label: p.provider === "openrouter" ? "OpenRouter" : "OpenAI",
          }))}
          disabled={!editable}
          allowDeselect={false}
          className={cn(selectClass, view && "gap-0.5 [&_label]:text-caption [&_label]:font-normal [&_label]:text-muted-foreground")}
          inputClassName={cn(inputClass, view && "font-medium")}
          onChange={(value) => {
            const next = providers?.find((p) => p.provider === value);
            if (next && next.provider !== policy.provider) {
              updateDraft({
                provider: next.provider,
                text_model: next.text_model ?? "",
                embedding_model: next.embedding_model,
                embedding_dimensions: next.embedding_dimensions,
                // Without an installation model the Brain must choose one
                // before any processing can be allowed.
                ...(!next.text_model
                  ? {
                      enabled: false,
                      automatic_learning: false,
                      automatic_embedding: false,
                    }
                  : {}),
              });
              refreshModels(next.provider);
            }
          }}
        />
        {!installed.credentials_present && (
          <Alert variant="warning">
            The selected provider credential is missing on this installation.
          </Alert>
        )}
        {policy.text_model === "meta/muse-spark-1.3-contributor" && (
          <Alert variant="warning">
            Meta may use prompts and outputs to improve its products for this
            Contributor model.
          </Alert>
        )}
        {needsModel(policy) &&
          (editable ? (
            <Alert variant="warning" title="Choose a model">
              Choose a text model below, then allow AI processing.
            </Alert>
          ) : (
            <ChooseModelNotice brain={brain} />
          ))}
        <div className="grid gap-1.5 border-t border-border pt-4">
          <Select
            label="Text model"
            placeholder="Choose a model"
            value={policy.text_model || null}
            data={modelChoices("text", policy.text_model)}
            disabled={!editable}
            allowDeselect={false}
            className={cn(selectClass, view && "gap-0.5 [&_label]:text-caption [&_label]:font-normal [&_label]:text-muted-foreground")}
            inputClassName={cn(inputClass, "mono text-label")}
            onChange={(value) => value && updateDraft({ text_model: value })}
          />
          {price(policy.text_model)}
        </div>
        <div className="grid gap-1.5 border-t border-border pt-4">
          <Select
            label="Embedding model"
            value={policy.embedding_model}
            data={modelChoices("embedding", policy.embedding_model)}
            disabled={!editable}
            allowDeselect={false}
            className={cn(selectClass, view && "gap-0.5 [&_label]:text-caption [&_label]:font-normal [&_label]:text-muted-foreground")}
            inputClassName={cn(inputClass, "mono text-label")}
            onChange={(value) =>
              value &&
              updateDraft({
                embedding_model: value,
                embedding_dimensions:
                  modelCatalogue?.models.find((m) => m.id === value)
                    ?.max_dimensions ?? policy.embedding_dimensions,
              })
            }
          />
          {price(policy.embedding_model)}
        </div>
        <div className="border-t border-border pt-4">
          <Select
            label="Embedding dimensions"
            value={String(policy.embedding_dimensions)}
            disabled={!editable}
            allowDeselect={false}
            className={cn(selectClass, view && "gap-0.5 [&_label]:text-caption [&_label]:font-normal [&_label]:text-muted-foreground")}
            inputClassName={cn(inputClass, "mono text-label")}
            data={[
              ...new Set([256, 512, 1024, 1536, 3072, policy.embedding_dimensions]),
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
              value && updateDraft({ embedding_dimensions: Number(value) })
            }
          />
        </div>
      </div>
      <div className="grid gap-1 border-t border-border pt-4">
        {brain.role === "admin" && !brain.archived && (
          <div className="-ml-2.5">
            <Button
              size="sm"
              variant="ghost"
              loading={refreshing}
              onClick={() => refreshModels(catalogueProvider)}
            >
              {!refreshing && <RefreshCw aria-hidden="true" />}
              Refresh available models
            </Button>
          </div>
        )}
        <Failure error={catalogueError} />
        {modelCatalogue?.error_code && (
          <p className="m-0 text-caption text-destructive-text">
            Model list unavailable. Current selection is retained.
          </p>
        )}
        <p className="m-0 text-caption text-muted-foreground">
          {modelCatalogue?.observed_at
            ? `Availability checked ${time(modelCatalogue.observed_at)}${modelCatalogue.stale ? " · stale" : ""}`
            : "Refresh to load models available to this account."}
        </p>
      </div>
      {changedEmbedding && (
        <Alert variant="warning">
          Saving rebuilds search embeddings and may incur provider usage. Exact
          and text search stay available.
          {!policy.enabled ||
          !policy.automatic_embedding ||
          !policy.purposes.includes("embedding")
            ? " Rebuilding waits until embedding permissions are enabled."
            : ""}
        </Alert>
      )}
    </Card>
  );
}
function OptionRow({
  icon: Icon,
  children,
  onClick,
}: {
  icon: LucideIcon;
  children: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="group flex h-9 w-full items-center gap-2.5 rounded-md px-2.5 text-left text-body text-foreground outline-none transition-colors duration-fast hover:bg-accent focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring"
    >
      <Icon className="size-4 text-muted-foreground" aria-hidden="true" />
      <span className="flex-1">{children}</span>
      <ChevronRight
        className="size-4 text-muted-foreground transition-transform duration-fast group-hover:translate-x-0.5"
        aria-hidden="true"
      />
    </button>
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
    <Card className="overflow-hidden p-0">
      <CardBar>
        <h3>More options</h3>
      </CardBar>
      <div className="grid gap-0.5 p-1">
        <OptionRow icon={FileText} onClick={onDiagnostics}>
          Diagnostics
        </OptionRow>
        {brain.role === "admin" && (
          <OptionRow icon={History} onClick={onHistory}>
            Policy history
          </OptionRow>
        )}
        {brain.role === "admin" && !brain.archived && !editing && (
          <OptionRow icon={ShieldCheck} onClick={onAdvanced}>
            Advanced policy
          </OptionRow>
        )}
        <OptionRow icon={Database} onClick={onDiagnostics}>
          Recent batches
        </OptionRow>
      </div>
    </Card>
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
      <div className="grid gap-4">
        {installed && (
          <p className="m-0 mono text-label text-foreground-secondary">
            {policy?.text_model || "No text model selected"} ·{" "}
            {policy?.embedding_model} ·{" "}
            {policy?.embedding_dimensions.toLocaleString()} dimensions
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" onClick={onHistory}>
            <History aria-hidden="true" />
            Policy history
          </Button>
          {brain.role === "admin" && !brain.archived && (
            <Button
              variant="secondary"
              loading={checkPending}
              disabled={!policy?.enabled}
              onClick={onCheck}
            >
              Check selected models
            </Button>
          )}
        </div>
        <p className="m-0 text-caption text-muted-foreground">
          A check sends fixed synthetic text to the provider and may incur
          usage. It does not change permissions.
        </p>
        <Failure error={checkError} />
        {checkPassed && (
          <Alert variant="brand" role="status">
            Both selected models responded successfully to the requested check.
          </Alert>
        )}
        {diagnostics && <SemanticPanel brain={brain} />}
      </div>
    </Modal>
  );
}
function UsageStat({ label: name, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid gap-0.5">
      <span className="text-caption text-muted-foreground">{name}</span>
      <span className="mono text-title font-semibold text-foreground">
        {children}
      </span>
    </div>
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
  const share = usage?.daily_limit
    ? (usage.charged_tokens / usage.daily_limit) * 100
    : 0;
  return (
    <div className="grid gap-4">
      <Card className="grid gap-4 p-5">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h3 className="m-0 text-title font-semibold text-foreground">
              Model usage and learning
            </h3>
            <p className="m-0 mt-1 text-body text-muted-foreground">
              Allowance accounted today, learning outcomes and admitted model
              requests.
            </p>
          </div>
          {usage && !usageError && (
            <Badge variant="outline" className="mono">
              {usage.day} UTC
            </Badge>
          )}
        </div>
        <Failure error={usageError} />
        {usagePending && <SkeletonRows rows={2} label="Loading model usage…" />}
        {!usageError && usage && (
          <>
            <div className="grid grid-cols-4 gap-4">
              <UsageStat label="Tokens accounted">
                {usage.charged_tokens.toLocaleString()}
              </UsageStat>
              <UsageStat label="Daily allowance">
                {usage.daily_limit.toLocaleString()}
              </UsageStat>
              <UsageStat label="Remaining">
                {usage.remaining_tokens.toLocaleString()}
              </UsageStat>
              <UsageStat label="Calls in flight">{usage.in_flight}</UsageStat>
            </div>
            <Progress
              label="Daily allowance used"
              value={share}
              className="h-1.5"
            />
            <p className="m-0 text-caption text-muted-foreground">
              Uncertain calls retain their reserved allowance. Starting a new
              attempt can incur another provider charge.
            </p>
          </>
        )}
      </Card>
      <Card className="overflow-hidden p-0">
        <CardBar>
          <h3>Learning results</h3>
        </CardBar>
        {learning}
      </Card>
      <Card className="overflow-hidden p-0">
        <CardBar>
          <h3>Model request history</h3>
          {usage && !usageError && (
            <span className="mono text-caption text-muted-foreground">
              {usage.total.toLocaleString()}
            </span>
          )}
        </CardBar>
        {usage?.total === 0 && (
          <EmptyState
            icon={Sparkles}
            title="No model calls have been admitted"
            description="Requests appear here once this Brain sends permitted content to its provider."
            className="py-8"
          />
        )}
        {!usageError &&
          usage?.requests.map((request) => (
            <div
              key={request.id}
              className="grid gap-0.5 border-b border-border px-4 py-2.5 last:border-b-0"
            >
              <div className="flex items-center justify-between gap-3">
                <span className="min-w-0 truncate text-body font-medium text-foreground">
                  {label(request.purpose)} ·{" "}
                  {request.detail_expired ? (
                    "Request detail expired"
                  ) : (
                    <span className="mono text-label">
                      {request.returned_model ?? request.model}
                    </span>
                  )}
                </span>
                {request.suppressed ? (
                  <StatusBadge state="attention">Output discarded</StatusBadge>
                ) : request.state === "succeeded" ? (
                  <StatusBadge state="positive">
                    <span className="capitalize">{label(request.state)}</span>
                  </StatusBadge>
                ) : (
                  <StatusBadge state="neutral">
                    <span className="capitalize">{label(request.state)}</span>
                  </StatusBadge>
                )}
              </div>
              <p className="m-0 text-caption text-muted-foreground">
                <span className="mono">{time(request.created_at)}</span> ·{" "}
                <span className="mono">
                  {request.charged_tokens.toLocaleString()}
                </span>{" "}
                tokens{" "}
                {request.detail_expired
                  ? "accounted; detailed history expired"
                  : request.total_tokens === null
                    ? "reserved"
                    : "used"}
              </p>
              {request.provider === "openrouter" && (
                <p className="m-0 text-caption text-muted-foreground">
                  OpenRouter ·{" "}
                  <span className="mono">
                    {request.cost_usd == null
                      ? "Billed cost unavailable"
                      : request.cost_usd > 0 && request.cost_usd < 0.000001
                        ? "< $0.000001 billed"
                        : `$${request.cost_usd.toFixed(6)} billed`}
                  </span>
                </p>
              )}
              {request.error_code && (
                <p className="m-0 text-label text-destructive-text">
                  {label(request.error_code)}
                </p>
              )}
            </div>
          ))}
        {usage && usage.total > 20 && (
          <div className="border-t border-border px-4 py-2">
            <Pages
              offset={offset}
              total={usage?.total ?? 0}
              change={onOffset}
            />
          </div>
        )}
      </Card>
    </div>
  );
}
