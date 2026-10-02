import { test, expect, type Page } from "@playwright/test";
import { randomUUID } from "node:crypto";

async function fixture(page: Page) {
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
  const session = await (await page.request.get("/api/auth/me")).json();
  const response = await page.request.post("/api/brains", {
    data: { name: "Assurance browser proof" },
    headers: { "x-csrf-token": session.csrf_token },
  });
  expect(response.ok()).toBeTruthy();
  const brain = await response.json();
  const base = `/api/brains/${brain.id}`;
  const automation = await (
    await page.request.get(`${base}/automation`)
  ).json();
  automation.models.current.policy.enabled = true;
  automation.models.current.policy.autonomous_memory = true;
  automation.models.installed.credentials_present = true;
  const latest = {
    id: randomUUID(),
    state: "succeeded",
    accepted: 7,
    revised: 3,
    retired: 1,
    created_at: new Date().toISOString(),
  };
  const responses: Record<string, unknown> = {
    automation,
    learning: {
      items: [latest, { ...latest, id: randomUUID(), accepted: 1000 }],
      total: 57,
      offset: 0,
    },
    processing: { state: "current", pending_jobs: 0, refreshed_at: null },
    jobs: [],
    "capture/events": { items: [], total: 42, offset: 0 },
    "capture/devices": { items: [], total: 0, offset: 0 },
    "mcp/calls": { calls: [], next_cursor: null },
    erasures: { items: [], total: 0, offset: 0 },
  };
  for (const name of Object.keys(responses))
    await page.route(
      (url) => url.pathname === `${base}/${name}`,
      (route) =>
        route.request().method() === "GET"
          ? route.fulfill({ json: responses[name] })
          : route.fallback(),
    );
  const writes: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes(base) && request.method() !== "GET")
      writes.push(request.method() + " " + new URL(request.url()).pathname);
  });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  return { brain, base, responses, writes, errors };
}

test("standing pulse labels its window and Brain total, with an exception-first Activity default", async ({
  page,
}) => {
  const f = await fixture(page);
  await page.goto(`/brains/${f.brain.id}/activity`);
  const band = page.getByRole("region", { name: "Brain assurance" });
  await expect(
    band.getByText("Nothing needs you", { exact: true }),
  ).toBeVisible();
  await expect(band).toContainText("7 learned · 3 revised · 1 retired");
  await expect(band).not.toContainText("1007");
  await expect(band).toContainText("42 captured events Brain total");
  await expect(
    page.getByRole("tab", { name: "Overview", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(
    page.getByRole("heading", { name: "No action required", exact: true }),
  ).toBeVisible();
  await expect(page.getByText(/They are not lifetime totals/)).toBeVisible();
  for (const width of [1280, 1440, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    await page.evaluate(async () => {
      await document.fonts.ready;
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
    });
    await expect(band).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page.screenshot({
      path: `../.cache/desktop-continuation-20261001/assurance-quiet-${width}.png`,
    });
  }
  await page.getByRole("tab", { name: "Timeline", exact: true }).click();
  await expect(
    page.getByText("Recent authorized audit events.", { exact: false }),
  ).toBeVisible();
  expect(f.writes).toEqual([]);
  expect(f.errors).toEqual([]);
});

test("exceptions open their canonical job, tool, erasure and capture detail", async ({
  page,
}) => {
  const f = await fixture(page);
  const job = {
    id: randomUUID(),
    brain_id: f.brain.id,
    kind: "source.process",
    state: "failed",
    attempts: 3,
    max_attempts: 3,
    progress: 0,
    error_code: "artifact_unavailable",
  };
  f.responses.jobs = [job];
  const call = {
    id: randomUUID(),
    brain_id: f.brain.id,
    actor_id: f.brain.owner_id,
    profile_id: randomUUID(),
    state: "unknown",
    resolutions: [],
    observations: [],
    output_access: "permission_required",
    capture_disposition: "disabled",
    tool_name: "test.read",
    runner_reference: "central",
    timeout_seconds: 30,
    result: { private: "DO_NOT_SHOW_PRIVATE_OUTPUT" },
  };
  f.responses["mcp/calls"] = { calls: [call], next_cursor: null };
  await page.route(`**${f.base}/mcp/calls/${call.id}`, (route) =>
    route.fulfill({ json: call }),
  );
  const erasure = {
    id: randomUUID(),
    brain_id: f.brain.id,
    state: "error",
    journaled: false,
    pending_artifacts: 1,
    graph_pending: true,
    created_at: new Date().toISOString(),
    cause: "erase",
    target: { id: randomUUID(), kind: "source" },
    acknowledged_devices: 0,
  };
  f.responses.erasures = { items: [erasure], total: 1, offset: 0 };
  f.responses["capture/devices"] = {
    items: [
      {
        device_id: randomUUID(),
        bindings: 1,
        last_publication: null,
        report: { device_gap_count: 2, denied: 0, pending: 0 },
        reported_at: new Date().toISOString(),
      },
    ],
    total: 1,
    offset: 0,
  };
  await page.goto(`/brains/${f.brain.id}/activity`);
  await expect(
    page.getByRole("region", { name: "Brain assurance" }),
  ).toContainText("A tool outcome is uncertain");
  await expect(
    page.getByRole("region", { name: "Brain assurance" }),
  ).not.toContainText("7 learned");
  await expect(
    page.getByRole("region", { name: "Brain assurance" }),
  ).not.toContainText("42 captured");
  await expect(page.getByText("DO_NOT_SHOW_PRIVATE_OUTPUT")).toHaveCount(0);
  await page.getByRole("link", { name: /Processing did not finish/ }).click();
  await expect(page.getByTestId("selected-job")).toContainText(
    "Process source text",
  );
  await expect(
    page.getByTestId("selected-job").getByRole("button", { name: "Retry job" }),
  ).toBeVisible();
  await page.getByRole("link", { name: "View activity", exact: false }).click();
  await page.getByRole("link", { name: /A tool outcome is uncertain/ }).click();
  await expect(
    page.getByRole("dialog", { name: "Tool call", exact: true }),
  ).toBeVisible();
  await expect(
    page
      .getByRole("dialog")
      .getByText("Completion unknown", { exact: true })
      .first(),
  ).toBeVisible();
  await expect(page.getByText("DO_NOT_SHOW_PRIVATE_OUTPUT")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await page.getByRole("link", { name: "View activity", exact: false }).click();
  await page
    .getByRole("link", { name: /Data cleanup needs attention/ })
    .click();
  await expect(page.getByTestId("selected-erasure")).toContainText(
    "journal pending",
  );
  await page.getByRole("link", { name: "View activity", exact: false }).click();
  await page
    .getByRole("link", { name: /A device reported a capture gap/ })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Capture coverage", exact: true }),
  ).toBeVisible();
  expect(f.writes).toEqual([]);
  expect(f.errors).toEqual([]);
});

test("historical failures and pending work stay quiet while current blockers name the problem", async ({
  page,
}) => {
  const f = await fixture(page);
  f.responses.jobs = [
    { id: randomUUID(), state: "failed", updated_at: "2026-09-14T19:41:18Z" },
  ];
  f.responses["mcp/calls"] = {
    calls: [{ id: randomUUID(), state: "tool_error", resolutions: [] }],
  };
  f.responses.erasures = {
    items: [{ id: randomUUID(), state: "pending", journaled: true }],
  };
  f.responses["capture/devices"] = {
    items: [
      { device_id: randomUUID(), report: { device_gap_count: 2, denied: 0 } },
    ],
  };
  await page.goto(`/brains/${f.brain.id}/activity`);
  const band = page.getByRole("region", { name: "Brain assurance" });
  await expect(band).toContainText("Nothing needs you");
  await expect(band).not.toHaveClass(/assurance-attention/);
  const history = page.getByRole("region", {
    name: "Activity history and pending work",
  });
  await expect(history.getByRole("link")).toHaveCount(4);
  await expect(history).toContainText("Processing did not finish");
  await expect(history).toContainText("Data cleanup is pending");
  await page.screenshot({
    path: "../.cache/desktop-continuation-20261001/assurance-history-quiet.png",
  });

  f.responses.processing = {
    state: "failed",
    pending_jobs: 0,
    refreshed_at: null,
  };
  await page.reload();
  await expect(band).toContainText("Brain processing is blocked");
  await expect(band).toHaveClass(/assurance-attention/);
  await expect(
    band.getByRole("link", { name: "Inspect problem" }),
  ).toHaveAttribute("href", /activity\?tab=processing$/);
  await expect(band).not.toContainText("captured events");
  await page.screenshot({
    path: "../.cache/desktop-continuation-20261001/assurance-current-blocker.png",
  });

  f.responses.processing = {
    state: "current",
    pending_jobs: 0,
    refreshed_at: null,
  };
  (
    f.responses.automation as {
      models: { installed: { credentials_present: boolean } };
    }
  ).models.installed.credentials_present = false;
  await page.reload();
  await expect(band).toContainText("Learning needs a provider credential");
  await expect(
    band.getByRole("link", { name: "Inspect problem" }),
  ).toHaveAttribute("href", /settings\?tab=ai$/);
  expect(f.writes).toEqual([]);
  expect(f.errors).toEqual([]);
});

test("partial read failure never claims healthy and role loss clears admin-only activity", async ({
  page,
}) => {
  const f = await fixture(page);
  f.responses["mcp/calls"] = {
    calls: [{ id: randomUUID(), state: "unknown", resolutions: [] }],
    next_cursor: null,
  };
  await page.route(`**${f.base}/learning`, (route) =>
    route.fulfill({
      status: 503,
      json: {
        code: "unavailable",
        message: "Learning temporarily unavailable",
      },
    }),
  );
  await page.goto(`/brains/${f.brain.id}/activity`);
  await expect(
    page.getByText("Some activity could not be checked", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("No action required", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("link", { name: /A tool outcome is uncertain/ }),
  ).toBeVisible();
  const restrictedReads: string[] = [];
  await page.route(`**${f.base}`, (route) =>
    route.fulfill({ json: { ...f.brain, role: "reader" } }),
  );
  // Wait for the existing authorization poll. No page reload clears the cache for us.
  await expect(
    page.getByRole("link", { name: /A tool outcome is uncertain/ }),
  ).toHaveCount(0, { timeout: 12_000 });
  page.on("request", (request) => {
    if (
      ["mcp/calls", "erasures", "capture/devices", "audit"].some((path) =>
        request.url().endsWith(`${f.base}/${path}`),
      )
    )
      restrictedReads.push(request.url());
  });
  await page
    .getByRole("button", { name: "Retry unavailable activity" })
    .click();
  await expect(
    page.getByText("Some activity could not be checked", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText("7 learned", { exact: false })).toHaveCount(0);
  expect(restrictedReads).toEqual([]);
  expect(f.writes).toEqual([]);
});

test("switching Brains discards a late pulse response", async ({ page }) => {
  const f = await fixture(page);
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  let started!: () => void;
  const entered = new Promise<void>((resolve) => {
    started = resolve;
  });
  await page.route(`**${f.base}/learning`, async (route) => {
    started();
    await gate;
    await route.fulfill({ json: f.responses.learning }).catch(() => {});
  });
  await page.goto(`/brains/${f.brain.id}/activity`);
  await entered;
  await expect(
    page.getByRole("region", { name: "Brain assurance" }),
  ).toContainText("Checking recent activity");
  await page.getByRole("link", { name: "All Brains", exact: true }).click();
  release();
  await expect(
    page.getByRole("region", { name: "Brain assurance" }),
  ).toHaveCount(0);
  await expect(page.getByText("7 learned", { exact: false })).toHaveCount(0);
  expect(f.writes).toEqual([]);
});
