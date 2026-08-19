import { test, expect } from "@playwright/test";
import { spawn, execFile, type ChildProcess } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdir, writeFile, rm } from "node:fs/promises";
import { resolve } from "node:path";
import { promisify } from "node:util";

test("paired workspace refresh and task/subagent history preserve operation scope", async ({
  page,
}) => {
  test.setTimeout(90_000);
  page.setDefaultTimeout(10_000);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const fixture = resolve(`../.cache/workspace-ui-${randomUUID()}`);
  const binary = resolve("../target/debug/recollect-agent");
  const env = {
    PATH: process.env.PATH,
    HOME: process.env.HOME,
    TMPDIR: process.env.TMPDIR,
    RECOLLECT_URL: process.env.RECOLLECT_UI_TEST_ORIGIN!,
    RECOLLECT_DEVICE_PROFILE: `workspace-ui-${randomUUID()}`,
  };
  const run = (args: string[]) =>
    new Promise<{ ok: boolean; stdout: string; stderr: string }>((done) =>
      execFile(
        binary,
        args,
        { env, timeout: 35_000 },
        (error, stdout, stderr) => done({ ok: !error, stdout, stderr }),
      ),
    );
  async function command(args: string[]) {
    const result = await run(args);
    expect(result.ok, result.stderr).toBe(true);
    return JSON.parse(result.stdout);
  }
  let companion: ChildProcess | undefined;
  await mkdir(resolve(fixture, ".recollect"), { recursive: true });
  try {
    await page.goto("/");
    await page
      .getByLabel(/^Username/)
      .fill(process.env.RECOLLECT_OWNER_USERNAME!);
    await page
      .getByLabel(/^Password/)
      .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
    await page.getByRole("button", { name: "Sign in", exact: true }).click();
    await page
      .getByRole("button", { name: "Create Brain", exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel(/^Name/).fill("Workspace investigation");
    await dialog
      .getByRole("button", { name: "Create Brain", exact: true })
      .click();
    await page.waitForURL(/\/brains\/[0-9a-f-]{36}$/);
    await expect(
      page.getByRole("heading", {
        name: "Workspace investigation",
        exact: true,
      }),
    ).toBeVisible();
    const brainURL = page.url();
    const brain = new URL(brainURL).pathname.split("/").at(-1)!;
    await writeFile(
      resolve(fixture, ".recollect/workspace.toml"),
      `brain = "${brain}"\n`,
    );
    const git = promisify(execFile);
    for (const name of ["infra", "app"]) {
      const path = resolve(fixture, name);
      await mkdir(path);
      await git("git", ["-C", path, "init", "-q"]);
      await git("git", [
        "-C",
        path,
        "remote",
        "add",
        "origin",
        `git@example.test:Team/${name}.git`,
      ]);
      await writeFile(
        resolve(path, "example.txt"),
        "Synthetic checkout metadata fixture\n",
      );
    }
    await page
      .getByRole("button", { name: "Manage views", exact: true })
      .click();
    for (const [kind, name] of [
      ["Area", "Vault"],
      ["Environment", "Production"],
    ]) {
      await dialog
        .getByRole("textbox", { name: "View kind", exact: true })
        .click();
      await page.getByRole("option", { name: kind, exact: true }).click();
      await dialog.getByLabel(/^View name/).fill(name);
      await dialog
        .getByRole("button", { name: "Create view", exact: true })
        .click();
      await expect(dialog.getByText(name, { exact: true })).toBeVisible();
    }
    await page.keyboard.press("Escape");
    let output = "";
    let pairingError = "";
    companion = spawn(binary, ["pair", "Workspace proof companion"], {
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    companion.stdout!.on("data", (chunk) => {
      output += chunk.toString();
    });
    companion.stderr!.on("data", (chunk) => {
      pairingError += chunk.toString();
    });
    const paired = new Promise<number | null>((done) => {
      companion!.once("exit", done);
      companion!.once("error", () => done(-1));
    });
    await expect
      .poll(() => output.includes("Open ") || pairingError.length > 0)
      .toBe(true);
    expect(pairingError).toBe("");
    const approval = output.match(/Open (\S+)/)?.[1];
    expect(approval).toBeTruthy();
    await page.goto(approval!);
    await page
      .getByRole("button", { name: "Approve device", exact: true })
      .click();
    await expect(page.getByText("Your companion is connected.")).toBeVisible({
      timeout: 20_000,
    });
    expect(await paired, pairingError).toBe(0);
    companion = undefined;
    const offline = await command([
      "workspace",
      "discover",
      resolve(fixture, "infra"),
    ]);
    expect(offline.brain_selector).toBe(brain);
    expect(offline.checkouts).toHaveLength(2);
    const refreshed = await command(["workspace", "refresh", fixture]);
    expect(refreshed.refresh.observed_checkouts).toBe(2);
    expect(refreshed.catalogue.repositories).toHaveLength(2);
    const otherRoot = resolve(fixture, "other-workspace");
    await mkdir(resolve(otherRoot, ".recollect"), { recursive: true });
    await writeFile(
      resolve(otherRoot, ".recollect/workspace.toml"),
      `brain = "${brain}"\n`,
    );
    const unregistered = await command(["workspace", "list", otherRoot]);
    expect(unregistered.selected_workspace).toBeNull();
    expect(unregistered.checkouts).toHaveLength(0);
    await command(["workspace", "refresh", otherRoot]);
    const selectedWorkspace = await command(["workspace", "list", fixture]);
    expect(selectedWorkspace.selected_workspace).toBe(
      refreshed.refresh.workspace.id,
    );
    expect(selectedWorkspace.checkouts).toHaveLength(2);
    expect(
      (await command(["workspace", "list", fixture])).repositories,
    ).toHaveLength(2);
    const repositories = refreshed.catalogue.repositories as {
      id: string;
      canonical_origin: string;
    }[];
    const infra = repositories.find((r) =>
      r.canonical_origin.endsWith("/infra"),
    )!.id;
    const app = repositories.find((r) =>
      r.canonical_origin.endsWith("/app"),
    )!.id;
    const area = refreshed.catalogue.areas[0].id;
    const environment = refreshed.catalogue.environments[0].id;
    const parent = await command([
      "scope",
      "start",
      brain,
      "Native investigation",
      "--repository",
      infra,
      "--area",
      area,
      "--environment",
      environment,
      "--workspace",
      refreshed.refresh.workspace.id,
    ]);
    const original = await command([
      "scope",
      "begin",
      brain,
      parent.task.id,
      "context",
    ]);
    await page.goto(brainURL);
    await page
      .getByRole("button", { name: "Native investigation", exact: true })
      .click();
    await expect(
      dialog.getByText("example.test/Team/infra", { exact: false }),
    ).toBeVisible();
    await expect(dialog.getByTestId("operation-binding")).toHaveCount(1);
    await dialog
      .getByRole("button", { name: "Change scope", exact: true })
      .click();
    await dialog
      .getByRole("textbox", { name: "Repositories", exact: true })
      .click();
    await dialog
      .getByRole("textbox", { name: "Repositories", exact: true })
      .fill("app");
    await page
      .getByRole("option", { name: "example.test/Team/app", exact: true })
      .click();
    await page.route("**/workspace/tasks/*/scope", (route) => route.abort(), {
      times: 1,
    });
    await dialog
      .getByRole("button", { name: "Save scope", exact: true })
      .click();
    await expect(
      dialog.getByText("Request failed", { exact: true }),
    ).toBeVisible();
    await dialog
      .getByRole("button", { name: "Save scope", exact: true })
      .click();
    await expect(
      dialog.getByRole("heading", {
        name: "Native investigation",
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      dialog.getByTestId("operation-binding").getByText(/Earlier scope/),
    ).toBeVisible();
    const current = await command(["scope", "inspect", brain, parent.task.id]);
    expect(current.task.scope.selection.repository_ids).toHaveLength(2);
    expect(current.operations[0].scope.id).toBe(original.scope.id);
    expect(current.operations[0].scope.selection.repository_ids).toEqual([
      infra,
    ]);
    await dialog
      .getByRole("button", { name: "Start subagent", exact: true })
      .click();
    await dialog.getByLabel(/^Task name/).fill("Terraform review");
    await dialog
      .getByRole("button", { name: "Start task", exact: true })
      .click();
    await expect(
      dialog.getByRole("heading", { name: "Terraform review", exact: true }),
    ).toBeVisible();
    const inventory = await command(["workspace", "list", fixture]);
    const child = inventory.tasks.find(
      (task: { label: string }) => task.label === "Terraform review",
    );
    expect(child.parent_task_id).toBe(parent.task.id);
    const childOperation = await command([
      "scope",
      "begin",
      brain,
      child.id,
      "context",
    ]);
    await command([
      "scope",
      "change",
      brain,
      child.id,
      child.scope.id,
      "--repository",
      app,
      "--area",
      area,
      "--environment",
      environment,
    ]);
    await expect(
      dialog.getByText("Repositories: example.test/Team/app", { exact: true }),
    ).toBeVisible();
    const unchangedParent = await command([
      "scope",
      "inspect",
      brain,
      parent.task.id,
    ]);
    expect(unchangedParent.task.scope.id).toBe(current.task.scope.id);
    const childHistory = await command(["scope", "inspect", brain, child.id]);
    expect(childHistory.operations[0].scope.id).toBe(childOperation.scope.id);
    expect(childHistory.task.scope.id).not.toBe(childOperation.scope.id);
    await dialog
      .getByRole("button", { name: "Bind operation", exact: true })
      .click();
    await expect(dialog.getByTestId("operation-binding")).toHaveCount(2);
    await dialog
      .getByTestId("operation-binding")
      .filter({ hasText: "Earlier scope" })
      .getByRole("button", { name: "Inspect scope" })
      .click();
    await expect(
      dialog.getByText("Recorded scope", { exact: true }),
    ).toBeVisible();
    await page.screenshot({
      path: "../.cache/ui/task-context.png",
      animations: "disabled",
    });
    await dialog
      .getByRole("button", { name: "Close task", exact: true })
      .click();
    await dialog
      .getByRole("button", { name: "Confirm close", exact: true })
      .click();
    await expect(dialog.getByText("Closed", { exact: true })).toBeVisible();
    const closed = await run(["scope", "begin", brain, child.id, "context"]);
    expect(closed.ok).toBe(false);
    expect(closed.stderr).toContain("This task is closed");
    const sibling = await command([
      "scope",
      "fork",
      brain,
      parent.task.id,
      "Command child",
    ]);
    expect(sibling.task.scope.selection.repository_ids).toHaveLength(2);
    await command(["scope", "close", brain, sibling.task.id]);
    await page.keyboard.press("Escape");
    await page.getByRole("tab", { name: "Repositories", exact: true }).click();
    const card = page
      .locator(".workspace-repository")
      .filter({ hasText: "example.test/Team/infra" })
      .locator("..");
    await card.getByRole("button", { name: "Add origin", exact: true }).click();
    await dialog
      .getByLabel(/^Repository origin/)
      .fill("https://example.test/Moved/infra.git");
    await dialog
      .getByRole("button", { name: "Attach origin", exact: true })
      .click();
    await expect(
      page.getByText("Also known as example.test/Moved/infra", { exact: true }),
    ).toBeVisible();
    await page
      .getByRole("tab", { name: "Your checkouts", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Registered workspace", exact: true })
      .click();
    await page
      .getByRole("option", {
        name: `${refreshed.refresh.workspace.root} · ${refreshed.refresh.workspace.device_id.slice(0, 8)}`,
        exact: true,
      })
      .click();
    await expect(page.getByText("Full refresh", { exact: true })).toBeVisible();
    await page.locator(".workspace-panel").scrollIntoViewIfNeeded();
    await page.screenshot({
      path: "../.cache/ui/workspace-checkouts.png",
      animations: "disabled",
    });
    await page.getByRole("tab", { name: "Your tasks", exact: true }).click();
    await page.screenshot({
      path: "../.cache/ui/workspace-tasks.png",
      fullPage: true,
      animations: "disabled",
    });
    expect(errors).toEqual([]);
  } finally {
    if (companion && companion.exitCode === null) companion.kill("SIGTERM");
    await run(["unpair"]);
    const cleanup = await run(["forget"]);
    expect(cleanup.ok, cleanup.stderr).toBe(true);
    // The unique directory and OS-store profile were created by this test only.
    await rm(fixture, { recursive: true, force: true });
  }
});
