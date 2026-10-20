import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  IconButton,
  MultiSelect,
  SearchInput,
  Select,
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  StatusBadge,
  Switch,
  TextField,
  TextareaField,
  Tile,
} from "../../components/ui";
import { DetailInspector } from "../../components/DetailInspector";
import { staggerStyle } from "../../components/Motion";
import { cn } from "../../lib/utils";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
  ArrowRight,
  BookOpen,
  ChevronRight,
  FileText,
  Plus,
  Trash2,
  Upload,
} from "lucide-react";
import { client, RequestError, result } from "../../api";
import { useBrain } from "../../app/context";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
import {
  bundleDraft,
  kindDraft,
  newSkill,
  publicationDraft,
  type SkillBundle,
  type SkillInput,
  type SkillSummary,
} from "./skillDraft";
import { usePollInterval } from "../../usePollInterval";
import { useWorkspaceCatalogue } from "../../catalogueQueries";

const denied = (error: Error | null) =>
  error instanceof RequestError && [401, 403, 404].includes(error.status);

export function SkillsPanel({
  startPublishing = false,
  onStartPublishing,
}: {
  /** Open the publish editor once the library has loaded (admin only). */
  startPublishing?: boolean;
  onStartPublishing?: () => void;
} = {}) {
  const pollNormal = usePollInterval("normal");
  const brain = useBrain();
  const cache = useQueryClient();
  const admin = brain.role === "admin" && !brain.archived;
  const [environment, setEnvironment] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [showDisabled, setShowDisabled] = useState(false);
  const [selected, setSelected] = useState<SkillSummary | null>(null);
  const [editing, setEditing] = useState<{
    id: string | null;
    draft: SkillInput;
  } | null>(null);
  const [notice, setNotice] = useState("");
  const inventory = useWorkspaceCatalogue(brain.id);
  const catalogue = useQuery({
    queryKey: ["mcp", brain.id, "skills"],
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp", {
          signal,
          params: { path: { brain: brain.id } },
        }),
      ),
  });
  const skills = useQuery({
    queryKey: [
      "managed-skills",
      brain.id,
      brain.role,
      environment,
      query,
      showDisabled,
    ],
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/skills", {
          signal,
          params: {
            path: { brain: brain.id },
            query: {
              environment_id: environment ?? undefined,
              query: query.trim() || undefined,
              include_disabled: admin && showDisabled ? true : undefined,
            },
          },
        }),
      ),
  });
  const detail = useQuery({
    queryKey: [
      "managed-skill",
      brain.id,
      brain.role,
      selected?.id,
      environment,
    ],
    enabled: !!selected && !skills.isError,
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/skills/{id}", {
          signal,
          params: {
            path: { brain: brain.id, id: selected!.id },
            query: { environment_id: environment ?? undefined },
          },
        }),
      ),
  });
  const environments = inventory.isError
    ? []
    : (inventory.data?.environments ?? []);
  const profiles = catalogue.isError ? [] : (catalogue.data?.profiles ?? []);
  const scopeName = (id?: string | null) =>
    id
      ? (environments.find((entry) => entry.id === id)?.name ??
        "Environment unavailable")
      : "Brain-wide";
  const noAccess = denied(skills.error) || denied(inventory.error);
  const visible = skills.isError ? [] : (skills.data ?? []);
  function openNew() {
    setNotice("");
    setEditing({ id: null, draft: newSkill(environment) });
  }
  useEffect(() => {
    if (!startPublishing || skills.isPending) return;
    if (admin && !noAccess && !skills.isError) openNew();
    onStartPublishing?.();
    // openNew reads current state; run once per request.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [startPublishing, skills.isPending]);
  return (
    <div className="grid min-w-0 gap-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div className="min-w-0">
          <span className="flex items-center gap-1.5 text-caption text-muted-foreground">
            <BookOpen aria-hidden="true" className="size-3.5" /> Shared
            guidance
          </span>
          <h2 className="m-0 mt-1 text-section font-semibold text-foreground">
            One library for your agents
          </h2>
          <p className="m-0 mt-0.5 max-w-[68ch] text-body text-muted-foreground">
            Publish skills and working instructions for this Brain. Agents fetch
            what they need, when they need it.
          </p>
        </div>
        {admin && !noAccess && (
          <Button onClick={openNew}>
            <Plus aria-hidden="true" />
            Publish skill
          </Button>
        )}
      </div>
      <ol className="m-0 flex list-none flex-wrap items-center gap-x-3 gap-y-1.5 p-0 text-label text-foreground-secondary">
        {[
          "Manager publishes",
          "Agent discovers",
          "Instructions fetched on demand",
        ].map((step, index) => (
          <li key={step} className="flex items-center gap-3">
            {index > 0 && (
              <ChevronRight
                aria-hidden="true"
                className="size-3.5 text-faint-foreground"
              />
            )}
            <span className="flex items-baseline gap-2">
              <span className="mono text-caption text-muted-foreground">
                0{index + 1}
              </span>
              {step}
            </span>
          </li>
        ))}
      </ol>
      {notice && !skills.isPending && !skills.isError && !noAccess && (
        <Alert
          variant="brand"
          role="status"
          title="Published"
          className="animate-enter"
        >
          {notice}
        </Alert>
      )}
      <div className="flex flex-wrap items-center gap-3">
        <div className="w-[320px]">
          <SearchInput
            aria-label="Find shared guidance"
            placeholder="Find skills, instructions or tags…"
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
          />
        </div>
        <Select
          className="w-[240px]"
          aria-label="Guidance environment"
          placeholder="Brain-wide guidance"
          clearable
          value={environment}
          onChange={(value) => {
            setEnvironment(value);
            setSelected(null);
          }}
          data={environments.map((entry) => ({
            value: entry.id,
            label: `${entry.name} + Brain-wide`,
          }))}
          disabled={inventory.isPending || inventory.isError}
        />
        {admin && (
          <Switch
            className="ml-auto"
            label="Include disabled"
            checked={showDisabled}
            onChange={(event) => setShowDisabled(event.currentTarget.checked)}
          />
        )}
      </div>
      {inventory.isError && !noAccess && (
        <Alert
          variant="warning"
          title="Environment list unavailable"
          actions={
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void inventory.refetch()}
            >
              Retry environments
            </Button>
          }
        >
          {inventory.error.message}
        </Alert>
      )}
      {noAccess ? (
        <Alert role="status" title="Skills access unavailable">
          Your access to this Brain is no longer available. Previous guidance is
          hidden.
        </Alert>
      ) : skills.isPending ? (
        <LoadingState label="Loading shared guidance…" />
      ) : skills.isError ? (
        <ErrorState error={skills.error} retry={() => void skills.refetch()} />
      ) : !visible.length ? (
        <div className="rounded-lg border border-border bg-card">
          <EmptyState
            icon={BookOpen}
            title={query ? "No matching guidance" : "Shared skills start here"}
            description={
              query
                ? "Try another search or select a different environment."
                : admin
                  ? "Publish a skill or an AGENTS.md instructions bundle. Your team’s agents can discover it through Recollect."
                  : "A Brain administrator can publish reusable skills and working instructions here."
            }
            action={
              admin && !query ? (
                <Button variant="secondary" onClick={openNew}>
                  <Plus aria-hidden="true" />
                  Publish your first skill
                </Button>
              ) : undefined
            }
          />
        </div>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3">
          {visible.map((skill, index) => (
            <button
              type="button"
              key={skill.id}
              className={cn(
                "group/skill flex min-w-0 animate-enter flex-col gap-2.5 rounded-lg border bg-card p-4 text-left transition-[border-color,box-shadow,transform] duration-fast ease-standard hover:-translate-y-px hover:border-border-strong hover:shadow-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
                selected?.id === skill.id
                  ? "border-brand/50 bg-brand-soft/30"
                  : "border-border",
              )}
              style={staggerStyle(index)}
              onClick={() => setSelected(skill)}
              aria-label={`Inspect ${skill.name}`}
            >
              <div className="flex items-start justify-between gap-2">
                <Tile tone={skill.enabled ? "brand" : "muted"}>
                  {skill.kind === "skill" ? (
                    <BookOpen aria-hidden="true" />
                  ) : (
                    <FileText aria-hidden="true" />
                  )}
                </Tile>
                <StatusBadge state={skill.enabled ? "positive" : "neutral"}>
                  {skill.enabled ? "Enabled" : "Disabled"}
                </StatusBadge>
              </div>
              <h3 className="mono m-0 truncate text-body font-semibold text-foreground">
                {skill.name}
              </h3>
              <p className="m-0 line-clamp-2 text-label text-muted-foreground">
                {skill.description}
              </p>
              {skill.tags.length > 0 && (
                <div className="flex flex-wrap gap-1">
                  {skill.tags.slice(0, 4).map((tag) => (
                    <Badge key={tag} variant="outline">
                      {tag}
                    </Badge>
                  ))}
                </div>
              )}
              <div className="mt-auto flex items-center justify-between gap-2 text-caption text-muted-foreground">
                <span>
                  {skill.kind === "skill" ? "Skill" : "Instructions"} ·{" "}
                  <span className="mono">v{skill.version}</span>
                </span>
                <span className="truncate">
                  {scopeName(skill.environment_id)}
                </span>
              </div>
              <div className="flex items-center justify-between gap-2 border-t border-border pt-2.5 text-caption text-muted-foreground">
                <span>
                  <span className="mono">{skill.file_paths.length}</span>{" "}
                  {skill.file_paths.length === 1 ? "file" : "files"}
                  {skill.profile_ids.length
                    ? ` · ${skill.profile_ids.length} tool ${skill.profile_ids.length === 1 ? "group" : "groups"}`
                    : ""}
                </span>
                <span className="inline-flex items-center gap-1 font-medium text-foreground-secondary group-hover/skill:text-foreground">
                  Inspect
                  <ArrowRight
                    aria-hidden="true"
                    className="size-3.5 transition-transform duration-fast group-hover/skill:translate-x-0.5"
                  />
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
      <div className="flex items-start gap-2.5 rounded-lg bg-muted px-3.5 py-3">
        <BookOpen
          aria-hidden="true"
          className="mt-0.5 size-4 shrink-0 text-muted-foreground"
        />
        <p className="m-0 text-label text-foreground-secondary">
          Agents load selected instructions during work. Optional local copies
          can be temporary or kept; disabling stops future fetches. Already
          saved copies and conversation context remain with the agent. Local
          instructions and human direction retain priority.
        </p>
      </div>
      <DetailInspector
        opened={!!selected && !skills.isError && !noAccess}
        onClose={() => setSelected(null)}
        title="Shared guidance"
      >
        {detail.isPending ? (
          <LoadingState label="Loading published files…" />
        ) : detail.isError ? (
          <ErrorState
            error={detail.error}
            retry={() => void detail.refetch()}
          />
        ) : (
          detail.data && (
            <SkillDetails
              key={`${detail.data.skill.id}:${detail.data.skill.version}`}
              bundle={detail.data}
              scope={scopeName(detail.data.skill.environment_id)}
              profiles={profiles}
              onEdit={
                admin
                  ? () => {
                      setEditing({
                        id: detail.data.skill.id,
                        draft: bundleDraft(detail.data),
                      });
                      setSelected(null);
                    }
                  : undefined
              }
            />
          )
        )}
      </DetailInspector>
      {editing && !noAccess && admin && (
        <SkillEditor
          key={editing.id ?? "new"}
          initial={editing.draft}
          id={editing.id}
          environmentOptions={environments.map((entry) => ({
            value: entry.id,
            label: entry.name,
          }))}
          profileOptions={profiles.map((profile) => ({
            value: profile.id,
            label: profile.name,
          }))}
          associationError={catalogue.error ?? inventory.error}
          onClose={() => setEditing(null)}
          onSaved={(bundle) => {
            setEditing(null);
            setEnvironment(bundle.skill.environment_id ?? null);
            if (!bundle.skill.enabled) setShowDisabled(true);
            setNotice(
              `${bundle.skill.name} · version ${bundle.skill.version}${bundle.skill.enabled ? " is available to applicable agents." : " is disabled for future agent fetches."}`,
            );
            void cache.invalidateQueries({
              queryKey: ["managed-skills", brain.id],
            });
            void cache.invalidateQueries({
              queryKey: ["managed-skill", brain.id],
            });
          }}
        />
      )}
    </div>
  );
}

function SkillDetails({
  bundle,
  scope,
  profiles,
  onEdit,
}: {
  bundle: SkillBundle;
  scope: string;
  profiles: { id: string; name: string }[];
  onEdit?: () => void;
}) {
  const brain = useBrain();
  const [path, setPath] = useState(bundle.files[0]?.path ?? "");
  const file = bundle.files.find((entry) => entry.path === path);
  return (
    <div className="grid min-w-0 gap-4">
      <div className="flex items-center justify-between gap-2">
        <StatusBadge state={bundle.skill.enabled ? "positive" : "neutral"}>
          {bundle.skill.enabled ? "Enabled" : "Disabled"} · v
          {bundle.skill.version}
        </StatusBadge>
        {onEdit && (
          <Button size="sm" variant="secondary" onClick={onEdit}>
            Edit guidance
          </Button>
        )}
      </div>
      <div className="grid gap-1">
        <h3 className="mono m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
          {bundle.skill.name}
        </h3>
        <p className="m-0 text-body text-foreground-secondary">
          {bundle.skill.description}
        </p>
        <p className="m-0 text-label text-muted-foreground">
          {scope} ·{" "}
          {bundle.skill.kind === "skill"
            ? "SKILL.md skill"
            : "AGENTS.md instructions"}
        </p>
      </div>
      {!!bundle.skill.profile_ids.length && (
        <div className="grid gap-1.5 border-t border-border pt-3">
          <p className="m-0 text-caption text-muted-foreground">
            Accompanying tool groups · independent tool permissions apply
          </p>
          {bundle.skill.profile_ids.map((id) => (
            <Link
              key={id}
              to="/brains/$brainId/connections"
              params={{ brainId: brain.id }}
              search={{ tab: "profiles", profile: id }}
              className="inline-flex w-fit items-center gap-1.5 text-label font-medium text-brand-text underline-offset-[3px] hover:underline"
            >
              <BookOpen aria-hidden="true" className="size-3.5" />
              {profiles.find((profile) => profile.id === id)?.name ??
                "Tool group unavailable"}
            </Link>
          ))}
        </div>
      )}
      <Select
        label="Published file"
        value={path}
        onChange={(value) => setPath(value ?? "")}
        data={bundle.files.map((entry) => entry.path)}
        allowDeselect={false}
      />
      <pre
        className="mono m-0 max-h-[420px] overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted px-3 py-2.5 text-caption leading-5 text-foreground [overflow-wrap:anywhere] scrollbar-thin"
        data-testid="skill-published-content"
      >
        {file?.content}
      </pre>
      <p className="m-0 text-caption text-muted-foreground">
        Published{" "}
        <span className="mono">
          {new Date(bundle.skill.updated_at).toLocaleString()}
        </span>
        . These files are fetched on demand; viewing them does not install a
        native host skill.
      </p>
    </div>
  );
}

function SkillEditor({
  initial,
  id,
  environmentOptions,
  profileOptions,
  associationError,
  onClose,
  onSaved,
}: {
  initial: SkillInput;
  id: string | null;
  environmentOptions: { value: string; label: string }[];
  profileOptions: { value: string; label: string }[];
  associationError: Error | null;
  onClose: () => void;
  onSaved: (bundle: SkillBundle) => void;
}) {
  const brain = useBrain();
  const [draft, setDraft] = useState(initial);
  const [tagsText, setTagsText] = useState(initial.tags.join(", "));
  const [fileIndex, setFileIndex] = useState(0);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [conflict, setConflict] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);
  const file = draft.files[fileIndex];
  function patch(values: Partial<SkillInput>) {
    setDraft((current) => ({ ...current, ...values }));
  }
  function patchFile(values: Partial<{ path: string; content: string }>) {
    patch({
      files: draft.files.map((entry, index) =>
        index === fileIndex ? { ...entry, ...values } : entry,
      ),
    });
  }
  async function importFiles(files: File[]) {
    if (!files.length) return;
    try {
      if (files.some((entry) => entry.size > 128 * 1024))
        throw new Error("Each imported file must be at most 128 KiB.");
      if (
        files.some(
          (entry) =>
            !/\.(md|txt)$/.test(entry.name) && entry.name !== "LICENSE",
        )
      )
        throw new Error(
          "Import Markdown, plain text or an exact LICENSE file. Scripts and binary files are unsupported.",
        );
      const imported = await Promise.all(
        files.map(async (entry) => ({
          path: entry.name,
          content: await entry.text(),
        })),
      );
      const merged = [...draft.files];
      for (const entry of imported) {
        const index = merged.findIndex(
          (current) => current.path === entry.path,
        );
        if (index < 0) merged.push(entry);
        else merged[index] = entry;
      }
      if (
        merged.length > 64 ||
        merged.reduce(
          (bytes, entry) =>
            bytes + new TextEncoder().encode(entry.content).length,
          0,
        ) >
          512 * 1024
      )
        throw new Error(
          "A bundle supports up to 64 files and 512 KiB of text in total.",
        );
      patch({ files: merged });
      setFileIndex(
        merged.findIndex((entry) => entry.path === imported[0]?.path),
      );
      setError(null);
    } catch (failure) {
      setError(failure as Error);
    }
    if (fileInput.current) fileInput.current.value = "";
  }
  async function publish() {
    setSaving(true);
    setError(null);
    try {
      const body = publicationDraft({
        ...draft,
        tags: [
          ...new Set(
            tagsText
              .split(",")
              .map((tag) => tag.trim())
              .filter(Boolean),
          ),
        ],
      });
      const response = id
        ? await client.PUT("/api/brains/{brain}/skills/{id}", {
            params: { path: { brain: brain.id, id } },
            body,
          })
        : await client.POST("/api/brains/{brain}/skills", {
            params: { path: { brain: brain.id } },
            body,
          });
      onSaved(result(response));
    } catch (failure) {
      setError(failure as Error);
      setConflict(failure instanceof RequestError && failure.status === 409);
    } finally {
      setSaving(false);
    }
  }
  async function reload() {
    if (!id) return;
    setSaving(true);
    try {
      const current = result(
        await client.GET("/api/brains/{brain}/skills/{id}", {
          params: {
            path: { brain: brain.id, id },
            query: { environment_id: initial.environment_id ?? undefined },
          },
        }),
      );
      setDraft(bundleDraft(current));
      setTagsText(current.skill.tags.join(", "));
      setFileIndex(0);
      setConflict(false);
      setError(null);
    } catch (failure) {
      setError(failure as Error);
    } finally {
      setSaving(false);
    }
  }
  const accessDenied = denied(error);
  return (
    <Sheet open onOpenChange={(open) => !open && !saving && onClose()}>
      <SheetContent
        width={720}
        onPointerDownOutside={(event) => event.preventDefault()}
        onInteractOutside={(event) => event.preventDefault()}
        onEscapeKeyDown={(event) => saving && event.preventDefault()}
      >
        <SheetHeader>
          <SheetTitle>
            {id ? "Edit shared guidance" : "Publish shared guidance"}
          </SheetTitle>
          <SheetDescription>
            Publish a complete, versioned bundle. Only saved versions are
            available to agents.
          </SheetDescription>
        </SheetHeader>
        <SheetBody className="grid content-start gap-4">
          {error && (
            <Alert
              title={
                accessDenied
                  ? "Publishing access unavailable"
                  : conflict
                    ? "Publication conflict · draft retained"
                    : "Could not publish"
              }
              variant={conflict ? "warning" : "destructive"}
              actions={
                conflict && id ? (
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => void reload()}
                    loading={saving}
                  >
                    Load current version (replace draft)
                  </Button>
                ) : undefined
              }
            >
              <p className="m-0">{error.message}</p>
              {conflict && id && (
                <p className="m-0 mt-1 text-caption">
                  Your input is preserved. Loading the current version replaces
                  this draft.
                </p>
              )}
            </Alert>
          )}
          {!accessDenied && (
            <>
              <div className="grid grid-cols-2 items-start gap-3">
                <TextField
                  label="Name"
                  description="Lowercase letters, numbers and hyphens"
                  maxLength={64}
                  inputClassName="mono text-label"
                  value={draft.name}
                  onChange={(event) =>
                    patch({ name: event.currentTarget.value })
                  }
                  required
                />
                <Select
                  label="Guidance type"
                  data={[
                    { value: "skill", label: "Skill · SKILL.md" },
                    {
                      value: "instructions",
                      label: "Instructions · AGENTS.md",
                    },
                  ]}
                  value={draft.kind}
                  allowDeselect={false}
                  onChange={(kind) => {
                    setDraft(kindDraft(draft, kind as SkillInput["kind"]));
                    setFileIndex(0);
                  }}
                />
                <TextareaField
                  className="col-span-2"
                  label="Description"
                  description="Helps agents decide when to load this guidance"
                  minRows={2}
                  autosize
                  maxLength={1024}
                  value={draft.description}
                  onChange={(event) =>
                    patch({ description: event.currentTarget.value })
                  }
                  required
                />
                <Select
                  label="Environment"
                  placeholder="Brain-wide"
                  clearable
                  value={draft.environment_id ?? null}
                  onChange={(value) => patch({ environment_id: value })}
                  data={environmentOptions}
                />
                <TextField
                  label="Tags"
                  description="Separate tags with commas"
                  value={tagsText}
                  onChange={(event) => setTagsText(event.currentTarget.value)}
                />
                <MultiSelect
                  className="col-span-2"
                  label="Accompanying MCP tool groups"
                  description="Advisory links. Tool use permissions remain separate."
                  searchable
                  clearable
                  maxValues={16}
                  value={draft.profile_ids}
                  onChange={(value) => patch({ profile_ids: value })}
                  data={[
                    ...profileOptions,
                    ...draft.profile_ids
                      .filter(
                        (value) =>
                          !profileOptions.some(
                            (option) => option.value === value,
                          ),
                      )
                      .map((value) => ({
                        value,
                        label: "Unavailable tool group",
                      })),
                  ]}
                  disabled={!!associationError}
                />
                <Switch
                  className="col-span-2"
                  label="Enabled for agent discovery"
                  checked={draft.enabled}
                  onChange={(event) =>
                    patch({ enabled: event.currentTarget.checked })
                  }
                />
              </div>
              {associationError && (
                <Alert variant="warning" title="Association choices unavailable">
                  Existing associations are preserved.{" "}
                  {associationError.message}
                </Alert>
              )}
              <section className="grid gap-3 border-t border-border pt-4">
                <div className="flex items-center justify-between gap-2">
                  <h3 className="m-0 text-title font-semibold text-foreground">
                    Bundle files
                  </h3>
                  <div className="flex items-center gap-1.5">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => fileInput.current?.click()}
                    >
                      <Upload aria-hidden="true" />
                      Import files
                    </Button>
                    <input
                      ref={fileInput}
                      type="file"
                      hidden
                      multiple
                      accept=".md,.txt,LICENSE"
                      onChange={(event) =>
                        void importFiles([
                          ...(event.currentTarget.files ?? []),
                        ])
                      }
                    />
                    <Button
                      size="sm"
                      variant="secondary"
                      disabled={draft.files.length >= 64}
                      onClick={() => {
                        patch({
                          files: [
                            ...draft.files,
                            {
                              path: `references/note-${draft.files.length}.md`,
                              content: "",
                            },
                          ],
                        });
                        setFileIndex(draft.files.length);
                      }}
                    >
                      <Plus aria-hidden="true" />
                      Add reference
                    </Button>
                  </div>
                </div>
                <div
                  className="flex flex-wrap gap-1 border-b border-border pb-2"
                  role="group"
                  aria-label="Bundle files"
                >
                  {draft.files.map((entry, index) => (
                    <button
                      type="button"
                      key={index}
                      aria-pressed={index === fileIndex}
                      className={cn(
                        "mono inline-flex h-7 max-w-[240px] items-center truncate rounded-md px-2.5 text-caption transition-colors duration-fast focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
                        index === fileIndex
                          ? "bg-secondary font-medium text-foreground"
                          : "text-muted-foreground hover:bg-accent hover:text-foreground",
                      )}
                      onClick={() => setFileIndex(index)}
                    >
                      {entry.path || "Unnamed file"}
                    </button>
                  ))}
                </div>
                {file && (
                  <>
                    <div className="flex items-end gap-2">
                      <TextField
                        className="min-w-0 flex-1"
                        label="Resource path"
                        inputClassName="mono text-label"
                        value={file.path}
                        onChange={(event) =>
                          patchFile({ path: event.currentTarget.value })
                        }
                      />
                      <IconButton
                        variant="destructive-ghost"
                        label={`Remove ${file.path || "file"}`}
                        disabled={draft.files.length === 1}
                        onClick={() => {
                          patch({
                            files: draft.files.filter(
                              (_, index) => index !== fileIndex,
                            ),
                          });
                          setFileIndex(0);
                        }}
                      >
                        <Trash2 aria-hidden="true" />
                      </IconButton>
                    </div>
                    <TextareaField
                      label="File content"
                      inputClassName="mono text-label"
                      minRows={14}
                      autosize
                      maxRows={26}
                      spellCheck={false}
                      value={file.content}
                      onChange={(event) =>
                        patchFile({ content: event.currentTarget.value })
                      }
                    />
                  </>
                )}
                <p className="m-0 text-caption text-muted-foreground">
                  Markdown, .txt and LICENSE only; up to 64 files, 128 KiB each
                  and 512 KiB total. Scripts, host permission overrides and
                  shell interpolation are unsupported. SKILL.md name and
                  description follow the form when published.
                </p>
              </section>
            </>
          )}
        </SheetBody>
        {!accessDenied && (
          <SheetFooter className="justify-between">
            <span className="text-caption text-muted-foreground">
              {id ? (
                <>
                  Publishing creates version{" "}
                  <span className="mono">
                    {(draft.expected_version ?? 0) + 1}
                  </span>
                  .
                </>
              ) : (
                "First publication creates version 1."
              )}
            </span>
            <div className="flex items-center gap-2">
              <Button variant="ghost" onClick={onClose} disabled={saving}>
                Cancel
              </Button>
              <Button onClick={() => void publish()} loading={saving}>
                {id ? "Publish new version" : "Publish guidance"}
              </Button>
            </div>
          </SheetFooter>
        )}
      </SheetContent>
    </Sheet>
  );
}
