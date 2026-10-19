import { test, expect, type Page } from "@playwright/test";
import { randomUUID } from "node:crypto";
import { spawn, type ChildProcess } from "node:child_process";
import { createServer } from "node:net";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import type { components } from "../src/api-schema";
import { openGraphDrawer } from "./desktop-helpers";
type Generation = components["schemas"]["GraphGeneration"];
let vite: ChildProcess, origin: string;
test.beforeAll(async () => {
  const server = createServer();
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string")
    throw Error("Missing fixture port");
  await new Promise<void>((resolve) => server.close(() => resolve()));
  origin = `http://127.0.0.1:${address.port}`;
  vite = spawn(
    process.execPath,
    [
      "node_modules/vite/bin/vite.js",
      "--host",
      "127.0.0.1",
      "--port",
      String(address.port),
      "--strictPort",
    ],
    { cwd: fileURLToPath(new URL("../", import.meta.url)), stdio: "ignore" },
  );
  vite.unref();
  await expect
    .poll(
      async () => {
        try {
          return (await fetch(origin)).ok;
        } catch {
          return false;
        }
      },
      { timeout: 20_000 },
    )
    .toBe(true);
});
test.afterAll(async () => {
  if (vite?.exitCode === null && vite.signalCode === null) {
    const exited = once(vite, "exit");
    vite.kill("SIGTERM");
    await Promise.race([
      exited,
      new Promise((resolve) => setTimeout(resolve, 2000)),
    ]);
  }
});
async function fixture(page: Page, combined = false) {
  const brain = randomUUID(),
    source = randomUUID(),
    version = randomUUID();
  const manifest = randomUUID(),
    environment = randomUUID(),
    snapshots = [randomUUID(), randomUUID()];
  const now = "2026-10-18T12:00:00Z";
  const brainData = {
    id: brain,
    name: "Generation proof",
    role: "admin",
    archived: false,
    owner_id: randomUUID(),
    created_at: now,
    updated_at: now,
  };
  const generation = (
    kind: string,
    seconds: number,
    snapshot: string | null = null,
  ): Generation => ({
    id: randomUUID(),
    brain_id: brain,
    actor_id: randomUUID(),
    job_id: randomUUID(),
    kind,
    snapshot_id: snapshot,
    input_snapshot_ids: kind === "combined" ? snapshots : [],
    adapter: kind === "combined" ? "repository-links-1" : "canonical-graph-1",
    input_epoch: 1,
    state: "ready",
    created_at: `2026-10-18T12:00:${String(seconds).padStart(2, "0")}Z`,
    published_at: now,
    node_count: 1,
    edge_count: 0,
    unresolved: 0,
    unsupported: 0,
    ambiguous: 0,
    error_code: null,
  });
  const original = generation(combined ? "combined" : "knowledge", 1);
  const replacement = {
    ...original,
    id: randomUUID(),
    created_at: "2026-10-18T12:00:02Z",
  };
  const inputs = combined
    ? snapshots.map((snapshot) => generation("repository", 0, snapshot))
    : [];
  const state = {
    current: original,
    inputs,
    generations: [original, ...inputs],
    total: 1 + inputs.length,
    polls: 0,
    explores: 0,
    posts: [] as string[],
    tracked: [] as string[][],
    trackedMissing: [] as string[],
    trackedRows: null as Generation[] | null,
    omitTracked: false,
    readWait: null as Promise<void> | null,
    readsEntered: 0,
    visibleCount: 1,
    hold: null as null | {
      entered: () => void;
      wait: Promise<void>;
      rows: Generation[];
      delivered?: () => void;
    },
  };
  const node = () => ({
    key: `source_version:${version}`,
    evidence: {
      id: source,
      revision_id: version,
      kind: "source_version",
      label: `Evidence from ${state.current.id === original.id ? "original" : "replacement"} generation`,
      text: "",
      recorded_at: now,
      score: 1,
      channels: ["graph"],
      selection: { repository_ids: [], area_ids: [], environment_id: null },
      qualifications: [],
      claim: null,
      provenance: [
        {
          kind: "source_version",
          id: version,
          source_id: source,
          availability: "retained",
          repository_id: null,
          snapshot_id: null,
          revision: null,
        },
      ],
    },
  });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (
      message.type() === "error" &&
      /Cannot update a component|Maximum update depth/.test(message.text())
    )
      errors.push(message.text());
  });
  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url()),
      path = url.pathname;
    const json = (body: unknown, status = 200) =>
      route.fulfill({
        status,
        contentType: "application/json",
        body: JSON.stringify(body),
      });
    if (route.request().method() === "POST") state.posts.push(path);
    if (path === "/api/auth/me")
      return json({
        user: {
          id: randomUUID(),
          username: "Graph owner",
          installation_owner: true,
        },
        csrf_token: "synthetic-csrf",
      });
    if (path === "/api/status") return json({ ready: true });
    if (path === "/api/brains") return json([brainData]);
    if (path === `/api/brains/${brain}`) return json(brainData);
    if (path.endsWith("/workspace")) return json({ repositories: [] });
    if (path.endsWith("/evidence")) return json({ groups: [] });
    if (path.endsWith("/graph")) {
      state.polls++;
      const ids =
        url.searchParams.get("tracked_generation_ids")?.split(",") ?? [];
      state.tracked.push(ids);
      let rows = structuredClone(state.generations);
      let trackedRows = structuredClone(state.trackedRows ?? rows);
      const missing = [...state.trackedMissing];
      let delivered: (() => void) | undefined;
      if (state.hold && ids.length) {
        const held = state.hold;
        state.hold = null;
        rows = held.rows;
        trackedRows = held.rows;
        delivered = held.delivered;
        held.entered();
        await held.wait;
      }
      await json({
        memory_epoch: 1,
        link_epoch: 1,
        organization_epoch: 1,
        generations: rows,
        jobs: [],
        total: state.total,
        offset: 0,
        tracked_generations: state.omitTracked
          ? undefined
          : trackedRows.filter((row) => ids.includes(row.id)),
        missing_tracked_generation_ids: state.omitTracked
          ? undefined
          : missing.filter((id) => ids.includes(id)),
      });
      delivered?.();
      return;
    }
    if (path.endsWith("/graph/explore")) {
      state.explores++;
      const request = route.request().postDataJSON();
      const nodes = [node()];
      if (state.visibleCount > 1) {
        const extra = structuredClone(node());
        extra.key = `source_version:${manifest}`;
        extra.evidence.id = manifest;
        extra.evidence.revision_id = manifest;
        extra.evidence.label = "Additional canonical evidence";
        extra.evidence.provenance[0].id = manifest;
        nodes.push(extra);
      }
      const view = {
        brain_id: brain,
        generation: state.current,
        inputs: state.inputs,
        memory_epoch: 1,
        state: "ready",
        scope: request.scope,
        nodes,
        total_nodes: nodes.length,
        total_edges: 0,
        relations: [],
        offset: 0,
        next_offset: null,
        link_issues: [],
        link_issues_total: 0,
        expires_at: null,
        coverage: { partial: false, reasons: [] },
      };
      state.readsEntered++;
      if (state.readWait) await state.readWait;
      return json({
        view,
        nodes: view.nodes,
        edges: [],
        reach: [],
        center: request.center ?? null,
        direction: request.direction,
        max_hops: request.max_hops,
        expires_at: null,
      });
    }
    if (path.includes("/claim-evidence/source_version/"))
      return json({
        evidence: {
          kind: "source_version",
          id: version,
          source_id: source,
          label: node().evidence.label,
          created_at: now,
          availability: "retained",
          revision: null,
          reference: null,
          observed_at: null,
        },
        text: "Exact generation source evidence.",
        data: { source_version_id: version },
        expires_at: null,
      });
    return json({ message: "Unused generation fixture endpoint" }, 503);
  });
  const query = combined
    ? `&kind=combined&manifest=${manifest}&environment=${environment}`
    : "";
  await page.goto(`${origin}/brains/${brain}/graph?tab=evidence${query}`);
  const canvas = page.getByTestId("graph-canvas");
  await expect(canvas).toHaveAttribute("data-dimension", "3d");
  await expect(canvas).toHaveAttribute("data-ready", "true");
  return {
    state,
    original,
    replacement,
    generation,
    canvas,
    brain,
    version,
    errors,
  };
}
async function inspect(page: Page) {
  const finder = page.getByRole("textbox", {
    name: "Inspect graph entity",
    exact: true,
  });
  await finder.fill("Evidence from");
  await finder.press("ArrowDown");
  await finder.press("Enter");
  await page
    .getByRole("button", { name: "Open evidence", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Supporting evidence", exact: true }),
  ).toContainText("Exact generation source evidence.");
}
test("same-epoch superseded generation clears scene and inspector until an explicit fresh read", async ({
  page,
}) => {
  const f = await fixture(page);
  await inspect(page);
  f.state.generations = [{ ...f.original, state: "superseded" }, f.replacement];
  await expect(f.canvas).toHaveCount(0, { timeout: 6500 });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Load graph view", exact: true }),
  ).toBeVisible();
  expect(f.state.explores).toBe(1);
  f.state.current = f.replacement;
  await page
    .getByRole("button", { name: "Load graph view", exact: true })
    .click();
  await expect(f.canvas).toHaveAttribute("data-ready", "true");
  expect(f.state.explores).toBe(2);
  expect(f.state.posts.every((path) => path.endsWith("/graph/explore"))).toBe(
    true,
  );
  expect(f.errors).toEqual([]);
});

// Reads, and optionally moves, the 3D camera through the canvas component's own
// force-graph ref, found on the React fiber as graph-3d.spec.ts does. The move
// uses the library's cameraPosition, the call a real orbit ends in.
async function camera(page: Page, to?: number[]) {
  return page
    .getByTestId("graph-canvas")
    .evaluate((element: any, move?: number[]) => {
      let fiber =
        element[
          Object.keys(element).find((key) => key.startsWith("__reactFiber"))!
        ];
      let api: any;
      while (fiber && !api) {
        let hook = fiber.memoizedState;
        while (hook) {
          const candidate = hook.memoizedState?.current;
          if (typeof candidate?.cameraPosition === "function") {
            api = candidate;
            break;
          }
          hook = hook.next;
        }
        fiber = fiber.return;
      }
      if (!api) throw Error("Graph canvas ref unavailable");
      if (move) {
        const target = api.controls().target;
        api.cameraPosition(
          { x: move[0], y: move[1], z: move[2] },
          { x: target.x, y: target.y, z: target.z },
          0,
        );
      }
      const position = api.camera().position;
      return [position.x, position.y, position.z];
    }, to);
}

test("an equal exploration request restores the camera, a different scene fits afresh", async ({
  page,
}) => {
  const f = await fixture(page);
  await expect(f.canvas).toHaveAttribute("data-layout-ready", "true");
  const fitted = await camera(page);
  const moved = await camera(page, [
    fitted[0] + 80,
    fitted[1] + 60,
    fitted[2] + 40,
  ]);
  expect(moved).not.toEqual(fitted);
  const reads = f.state.explores;
  // The same center, direction and hop bound is a new read but not a new
  // renderer key: the canvas unmounts around the read and the explorer-owned
  // session puts the camera back where the user left it.
  const gate = deferred();
  f.state.readWait = gate.promise;
  await openGraphDrawer(page, "Explore relationships", "Explore");
  await page
    .getByRole("button", { name: "Show eligible overview", exact: true })
    .click();
  await expect(f.canvas).toHaveCount(0);
  f.state.readWait = null;
  gate.resolve();
  await expect(f.canvas).toHaveAttribute("data-layout-ready", "true");
  expect(f.state.explores).toBe(reads + 1);
  const restored = await camera(page);
  restored.forEach((value, axis) => expect(value).toBeCloseTo(moved[axis], 1));
  // A different center reads a different scene (a new renderer key and a new
  // topology): the camera is fitted to it instead of inheriting the moved one.
  f.state.visibleCount = 2;
  const next = deferred();
  f.state.readWait = next.promise;
  await openGraphDrawer(page, "Explore relationships", "Explore");
  await page
    .getByLabel("Exploration center", { exact: true })
    .fill(`source_version:${f.version}`);
  await page
    .getByRole("button", { name: "Explore from center", exact: true })
    .click();
  await expect(f.canvas).toHaveCount(0);
  f.state.readWait = null;
  next.resolve();
  await expect(f.canvas).toHaveAttribute("data-layout-ready", "true");
  expect(f.state.explores).toBe(reads + 2);
  const refitted = await camera(page);
  expect(
    refitted.some((value, axis) => Math.abs(value - moved[axis]) > 1),
  ).toBe(true);
  expect(f.errors).toEqual([]);
});

test("a background generation poll keeps the renderer and its camera", async ({
  page,
}) => {
  const f = await fixture(page);
  await expect(f.canvas).toHaveAttribute("data-layout-ready", "true");
  const fitted = await camera(page);
  const moved = await camera(page, [
    fitted[0] + 80,
    fitted[1] + 60,
    fitted[2] + 40,
  ]);
  await f.canvas.evaluate((element) => {
    (element as HTMLElement & { cameraProbe?: string }).cameraProbe = "kept";
  });
  const reads = f.state.explores;
  // Two heartbeats verify the accepted scene against the server. Neither may
  // remount the canvas, refit the camera or issue another exploration read.
  await heartbeat(f);
  await heartbeat(f);
  expect(
    await f.canvas.evaluate(
      (element) =>
        (element as HTMLElement & { cameraProbe?: string }).cameraProbe,
    ),
  ).toBe("kept");
  (await camera(page)).forEach((value, axis) =>
    expect(value).toBeCloseTo(moved[axis], 1),
  );
  expect(f.state.explores).toBe(reads);
  expect(f.errors).toEqual([]);
});

test("a scene replaced after a superseded generation starts with a fresh camera", async ({
  page,
}) => {
  const f = await fixture(page);
  await expect(f.canvas).toHaveAttribute("data-layout-ready", "true");
  const fitted = await camera(page);
  const moved = await camera(page, [
    fitted[0] + 80,
    fitted[1] + 60,
    fitted[2] + 40,
  ]);
  // The superseded generation discards the scene with its explorer, and with it
  // the camera session; only an explicit read brings a new scene back.
  f.state.generations = [{ ...f.original, state: "superseded" }, f.replacement];
  await expect(f.canvas).toHaveCount(0, { timeout: 6500 });
  f.state.current = f.replacement;
  await page
    .getByRole("button", { name: "Load graph view", exact: true })
    .click();
  await expect(f.canvas).toHaveAttribute("data-layout-ready", "true");
  const fresh = await camera(page);
  expect(fresh.some((value, axis) => Math.abs(value - moved[axis]) > 1)).toBe(
    true,
  );
  fresh.forEach((value, axis) => expect(value).toBeCloseTo(fitted[axis], 0));
  expect(f.errors).toEqual([]);
});

async function heartbeat(f: Awaited<ReturnType<typeof fixture>>) {
  const before = f.state.tracked.filter((ids) => ids.length > 0).length;
  await expect
    .poll(() => f.state.tracked.filter((ids) => ids.length > 0).length, {
      timeout: 6500,
    })
    .toBeGreaterThan(before);
}
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

test("combined inputs stay visible through unrelated, pending and paginated rows, then exact tracked removal clears them", async ({
  page,
}) => {
  const f = await fixture(page, true);
  await inspect(page);
  f.state.total = 200;
  f.state.generations = Array.from({ length: 15 }, (_, i) =>
    f.generation("repository", i + 5, randomUUID()),
  );
  f.state.generations.push(
    {
      ...f.original,
      id: randomUUID(),
      state: "queued",
      created_at: "2026-10-18T12:00:30Z",
    },
    {
      ...f.original,
      id: randomUUID(),
      state: "running",
      created_at: "2026-10-18T12:00:31Z",
    },
    { ...f.replacement, adapter: "another-adapter" },
    { ...f.replacement, id: randomUUID(), input_snapshot_ids: [randomUUID()] },
    { ...f.replacement, id: randomUUID(), input_epoch: 2 },
  );
  f.state.trackedRows = [f.original, ...f.state.inputs];
  await heartbeat(f);
  await expect(f.canvas).toHaveAttribute("data-ready", "true");
  await expect(page.getByRole("dialog")).toBeVisible();
  expect(
    f.state.tracked.some(
      (ids) =>
        ids.length === 3 &&
        [f.original, ...f.state.inputs].every((g) => ids.includes(g.id)),
    ),
  ).toBe(true);
  expect(f.state.explores).toBe(1);
  f.state.trackedRows = [
    f.original,
    { ...f.state.inputs[0], state: "removed" },
    f.state.inputs[1],
  ];
  await expect(f.canvas).toHaveCount(0, { timeout: 6500 });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(f.state.explores).toBe(1);
  expect(f.errors).toEqual([]);
});

test("inventory absence and an older ready generation preserve the scene, but a newer exact binding clears it", async ({
  page,
}) => {
  const f = await fixture(page);
  await inspect(page);
  f.state.total = 100;
  f.state.generations = [
    { ...f.original, id: randomUUID(), created_at: "2026-10-18T12:00:00Z" },
  ];
  f.state.trackedRows = [f.original];
  await heartbeat(f);
  await expect(f.canvas).toHaveAttribute("data-ready", "true");
  await expect(page.getByRole("dialog")).toBeVisible();
  f.state.generations = [f.replacement];
  await expect(f.canvas).toHaveCount(0, { timeout: 6500 });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(f.state.explores).toBe(1);
  expect(f.errors).toEqual([]);
});

test("an explicit tracked missing generation clears evidence even when its inventory page omits it", async ({
  page,
}) => {
  const f = await fixture(page);
  await inspect(page);
  f.state.generations = Array.from({ length: 20 }, (_, i) =>
    f.generation("repository", i + 5, randomUUID()),
  );
  f.state.total = 500;
  f.state.trackedRows = [];
  f.state.trackedMissing = [f.original.id];
  await expect(f.canvas).toHaveCount(0, { timeout: 6500 });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(f.state.explores).toBe(1);
  expect(f.errors).toEqual([]);
});

test("a held old heartbeat cannot cancel an explicit child read, and later polls track the replacement scene", async ({
  page,
}) => {
  const f = await fixture(page, true);
  const entered = deferred(),
    release = deferred(),
    delivered = deferred(),
    read = deferred();
  f.state.hold = {
    entered: entered.resolve,
    wait: release.promise,
    rows: [
      { ...f.original, state: "superseded" },
      f.replacement,
      ...f.state.inputs,
    ],
    delivered: delivered.resolve,
  };
  await entered.promise;
  const replacementInput = {
    ...f.state.inputs[0],
    id: randomUUID(),
    created_at: "2026-10-18T12:00:02Z",
  };
  f.state.current = f.replacement;
  f.state.visibleCount = 2;
  f.state.inputs = [replacementInput, f.state.inputs[1]];
  f.state.generations = [f.replacement, ...f.state.inputs];
  f.state.readWait = read.promise;
  await page.getByRole("button", { name: "Explore", exact: true }).click();
  await page
    .getByRole("button", { name: "Show eligible overview", exact: true })
    .click();
  release.resolve();
  await delivered.promise;
  await expect(
    page.getByRole("button", { name: "Load graph view", exact: true }),
  ).toHaveCount(0);
  read.resolve();
  f.state.readWait = null;
  await expect(f.canvas).toHaveAttribute("data-ready", "true");
  await page.getByRole("button", { name: "Graph tools", exact: true }).click();
  await page
    .getByRole("menuitem", { name: "Graph status", exact: true })
    .click();
  const statusDrawer = page.getByRole("dialog", {
    name: "Graph status and maintenance",
  });
  await expect(f.canvas).toHaveAttribute("data-nodes", "2");
  await expect(statusDrawer).toContainText(`Generation ${f.replacement.id}`);
  await expect(statusDrawer).toContainText("2 eligible entities");
  await expect(statusDrawer).toContainText(`generation ${replacementInput.id}`);
  await expect(statusDrawer).not.toContainText(`Generation ${f.original.id}`);
  await page.keyboard.press("Escape");
  await expect(statusDrawer).not.toBeVisible();
  await inspect(page);
  await expect(page.getByRole("dialog")).toContainText(
    "replacement generation",
  );
  expect(f.state.explores).toBe(2);
  await expect
    .poll(() => f.state.tracked.some((ids) => ids.includes(f.replacement.id)), {
      timeout: 6500,
    })
    .toBe(true);
  f.state.generations = [
    { ...f.replacement, state: "failed" },
    ...f.state.inputs,
  ];
  await expect(f.canvas).toHaveCount(0, { timeout: 6500 });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(f.state.explores).toBe(2);
  expect(f.errors).toEqual([]);
});

test("a same-ID newer explicit read ignores an older held tracked response", async ({
  page,
}) => {
  const f = await fixture(page);
  const entered = deferred(),
    release = deferred(),
    delivered = deferred();
  // Exact status can synthesize removed while an input is unavailable. A later
  // authorized read is authoritative even if it reuses that immutable graph.
  f.state.hold = {
    entered: entered.resolve,
    wait: release.promise,
    rows: [{ ...f.original, state: "removed" }],
    delivered: delivered.resolve,
  };
  await entered.promise;
  await page.getByRole("button", { name: "Explore", exact: true }).click();
  await page
    .getByRole("button", { name: "Show eligible overview", exact: true })
    .click();
  await expect.poll(() => f.state.explores).toBe(2);
  await expect(f.canvas).toHaveAttribute("data-ready", "true");
  release.resolve();
  await delivered.promise;
  await heartbeat(f);
  await expect(f.canvas).toHaveAttribute("data-ready", "true");
  expect(f.state.explores).toBe(2);
  expect(f.errors).toEqual([]);
});

for (const mode of [
  "missing fields",
  "partial rows",
  "foreign binding",
  "unknown state",
] as const) {
  test(`an incomplete exact status response fails closed (${mode})`, async ({
    page,
  }) => {
    const f = await fixture(page);
    await inspect(page);
    if (mode === "missing fields") f.state.omitTracked = true;
    if (mode === "partial rows") f.state.trackedRows = [];
    if (mode === "foreign binding")
      f.state.trackedRows = [{ ...f.original, brain_id: randomUUID() }];
    if (mode === "unknown state")
      f.state.trackedRows = [{ ...f.original, state: "unknown" }];
    await expect(f.canvas).toHaveCount(0, { timeout: 6500 });
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.getByRole("alert")).toContainText(
      "graph inputs could not be verified",
    );
    expect(f.state.explores).toBe(1);
    expect(f.errors).toEqual([]);
  });
}

test("a newer explicit parent read also survives an older held heartbeat", async ({
  page,
}) => {
  const f = await fixture(page);
  const entered = deferred(),
    release = deferred(),
    delivered = deferred();
  f.state.hold = {
    entered: entered.resolve,
    wait: release.promise,
    rows: [{ ...f.original, state: "superseded" }, f.replacement],
    delivered: delivered.resolve,
  };
  await entered.promise;
  f.state.current = f.replacement;
  f.state.generations = [f.replacement];
  await page.getByRole("button", { name: "Filters", exact: true }).click();
  await page
    .getByRole("button", { name: "Apply graph filters", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Graph filters", exact: true }),
  ).not.toBeVisible();
  await expect.poll(() => f.state.explores).toBe(2);
  await expect(f.canvas).toHaveAttribute("data-ready", "true");
  release.resolve();
  await delivered.promise;
  await inspect(page);
  await expect(
    page.getByRole("dialog", { name: "Supporting evidence", exact: true }),
  ).toContainText("replacement generation");
  await heartbeat(f);
  await expect(f.canvas).toHaveAttribute("data-ready", "true");
  expect(f.state.explores).toBe(2);
  expect(f.errors).toEqual([]);
  expect(f.state.posts.every((path) => path.endsWith("/graph/explore"))).toBe(
    true,
  );
});
