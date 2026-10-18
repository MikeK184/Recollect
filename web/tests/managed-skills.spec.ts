import { test, expect, type Page } from "@playwright/test";
import { randomUUID } from "node:crypto";
import { spawn, type ChildProcess } from "node:child_process";
import { createServer } from "node:net";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { parse } from "yaml";
import type {
  SkillBundle,
  SkillInput,
} from "../src/features/skills/skillDraft";

let vite: ChildProcess;
let origin: string;
test.beforeAll(async () => {
  const server = createServer();
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string") throw Error("No fixture port");
  const port = address.port;
  await new Promise<void>((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  origin = `http://127.0.0.1:${port}`;
  vite = spawn(
    process.execPath,
    [
      "node_modules/vite/bin/vite.js",
      "--host",
      "127.0.0.1",
      "--port",
      String(port),
      "--strictPort",
    ],
    { cwd: fileURLToPath(new URL("../", import.meta.url)), stdio: "ignore" },
  );
  await expect
    .poll(
      async () => {
        try {
          return (await fetch(origin)).ok;
        } catch {
          return false;
        }
      },
      { timeout: 20000 },
    )
    .toBe(true);
});
test.afterAll(() => {
  vite?.kill("SIGTERM");
});

async function fixture(page: Page, role = "admin", seed = true) {
  const brain = randomUUID(),
    environment = randomUUID(),
    profile = randomUUID();
  const now = "2026-10-18T12:00:00Z";
  const brainData = {
    id: brain,
    name: "Platform engineering",
    description: "Shared team memory",
    role,
    archived: false,
    owner_id: randomUUID(),
    created_at: now,
    updated_at: now,
  };
  const skill: SkillBundle = {
    skill: {
      id: randomUUID(),
      brain_id: brain,
      name: "vault-operations",
      description:
        "Investigate Vault configuration with evidence and use the approved read tools.",
      kind: "skill",
      version: 1,
      environment_id: null,
      profile_ids: [profile],
      tags: ["vault", "platform"],
      enabled: true,
      file_paths: ["SKILL.md", "references/checklist.md"],
      updated_at: now,
    },
    files: [
      {
        path: "SKILL.md",
        content:
          '---\nname: vault-operations\ndescription: Investigate Vault configuration with evidence and use the approved read tools.\n---\n\n# Vault operations\n\nRead [the checklist](references/checklist.md).\n<img src=x onerror="window.skillInjected=true">',
      },
      {
        path: "references/checklist.md",
        content: "Check repository evidence before describing runtime state.",
      },
    ],
  };
  const environmentSkill: SkillBundle = {
    skill: {
      ...skill.skill,
      id: randomUUID(),
      name: "production-guidance",
      description:
        "Production checks and ownership guidance for the on-call team.",
      kind: "instructions",
      environment_id: environment,
      file_paths: ["AGENTS.md"],
      tags: ["production"],
    },
    files: [
      {
        path: "AGENTS.md",
        content:
          "# Production guidance\n\nConfirm the selected environment before running a check.",
      },
    ],
  };
  const state = {
    denied: false,
    deniedStatus: 403,
    conflict: false,
    failPublish: false,
    failList: false,
    bundles: seed ? [skill, environmentSkill] : ([] as SkillBundle[]),
    writes: [] as SkillInput[],
    selections: [] as string[],
    includeDisabled: [] as boolean[],
  };
  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    const method = route.request().method();
    const json = (body: unknown, status = 200) =>
      route.fulfill({
        status,
        contentType: "application/json",
        body: JSON.stringify(body),
      });
    if (path === "/api/auth/me")
      return json({
        user: {
          id: randomUUID(),
          username: "Platform manager",
          installation_owner: true,
        },
        csrf_token: "synthetic-csrf",
      });
    if (path === "/api/brains") return json([brainData]);
    if (path === `/api/brains/${brain}`) return json(brainData);
    if (path === "/api/status") return json({ ready: true });
    if (path === `/api/brains/${brain}/workspace`)
      return json({
        areas: [],
        repositories: [],
        environments: [{ id: environment, name: "Production" }],
        checkouts: [],
        workspaces: [],
        tasks: [],
        checkout_total: 0,
        checkout_offset: 0,
        task_total: 0,
        task_offset: 0,
      });
    if (path === `/api/brains/${brain}/mcp`)
      return json({
        can_configure: role === "admin",
        connections: [],
        definitions: [],
        profiles: [
          {
            id: profile,
            name: "Vault read tools",
            brain_id: brain,
            enabled: true,
            environment_id: null,
            connection_ids: [],
            rights: { use_profile: false, manage: true, share: true },
            description: "Approved tooling",
            revision: randomUUID(),
            updated_at: now,
          },
        ],
        execution_state: "enabled",
      });
    if (path === `/api/brains/${brain}/agents`)
      return json({ groups: [], hidden_count: 0 });
    if (path === `/api/brains/${brain}/skills`) {
      if (state.denied)
        return json({ message: "Brain access revoked" }, state.deniedStatus);
      if (state.failList)
        return json({ message: "Guidance service unavailable" }, 503);
      if (method === "GET") {
        const selected = url.searchParams.get("environment_id");
        const includeDisabled =
          url.searchParams.get("include_disabled") === "true";
        state.selections.push(selected ?? "global");
        state.includeDisabled.push(includeDisabled);
        const term = (url.searchParams.get("query") ?? "").toLowerCase();
        return json(
          state.bundles
            .filter(
              ({ skill: value }) =>
                (!value.environment_id || value.environment_id === selected) &&
                (value.enabled || includeDisabled) &&
                `${value.name} ${value.description} ${value.tags.join(" ")}`
                  .toLowerCase()
                  .includes(term),
            )
            .map(({ skill: value }) => value),
        );
      }
      const input = route.request().postDataJSON() as SkillInput;
      state.writes.push(input);
      if (state.failPublish)
        return json({ message: "Publication service unavailable" }, 503);
      const created: SkillBundle = {
        skill: {
          id: randomUUID(),
          brain_id: brain,
          ...input,
          version: 1,
          environment_id: input.environment_id ?? null,
          file_paths: input.files.map((file) => file.path),
          updated_at: now,
        },
        files: input.files,
      };
      state.bundles.push(created);
      return json(created);
    }
    if (path.startsWith(`/api/brains/${brain}/skills/`)) {
      if (state.denied)
        return json({ message: "Brain access revoked" }, state.deniedStatus);
      const index = state.bundles.findIndex(
        (entry) => entry.skill.id === path.split("/").at(-1),
      );
      if (index < 0) return json({ message: "Guidance unavailable" }, 404);
      if (method === "GET") return json(state.bundles[index]);
      const input = route.request().postDataJSON() as SkillInput;
      state.writes.push(input);
      if (state.conflict)
        return json(
          {
            message:
              "This guidance changed elsewhere. Reload the current version.",
          },
          409,
        );
      state.bundles[index] = {
        skill: {
          ...state.bundles[index].skill,
          ...input,
          version: state.bundles[index].skill.version + 1,
          file_paths: input.files.map((file) => file.path),
        },
        files: input.files,
      };
      return json(state.bundles[index]);
    }
    return json({ message: "Unneeded fixture route" }, 503);
  });
  await page.goto(`${origin}/brains/${brain}/agents?tab=skills`);
  await expect(
    page.getByRole("heading", { name: "One library for your agents" }),
  ).toBeVisible();
  return { brain, environment, profile, state, skill };
}

test("manager publishes an environment skill with inert reference files and synchronized metadata", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const { state, environment, profile } = await fixture(page, "admin", false);
  await expect(
    page.getByText("Shared skills start here", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Publish skill", exact: true })
    .click();
  const editor = page.getByRole("dialog", { name: "Publish shared guidance" });
  await editor
    .getByRole("textbox", { name: "Name", exact: true })
    .fill("postgres-maintenance");
  await editor
    .getByRole("textbox", { name: "Description", exact: true })
    .fill("Use when diagnosing PostgreSQL performance.");
  await editor
    .getByRole("textbox", { name: "Tags", exact: true })
    .fill("postgres, operations");
  await editor
    .getByRole("textbox", { name: "Environment", exact: true })
    .click();
  await page.getByRole("option", { name: "Production", exact: true }).click();
  await editor
    .getByRole("textbox", { name: "Accompanying MCP tool groups" })
    .click();
  await page
    .getByRole("option", { name: "Vault read tools", exact: true })
    .click();
  await editor.getByRole("textbox", { name: "Name", exact: true }).click();
  await editor.locator('input[type="file"]').setInputFiles({
    name: "checklist.md",
    mimeType: "text/markdown",
    buffer: Buffer.from(
      "# Diagnostic reference\nInspect evidence before reporting runtime state.",
    ),
  });
  await expect(
    editor.getByRole("textbox", { name: "Resource path" }),
  ).toHaveValue("checklist.md");
  await editor
    .getByRole("textbox", { name: "Resource path" })
    .fill("references/checklist.md");
  await editor
    .getByRole("button", { name: "Publish guidance", exact: true })
    .click();
  await expect(editor).toHaveCount(0);
  expect(state.writes).toHaveLength(1);
  const input = state.writes[0];
  expect(input.environment_id).toBe(environment);
  expect(input.profile_ids).toEqual([profile]);
  expect(input.tags).toEqual(["postgres", "operations"]);
  expect(input.files.map((file) => file.path)).toEqual([
    "SKILL.md",
    "references/checklist.md",
  ]);
  const frontmatter = parse(input.files[0].content.split("---")[1]);
  expect(frontmatter.name).toBe("postgres-maintenance");
  expect(frontmatter.description).toBe(input.description);
  await expect(
    page.getByRole("button", { name: "Inspect postgres-maintenance" }),
  ).toBeVisible();
  expect(state.selections).toContain(environment);
  await page
    .getByRole("button", { name: "Inspect postgres-maintenance" })
    .click();
  await expect(page.getByTestId("skill-published-content")).toContainText(
    "postgres-maintenance",
  );
  await page.getByRole("button", { name: "Close dialog", exact: true }).click();
  await page.getByRole("tab", { name: "Agent setup", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: "Connect your coding tool",
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("tab", { name: "Skills", exact: true }).click();
  await expect(page).toHaveURL(/tab=skills/);
  expect(errors).toEqual([]);
});

test("version conflicts preserve the draft and an explicit reload supports a new disabled version", async ({
  page,
}) => {
  const { state, skill } = await fixture(page);
  await page.getByRole("button", { name: "Inspect vault-operations" }).click();
  await page
    .getByRole("button", { name: "Edit guidance", exact: true })
    .click();
  const editor = page.getByRole("dialog", {
    name: "Edit shared guidance",
    exact: true,
  });
  await editor
    .getByRole("textbox", { name: "Description", exact: true })
    .fill("My unsaved update survives a conflict.");
  state.conflict = true;
  state.bundles[0] = {
    ...skill,
    skill: {
      ...skill.skill,
      version: 2,
      description: "A colleague published version two.",
    },
  };
  await editor.getByRole("button", { name: "Publish new version" }).click();
  await expect(
    editor.getByText("Publication conflict · draft retained", { exact: true }),
  ).toBeVisible();
  await expect(
    editor.getByRole("textbox", { name: "Description", exact: true }),
  ).toHaveValue("My unsaved update survives a conflict.");
  expect(state.writes[0].expected_version).toBe(1);
  await editor
    .getByRole("button", {
      name: "Load current version (replace draft)",
      exact: true,
    })
    .click();
  await expect(
    editor.getByRole("textbox", { name: "Description", exact: true }),
  ).toHaveValue("A colleague published version two.");
  state.conflict = false;
  await editor
    .getByRole("switch", { name: "Enabled for agent discovery", exact: true })
    .uncheck();
  await editor.getByRole("button", { name: "Publish new version" }).click();
  await expect(editor).toHaveCount(0);
  expect(state.writes.at(-1)?.expected_version).toBe(2);
  expect(state.writes.at(-1)?.enabled).toBe(false);
  await expect(
    page.getByRole("switch", { name: "Include disabled" }),
  ).toBeChecked();
  await expect(
    page.getByRole("button", { name: "Inspect vault-operations" }),
  ).toContainText("Disabled");
  expect(state.includeDisabled).toContain(true);
});

test("reader sees applicable guidance without mutation controls and revoked content disappears", async ({
  page,
}) => {
  const { state, environment } = await fixture(page, "reader");
  await expect(
    page.getByRole("button", { name: "Publish skill", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Inspect production-guidance" }),
  ).toHaveCount(0);
  await page.getByRole("textbox", { name: "Guidance environment" }).click();
  await page
    .getByRole("option", { name: "Production + Brain-wide", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Inspect production-guidance" }),
  ).toBeVisible();
  expect(state.selections).toContain(environment);
  expect(state.includeDisabled).not.toContain(true);
  await page.getByRole("button", { name: "Inspect vault-operations" }).click();
  await expect(page.getByTestId("skill-published-content")).toContainText(
    '<img src=x onerror="window.skillInjected=true">',
  );
  expect(
    await page.evaluate(
      () => (window as unknown as { skillInjected?: boolean }).skillInjected,
    ),
  ).toBeUndefined();
  await expect(
    page.getByRole("button", { name: "Edit guidance", exact: true }),
  ).toHaveCount(0);
  state.denied = true;
  await expect(
    page.getByText("Skills access unavailable", { exact: true }),
  ).toBeVisible({ timeout: 12000 });
  await expect(page.getByTestId("skill-published-content")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Inspect vault-operations" }),
  ).toHaveCount(0);
  expect(state.writes).toHaveLength(0);
});

test("missing Brain 404 clears a copied publication draft and prevents further management", async ({
  page,
}) => {
  const { state } = await fixture(page);
  await page.getByRole("button", { name: "Inspect vault-operations" }).click();
  await page
    .getByRole("button", { name: "Edit guidance", exact: true })
    .click();
  await expect(
    page.getByRole("textbox", { name: "File content", exact: true }),
  ).toHaveValue(/Vault operations/);
  state.deniedStatus = 404;
  state.denied = true;
  await expect(
    page.getByText("Skills access unavailable", { exact: true }),
  ).toBeVisible({ timeout: 12000 });
  await expect(
    page.getByRole("textbox", { name: "File content", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Publish skill", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Inspect vault-operations" }),
  ).toHaveCount(0);
});

test("network and YAML validation failures retain editor input, with scoped cards and visual capture", async ({
  page,
}) => {
  const { state } = await fixture(page);
  await expect(
    page.getByRole("button", { name: "Inspect vault-operations" }),
  ).toBeVisible();
  await mkdir("../.cache", { recursive: true });
  await page.screenshot({
    path: "../.cache/managed-skills-ui.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 760, height: 960 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.setViewportSize({ width: 1440, height: 960 });
  await page
    .getByRole("button", { name: "Publish skill", exact: true })
    .click();
  const editor = page.getByRole("dialog", { name: "Publish shared guidance" });
  await editor
    .getByRole("textbox", { name: "Name", exact: true })
    .fill("network-recovery");
  await editor
    .getByRole("textbox", { name: "Description", exact: true })
    .fill("Keep this draft during a failed request.");
  await editor
    .getByRole("textbox", { name: "File content", exact: true })
    .fill("---\nname: [bad\n---\nRetained draft");
  await editor
    .getByRole("button", { name: "Publish guidance", exact: true })
    .click();
  await expect(
    editor.getByText(
      "SKILL.md frontmatter is invalid YAML. Fix it before publishing.",
    ),
  ).toBeVisible();
  expect(state.writes).toHaveLength(0);
  await editor
    .getByRole("textbox", { name: "File content", exact: true })
    .fill(
      "---\nname: old-name\ndescription: old description\n---\nRetained draft",
    );
  state.failPublish = true;
  await editor
    .getByRole("button", { name: "Publish guidance", exact: true })
    .click();
  await expect(
    editor.getByText("Publication service unavailable", { exact: true }),
  ).toBeVisible();
  await expect(
    editor.getByRole("textbox", { name: "File content", exact: true }),
  ).toHaveValue(/Retained draft/);
  await expect(
    editor.getByRole("textbox", { name: "Name", exact: true }),
  ).toHaveValue("network-recovery");
  state.failPublish = false;
  await editor
    .getByRole("button", { name: "Publish guidance", exact: true })
    .click();
  await expect(editor).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Inspect network-recovery" }),
  ).toBeVisible();
});
