import { useRef, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Drawer,
  FileButton,
  Group,
  MultiSelect,
  Select,
  Stack,
  Switch,
  Text,
  Textarea,
  TextInput,
  ActionIcon,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
  BookOpen,
  ChevronRight,
  FileText,
  Plus,
  Search,
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
import "./skills.css";

const denied = (error: Error | null) =>
  error instanceof RequestError && [401, 403, 404].includes(error.status);

export function SkillsPanel() {
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
  const inventory = useQuery({
    queryKey: ["workspace", brain.id, "skills"],
    retry: false,
    gcTime: 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          signal,
          params: { path: { brain: brain.id } },
        }),
      ),
  });
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
    refetchInterval: 5000,
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
    refetchInterval: 5000,
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
  return (
    <div className="skills-panel">
      <div className="skills-heading">
        <div>
          <div className="skills-eyebrow">
            <BookOpen size={16} /> Shared guidance
          </div>
          <h2>One library for your agents</h2>
          <p>
            Publish skills and working instructions for this Brain. Agents fetch
            what they need, when they need it.
          </p>
        </div>
        {admin && !noAccess && (
          <Button leftSection={<Plus size={18} />} onClick={openNew}>
            Publish skill
          </Button>
        )}
      </div>
      <div className="skills-flow">
        <span>
          <b>01</b> Manager publishes
        </span>
        <ChevronRight size={15} />
        <span>
          <b>02</b> Agent discovers
        </span>
        <ChevronRight size={15} />
        <span>
          <b>03</b> Instructions fetched on demand
        </span>
      </div>
      {notice && !skills.isPending && !skills.isError && !noAccess && (
        <Alert color="brand" title="Published" mb="md">
          {notice}
        </Alert>
      )}
      <div className="skills-toolbar">
        <TextInput
          aria-label="Find shared guidance"
          placeholder="Find skills, instructions or tags…"
          leftSection={<Search size={17} />}
          value={query}
          onChange={(event) => setQuery(event.currentTarget.value)}
        />
        <Select
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
            label="Include disabled"
            checked={showDisabled}
            onChange={(event) => setShowDisabled(event.currentTarget.checked)}
          />
        )}
      </div>
      {inventory.isError && !noAccess && (
        <Alert color="yellow" title="Environment list unavailable" mb="md">
          {inventory.error.message}{" "}
          <Button
            variant="subtle"
            size="xs"
            onClick={() => void inventory.refetch()}
          >
            Retry environments
          </Button>
        </Alert>
      )}
      {noAccess ? (
        <Alert title="Skills access unavailable" color="gray">
          Your access to this Brain is no longer available. Previous guidance is
          hidden.
        </Alert>
      ) : skills.isPending ? (
        <LoadingState label="Loading shared guidance…" />
      ) : skills.isError ? (
        <ErrorState error={skills.error} retry={() => void skills.refetch()} />
      ) : !visible.length ? (
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
              <Button variant="light" onClick={openNew}>
                Publish your first skill
              </Button>
            ) : undefined
          }
        />
      ) : (
        <div className="skills-grid">
          {visible.map((skill) => (
            <button
              type="button"
              key={skill.id}
              className="management-surface skill-card"
              onClick={() => setSelected(skill)}
              aria-label={`Inspect ${skill.name}`}
            >
              <div className="skill-card-top">
                <span className="skill-icon">
                  {skill.kind === "skill" ? (
                    <BookOpen size={21} />
                  ) : (
                    <FileText size={21} />
                  )}
                </span>
                <Badge
                  size="sm"
                  variant="light"
                  color={skill.enabled ? "brand" : "gray"}
                >
                  {skill.enabled ? "Enabled" : "Disabled"}
                </Badge>
              </div>
              <h3>{skill.name}</h3>
              <p>{skill.description}</p>
              <div className="skill-card-tags">
                {skill.tags.slice(0, 4).map((tag) => (
                  <span key={tag}>{tag}</span>
                ))}
              </div>
              <div className="skill-card-meta">
                <span>
                  {skill.kind === "skill" ? "Skill" : "Instructions"} · v
                  {skill.version}
                </span>
                <span>{scopeName(skill.environment_id)}</span>
              </div>
              <div className="skill-card-footer">
                <span>
                  {skill.file_paths.length}{" "}
                  {skill.file_paths.length === 1 ? "file" : "files"}
                  {skill.profile_ids.length
                    ? ` · ${skill.profile_ids.length} tool ${skill.profile_ids.length === 1 ? "group" : "groups"}`
                    : ""}
                </span>
                <span>
                  Inspect <ChevronRight size={14} />
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
      <div className="skills-lifetime-note">
        <BookOpen size={17} />
        <p>
          Agents load selected instructions during work. Optional local copies
          can be temporary or kept; disabling stops future fetches. Already
          saved copies and conversation context remain with the agent. Local
          instructions and human direction retain priority.
        </p>
      </div>
      <Drawer
        opened={!!selected && !skills.isError && !noAccess}
        onClose={() => setSelected(null)}
        title="Shared guidance"
        position="right"
        size="lg"
        className="feature-drawer"
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
      </Drawer>
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
    <Stack gap="md" className="skill-details">
      <Group justify="space-between">
        <Badge color={bundle.skill.enabled ? "brand" : "gray"}>
          {bundle.skill.enabled ? "Enabled" : "Disabled"} · v
          {bundle.skill.version}
        </Badge>
        {onEdit && (
          <Button size="xs" variant="light" onClick={onEdit}>
            Edit guidance
          </Button>
        )}
      </Group>
      <h2>{bundle.skill.name}</h2>
      <Text size="sm">{bundle.skill.description}</Text>
      <Text size="sm" c="dimmed">
        {scope} ·{" "}
        {bundle.skill.kind === "skill"
          ? "SKILL.md skill"
          : "AGENTS.md instructions"}
      </Text>
      {!!bundle.skill.profile_ids.length && (
        <div className="skill-profile-links">
          <Text size="xs" c="dimmed">
            Accompanying tool groups · independent tool permissions apply
          </Text>
          {bundle.skill.profile_ids.map((id) => (
            <Link
              key={id}
              to="/brains/$brainId/connections"
              params={{ brainId: brain.id }}
              search={{ tab: "profiles", profile: id }}
            >
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
      <pre className="skill-file-preview" data-testid="skill-published-content">
        {file?.content}
      </pre>
      <Text size="xs" c="dimmed">
        Published {new Date(bundle.skill.updated_at).toLocaleString()}. These
        files are fetched on demand; viewing them does not install a native host
        skill.
      </Text>
    </Stack>
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
  const resetRef = useRef<() => void>(null);
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
    resetRef.current?.();
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
    <Drawer
      opened
      onClose={onClose}
      closeOnClickOutside={false}
      closeOnEscape={!saving}
      title={id ? "Edit shared guidance" : "Publish shared guidance"}
      position="right"
      size="xl"
      className="feature-drawer skill-editor-drawer"
    >
      <Stack gap="md">
        <Text size="sm" c="dimmed">
          Publish a complete, versioned bundle. Only saved versions are
          available to agents.
        </Text>
        {error && (
          <Alert
            title={
              accessDenied
                ? "Publishing access unavailable"
                : conflict
                  ? "Publication conflict · draft retained"
                  : "Could not publish"
            }
            color={conflict ? "yellow" : "red"}
          >
            <Text size="sm">{error.message}</Text>
            {conflict && id && (
              <>
                <Text size="xs" mt="xs">
                  Your input is preserved. Loading the current version replaces
                  this draft.
                </Text>
                <Button
                  size="xs"
                  variant="outline"
                  mt="sm"
                  onClick={() => void reload()}
                  loading={saving}
                >
                  Load current version (replace draft)
                </Button>
              </>
            )}
          </Alert>
        )}
        {!accessDenied && (
          <>
            <div className="skill-editor-fields">
              <TextInput
                label="Name"
                description="Lowercase letters, numbers and hyphens"
                maxLength={64}
                value={draft.name}
                onChange={(event) => patch({ name: event.currentTarget.value })}
                required
              />
              <Select
                label="Guidance type"
                data={[
                  { value: "skill", label: "Skill · SKILL.md" },
                  { value: "instructions", label: "Instructions · AGENTS.md" },
                ]}
                value={draft.kind}
                allowDeselect={false}
                onChange={(kind) => {
                  setDraft(kindDraft(draft, kind as SkillInput["kind"]));
                  setFileIndex(0);
                }}
              />
              <Textarea
                className="skill-description-field"
                label="Description"
                description="Helps agents decide when to load this guidance"
                minRows={2}
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
              <TextInput
                label="Tags"
                description="Separate tags with commas"
                value={tagsText}
                onChange={(event) => setTagsText(event.currentTarget.value)}
              />
              <MultiSelect
                className="skill-description-field"
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
                label="Enabled for agent discovery"
                checked={draft.enabled}
                onChange={(event) =>
                  patch({ enabled: event.currentTarget.checked })
                }
              />
            </div>
            {associationError && (
              <Alert color="yellow" title="Association choices unavailable">
                Existing associations are preserved. {associationError.message}
              </Alert>
            )}
            <section className="skill-editor-files">
              <Group justify="space-between">
                <h3>Bundle files</h3>
                <Group gap="xs">
                  <FileButton
                    multiple
                    onChange={(files) => void importFiles(files)}
                    resetRef={resetRef}
                    accept=".md,.txt,LICENSE"
                  >
                    {(props) => (
                      <Button
                        {...props}
                        size="xs"
                        variant="subtle"
                        leftSection={<Upload size={15} />}
                      >
                        Import files
                      </Button>
                    )}
                  </FileButton>
                  <Button
                    size="xs"
                    variant="light"
                    leftSection={<Plus size={15} />}
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
                    Add reference
                  </Button>
                </Group>
              </Group>
              <div className="skill-file-tabs">
                {draft.files.map((entry, index) => (
                  <button
                    type="button"
                    className={index === fileIndex ? "selected" : ""}
                    key={index}
                    onClick={() => setFileIndex(index)}
                  >
                    {entry.path || "Unnamed file"}
                  </button>
                ))}
              </div>
              {file && (
                <>
                  <Group align="end">
                    <TextInput
                      style={{ flex: 1 }}
                      label="Resource path"
                      value={file.path}
                      onChange={(event) =>
                        patchFile({ path: event.currentTarget.value })
                      }
                    />
                    <ActionIcon
                      variant="subtle"
                      color="red"
                      size="lg"
                      aria-label={`Remove ${file.path || "file"}`}
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
                      <Trash2 size={18} />
                    </ActionIcon>
                  </Group>
                  <Textarea
                    label="File content"
                    className="skill-content-editor"
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
              <Text size="xs" c="dimmed" mt="sm">
                Markdown, .txt and LICENSE only; up to 64 files, 128 KiB each
                and 512 KiB total. Scripts, host permission overrides and shell
                interpolation are unsupported. SKILL.md name and description
                follow the form when published.
              </Text>
            </section>
            <Group justify="space-between" className="skill-publish-actions">
              <Text size="xs" c="dimmed">
                {id
                  ? `Publishing creates version ${(draft.expected_version ?? 0) + 1}.`
                  : "First publication creates version 1."}
              </Text>
              <Group>
                <Button variant="subtle" onClick={onClose} disabled={saving}>
                  Cancel
                </Button>
                <Button onClick={() => void publish()} loading={saving}>
                  {id ? "Publish new version" : "Publish guidance"}
                </Button>
              </Group>
            </Group>
          </>
        )}
      </Stack>
    </Drawer>
  );
}
