import { test, expect } from "@playwright/test";

test("capture policy, companion delivery states and retained source erasure", async ({
  page,
}) => {
  test.setTimeout(90_000);
  page.setDefaultTimeout(12_000);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
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
  const fixture = await page.evaluate(async () => {
    const me = await (await fetch("/api/auth/me")).json();
    const headers = {
      "content-type": "application/json",
      "x-csrf-token": me.csrf_token,
    };
    const request = async (url: string, body: unknown) => {
      const r = await fetch(url, {
        method: "POST",
        headers,
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error(`Capture fixture setup ${r.status}`);
      return r.status === 204 ? null : r.json();
    };
    const brain = await request("/api/brains", {
      name: "Synthetic capture browser proof",
    });
    const pair = await request("/api/devices/pairings", {
      name: "Capture browser companion fixture",
    });
    await request(`/api/devices/pairings/${pair.user_code}/approve`, {
      approve: true,
    });
    const poll = await request("/api/devices/pairings/poll", {
      device_code: pair.device_code,
    });
    await request("/api/devices/pairings/finish", {
      device_code: pair.device_code,
    });
    const deviceRequest = async (url: string, body: unknown) => {
      const r = await fetch(url, {
        method: "POST",
        credentials: "omit",
        headers: {
          "content-type": "application/json",
          authorization: `Bearer ${poll.token}`,
        },
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error(`Capture binding fixture ${r.status}`);
      return r.json();
    };
    const base = `/api/brains/${brain.id}`;
    const task = await deviceRequest(`${base}/workspace/tasks`, {
      label: "Browser capture",
      selection: {},
    });
    const operation = await deviceRequest(
      `${base}/workspace/tasks/${task.task.id}/operations`,
      { kind: "capture" },
    );
    const binding = await deviceRequest(`${base}/capture/bindings`, {
      id: crypto.randomUUID(),
      operation_id: operation.id,
      host: "codex",
      host_version: "0.154.0",
    });
    return { brain: brain.id, binding: binding.id, token: poll.token };
  });
  await page.goto(`/brains/${fixture.brain}`);
  const panel = page.getByRole("region", {
    name: "Session capture",
    exact: true,
  });
  await expect(
    panel.getByText("Capture disabled", { exact: true }),
  ).toBeVisible();
  await expect(
    panel.getByText("Configured only", { exact: true }),
  ).toBeVisible();
  await panel
    .getByRole("button", { name: "Connect capture", exact: true })
    .click();
  let dialog = page.getByRole("dialog", {
    name: "Connect session capture",
    exact: true,
  });
  await expect(
    dialog.getByText(/cargo run -p recollect-agent -- capture setup/),
  ).toContainText(fixture.brain);
  await page.keyboard.press("Escape");
  await panel
    .getByRole("button", { name: "Capture policy", exact: true })
    .click();
  dialog = page.getByRole("dialog", {
    name: "Session capture policy",
    exact: true,
  });
  await dialog
    .getByLabel("Enable automatic session capture", { exact: true })
    .check();
  await dialog
    .getByLabel("Excluded tools", { exact: true })
    .fill("sensitive_fixture_tool");
  await dialog
    .getByRole("button", { name: "Save capture policy", exact: true })
    .click();
  await expect(
    panel.getByText("Capture enabled", { exact: true }),
  ).toBeVisible();
  await page.evaluate(async (f) => {
    const headers = {
      "content-type": "application/json",
      authorization: `Bearer ${f.token}`,
    };
    const base = `/api/brains/${f.brain}/capture`;
    const event = {
      id: crypto.randomUUID(),
      binding_id: f.binding,
      event: {
        host_event: "UserPromptSubmit",
        host_session_id: "browser-session",
        turn_id: "browser-turn",
        agent_id: null,
        tool_use_id: null,
        tool_name: null,
        kind: "prompt",
        outcome: "reported",
        content:
          "SYNTHETIC_CAPTURE_BROWSER_FACT: Amber.port = 8080\nAPI_KEY=synthetic-browser-credential",
        coverage: ["partial_host_coverage"],
        captured_at: new Date().toISOString(),
      },
    };
    for (const [url, body] of [
      [`${base}/events`, event],
      [
        `${base}/devices`,
        {
          pending: 1,
          denied: 0,
          device_gap_count: 2,
          issue: "transport_unavailable",
        },
      ],
    ] as const) {
      const r = await fetch(url, {
        method: "POST",
        credentials: "omit",
        headers,
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error(`Capture fixture delivery ${r.status}`);
    }
  }, fixture);
  await expect(
    panel.getByText("Partial delivery", { exact: true }),
  ).toBeVisible();
  await expect(panel.getByText(/1 queued · 0 denied/)).toBeVisible();
  await expect(
    panel.getByText(/2 gaps reported across this device/),
  ).toBeVisible();
  await panel
    .getByRole("button", { name: "View captured source", exact: true })
    .click();
  dialog = page.getByRole("dialog", {
    name: "Captured source evidence",
    exact: true,
  });
  await expect(dialog.getByTestId("capture-source-content")).toContainText(
    "Amber.port = 8080",
  );
  await expect(dialog.getByTestId("capture-source-content")).not.toContainText(
    "synthetic-browser-credential",
  );
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: "../.cache/capture-source-mobile.png" });
  await dialog
    .getByRole("button", { name: "Erase source", exact: true })
    .click();
  const erase = page.getByRole("dialog", {
    name: "Erase controlled memory",
    exact: true,
  });
  await expect(
    erase.getByText(/1 capture events will be removed/),
  ).toBeVisible();
  await erase
    .getByRole("button", { name: "Confirm erasure", exact: true })
    .click();
  await expect(erase.getByText(/Content is now unavailable/)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("capture-source-content")).toHaveCount(0);
  await expect(
    panel.getByRole("button", { name: "View captured source", exact: true }),
  ).toHaveCount(0);
  await page.evaluate(async (f) => {
    const r = await fetch(`/api/brains/${f.brain}/capture/devices`, {
      method: "POST",
      credentials: "omit",
      headers: {
        "content-type": "application/json",
        authorization: `Bearer ${f.token}`,
      },
      body: JSON.stringify({
        pending: 0,
        denied: 0,
        device_gap_count: 2,
        issue: null,
      }),
    });
    if (!r.ok) throw new Error(`Capture report recovery ${r.status}`);
  }, fixture);
  await expect(
    panel.getByText("Connected · queue empty", { exact: true }),
  ).toBeVisible();
  // A stale server timestamp is displayed as offline/stopped, not connected.
  await page.route(
    `**/api/brains/${fixture.brain}/capture/devices*`,
    async (route) => {
      const response = await route.fetch();
      const body = await response.json();
      body.items[0].reported_at = new Date(Date.now() - 60_000).toISOString();
      await route.fulfill({ response, json: body });
    },
  );
  await expect(
    panel.getByText("Offline or stopped", { exact: true }),
  ).toBeVisible();
  await page.setViewportSize({ width: 1440, height: 960 });
  await panel.scrollIntoViewIfNeeded();
  await panel.screenshot({ path: "../.cache/capture-panel-desktop.png" });
  expect(errors).toEqual([]);
});
