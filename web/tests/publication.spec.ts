import { test, expect } from "@playwright/test";
import { execFile, spawn, type ChildProcess } from "node:child_process";
import { createServer, request } from "node:http";
import { randomUUID } from "node:crypto";
import { mkdir, writeFile, readFile, readdir, rm } from "node:fs/promises";
import { resolve } from "node:path";
import { promisify } from "node:util";

test("native committed publication resumes and browser preserves manifest history", async ({
  page,
}) => {
  test.setTimeout(150_000);
  page.setDefaultTimeout(12_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const fixture = resolve(`../.cache/publication-ui-${randomUUID()}`);
  const checkout = resolve(fixture, "repo");
  const bundles = resolve(fixture, "bundles");
  await mkdir(resolve(checkout, "src"), { recursive: true });
  await mkdir(resolve(fixture, ".recollect"), { recursive: true });
  const binary = resolve("../target/debug/recollect-agent");
  let failUpload = true;
  const origin = process.env.RECOLLECT_UI_TEST_ORIGIN!;
  const proxy = createServer((incoming, outgoing) => {
    if (
      failUpload &&
      incoming.method === "POST" &&
      incoming.url?.endsWith("/snapshots")
    ) {
      incoming.resume();
      outgoing.writeHead(503, { "content-type": "application/json" });
      outgoing.end(
        JSON.stringify({
          code: "fixture_interruption",
          message: "Synthetic upload interruption",
        }),
      );
      return;
    }
    const upstream = request(
      new URL(incoming.url!, origin),
      {
        method: incoming.method,
        headers: { ...incoming.headers, host: new URL(origin).host },
      },
      (response) => {
        outgoing.writeHead(response.statusCode!, response.headers);
        response.pipe(outgoing);
      },
    );
    upstream.on("error", () => {
      outgoing.writeHead(503);
      outgoing.end();
    });
    incoming.pipe(upstream);
  });
  await new Promise<void>((done) => proxy.listen(0, "127.0.0.1", done));
  const address = proxy.address();
  if (!address || typeof address === "string")
    throw Error("Fixture proxy unavailable");
  const env = {
    PATH: process.env.PATH,
    HOME: process.env.HOME,
    TMPDIR: process.env.TMPDIR,
    RECOLLECT_URL: `http://127.0.0.1:${address.port}`,
    RECOLLECT_DEVICE_PROFILE: `publication-ui-${randomUUID()}`,
    RECOLLECT_PUBLICATION_DIR: bundles,
  };
  const run = (args: string[]) =>
    new Promise<{ ok: boolean; stdout: string; stderr: string }>((done) =>
      execFile(
        binary,
        args,
        { env, timeout: 35_000, maxBuffer: 2 * 1024 * 1024 },
        (error, stdout, stderr) => done({ ok: !error, stdout, stderr }),
      ),
    );
  const command = async (args: string[]) => {
    const result = await run(args);
    expect(result.ok, result.stderr).toBe(true);
    return JSON.parse(result.stdout);
  };
  const git = promisify(execFile);
  const gitEnv = {
    PATH: process.env.PATH,
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_AUTHOR_NAME: "Recollect fixture",
    GIT_AUTHOR_EMAIL: "fixture@example.test",
    GIT_COMMITTER_NAME: "Recollect fixture",
    GIT_COMMITTER_EMAIL: "fixture@example.test",
  };
  let pairing: ChildProcess | undefined;
  let worker: ChildProcess | undefined;
  try {
    await git("git", ["-C", checkout, "init", "-q"], { env: gitEnv });
    await git(
      "git",
      [
        "-C",
        checkout,
        "remote",
        "add",
        "origin",
        "https://example.test/team/ui-fixture.git",
      ],
      { env: gitEnv },
    );
    await writeFile(
      resolve(checkout, "Cargo.toml"),
      "[package]\nname = 'fixture'\nversion = '0.1.0'\n",
    );
    const committed =
      "pub fn committed_welcome() { greet(); }\npub fn greet() {}\n";
    await writeFile(resolve(checkout, "src/lib.rs"), committed);
    await writeFile(
      resolve(checkout, "deployment.yaml"),
      "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: fixture\n",
    );
    await git("git", ["-C", checkout, "add", "."], { env: gitEnv });
    await git(
      "git",
      ["-C", checkout, "commit", "-qm", "First committed fixture"],
      { env: gitEnv },
    );
    const revision = (
      await git("git", ["-C", checkout, "rev-parse", "HEAD"], { env: gitEnv })
    ).stdout.trim();
    await writeFile(
      resolve(checkout, "src/lib.rs"),
      "dirty_only_browser_marker\n",
    );
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
    let dialog = page.getByRole("dialog");
    await dialog.getByLabel(/^Name/).fill("Committed publication proof");
    await dialog
      .getByRole("button", { name: "Create Brain", exact: true })
      .click();
    await expect(
      page.getByRole("heading", {
        name: "Committed publication proof",
        exact: true,
      }),
    ).toBeVisible();
    const brainURL = page.url();
    const brain = new URL(brainURL).pathname.split("/").at(-1)!;
    await writeFile(
      resolve(fixture, ".recollect/workspace.toml"),
      `brain = "${brain}"\n`,
    );
    await page
      .getByRole("button", { name: "Manage views", exact: true })
      .click();
    for (const name of ["Production", "Development"]) {
      await dialog
        .getByRole("textbox", { name: "View kind", exact: true })
        .click();
      await page
        .getByRole("option", { name: "Environment", exact: true })
        .click();
      await dialog.getByLabel(/^View name/).fill(name);
      await dialog
        .getByRole("button", { name: "Create view", exact: true })
        .click();
      await expect(dialog.getByText(name, { exact: true })).toBeVisible();
    }
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await page
      .getByRole("switch", {
        name: "Allow explicitly selected repository file text",
      })
      .click();
    await expect(
      page.getByRole("switch", {
        name: "Allow explicitly selected repository file text",
      }),
    ).toBeChecked();
    let output = "";
    let diagnostic = "";
    pairing = spawn(binary, ["pair", "Publication proof companion"], {
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    pairing.stdout!.on("data", (v) => (output += v.toString()));
    pairing.stderr!.on("data", (v) => (diagnostic += v.toString()));
    const paired = new Promise<number | null>((done) => {
      pairing!.once("exit", done);
      pairing!.once("error", () => done(-1));
    });
    await expect
      .poll(() => output.includes("Open ") || !!diagnostic)
      .toBe(true);
    expect(diagnostic).toBe("");
    await page.goto(output.match(/Open (\S+)/)![1]);
    await page
      .getByRole("button", { name: "Approve device", exact: true })
      .click();
    expect(await paired, diagnostic).toBe(0);
    pairing = undefined;
    const refreshed = await command(["workspace", "refresh", fixture]);
    const repo = refreshed.catalogue.repositories[0].id;
    const task = await command([
      "scope",
      "start",
      brain,
      "Committed publication task",
      "--repository",
      repo,
    ]);
    const failed = await run([
      "repository",
      "publish",
      brain,
      repo,
      task.task.id,
      checkout,
      "--retain-file",
      "src/lib.rs",
    ]);
    expect(failed.ok).toBe(false);
    expect(failed.stderr).toContain("Prepared bundle retained");
    const names = (await readdir(bundles)).filter((n) => n.endsWith(".json"));
    expect(names).toHaveLength(1);
    const bundle = JSON.parse(
      await readFile(resolve(bundles, names[0]), "utf8"),
    );
    expect(bundle.input.revision).toBe(revision);
    expect(bundle.input.dirty).toBe(true);
    // A changed worktree during outage cannot relabel the prepared upload.
    await writeFile(
      resolve(checkout, "src/lib.rs"),
      "changed_again_during_outage\n",
    );
    failUpload = false;
    const publication = await command([
      "repository",
      "resume",
      brain,
      bundle.input.publication_id,
    ]);
    expect(publication.snapshot.revision).toBe(revision);
    expect(
      (await readdir(bundles)).filter((n) => n.endsWith(".json")),
    ).toHaveLength(0);
    worker = spawn(resolve("../target/debug/recollect-server"), ["worker"], {
      env: process.env,
      stdio: "ignore",
    });
    await page.goto(brainURL);
    await page.getByRole("tab", { name: "Repositories", exact: true }).click();
    await page.getByRole("button", { name: "Snapshots", exact: true }).click();
    await page
      .getByRole("button", { name: "Inspect snapshot", exact: true })
      .click();
    dialog = page.getByRole("dialog");
    await expect(dialog.getByText("ready", { exact: true })).toBeVisible({
      timeout: 20_000,
    });
    await dialog.getByRole("button", { name: /src\/lib.rs/ }).click();
    await expect(dialog.locator("pre:visible")).toContainText(
      "committed_welcome",
    );
    await expect(dialog.locator("pre:visible")).not.toContainText(
      "dirty_only_browser_marker",
    );
    await page.screenshot({
      path: "../.cache/ui/publication-retained.png",
      fullPage: true,
    });
    await dialog.getByRole("button", { name: "Back to files" }).click();
    await dialog.getByRole("button", { name: /deployment.yaml/ }).click();
    await expect(
      dialog.getByText("File content unavailable", { exact: true }),
    ).toBeVisible();
    await dialog.getByRole("tab", { name: "Facts", exact: true }).click();
    await expect(
      dialog
        .locator("pre:visible")
        .filter({ hasText: "committed_welcome" })
        .first(),
    ).toBeVisible();
    await dialog.getByRole("tab", { name: "Coverage", exact: true }).click();
    await expect(
      dialog.getByText(/Kubernetes YAML has file inventory/),
    ).toBeVisible();
    await dialog
      .getByRole("tab", { name: "Contributors", exact: true })
      .click();
    await expect(
      dialog.getByText(
        "Working copy was dirty; committed bytes were published.",
      ),
    ).toBeVisible();
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");
    await page
      .getByRole("button", { name: "New manifest", exact: true })
      .click();
    dialog = page.getByRole("dialog");
    await dialog.getByLabel(/^Manifest name/).fill("Production selection");
    await dialog
      .getByRole("textbox", { name: "Environment", exact: true })
      .click();
    await page.getByRole("option", { name: "Production", exact: true }).click();
    await dialog
      .getByRole("textbox", { name: "Repository 1", exact: true })
      .click();
    await page
      .getByRole("option", {
        name: "example.test/team/ui-fixture",
        exact: true,
      })
      .click();
    await dialog
      .getByRole("textbox", { name: "Published snapshot 1", exact: true })
      .click();
    await page.getByRole("option", { name: new RegExp(revision) }).click();
    await dialog
      .getByLabel("Configuration paths 1", { exact: true })
      .fill("deployment.yaml\n");
    await dialog
      .getByRole("button", { name: "Save manifest revision", exact: true })
      .click();
    await expect(
      dialog.getByRole("heading", {
        name: "Production selection",
        exact: true,
      }),
    ).toBeVisible();
    await dialog
      .getByRole("button", { name: "Edit current selection", exact: true })
      .click();
    await dialog
      .getByLabel("Notes", { exact: true })
      .fill("A stale browser draft");
    const externallyUpdated = await page.evaluate(async (brain) => {
      const me = await (await fetch("/api/auth/me")).json();
      const manifests = await (
        await fetch(`/api/brains/${brain}/revision-manifests`)
      ).json();
      const m = manifests.items[0];
      const response = await fetch(
        `/api/brains/${brain}/revision-manifests/${m.manifest_id}`,
        {
          method: "PUT",
          headers: {
            "content-type": "application/json",
            "X-CSRF-Token": me.csrf_token,
          },
          body: JSON.stringify({
            ...m,
            base_revision: m.id,
            operation_id: null,
            notes: "Concurrent saved selection",
          }),
        },
      );
      return response.status;
    }, brain);
    expect(externallyUpdated).toBe(200);
    await dialog
      .getByRole("button", { name: "Save manifest revision", exact: true })
      .click();
    await expect(
      dialog.getByText(
        "This manifest changed. Reload the current revision before saving.",
      ),
    ).toBeVisible();
    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(
      dialog.getByText("Concurrent saved selection", { exact: true }),
    ).toBeVisible();
    await dialog
      .getByRole("button", { name: "Edit current selection", exact: true })
      .click();
    await dialog
      .getByRole("textbox", { name: "Revision meaning", exact: true })
      .click();
    await page
      .getByRole("option", {
        name: "Recorded deployment observation",
        exact: true,
      })
      .click();
    await dialog.getByLabel(/^Observation time/).fill("2026-09-14T12:00:00Z");
    await dialog
      .getByLabel(/^Supporting observation reference/)
      .fill("https://example.test/deployment/ui-proof");
    await dialog
      .getByRole("button", { name: "Save manifest revision", exact: true })
      .click();
    await expect(
      dialog.getByText("Recorded observation", { exact: true }),
    ).toBeVisible();
    await expect(
      dialog.getByText("Revision history", { exact: true }),
    ).toBeVisible();
    await page.screenshot({
      path: "../.cache/ui/publication-manifest.png",
      fullPage: true,
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({
      path: "../.cache/ui/publication-mobile.png",
      fullPage: true,
    });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    expect(errors).toEqual([]);
    await page.keyboard.press("Escape");
  } finally {
    pairing?.kill("SIGINT");
    if (worker) {
      const closed = new Promise<void>((done) =>
        worker!.once("exit", () => done()),
      );
      worker.kill("SIGINT");
      await closed;
    }
    const unpaired = await run(["unpair"]);
    if (!unpaired.ok) await run(["forget"]);
    await new Promise<void>((done) => proxy.close(() => done()));
    await rm(fixture, { recursive: true, force: true });
  }
});
