import {
  test,
  expect as baseExpect,
  type Page,
  type Locator,
} from "@playwright/test";

const expect = baseExpect.configure({ timeout: 12_000 });

async function expectRights(
  card: Locator,
  use: boolean,
  manage: boolean,
  share: boolean,
) {
  for (const [label, allowed] of [
    ["Use", use],
    ["Manage", manage],
    ["Share", share],
  ] as const)
    await expect(
      card.getByRole("img", {
        name: `${label} for you: ${allowed ? "Allowed" : "Denied"}`,
        exact: true,
      }),
    ).toHaveClass(allowed ? /allowed/ : /denied/);
}
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
  expect(
    response.ok(),
    `Synthetic catalogue command ${method} ${path}: ${response.status()}`,
  ).toBeTruthy();
  return response.status() === 204 ? null : response.json();
}
async function setup(page: Page, name: string) {
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
  const brain = await command(page, "/api/brains", { name });
  const base = `/api/brains/${brain.id}`;
  const environment = await command(page, `${base}/evidence/groups`, {
    kind: "environment",
    name: "Laboratory",
  });
  await page.goto(`/brains/${brain.id}/connections?tab=connections`);
  const panel = page.getByRole("region", {
    name: "MCP connections and tool groups",
    exact: true,
  });
  await expect(
    panel.getByRole("button", { name: "Add connection", exact: true }),
  ).toBeEnabled();
  return { brain, base, environment, panel };
}

test("desktop catalogue configures scoped profiles, explicit use and cached schemas without connecting", async ({
  page,
}) => {
  test.setTimeout(150_000);
  page.setDefaultTimeout(12_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const f = await setup(page, "Desktop MCP catalogue");
  await page.goto(`/brains/${f.brain.id}/settings?tab=general#environments`);
  await page
    .getByRole("button", { name: "Add environment", exact: true })
    .click();
  const envEditor = page.getByRole("region", {
    name: "Add environment",
    exact: true,
  });
  await expect(envEditor).toHaveAccessibleName("Add environment");
  await envEditor
    .getByRole("textbox", { name: "Environment name", exact: true })
    .fill("Production");
  await envEditor
    .getByRole("button", { name: "Save environment", exact: true })
    .click();
  await expect(envEditor).not.toBeVisible();
  const envCatalogue = await command(page, `${f.base}/evidence`);
  const production = envCatalogue.groups.find(
    (e: { kind: string; name: string }) =>
      e.kind === "environment" && e.name === "Production",
  );
  expect(production).toBeTruthy();
  await command(
    page,
    `${f.base}/evidence/groups/${production.id}`,
    { name: "Production", description: "Keep this environment description" },
    "PATCH",
  );
  const envRow = page
    .getByTestId("general-environment-row")
    .filter({ has: page.getByText("Production", { exact: true }) });
  await envRow
    .getByRole("button", { name: "Rename Production", exact: true })
    .click();
  const renamed = page.getByRole("region", {
    name: "Rename environment",
    exact: true,
  });
  await expect(renamed).toHaveAccessibleName("Rename environment");
  await renamed
    .getByRole("textbox", { name: "Environment name", exact: true })
    .fill("Production EU");
  await renamed
    .getByRole("button", { name: "Save environment", exact: true })
    .click();
  await expect(renamed).not.toBeVisible();
  const updatedEnvironment = (
    await command(page, `${f.base}/evidence`)
  ).groups.find((e: { id: string }) => e.id === production.id);
  expect(updatedEnvironment.name).toBe("Production EU");
  expect(updatedEnvironment.description).toBe(
    "Keep this environment description",
  );
  await page.goto(`/brains/${f.brain.id}/connections`);

  await f.panel
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  await page
    .getByRole("dialog", { name: "Add connection", exact: true })
    .getByText("Approved connector", { exact: true })
    .click();
  await page
    .getByRole("dialog", { name: "Add connection", exact: true })
    .getByRole("button", { name: "Choose connector", exact: true })
    .click();
  let dialog = page.getByRole("dialog", {
    name: "Add connection",
    exact: true,
  });
  await dialog.getByLabel(/^Connection name/).fill("Laboratory inspector");
  await dialog.getByLabel(/^Approved connector/).click();
  await page
    .getByRole("option", { name: "Approved fixture", exact: true })
    .click();
  await dialog.getByLabel(/^Target label/).fill("laboratory-target");
  await dialog.getByLabel("Connection environment", { exact: true }).click();
  await page.getByRole("option", { name: "Laboratory", exact: true }).click();
  await dialog.getByLabel("Credential alias", { exact: true }).click();
  await page.getByRole("option", { name: "fixture-read", exact: true }).click();
  await dialog
    .getByLabel("Non-secret settings", { exact: true })
    .fill('{"region":"invalid"}');
  await dialog
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await expect(dialog).toContainText("Configuration must match");
  await dialog
    .getByLabel("Non-secret settings", { exact: true })
    .fill('{"region":"test"}');
  await dialog
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await expect(dialog).not.toBeVisible();
  await expect(f.panel.getByTestId("mcp-connection")).toContainText(
    "Configured",
  );
  await expect
    .poll(() =>
      f.panel
        .getByTestId("mcp-connection")
        .first()
        .evaluate((el) => getComputedStyle(el).opacity),
    )
    .toBe("1");
  await page.screenshot({
    path: "../.cache/guided-connections.png",
    animations: "disabled",
  });
  await f.panel
    .getByRole("textbox", { name: "Find a connection", exact: true })
    .fill("does-not-exist");
  await expect(f.panel.getByTestId("mcp-connection")).toHaveCount(0);
  await expect(
    f.panel.getByText("No connections match these filters.", { exact: true }),
  ).toBeVisible();
  await f.panel
    .getByRole("textbox", { name: "Find a connection", exact: true })
    .fill("lab");
  await expect(f.panel.getByTestId("mcp-connection")).toHaveCount(1);
  await f.panel
    .getByRole("textbox", { name: "Find a connection", exact: true })
    .fill("");
  const connectionCard = f.panel.getByTestId("mcp-connection");
  const inspector = f.panel.getByRole("complementary", {
    name: "Connection details",
  });
  await inspector
    .getByRole("button", { name: "Pause use…", exact: true })
    .click();
  const power = page.getByRole("dialog", {
    name: "Pause connection use",
    exact: true,
  });
  await expect(power).toContainText("Already dispatched calls may finish");
  await power.getByRole("button", { name: "Pause use", exact: true }).click();
  await expect(connectionCard).toContainText("Connection disabled");
  await inspector
    .getByRole("button", { name: "Enable use…", exact: true })
    .click();
  await page
    .getByRole("dialog", { name: "Enable connection use", exact: true })
    .getByRole("button", { name: "Enable use", exact: true })
    .click();
  await expect(connectionCard).toContainText("Configured");
  await page.getByRole("tab", { name: "Tool access", exact: true }).click();
  await f.panel
    .getByRole("button", { name: "Create tool group", exact: true })
    .click();
  dialog = f.panel.getByTestId("mcp-profile").first();
  await dialog.getByLabel(/^Tool group name/).fill("Laboratory tools");
  await dialog.getByLabel("Tool group environment", { exact: true }).click();
  await page.getByRole("option", { name: "Laboratory", exact: true }).click();
  await dialog.getByLabel("Add MCP", { exact: true }).click();
  await page
    .getByRole("option", { name: "Laboratory inspector", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "Save changes", exact: true })
    .click();
  await expect(
    dialog.getByLabel("Tool group name", { exact: true }),
  ).not.toBeVisible();
  const card = f.panel.getByTestId("mcp-profile");
  await expectRights(card, false, true, true);
  await page.screenshot({
    path: "../.cache/guided-tool-access.png",
    animations: "disabled",
  });
  await expect(
    card.getByRole("button", { name: "Tools & testing", exact: true }),
  ).toBeDisabled();
  await card
    .getByRole("button", { name: "Edit Laboratory tools", exact: true })
    .click();
  await card
    .getByRole("button", {
      name: `Use for ${process.env.RECOLLECT_OWNER_USERNAME} direct: Denied`,
      exact: true,
    })
    .click();
  await card.getByRole("button", { name: "Save changes", exact: true }).click();
  await expect(
    card.getByRole("button", { name: "Edit Laboratory tools", exact: true }),
  ).toBeVisible();
  await expect(
    card.getByRole("button", { name: "Tools & testing", exact: true }),
  ).toBeEnabled();
  await card
    .getByRole("button", { name: "Tools & testing", exact: true })
    .click();
  const cached = page.getByRole("dialog", {
    name: "Tools · Laboratory tools",
    exact: true,
  });
  await expect(cached).toContainText("1 tools · Approved catalogue");
  await expect(
    cached.getByLabel("Tool environment", { exact: true }),
  ).toHaveValue("Laboratory");
  await expect(
    cached.getByLabel("Tool environment", { exact: true }),
  ).toBeDisabled();
  await cached.getByText("Tool details", { exact: true }).first().click();
  await cached.getByText("Output schema", { exact: true }).click();
  await expect(cached).toContainText('"type": "array"');
  expect(
    await page.evaluate(
      () => (window as unknown as { mcpInjected?: boolean }).mcpInjected,
    ),
  ).toBeUndefined();
  await cached.screenshot({
    path: "../.cache/mcp-catalogue-tools.png",
    animations: "disabled",
  });
  const catalogue = await command(page, `${f.base}/mcp`);
  const profile = catalogue.profiles[0];
  const connection = catalogue.connections[0];
  await command(
    page,
    `${f.base}/mcp/profiles/${profile.id}/grants`,
    {
      username: process.env.RECOLLECT_OWNER_USERNAME,
      rights: { use_profile: false, manage: false, share: false },
    },
    "PUT",
  );
  await expect(cached).not.toBeVisible();
  await expect(f.panel).toContainText("Cached tool results were cleared");
  await expect(
    card.getByRole("button", { name: "Tools & testing", exact: true }),
  ).toBeDisabled();
  // A remote edit retains the inline draft and blocks stale configuration writes.
  await page.goto(`/brains/${f.brain.id}/connections`);
  await f.panel
    .getByRole("button", { name: "Edit connection", exact: true })
    .click();
  dialog = f.panel.getByRole("complementary", { name: "Connection details" });
  await dialog.getByText("More configuration", { exact: true }).click();
  await dialog
    .getByLabel("Description", { exact: true })
    .fill("Unsaved local text");
  const detail = await command(
    page,
    `${f.base}/mcp/connections/${connection.id}`,
  );
  const input = {
    name: detail.summary.name,
    description: "Updated from another session",
    definition_key: detail.summary.definition_key,
    environment_id: detail.summary.environment_id,
    target: detail.target,
    placement: detail.summary.placement,
    runner_reference: detail.runner_reference,
    credential_alias: detail.credential_alias,
    configuration: detail.configuration,
    enabled: true,
    base_revision: detail.summary.revision,
  };
  await command(
    page,
    `${f.base}/mcp/connections/${connection.id}`,
    input,
    "PUT",
  );
  await expect(dialog).toContainText("Your draft is retained");
  await expect(dialog.getByLabel("Description", { exact: true })).toHaveValue(
    "Unsaved local text",
  );
  await dialog
    .getByRole("button", { name: "Reload settings", exact: true })
    .click();
  await dialog.getByText("More configuration", { exact: true }).click();
  await expect(dialog.getByLabel("Description", { exact: true })).toHaveValue(
    "Updated from another session",
  );
  const detailUrl = `**${f.base}/mcp/connections/${connection.id}`;
  await page.route(detailUrl, (route) =>
    route.request().method() === "GET"
      ? route.fulfill({
          status: 503,
          json: {
            code: "controlled_outage",
            message: "Controlled metadata outage",
          },
        })
      : route.continue(),
  );
  await expect(dialog).toContainText("Controlled metadata outage");
  await expect(dialog.getByLabel(/^Target label/)).not.toBeVisible();
  await page.unroute(detailUrl);
  await dialog
    .getByRole("button", { name: "Reload connection", exact: true })
    .click();
  await expect(dialog.getByLabel(/^Target label/)).toHaveValue(
    "laboratory-target",
  );
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await f.panel.screenshot({
    path: "../.cache/mcp-catalogue-desktop.png",
    animations: "disabled",
  });
  const usage = await command(page, `${f.base}/models/usage`);
  expect(usage.total).toBe(0);
  expect(errors).toEqual([]);
});

test("delegated reader can manage or share independently and loses cached tools after Brain revocation", async ({
  page,
  browser,
}) => {
  test.setTimeout(150_000);
  page.setDefaultTimeout(12_000);
  const f = await setup(page, "Delegated MCP catalogue");
  const connection = await command(page, `${f.base}/mcp/connections`, {
    name: "Shared inspector",
    description: "Synthetic fixture",
    definition_key: "fixture",
    target: "synthetic",
    placement: "central",
    configuration: { region: "test" },
  });
  const profile = (
    await command(page, `${f.base}/mcp/profiles`, {
      name: "Shared tools",
      connection_ids: [connection.summary.id],
    })
  ).profile;
  const username = `mcp-reader-${crypto.randomUUID()}`;
  const invitation = await command(page, "/api/team/invitations", { username });
  const guest = await browser.newContext({
    viewport: { width: 1440, height: 1000 },
  });
  const reader = await guest.newPage();
  reader.setDefaultTimeout(12_000);
  const errors: string[] = [];
  reader.on("pageerror", (e) => errors.push(e.message));
  const enrolled = await reader.request.post("/api/auth/enroll", {
    data: { token: invitation.token, password: crypto.randomUUID() },
  });
  expect(enrolled.ok()).toBeTruthy();
  const account = (await enrolled.json()).user.id;
  await command(page, `${f.base}/grants/${account}`, { role: "reader" }, "PUT");
  await command(
    page,
    `${f.base}/mcp/profiles/${profile.id}/grants`,
    { username, rights: { use_profile: false, manage: true, share: false } },
    "PUT",
  );
  await reader.goto(`/brains/${f.brain.id}/connections?tab=profiles`);
  const panel = reader.getByRole("region", {
    name: "MCP connections and tool groups",
    exact: true,
  });
  const card = panel.getByTestId("mcp-profile");
  await expectRights(card, false, true, false);
  await expect(
    panel.getByRole("button", { name: "Add connection", exact: true }),
  ).not.toBeVisible();
  await expect(
    panel.getByRole("button", { name: "Edit connection", exact: true }),
  ).not.toBeVisible();
  await card
    .getByRole("button", { name: "Edit Shared tools", exact: true })
    .click();
  let dialog = card;
  await expect(
    card.getByRole("button", { name: "Add person", exact: true }),
  ).not.toBeVisible();
  await dialog
    .getByLabel("Tool group description", { exact: true })
    .fill("Managed by a Brain reader");
  await dialog
    .getByRole("button", { name: "Save changes", exact: true })
    .click();
  await expect(
    card.getByLabel("Tool group description", { exact: true }),
  ).not.toBeVisible();
  await expect(card).toContainText("Managed by a Brain reader");
  await expect(
    card.getByRole("button", { name: "Tools & testing", exact: true }),
  ).toBeDisabled();
  await command(
    page,
    `${f.base}/mcp/profiles/${profile.id}/grants`,
    { username, rights: { use_profile: false, manage: false, share: true } },
    "PUT",
  );
  await expectRights(card, false, false, true);
  await card
    .getByRole("button", { name: "Edit Shared tools", exact: true })
    .click();
  await expect(
    card.getByLabel("Tool group name", { exact: true }),
  ).not.toBeVisible();
  await card
    .getByRole("button", {
      name: `Use for ${username} direct: Denied`,
      exact: true,
    })
    .click();
  await card.getByRole("button", { name: "Save changes", exact: true }).click();
  await expect(
    card.getByRole("button", { name: "Edit Shared tools", exact: true }),
  ).toBeVisible();
  await expectRights(card, true, false, true);
  await card
    .getByRole("button", { name: "Tools & testing", exact: true })
    .click();
  dialog = reader.getByRole("dialog", {
    name: "Tools · Shared tools",
    exact: true,
  });
  await expect(dialog).toContainText("Shared inspector · inspect");
  await command(page, `${f.base}/grants/${account}`, undefined, "DELETE");
  await expect(dialog).not.toBeVisible();
  await expect(panel).not.toBeVisible();
  await expect(
    reader.getByText("Shared inspector · inspect", { exact: true }),
  ).not.toBeVisible();
  expect((await command(page, `${f.base}/mcp`)).profiles).toHaveLength(1);
  expect(errors).toEqual([]);
  await guest.close();
});

test("desktop registers a paired private runner, selects exact placement and handles stale edits", async ({
  page,
}) => {
  test.setTimeout(120_000);
  page.setDefaultTimeout(12_000);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const f = await setup(page, "Desktop private runners");
  await page.goto(`/brains/${f.brain.id}/connections?tab=runners`);
  const panel = page.getByRole("region", {
    name: "Private runners",
    exact: true,
  });
  await expect(panel).toContainText("No private runners yet");
  await panel.getByRole("button", { name: "Add runner", exact: true }).click();
  let dialog = page.getByRole("form", {
    name: "Add private runner",
    exact: true,
  });
  const existingDevices = await command(page, "/api/devices");
  if (
    !existingDevices.some(
      (d: {
        claimed: boolean;
        revoked_at: string | null;
        expires_at: string;
      }) => d.claimed && !d.revoked_at && Date.parse(d.expires_at) > Date.now(),
    )
  )
    await expect(dialog).toContainText(
      "Connect the Recollect plugin on an active device first.",
    );
  await expect(
    dialog.getByRole("button", { name: "Add runner", exact: true }),
  ).toBeDisabled();
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  const pairing = await command(page, "/api/devices/pairings", {
    name: "Desktop private host",
  });
  await command(page, `/api/devices/pairings/${pairing.user_code}/approve`, {
    approve: true,
  });
  const poll = await command(page, "/api/devices/pairings/poll", {
    device_code: pairing.device_code,
  });
  const device = poll.device.id;
  await command(page, "/api/devices/pairings/finish", {
    device_code: pairing.device_code,
  });
  await panel.getByRole("button", { name: "Add runner", exact: true }).click();
  dialog = page.getByRole("form", {
    name: "Add private runner",
    exact: true,
  });
  await dialog.getByLabel(/^Private runner name/).fill("Private lab runner");
  await dialog.getByLabel(/^Paired runner device/).click();
  await page
    .getByRole("option", { name: "Desktop private host", exact: true })
    .click();
  await dialog.getByRole("button", { name: "Add runner", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  const card = panel.getByTestId("mcp-private-runner");
  await expect(card).toContainText("Private lab runner");
  await expect(card).toContainText("Offline");
  await expect(card).toContainText("No assigned connections.");
  await expect(
    card.getByRole("button", { name: "Copy setup command", exact: true }),
  ).toBeVisible();
  const records = await command(page, `${f.base}/mcp/private-runners`);
  expect(records).toHaveLength(1);
  expect(records[0].device_id).toBe(device);
  await page.goto(`/brains/${f.brain.id}/connections`);
  await f.panel
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  const setupDialog = page.getByRole("dialog", {
    name: "Add connection",
    exact: true,
  });
  await setupDialog.getByText("Approved connector", { exact: true }).click();
  await setupDialog
    .getByRole("button", { name: "Choose connector", exact: true })
    .click();
  dialog = page.getByRole("dialog", { name: "Add connection", exact: true });
  await dialog.getByLabel(/^Connection name/).fill("Private inspector");
  await dialog.getByLabel(/^Approved connector/).click();
  await page
    .getByRole("option", { name: "Approved fixture", exact: true })
    .click();
  await dialog.getByLabel(/^Target label/).fill("private-lab");
  await dialog
    .getByLabel("Non-secret settings", { exact: true })
    .fill('{"region":"test"}');
  await dialog.getByLabel(/^Where this connection runs/).click();
  await page.getByRole("option", { name: /^Private-network runner/ }).click();
  await dialog.getByLabel(/^Private runner/).click();
  await page
    .getByRole("option", { name: "Private lab runner · Offline", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await expect(dialog).not.toBeVisible();
  const catalogue = await command(page, `${f.base}/mcp`);
  const connection = await command(
    page,
    `${f.base}/mcp/connections/${catalogue.connections[0].id}`,
  );
  expect(connection.runner_reference).toBe(`private:${records[0].id}`);
  expect(connection.credential_alias).toBeNull();
  await page.goto(`/brains/${f.brain.id}/connections?tab=runners`);
  await card
    .getByRole("button", { name: "Edit private runner", exact: true })
    .click();
  dialog = page.getByRole("form", {
    name: "Edit private runner",
    exact: true,
  });
  const renamed = await command(
    page,
    `${f.base}/mcp/private-runners/${records[0].id}`,
    {
      name: "Renamed elsewhere",
      device_id: device,
      enabled: true,
      base_revision: records[0].revision,
    },
    "PUT",
  );
  await expect(dialog).toContainText("This registration changed");
  await expect(
    dialog.getByRole("button", { name: "Save changes", exact: true }),
  ).toBeDisabled();
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await card
    .getByRole("button", { name: "Edit private runner", exact: true })
    .click();
  dialog = page.getByRole("form", {
    name: "Edit private runner",
    exact: true,
  });
  await dialog.getByLabel(/^Private runner name/).fill("Private lab runner");
  await dialog.getByLabel("Private runner enabled", { exact: true }).uncheck();
  await dialog
    .getByRole("button", { name: "Save changes", exact: true })
    .click();
  await expect(dialog).not.toBeVisible();
  await expect(card).toContainText("Disabled");
  const disabled = (await command(page, `${f.base}/mcp/private-runners`))[0];
  expect(disabled.revision).not.toBe(renamed.revision);
  expect(disabled.enabled).toBe(false);
  await panel.scrollIntoViewIfNeeded();
  await page.screenshot({ path: "../.cache/ui-mcp-private-desktop.png" });
  expect(errors).toEqual([]);
});
