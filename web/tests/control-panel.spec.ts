import { test, expect, type Page } from "@playwright/test";
import { randomUUID } from "node:crypto";
import {
  itemState,
  inputStatus,
  observedStages,
  jobState,
  originKey,
  type PipelineFeed,
  type PipelineItem,
} from "../src/features/activity/pipelineModel";
import AxeBuilder from "@axe-core/playwright";

const iso = (offset = 0) => new Date(Date.now() + offset).toISOString();
const item = (id: string, title = id): PipelineItem => ({
  id,
  capture_id: id,
  binding_id: randomUUID(),
  source_id: randomUUID(),
  source_version_id: randomUUID(),
  actor_id: randomUUID(),
  contributor: "Fixture contributor",
  device_id: randomUUID(),
  agent_name: "Fixture agent",
  host: "codex",
  host_session_id: id,
  agent_id: id,
  coverage: [],
  kind: "prompt",
  tool_name: null,
  title,
  received_at: iso(-60000),
  activity_at: iso(-60000),
  expires_at: null,
  processing: "ready",
  processing_job: {
    id: randomUUID(),
    state: "succeeded",
    updated_at: iso(-60000),
    lease_until: null,
    error_code: null,
  },
  learning: null,
  active: false,
});
const feed = (brain: string, items: PipelineItem[]): PipelineFeed => ({
  brain_id: brain,
  items,
  observed_at: iso(),
  valid_until: iso(6000),
  has_more: false,
  graph: null,
});

test("observed pipeline changes do not replay history or unrelated edges", () => {
  const a = item(randomUUID()),
    first = feed(randomUUID(), [a]);
  expect(itemState(a, first.observed_at)).toBe("ready");
  const retry = structuredClone(a);
  retry.processing_job!.state = "running";
  retry.processing_job!.lease_until = iso(30000);
  retry.learning = {
    id: randomUUID(),
    state: "failed",
    job: { ...a.processing_job!, state: "failed" },
    created_at: iso(-60000),
    finished_at: iso(-30000),
    accepted: 0,
    proposed: 0,
    reused: 0,
    revised: 0,
    retired: 0,
    blocked: 0,
    conflicting: 0,
    claim_ids: [],
  };
  expect(itemState(retry, first.observed_at)).toBe("running");
  expect(observedStages(null, first).size).toBe(0);
  expect(observedStages(first, { ...first, observed_at: iso(2000) }).size).toBe(
    0,
  );
  const next = structuredClone(first);
  next.items[0].processing_job!.state = "running";
  // A commit can become visible after an observation with an earlier timestamp.
  next.items[0].processing_job!.updated_at = iso(-2000);
  expect(observedStages(first, next).get(a.id)).toEqual(["process"]);
  const old = item(randomUUID());
  expect(observedStages(first, { ...first, items: [a, old] }).size).toBe(0);
  const arrival = item(randomUUID());
  arrival.received_at = iso(1000);
  expect(
    observedStages(first, { ...first, items: [a, arrival] }).get(arrival.id),
  ).toEqual(["contribution"]);
  expect(originKey(a)).not.toBe(
    originKey({ ...a, host_session_id: "another session" }),
  );
  expect(
    jobState(
      { ...a.processing_job!, state: "running", lease_until: iso(-1) },
      iso(),
    ),
  ).toBe("waiting");
  expect(observedStages(first, { ...next, brain_id: randomUUID() }).size).toBe(
    0,
  );
});
async function signIn(page: Page) {
  await page.goto("/");
  await page
    .getByLabel(/^Username/)
    .fill(process.env.RECOLLECT_OWNER_USERNAME!);
  await page
    .getByLabel(/^Password/)
    .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Your Brains", exact: true }),
  ).toBeVisible();
}
async function api(page: Page, path: string, data?: unknown) {
  const session = await (await page.request.get("/api/auth/me")).json();
  const response = await page.request.fetch(path, {
    method: data ? "POST" : "GET",
    data,
    headers: { "x-csrf-token": session.csrf_token },
  });
  expect(response.ok(), `${path} ${response.status()}`).toBe(true);
  return response.json();
}

test("real source projection and global control-panel actions", async ({
  page,
}) => {
  test.setTimeout(90000);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await signIn(page);
  const brain = await api(page, "/api/brains", {
    name: "Control panel proof",
    description: "Disposable browser evidence",
  });
  const source = await api(page, `/api/brains/${brain.id}/sources`, {
    title: "Actual imported architecture note",
    media_type: "text/plain",
    content: "The disposable service uses a durable processing queue.",
    retain_content: true,
  });
  await page.goto("/");
  await expect(page.getByTestId("live-pipeline")).toHaveCount(0);
  await page
    .getByRole("link", { name: `Open ${brain.name}`, exact: true })
    .click();
  await page.goto(`/brains/${brain.id}/activity?tab=pipeline`);
  const pipeline = page.getByTestId("live-pipeline");
  await expect(
    pipeline
      .getByText("Actual imported architecture note", { exact: true })
      .first(),
  ).toBeVisible();
  const record = await api(page, `/api/brains/${brain.id}/pipeline`);
  expect(record.items[0].source_version_id).toBe(source.version.id);
  expect(record.items[0].learning).toBeNull();
  await pipeline.getByRole("button", { name: "Inspect input" }).click();
  const drawer = page.getByRole("dialog", { name: "Processing record" });
  await expect(
    drawer.getByRole("link", { name: "Read exact source version" }),
  ).toHaveAttribute("href", new RegExp(source.version.id));
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/control-panel-input.png",
  });
  await page.keyboard.press("Escape");
  for (const width of [1440, 1920, 1024]) {
    await page.setViewportSize({ width, height: 1000 });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
    ).toBe(false);
    await expect
      .poll(() =>
        pipeline.locator(".flow-canvas").evaluate((canvas) => {
          const box = canvas.getBoundingClientRect();
          return [...canvas.querySelectorAll(".react-flow__node")].every(
            (node) => {
              const b = node.getBoundingClientRect();
              return (
                b.left >= box.left - 1 &&
                b.right <= box.right + 1 &&
                b.top >= box.top - 1 &&
                b.bottom <= box.bottom + 1
              );
            },
          );
        }),
      )
      .toBe(true);
    await page.screenshot({
      animations: "disabled",
      path: `../.cache/ui/control-panel-brains-${width}.png`,
      fullPage: true,
    });
  }
  const audit = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(
    audit.violations.map((v) => ({
      id: v.id,
      targets: v.nodes.map((n) => n.target),
    })),
  ).toEqual([]);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/");
  await page.getByRole("button", { name: "Create Brain", exact: true }).click();
  await page.getByRole("dialog").getByLabel(/^Name/).fill("Preview only");
  await expect(
    page.getByRole("dialog").getByText("Preview only", { exact: true }),
  ).toBeVisible();
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/control-panel-create.png",
  });
  await page.keyboard.press("Escape");
  const pairing = await api(page, "/api/devices/pairings", {
    name: "Design proof Codex",
    host_kind: "codex",
    integration: "plugin",
  });
  await api(page, `/api/devices/pairings/${pairing.user_code}/approve`, {
    approve: true,
  });
  await page.goto("/agents");
  await expect(
    page.getByRole("heading", { name: "Agents", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/control-panel-agents.png",
  });
  await page
    .getByRole("button", { name: "Access tokens", exact: true })
    .click();
  await expect(
    page
      .getByRole("dialog", { name: "Access tokens" })
      .getByText("Design proof Codex", { exact: true }),
  ).toBeVisible();
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/control-panel-agent-inspector.png",
  });
  await page.keyboard.press("Escape");
  await api(page, "/api/team/invitations", {
    username: `design-teammate-${randomUUID().slice(0, 8)}`,
  });
  await page.goto("/team");
  for (const name of ["Accounts", "Invitations", "Account activity"]) {
    await page.getByRole("tab", { name, exact: true }).click();
    await page.screenshot({
      animations: "disabled",
      path: `../.cache/ui/control-panel-team-${name.replaceAll(" ", "-")}.png`,
    });
  }
  await page.getByRole("button", { name: "Add teammate", exact: true }).click();
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/control-panel-add-teammate.png",
  });
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Runtime diagnostics", exact: true })
    .click();
  await expect(
    page.getByRole("dialog").getByText("Requests started", { exact: true }),
  ).toBeVisible();
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/control-panel-diagnostics.png",
  });
  await page.keyboard.press("Escape");
  expect(errors).toEqual([]);
});

test("pipeline polling pins inspection and clears failed, stale and removed inputs", async ({
  page,
}) => {
  test.setTimeout(90000);
  await signIn(page);
  const brain = await api(page, "/api/brains", {
    name: "Polling isolation proof",
  });
  const a = item(randomUUID(), "INPUT_A"),
    b = item(randomUUID(), "INPUT_B");
  let rows = [a, b],
    mode: "ok" | "error" | "hang" = "ok",
    calls = 0;
  await page.route(`**/api/brains/${brain.id}/pipeline`, async (route) => {
    calls++;
    if (mode === "error") {
      await route.fulfill({
        status: 503,
        json: { message: "Fixture unavailable" },
      });
      return;
    }
    if (mode === "hang") {
      await new Promise((resolve) => setTimeout(resolve, 7500));
    }
    await route.fulfill({ json: feed(brain.id, rows) }).catch(() => {});
  });
  await page.goto(`/brains/${brain.id}/activity?tab=pipeline`);
  const pipeline = page.getByTestId("live-pipeline");
  await expect(
    pipeline.getByText("INPUT_A", { exact: true }).first(),
  ).toBeVisible();
  await expect(page.getByTestId("pipeline-pulse")).toHaveCount(0);
  await pipeline.getByRole("button", { name: "Inspect input" }).click();
  const drawer = page.getByRole("dialog", { name: "Processing record" });
  await expect(drawer.getByText("INPUT_A", { exact: true })).toBeVisible();
  rows = [b, a];
  await expect.poll(() => calls).toBeGreaterThan(1);
  await expect(drawer.getByText("INPUT_A", { exact: true })).toBeVisible();
  await expect(drawer.getByText("INPUT_B", { exact: true })).toHaveCount(0);
  rows = [b];
  await expect(drawer).toHaveCount(0, { timeout: 6000 });
  await expect(page.getByText("INPUT_A", { exact: true })).toHaveCount(0);
  b.processing_job!.state = "running";
  b.processing_job!.lease_until = iso(60000);
  await expect(page.getByTestId("pipeline-pulse")).toHaveCount(1, {
    timeout: 6000,
  });
  await expect(page.getByTestId("pipeline-pulse")).toHaveCount(0, {
    timeout: 3000,
  });
  mode = "error";
  await expect(
    pipeline.getByText("Activity unavailable", { exact: true }),
  ).toBeVisible({ timeout: 6000 });
  await expect(page.getByText("INPUT_B", { exact: true })).toHaveCount(0);
  mode = "ok";
  await pipeline.getByRole("button", { name: "Retry", exact: true }).click();
  await expect(
    pipeline.getByText("INPUT_B", { exact: true }).first(),
  ).toBeVisible();
  await expect(page.getByTestId("pipeline-pulse")).toHaveCount(0);
  mode = "hang";
  await expect(
    pipeline.getByText("Activity needs a fresh check", { exact: true }),
  ).toBeVisible({ timeout: 9000 });
  await expect(page.getByText("INPUT_B", { exact: true })).toHaveCount(0);
  await page.unroute(`**/api/brains/${brain.id}/pipeline`);
});

test("dashboard retains fresh graph and inspection across visibility and metadata refreshes", async ({
  page,
}) => {
  await signIn(page);
  const brain = await api(page, "/api/brains", {
    name: "Dashboard refresh proof",
  });
  const a = item(randomUUID(), "STABLE_INPUT");
  let revision = brain.updated_at;
  let paused: (() => void) | undefined;
  let hold = false;
  await page.route(`**/api/brains/${brain.id}`, (route) =>
    route.fulfill({ json: { ...brain, updated_at: revision } }),
  );
  await page.route(`**/api/brains/${brain.id}/pipeline`, async (route) => {
    if (hold)
      await new Promise<void>((resolve) => {
        paused = resolve;
      });
    await route.fulfill({ json: feed(brain.id, [a]) }).catch(() => {});
  });
  await page.goto(`/brains/${brain.id}/dashboard`);
  const pipeline = page.getByTestId("live-pipeline");
  await expect(
    pipeline.getByText("STABLE_INPUT", { exact: true }).first(),
  ).toBeVisible();
  await pipeline.getByRole("button", { name: "Inspect input" }).click();
  const drawer = page.getByRole("dialog", { name: "Processing record" });
  await expect(drawer).toBeVisible();
  const canvas = await pipeline.locator(".flow-canvas").elementHandle();
  // Keep a replacement read pending so a transient teardown cannot hide behind
  // a fast response. A visibility signal cannot renew the original deadline.
  hold = true;
  await pipeline
    .getByRole("button", { name: "Refresh pipeline", exact: true })
    .click({ force: true });
  await expect.poll(() => !!paused).toBe(true);
  await page.evaluate(() =>
    document.dispatchEvent(new Event("visibilitychange")),
  );
  await expect(drawer).toBeVisible();
  expect(await canvas!.evaluate((node) => node.isConnected)).toBe(true);
  hold = false;
  paused!();
  revision = new Date(Date.now() + 1000).toISOString();
  // The five-second Brain poll applies the changed metadata without replacing
  // the selected canvas or closing inspection.
  const refreshed = await page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/brains/${brain.id}`) &&
      response.request().resourceType() === "fetch",
  );
  expect((await refreshed.json()).updated_at).toBe(revision);
  await expect(drawer).toBeVisible();
  expect(await canvas!.evaluate((node) => node.isConnected)).toBe(true);
  await expect(
    pipeline.getByText("Loading recorded activity…", { exact: true }),
  ).toHaveCount(0);
});

test("reduced motion suppresses observed-change packets", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await signIn(page);
  const brain = await api(page, "/api/brains", {
      name: "Reduced motion proof",
    }),
    a = item(randomUUID(), "Reduced motion input");
  let seq = 0;
  await page.route(`**/api/brains/${brain.id}/pipeline`, (route) => {
    seq++;
    a.processing_job!.state = seq % 2 ? "queued" : "running";
    a.processing_job!.lease_until = iso(10000);
    return route.fulfill({ json: feed(brain.id, [a]) });
  });
  await page.goto(`/brains/${brain.id}/activity?tab=pipeline`);
  await expect.poll(() => seq).toBeGreaterThan(1);
  await expect(page.getByTestId("pipeline-pulse")).toHaveCount(0);
});

test("ambient memories keep exact identity and clear every card on failure and expiry", async ({
  page,
}) => {
  test.setTimeout(90000);
  await signIn(page);
  const brain = await api(page, "/api/brains", {
    name: "Ambient display proof",
  });
  const base = `/api/brains/${brain.id}`;
  const source = await api(page, `${base}/sources`, {
    title: "Ambient recorded source",
    media_type: "text/plain",
    retain_content: true,
    content: "This disposable service uses recorded source evidence.",
  });
  const remember = (subject: string) =>
    api(page, `${base}/claims`, {
      content: {
        kind: "claim",
        subject,
        predicate: "uses",
        value: "Recorded source evidence",
        rationale: "Disposable ambient acceptance",
        selection: { repository_ids: [], area_ids: [], environment_id: null },
        manifest_revision_id: null,
        validity: {
          kind: "unknown",
          from: null,
          to: null,
          precision: "unknown",
        },
        freshness: "current",
        operational: "declared",
        observed_at: null,
        observation: "",
        supports: [
          {
            kind: "source_version",
            id: source.version.id,
            line_from: null,
            line_to: null,
          },
        ],
      },
    });
  await remember("Ambient first memory");
  await remember("Ambient second memory");
  let mode: "ok" | "error" | "expiry" | "hang" = "ok",
    reads = 0;
  let lastPage: Record<string, unknown> | undefined;
  await page.route(`${base}/claims?offset=0`, async (route) => {
    reads++;
    if (mode === "error") {
      await route.fulfill({
        status: 503,
        json: { message: "Fixture refresh failure" },
      });
      return;
    }
    if (mode === "expiry" && lastPage) {
      mode = "hang";
      await route.fulfill({ json: { ...lastPage, expires_at: iso(1200) } });
      return;
    }
    if (mode === "hang") {
      await new Promise((resolve) => setTimeout(resolve, 6500));
      await route.fulfill({ json: lastPage }).catch(() => {});
      return;
    }
    // The content is always an actual read from the owned disposable Brain.
    // Only failure/deadline/latency are controlled at the transport boundary.
    try {
      const response = await route.fetch();
      lastPage = await response.json();
      await route.fulfill({ response, json: lastPage });
    } catch (error) {
      if (!page.isClosed()) throw error;
    }
  });
  await page.goto(`/brains/${brain.id}/activity?tab=capture`);
  await page
    .getByRole("button", { name: "Ambient display", exact: true })
    .click();
  const tv = page.getByRole("dialog", {
    name: `Ambient memory display for ${brain.name}`,
    exact: true,
  });
  await expect(tv.locator(".tv-item-enter .tv-title")).toHaveText(
    "Ambient second memory",
  );
  await expect(page).toHaveTitle(`${brain.name} · Ambient display · Recollect`);
  await expect(tv).toContainText("Review: proposed");
  await expect(tv).toContainText("Operational: declared");
  await expect(tv).toBeFocused();
  expect(
    await page.locator("#root").evaluate((el) => (el as HTMLElement).inert),
  ).toBe(true);
  await page.screenshot({
    path: "../.cache/desktop-final-tv.png",
    animations: "disabled",
  });
  await remember("Ambient newly inserted memory");
  await expect.poll(() => reads).toBeGreaterThan(1);
  await expect(tv.locator(".tv-item-enter .tv-title")).toHaveText(
    "Ambient second memory",
  );
  await expect(tv.locator(".tv-item-enter .tv-title")).toHaveText(
    "Ambient first memory",
    { timeout: 11000 },
  );
  mode = "error";
  await expect(tv).toContainText("Displayed content has been cleared.", {
    timeout: 6000,
  });
  await expect(tv.locator(".tv-item")).toHaveCount(0);
  await expect(tv.getByText(/last updated/)).toHaveCount(0);
  mode = "ok";
  await tv.getByRole("button", { name: "Try again", exact: true }).click();
  await expect(tv.locator(".tv-item-enter .tv-title")).toHaveText(
    "Ambient newly inserted memory",
  );
  mode = "expiry";
  await page.reload();
  await expect.poll(() => mode, { timeout: 15000 }).toBe("hang");
  await expect(tv).toContainText("content deadline", { timeout: 4500 });
  await expect(tv.locator(".tv-item")).toHaveCount(0);
  await expect(tv.getByText(/last updated/)).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(tv).toHaveCount(0);
  await expect(page).toHaveURL(new RegExp(`/brains/${brain.id}/activity`));
  await expect(page).toHaveTitle(`${brain.name} · Activity · Recollect`);
  expect(
    await page.locator("#root").evaluate((el) => (el as HTMLElement).inert),
  ).toBe(false);
  mode = "ok";
  await page.unroute(`${base}/claims?offset=0`);
  await page.goto(`/brains/${brain.id}/tv`);
  await expect(tv.locator(".tv-item-enter .tv-title")).toHaveText(
    "Ambient newly inserted memory",
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  const motion = await tv
    .locator(".tv-item-enter")
    .evaluate((el) => getComputedStyle(el).transform);
  expect(motion).toBe("none");
  // A controlled visibility event exercises the same hidden-tab input in the
  // owned browser fixture; the production page does not override visibility.
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      value: true,
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  const paused = await tv.locator(".tv-item-enter .tv-title").innerText();
  await page.waitForTimeout(8500);
  await expect(tv.locator(".tv-item-enter .tv-title")).toHaveText(paused);
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      value: false,
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await page.route(`${base}`, (route) =>
    route.fulfill({
      status: 403,
      json: { message: "Fixture access withdrawn" },
    }),
  );
  await expect(tv).toHaveCount(0, { timeout: 6000 });
  expect(
    await page.locator("#root").evaluate((el) => (el as HTMLElement).inert),
  ).toBe(false);
});

test("ambient empty Brain exits without creating content", async ({ page }) => {
  await signIn(page);
  const brain = await api(page, "/api/brains", { name: "Empty ambient Brain" });
  await page.goto(`/brains/${brain.id}/tv`);
  await expect(
    page.getByRole("heading", { name: "Nothing remembered yet", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".tv-item")).toHaveCount(0);
  await page
    .getByRole("button", { name: "Exit display · Esc", exact: true })
    .click();
  await expect(page.locator(".tv-root")).toHaveCount(0);
});

test("input reasons distinguish learning limits from capture and processing failures", () => {
  const a = item(randomUUID());
  const at = iso();
  a.learning = {
    id: randomUUID(),
    state: "failed",
    created_at: at,
    finished_at: at,
    accepted: 0,
    proposed: 0,
    reused: 0,
    revised: 0,
    retired: 0,
    blocked: 0,
    conflicting: 0,
    claim_ids: [],
    job: {
      ...a.processing_job!,
      state: "failed",
      error_code: "model_budget_exhausted",
    },
  };
  expect(inputStatus(a, at).label).toBe("Daily limit reached");
  expect(inputStatus(a, at).detail).toContain("midnight UTC");
  a.learning.job.error_code = "model_input_sensitive";
  expect(inputStatus(a, at).label).toBe("Sensitive input");
  a.learning.job.error_code = "model_input_too_large";
  expect(inputStatus(a, at).label).toBe("Input limit reached");
  a.learning.job.error_code = "invalid_input";
  expect(inputStatus(a, at).label).toBe("Learning failed");
  a.processing_job!.state = "queued";
  expect(inputStatus(a, at).label).toBe("Queued");
  a.processing_job!.state = "failed";
  expect(inputStatus(a, at).label).toBe("Processing failed");
  a.learning = null;
  a.processing_job = null;
  a.source_version_id = null;
  expect(inputStatus(a, at).label).toBe("Capture only");
});
