import { openDetails } from "./desktop-helpers";
import { test, expect as baseExpect, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";

const expect = baseExpect.configure({ timeout: 15_000 });
async function command(
  page: Page,
  path: string,
  body?: unknown,
  method = body === undefined ? "GET" : "POST",
) {
  const me = await (await page.request.get("/api/auth/me")).json();
  const response = await page.request.fetch(path, {
    method,
    data: body,
    headers: { "x-csrf-token": me.csrf_token },
  });
  expect(response.ok(), `${method} ${path}: ${response.status()}`).toBeTruthy();
  return response.status() === 204 ? null : response.json();
}
async function setup(page: Page) {
  await page.setViewportSize({ width: 1440, height: 1000 });
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
  const brain = await command(page, "/api/brains", {
    name: "Desktop MCP execution",
  });
  const base = `/api/brains/${brain.id}`;
  const connection = await command(page, `${base}/mcp/connections`, {
    name: "Owned runtime",
    description: "Disposable synthetic execution",
    definition_key: "desktop-runtime-fixture",
    target: "synthetic-runtime",
    placement: "central",
    runner_reference: null,
    credential_alias: null,
    environment_id: null,
    configuration: { marker: process.env.RECOLLECT_UI_MCP_MARKER },
    enabled: true,
    base_revision: null,
  });
  const profile = (
    await command(page, `${base}/mcp/profiles`, {
      name: "Runnable tools",
      description: "Desktop runtime proof",
      environment_id: null,
      connection_ids: [connection.summary.id],
      enabled: true,
      base_revision: null,
    })
  ).profile;
  const grant = (allowed: boolean) =>
    command(
      page,
      `${base}/mcp/profiles/${profile.id}/grants`,
      {
        username: process.env.RECOLLECT_OWNER_USERNAME!,
        rights: { use_profile: allowed, manage: true, share: true },
      },
      "PUT",
    );
  await grant(true);
  await page.goto(`/brains/${brain.id}/connections?tab=profiles`);
  const panel = page.getByRole("region", {
    name: "MCP connections and profiles",
    exact: true,
  });
  await panel
    .getByRole("button", { name: "Runtime diagnostics", exact: true })
    .click();
  await expect(
    page.getByText("Runner available", { exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(
    panel.getByRole("button", { name: "Runtime diagnostics", exact: true }),
  ).toBeFocused();
  const run = async (name: string, args: string) => {
    for (let i = 0; i < 4 && (await page.getByRole("dialog").count()); i++) {
      await page.keyboard.press("Escape");
      await page.waitForTimeout(180);
    }
    if (!page.url().includes(`/brains/${brain.id}/connections`))
      await page.goto(`/brains/${brain.id}/connections?tab=profiles`);
    await panel
      .getByRole("button", { name: "Inspect cached tools", exact: true })
      .click();
    const cached = page.getByRole("dialog", {
      name: "Cached tools · Runnable tools",
      exact: true,
    });
    await cached
      .getByTestId("mcp-cached-tool")
      .filter({
        has: page.getByText(`Owned runtime · ${name}`, { exact: true }),
      })
      .getByRole("button", { name: "Run tool", exact: true })
      .click();
    const form = page.getByRole("dialog", {
      name: `Run tool · ${name}`,
      exact: true,
    });
    await form.getByLabel("Tool arguments", { exact: true }).fill(args);
    await form.getByRole("button", { name: "Run tool", exact: true }).click();
    return page.getByRole("dialog", { name: "Tool call", exact: true });
  };
  return { brain, base, connection, profile, grant, panel, run };
}

test("desktop executes once after lost submission response and reconciles an uncertain effect", async ({
  page,
}) => {
  test.setTimeout(150_000);
  page.setDefaultTimeout(15_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const f = await setup(page);
  let lost = false;
  await page.route(`**${f.base}/mcp/calls`, async (route) => {
    if (route.request().method() === "POST" && !lost) {
      lost = true;
      await route.fetch();
      await route.abort("failed");
    } else await route.continue();
  });
  let dialog = await f.run("inspect", '{"text":"Desktop retained result"}');
  const form = page.getByRole("dialog", {
    name: "Run tool · inspect",
    exact: true,
  });
  await expect(
    form.getByRole("button", { name: "Check submission", exact: true }),
  ).toBeVisible();
  await form
    .getByRole("button", { name: "Check submission", exact: true })
    .click();
  await expect(dialog.getByText("Succeeded", { exact: true })).toBeVisible();
  await expect(dialog).toContainText("Desktop retained result");
  const calls = (await command(page, `${f.base}/mcp/calls`)).calls;
  expect(calls).toHaveLength(1);
  expect(
    (await command(page, `${f.base}/mcp/calls/${calls[0].id}`)).result
      .structuredContent.call,
  ).toBe(1);
  await page.screenshot({ path: "../.cache/ui-mcp-result-desktop.png" });
  await page.keyboard.press("Escape");
  dialog = await f.run("effect", "{}");
  await expect(
    dialog.getByText("Completion unknown", { exact: true }).first(),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Look up connector receipt", exact: true })
    .click();
  await expect(dialog.getByText("Succeeded", { exact: true })).toBeVisible();
  await dialog
    .getByRole("button", { name: "Inspect original call", exact: true })
    .click();
  await expect(dialog).toContainText("connector receipt · succeeded");
  await expect(
    dialog.getByText("Completion unknown", { exact: true }).first(),
  ).toBeVisible();
  const source = await command(page, `${f.base}/sources`, {
    title: "Observed fixture outcome",
    media_type: "text/plain",
    content: "The isolated fixture recorded its effect.",
    retain_content: true,
  });
  await dialog
    .getByRole("button", { name: "Record evidence", exact: true })
    .click();
  await dialog.getByLabel("Evidence source", { exact: true }).click();
  await page
    .getByRole("option", { name: "Observed fixture outcome", exact: true })
    .click();
  await dialog.getByLabel("Retained evidence version", { exact: true }).click();
  await page
    .getByRole("option", { name: /^Observed fixture outcome ·/ })
    .click();
  await dialog.getByLabel("Observed outcome", { exact: true }).click();
  await page.getByRole("option", { name: "Succeeded", exact: true }).click();
  await dialog
    .getByLabel("Evidence explanation", { exact: true })
    .fill("Evidence confirms the isolated effect.");
  await dialog
    .getByRole("button", { name: "Save evidence resolution", exact: true })
    .click();
  await expect(
    dialog.getByLabel("Evidence explanation", { exact: true }),
  ).not.toBeVisible();
  await expect(dialog).toContainText("evidence · succeeded");
  await expect(dialog).toContainText("Evidence confirms the isolated effect.");
  await page.screenshot({
    path: "../.cache/ui-mcp-reconciliation-desktop.png",
  });
  const preview = await command(page, `${f.base}/erasures/preview`, {
    kind: "source",
    id: source.id,
  });
  await command(page, `${f.base}/erasures`, {
    target: { kind: "source", id: source.id },
    eligibility_epoch: preview.eligibility_epoch,
  });
  await expect(dialog).not.toContainText(
    "Evidence confirms the isolated effect.",
  );
  const history = (await command(page, `${f.base}/mcp/calls`)).calls;
  expect(
    history.filter((c: { tool_name: string }) => c.tool_name === "effect"),
  ).toHaveLength(1);
  expect(
    history.filter((c: { tool_name: string }) => c.tool_name === "receipt"),
  ).toHaveLength(1);
  expect(errors).toEqual([]);
});

test("managed capture consent publishes scoped evidence and exposes filtering and retry states", async ({
  page,
}) => {
  test.setTimeout(120_000);
  page.setDefaultTimeout(15_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const f = await setup(page);
  await page.goto(`/brains/${f.brain.id}/settings?tab=capture`);
  const capture = page.getByRole("region", {
    name: "Session capture",
    exact: true,
  });
  await capture
    .getByRole("button", { name: "Capture policy", exact: true })
    .click();
  const policy = page.getByRole("dialog", {
    name: "Session capture policy",
    exact: true,
  });
  await policy
    .getByLabel("Enable automatic session capture", { exact: true })
    .check();
  await policy
    .getByRole("switch", { name: /^Capture managed tool results/ })
    .check();
  await expect(policy).toContainText("shared with this Brain's readers");
  await policy
    .getByRole("button", { name: "Save capture policy", exact: true })
    .click();
  await expect(policy).not.toBeVisible();
  await expect(
    capture.getByText("Managed tools enabled", { exact: true }),
  ).toBeVisible();
  let dialog = await f.run("inspect", '{"text":"DesktopObservationEvidence"}');
  await expect(dialog.getByText("Succeeded", { exact: true })).toBeVisible();
  await dialog
    .getByRole("button", { name: "Inspect captured evidence", exact: true })
    .click();
  const source = page.getByRole("dialog", {
    name: "Captured source evidence",
    exact: true,
  });
  await expect(source.getByTestId("capture-source-content")).toContainText(
    "DesktopObservationEvidence",
  );
  await expect(source.getByTestId("capture-source-content")).toContainText(
    "reported_tool_observation",
  );
  await page.screenshot({
    path: "../.cache/ui-managed-source-desktop.png",
    animations: "disabled",
  });
  await page.keyboard.press("Escape");
  await expect(
    dialog.getByText("Captured observations", { exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByRole("button", {
      name: "Inspect captured evidence",
      exact: true,
    }),
  ).toBeFocused();
  await page.screenshot({
    path: "../.cache/ui-managed-call-desktop.png",
    animations: "disabled",
  });
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible();
  await expect(
    page.getByRole("dialog", { name: "Tool runtime diagnostics", exact: true }),
  ).toBeVisible();
  await page.goto(`/brains/${f.brain.id}/agents?tab=sessions`);
  await capture
    .getByRole("button", { name: "Capture coverage", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Capture coverage", exact: true }),
  ).toContainText("1 published");
  await page.keyboard.press("Escape");
  await expect(capture).toContainText("Managed tool");
  await page.screenshot({
    path: "../.cache/ui-managed-activity-desktop.png",
    animations: "disabled",
  });
  dialog = await f.run("inspect", '{"text":"read /tmp/.env"}');
  await expect(dialog.getByText("filtered", { exact: true })).toBeVisible();
  await expect(dialog).toContainText("excluded tool content");
  await expect(
    dialog.getByRole("button", {
      name: "Inspect captured evidence",
      exact: true,
    }),
  ).toHaveCount(0);
  await page.route(`**${f.base}/mcp/calls/*/observations*`, (route) =>
    route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({
        code: "fixture_unavailable",
        message: "Observation status temporarily unavailable.",
      }),
    }),
  );
  await expect(dialog).toContainText(
    "Observation status temporarily unavailable.",
  );
  await page.unroute(`**${f.base}/mcp/calls/*/observations*`);
  await dialog
    .getByRole("button", { name: "Refresh observations", exact: true })
    .click();
  await expect(dialog.getByText("filtered", { exact: true })).toBeVisible();
  expect(errors).toEqual([]);
});

test("desktop cancellation, output expiry and Use revocation clear retained content", async ({
  page,
}) => {
  test.setTimeout(120_000);
  page.setDefaultTimeout(15_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const f = await setup(page);
  let dialog = await f.run("slow", '{"millis":5000}');
  await expect(
    dialog.getByRole("button", { name: "Request cancellation", exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Request cancellation", exact: true })
    .click();
  await expect(dialog).toContainText("Cancellation requested.");
  // A completed response may win the cancellation race; neither UI nor executor
  // promises rollback. All terminal states must stop offering cancellation.
  await expect(
    dialog.getByRole("button", { name: "Request cancellation", exact: true }),
  ).not.toBeVisible();
  await page.keyboard.press("Escape");
  dialog = await f.run("inspect", '{"text":"Ephemeral visible result"}');
  await expect(dialog).toContainText("Ephemeral visible result");
  const calls = (await command(page, `${f.base}/mcp/calls`)).calls;
  const id = calls.find(
    (c: { tool_name: string }) => c.tool_name === "inspect",
  ).id;
  expect(id).toMatch(/^[0-9a-f-]{36}$/);
  const database = new URL(process.env.DATABASE_URL!).pathname.slice(1);
  expect(database).toMatch(/^recollect_ui_[0-9a-f]{32}$/);
  execFileSync(
    "../scripts/docker.sh",
    [
      "compose",
      "exec",
      "-T",
      "postgres",
      "psql",
      "-U",
      "recollect_admin",
      "-d",
      database,
      "-c",
      `UPDATE mcp_calls SET payload_expires_at=clock_timestamp()+interval '3 seconds' WHERE id='${id}'`,
    ],
    { stdio: "ignore" },
  );
  await expect(dialog).toContainText("Retained output has expired.");
  await expect(dialog).not.toContainText("Ephemeral visible result");
  await page.keyboard.press("Escape");
  dialog = await f.run("inspect", '{"text":"Visible only with Use"}');
  await expect(dialog).toContainText("Visible only with Use");
  await f.grant(false);
  await expect(dialog).toContainText("Profile Use is required");
  await expect(dialog).not.toContainText("Visible only with Use");
  await page.keyboard.press("Escape");
  await expect(
    f.panel.getByRole("button", { name: "Inspect cached tools", exact: true }),
  ).toBeDisabled();
  const runtime = page.getByRole("dialog", {
    name: "Tool runtime diagnostics",
    exact: true,
  });
  await expect(runtime).toBeVisible();
  await runtime
    .getByRole("button", { name: "End tool session", exact: true })
    .click();
  expect(errors).toEqual([]);
});
