import { parseDocument, stringify } from "yaml";
import type { components } from "../../api-schema";

export type SkillInput = components["schemas"]["ManagedSkillInput"];
export type SkillBundle = components["schemas"]["ManagedSkillBundle"];
export type SkillSummary = components["schemas"]["ManagedSkillSummary"];
export type SkillFile = components["schemas"]["ManagedSkillFile"];

export function newSkill(environment: string | null): SkillInput {
  return {
    name: "",
    description: "",
    kind: "skill",
    environment_id: environment,
    profile_ids: [],
    tags: [],
    enabled: true,
    expected_version: null,
    files: [
      {
        path: "SKILL.md",
        content: '---\nname: ""\ndescription: ""\n---\n\n# Instructions\n\n',
      },
    ],
  };
}

export function bundleDraft(bundle: SkillBundle): SkillInput {
  const { skill, files } = bundle;
  return {
    name: skill.name,
    description: skill.description,
    kind: skill.kind,
    environment_id: skill.environment_id ?? null,
    profile_ids: [...skill.profile_ids],
    tags: [...skill.tags],
    enabled: skill.enabled,
    expected_version: skill.version,
    files: files.map((file) => ({ ...file })),
  };
}

// Metadata has one owner in the form. Preserve optional standard frontmatter
// fields and the authored Markdown when publishing a renamed/re-described skill.
export function publicationDraft(draft: SkillInput): SkillInput {
  if (!draft.name.trim() || !draft.description.trim())
    throw new Error("Enter a name and description before publishing.");
  const files = draft.files.map((file) => ({ ...file }));
  if (draft.kind === "skill") {
    const entry = files.find((file) => file.path === "SKILL.md");
    if (!entry) throw new Error("A skill requires a root SKILL.md file.");
    const match = /^(?:\uFEFF)?---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/.exec(
      entry.content,
    );
    if (!match)
      throw new Error(
        "SKILL.md must start with YAML frontmatter between --- lines.",
      );
    const document = parseDocument(match[1], { uniqueKeys: true });
    if (document.errors.length)
      throw new Error(
        "SKILL.md frontmatter is invalid YAML. Fix it before publishing.",
      );
    const metadata = document.toJS();
    if (!metadata || typeof metadata !== "object" || Array.isArray(metadata))
      throw new Error("SKILL.md frontmatter must be a YAML mapping.");
    document.set("name", draft.name);
    document.set("description", draft.description);
    entry.content = `---\n${document.toString()}---\n${entry.content.slice(match[0].length)}`;
  } else if (!files.some((file) => file.path === "AGENTS.md")) {
    throw new Error("An instructions bundle requires a root AGENTS.md file.");
  }
  return { ...draft, files };
}

export function kindDraft(
  draft: SkillInput,
  kind: SkillInput["kind"],
): SkillInput {
  if (draft.kind === kind) return draft;
  const oldPath = draft.kind === "skill" ? "SKILL.md" : "AGENTS.md";
  const newPath = kind === "skill" ? "SKILL.md" : "AGENTS.md";
  const files = draft.files.map((file) => {
    if (
      file.path !== oldPath ||
      draft.files.some((other) => other.path === newPath)
    )
      return file;
    const content = file.content.replace(
      /^(?:\uFEFF)?---\r?\n[\s\S]*?\r?\n---(?:\r?\n|$)/,
      "",
    );
    return {
      path: newPath,
      content:
        kind === "skill"
          ? `---\n${stringify({ name: draft.name, description: draft.description })}---\n${content}`
          : content,
    };
  });
  return { ...draft, kind, files };
}
