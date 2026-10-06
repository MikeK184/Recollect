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
  await page.goto(`/brains/${fixture.brain}/settings?tab=capture`);
  const panel = page.getByRole("region", {
    name: /Capture permissions|Captured activity/,
  });
  const captureEnabled = page.getByRole("switch", {
    name: "Enable automatic session capture",
    exact: true,
  });
  await expect(captureEnabled).not.toBeChecked();
  await expect(captureEnabled).toBeDisabled();
  await page.screenshot({
    path: "../.cache/guided-privacy-off.png",
    animations: "disabled",
  });
  await page.goto(`/brains/${fixture.brain}/agents?tab=contexts`);
  await expect(page).toHaveURL(new RegExp(`/brains/${fixture.brain}/agents$`));
  await expect(
    page.getByRole("dialog", { name: "Your private context" }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Your private context" }),
  ).toHaveCount(0);
  // Hiding automatic context controls does not remove the native scope record.
  const scope = await page.request.get(
    `/api/brains/${fixture.brain}/workspace`,
  );
  expect(scope.ok()).toBe(true);
  expect(
    (await scope.json()).tasks.some(
      (task: { label: string }) => task.label === "Browser capture",
    ),
  ).toBe(true);
  await page
    .getByRole("button", { name: "Connect agent", exact: true })
    .click();
  const setup = page.getByRole("dialog", {
    name: "Connect a coding agent",
    exact: true,
  });
  await setup.getByRole("button", { name: "Next", exact: true }).click();
  await setup
    .getByLabel("Plugin package folder", { exact: true })
    .fill("/tmp/recollect-plugin");
  await expect(setup.getByTestId("agent-plugin-command")).toContainText(
    "codex plugin marketplace add",
  );
  await page.screenshot({
    path: "../.cache/guided-setup-install.png",
    animations: "disabled",
  });
  await page.keyboard.press("Escape");

  await page.goto(`/brains/${fixture.brain}/agents?tab=sessions`);
  await panel
    .getByRole("button", { name: "Capture diagnostics", exact: true })
    .click();
  const coverage = page.getByRole("dialog", {
    name: "Capture diagnostics",
    exact: true,
  });
  await expect(
    coverage.getByText("Configured only", { exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await page.goto(`/brains/${fixture.brain}/settings?tab=capture`);
  await expect(
    page.getByRole("tab", { name: "Privacy", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page
    .getByRole("button", { name: "Edit privacy settings", exact: true })
    .click();
  let dialog = page.getByRole("form", {
    name: "Privacy settings",
    exact: true,
  });
  await page.evaluate(async (brain) => {
    const me = await (await fetch("/api/auth/me")).json();
    const path = `/api/brains/${brain}/capture/policy`;
    const current = await (await fetch(path)).json();
    current.policy.excluded_tools = ["concurrent-policy-exclusion"];
    const response = await fetch(path, {
      method: "PUT",
      headers: {
        "content-type": "application/json",
        "x-csrf-token": me.csrf_token,
      },
      body: JSON.stringify({
        base_change: current.change_id,
        policy: current.policy,
      }),
    });
    if (!response.ok)
      throw new Error(`Concurrent capture policy ${response.status}`);
  }, fixture.brain);
  await expect
    .poll(async () =>
      page.evaluate(async (brain) => {
        const current = await (
          await fetch(`/api/brains/${brain}/capture/policy`)
        ).json();
        return current.policy.excluded_tools;
      }, fixture.brain),
    )
    .toEqual(["concurrent-policy-exclusion"]);
  await page.screenshot({
    path: "../.cache/guided-privacy-custom.png",
    animations: "disabled",
  });
  await expect(dialog).toContainText(/changed elsewhere/i);
  await expect(dialog.getByRole("button", { name: "Save changes", exact: true })).toBeDisabled();
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(page.getByText("concurrent-policy-exclusion", { exact: true })).toHaveCount(0);
  await page
    .getByRole("button", { name: "Edit privacy settings", exact: true })
    .click();
  await expect(
    dialog.getByLabel("Excluded tools", { exact: true }),
  ).toHaveCount(0);
  await dialog
    .getByLabel("Enable automatic session capture", { exact: true })
    .check();
  await dialog
    .getByRole("button", { name: "Save changes", exact: true })
    .click();
  await expect(page.getByRole("button", { name: "Edit privacy settings", exact: true })).toBeVisible();
  await expect(captureEnabled).toBeChecked();
  expect(await page.evaluate(async (brain) => {
    const current = await (await fetch(`/api/brains/${brain}/capture/policy`)).json();
    return current.policy.excluded_tools;
  }, fixture.brain)).toEqual(["concurrent-policy-exclusion"]);
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
  await page.goto(`/brains/${fixture.brain}/agents?tab=sessions`);
  await panel
    .getByRole("button", { name: "Capture diagnostics", exact: true })
    .click();
  await expect(
    coverage.getByText("Partial delivery", { exact: true }),
  ).toBeVisible();
  await expect(coverage.getByText(/1 queued · 0 denied/)).toBeVisible();
  await expect(
    coverage.getByText(/2 gaps reported across this device/),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await panel
    .getByRole("button", { name: "Open evidence", exact: true })
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
  await page.setViewportSize({ width: 1280, height: 800 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: "../.cache/capture-source-1280.png" });
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
    panel.getByRole("button", { name: "Open evidence", exact: true }),
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
  await panel
    .getByRole("button", { name: "Capture diagnostics", exact: true })
    .click();
  await expect(
    coverage.getByText("Connected · queue empty", { exact: true }),
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
    coverage.getByText("Offline or stopped", { exact: true }),
  ).toBeVisible();
  await page.setViewportSize({ width: 1440, height: 960 });
  await panel.scrollIntoViewIfNeeded();
  await panel.screenshot({ path: "../.cache/capture-panel-desktop.png" });
  expect(errors).toEqual([]);
});
