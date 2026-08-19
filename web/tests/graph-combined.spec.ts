import { test, expect } from "@playwright/test";
import { execFile, spawn, type ChildProcess } from "node:child_process";
import { mkdir, writeFile, readFile, rm } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { resolve } from "node:path";
import { promisify } from "node:util";

test("committed HCL publications supply combined graph paths and browser evidence", async ({
  page,
}) => {
  test.setTimeout(150_000);
  page.setDefaultTimeout(12_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const root = resolve(`../.cache/cross-graph-proof/browser-${randomUUID()}`);
  const binary = resolve("../target/debug/recollect-agent");
  const env = {
    PATH: process.env.PATH,
    HOME: process.env.HOME,
    TMPDIR: process.env.TMPDIR,
    RECOLLECT_URL: process.env.RECOLLECT_UI_TEST_ORIGIN!,
    RECOLLECT_DEVICE_PROFILE: `combined-${randomUUID()}`,
    RECOLLECT_PUBLICATION_DIR: resolve(root, "bundles"),
  };
  const execute = promisify(execFile);
  const run = async (args: string[]) => {
    const output = await execute(binary, args, {
      env,
      timeout: 40_000,
      maxBuffer: 3 * 1024 * 1024,
    });
    return JSON.parse(output.stdout);
  };
  const gitEnv = {
    PATH: process.env.PATH,
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_AUTHOR_NAME: "Recollect fixture",
    GIT_AUTHOR_EMAIL: "fixture@example.test",
    GIT_COMMITTER_NAME: "Recollect fixture",
    GIT_COMMITTER_EMAIL: "fixture@example.test",
  };
  const git = async (path: string, ...args: string[]) =>
    (
      await execute("git", ["-C", path, ...args], {
        env: gitEnv,
        timeout: 10_000,
      })
    ).stdout.trim();
  const post = async (path: string, body: unknown) =>
    page.evaluate(
      async ({ path, body }) => {
        const me = await (await fetch("/api/auth/me")).json();
        const response = await fetch(path, {
          method: "POST",
          headers: {
            "content-type": "application/json",
            "x-csrf-token": me.csrf_token,
          },
          body: JSON.stringify(body),
        });
        if (!response.ok)
          throw new Error(
            `Combined fixture request failed ${response.status}: ${await response.text()}`,
          );
        return response.json();
      },
      { path, body },
    );
  let pairing: ChildProcess | undefined;
  let paired = false;
  let passed = false;
  try {
    await page.goto("/");
    await page
      .getByLabel(/^Username/)
      .fill(process.env.RECOLLECT_OWNER_USERNAME!);
    await page
      .getByLabel(/^Password/)
      .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
    await page.getByRole("button", { name: "Sign in", exact: true }).click();
    await expect(
      page.getByRole("button", { name: "Create Brain", exact: true }),
    ).toBeVisible();
    const brain = await post("/api/brains", {
      name: "Native combined graph proof",
    });
    const base = `/api/brains/${brain.id}`;
    await mkdir(resolve(root, ".recollect"), { recursive: true });
    await writeFile(
      resolve(root, ".recollect/workspace.toml"),
      `brain = "${brain.id}"\n`,
    );
    const a = resolve(root, "caller");
    const b = resolve(root, "callee");
    for (const [path, origin] of [
      [a, "example.test/caller/core"],
      [b, "example.test/target/core"],
    ]) {
      await mkdir(path, { recursive: true });
      await git(path, "init", "-q");
      await git(path, "remote", "add", "origin", `https://${origin}.git`);
    }
    await writeFile(
      resolve(b, "main.tf"),
      'variable "target" { type = string }\n',
    );
    await git(b, "add", "main.tf");
    await git(b, "commit", "-qm", "Exact target fixture");
    const targetCommit = await git(b, "rev-parse", "HEAD");
    const callerText = `module "bridge" {\n  # source = "git::https://example.test/wrong/core.git?ref=${"0".repeat(40)}"\n  source = "git::https://example.test/target/core.git?ref=${targetCommit}"\n}\nmodule "unresolved" {\n  source = "git::https://example.test/target/core.git?ref=main"\n}\n`;
    await writeFile(resolve(a, "main.tf"), callerText);
    await git(a, "add", "main.tf");
    await git(a, "commit", "-qm", "Exact caller fixture");
    const callerCommit = await git(a, "rev-parse", "HEAD");
    const dirty = callerText + "# DIRTY_COMBINED_MARKER\n";
    await writeFile(resolve(a, "main.tf"), dirty);
    const beforeStatus = await git(a, "status", "--porcelain");
    let output = "";
    let diagnostic = "";
    pairing = spawn(binary, ["pair", "Combined graph companion"], {
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    pairing.stdout!.on("data", (data) => (output += data.toString()));
    pairing.stderr!.on("data", (data) => (diagnostic += data.toString()));
    const pairDone = new Promise<number | null>((done) => {
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
    expect(await pairDone, diagnostic).toBe(0);
    paired = true;
    pairing = undefined;
    const refreshed = await run(["workspace", "refresh", root]);
    const repositories = refreshed.catalogue.repositories as {
      id: string;
      canonical_origin: string;
    }[];
    const callerRepo = repositories.find(
      (r) => r.canonical_origin === "example.test/caller/core",
    )!.id;
    const targetRepo = repositories.find(
      (r) => r.canonical_origin === "example.test/target/core",
    )!.id;
    const entries = [];
    const snapshots: string[] = [];
    for (const [path, repo, revision] of [
      [a, callerRepo, callerCommit],
      [b, targetRepo, targetCommit],
    ]) {
      const task = await run([
        "scope",
        "start",
        brain.id,
        "Combined publication",
        "--repository",
        repo,
      ]);
      const publication = await run([
        "repository",
        "publish",
        brain.id,
        repo,
        task.task.id,
        path,
      ]);
      expect(publication.snapshot.revision).toBe(revision);
      snapshots.push(publication.snapshot.id);
      entries.push({
        repository_id: repo,
        snapshot_id: publication.snapshot.id,
        revision,
        config_paths: [],
      });
    }
    expect(await readFile(resolve(a, "main.tf"), "utf8")).toBe(dirty);
    expect(await git(a, "status", "--porcelain")).toBe(beforeStatus);
    for (const snapshot of snapshots)
      await expect
        .poll(
          async () =>
            (
              await (
                await page.request.get(
                  `${base}/repository-snapshots/${snapshot}/facts`,
                )
              ).json()
            ).processing,
        )
        .toBe("ready");
    const environment = await post(`${base}/evidence/groups`, {
      kind: "environment",
      name: "Exact native inputs",
    });
    const manifest = await post(`${base}/revision-manifests`, {
      name: "Pinned native pair",
      environment_id: environment.id,
      kind: "committed",
      entries,
      notes: "Synthetic committed source; no deployed-state assertion.",
      base_revision: null,
    });
    await page.goto(`/brains/${brain.id}`);
    const panel = page.getByRole("region", {
      name: "Evidence graphs",
      exact: true,
    });
    await panel.getByRole("button", { name: /Evidence graphs/ }).click();
    await panel.getByLabel("Graph kind", { exact: true }).click();
    await page
      .getByRole("option", { name: "Combined repositories", exact: true })
      .click();
    const load = panel.getByRole("button", {
      name: "Load graph view",
      exact: true,
    });
    await expect(load).toBeDisabled();
    await panel.getByLabel("Graph environment", { exact: true }).click();
    await page
      .getByRole("option", { name: "Exact native inputs", exact: true })
      .click();
    await panel.getByLabel("Graph manifest", { exact: true }).click();
    await page.getByRole("option", { name: /Pinned native pair/ }).click();
    await panel
      .getByRole("button", { name: "Rebuild graph", exact: true })
      .click();
    await expect(
      panel.getByText("Graph rebuild queued. Progress appears below.", {
        exact: true,
      }),
    ).toBeVisible();
    await expect
      .poll(
        async () =>
          (
            await (await page.request.get(`${base}/graph`)).json()
          ).generations.filter(
            (g: { kind: string; state: string }) =>
              g.state === "ready" &&
              ["repository", "combined"].includes(g.kind),
          ).length,
      )
      .toBeGreaterThanOrEqual(3);
    await load.click();
    await expect(
      panel.getByText("Exact repository inputs", { exact: true }),
    ).toBeVisible();
    await expect(panel.getByText(/unresolved source ref/)).toBeVisible();
    const facts = await Promise.all(
      snapshots.map(
        async (snapshot) =>
          (
            await (
              await page.request.get(
                `${base}/repository-snapshots/${snapshot}/facts`,
              )
            ).json()
          ).items,
      ),
    );
    const callerRoot = facts[0].find(
      (f: { record: { name: string; kind: string } }) =>
        f.record.name === "." && f.record.kind === "module",
    );
    const targetVariable = facts[1].find(
      (f: { record: { name: string } }) => f.record.name === "var.target",
    );
    expect(callerRoot).toBeTruthy();
    expect(targetVariable).toBeTruthy();
    await panel
      .getByLabel("Path start", { exact: true })
      .fill(`repository_fact:${callerRoot.id}`);
    await panel
      .getByLabel("Path end", { exact: true })
      .fill(`repository_fact:${targetVariable.id}`);
    await panel.getByLabel("Path direction", { exact: true }).click();
    await page.getByRole("option", { name: "both", exact: true }).click();
    const pathResponse = page.waitForResponse(
      (r) => r.url().endsWith("/graph/path") && r.request().method() === "POST",
    );
    await panel
      .getByRole("button", { name: "Find shortest eligible path", exact: true })
      .click();
    const path = await (await pathResponse).json();
    expect(path.status).toBe("path");
    expect(path.edges.map((e: { relation: string }) => e.relation)).toEqual([
      "declares",
      "terraform_module",
      "declares",
    ]);
    expect(path.edges[0].to).toBe(path.nodes[0].key);
    expect(path.edges[2].from).toBe(path.nodes[3].key);
    expect(path.view.scope.manifest_revision_id).toBe(manifest.id);
    await expect(
      panel.getByRole("heading", {
        name: "3 hops in the eligible graph",
        exact: true,
      }),
    ).toBeVisible();
    await expect(panel.getByTestId("graph-path-entity")).toHaveCount(4);
    await expect(
      panel.getByText("Followed in reverse for this path.", { exact: true }),
    ).toHaveCount(2);
    await expect(
      panel.getByText(/This is a declared dependency/),
    ).toBeVisible();
    const bridge = panel
      .getByTestId("graph-entity")
      .filter({ has: page.getByText("module.bridge", { exact: true }) });
    await bridge
      .getByRole("button", { name: "Inspect evidence", exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.locator("pre")).toContainText("verified_literal");
    await expect(dialog.locator("pre")).toContainText(targetCommit);
    await expect(dialog.locator("pre")).not.toContainText(
      "DIRTY_COMBINED_MARKER",
    );
    await dialog
      .getByRole("button", { name: "Back to graph", exact: true })
      .click();
    for (const [width, height, name] of [
      [1440, 960, "desktop"],
      [390, 844, "mobile"],
    ] as const) {
      await page.setViewportSize({ width, height });
      await panel
        .getByRole("heading", {
          name: "3 hops in the eligible graph",
          exact: true,
        })
        .scrollIntoViewIfNeeded();
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      await page.screenshot({
        path: `../.cache/cross-graph-proof/native-${name}.png`,
      });
    }
    expect(
      (await (await page.request.get(`${base}/models/usage`)).json()).total,
    ).toBe(0);
    expect(errors).toEqual([]);
    await writeFile(
      resolve(root, "proof.json"),
      JSON.stringify(
        {
          brain: brain.id,
          snapshots,
          manifest: manifest.id,
          path,
          model_calls: 0,
        },
        null,
        2,
      ),
    );
    passed = true;
  } finally {
    pairing?.kill("SIGTERM");
    if (paired)
      await execute(binary, ["unpair"], { env, timeout: 10_000 }).catch(() =>
        execute(binary, ["forget"], { env, timeout: 10_000 }),
      );
    if (passed) {
      await rm(resolve(root, "caller"), { recursive: true });
      await rm(resolve(root, "callee"), { recursive: true });
    }
  }
});
