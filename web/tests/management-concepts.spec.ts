import { createServer } from "node:http";
import { test, expect, type Page } from "@playwright/test";
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
    method + " " + path + ": " + response.status(),
  ).toBeTruthy();
  return response.json();
}
test("approved library, inert config import, masked credentials and stable privacy editing", async ({
  page,
}) => {
  test.setTimeout(120000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
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
  const brain = await command(page, "/api/brains", {
    name: "Management concept proof",
  });
  const manifest = {
    key: "management-" + crypto.randomUUID(),
    name: "Management proof connector",
    description: "Synthetic inert command",
    transport: "stdio",
    command: "/usr/bin/false",
    arguments: [],
    placements: ["central"],
    credential_aliases: ["authentication"],
    configuration_schema: { type: "object", additionalProperties: false },
    tools: [
      {
        name: "inspect",
        description: "Synthetic proof",
        inputSchema: { type: "object", additionalProperties: false },
        annotations: { readOnlyHint: true },
      },
    ],
  };
  await page.goto("/connectors");
  await page
    .getByRole("button", { name: "Add connector", exact: true })
    .click();
  let dialog = page.getByRole("dialog", {
    name: "Add a connector",
    exact: true,
  });
  await dialog.getByText("Paste config", { exact: true }).click();
  await dialog
    .getByRole("textbox", { name: "MCP configuration", exact: true })
    .fill(JSON.stringify(manifest));
  await dialog
    .getByRole("button", { name: "Parse configuration", exact: true })
    .click();
  await expect(
    dialog.getByRole("heading", { name: "Review connector", exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Review connector", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "Approve connector", exact: true })
    .click();
  await expect(dialog).not.toBeVisible();
  await expect(
    page.getByRole("heading", { name: manifest.name, exact: true }),
  ).toBeVisible();
  await page.goto(`/brains/${brain.id}/connections`);
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  dialog = page.getByRole("dialog", { name: "Add connection", exact: true });
  await dialog.getByText("Paste config", { exact: true }).click();
  await dialog
    .getByRole("textbox", { name: "MCP configuration", exact: true })
    .fill(
      "mcpServers:\n  Management proof:\n    command: /usr/bin/false\n    env:\n      CONNECTOR_API_KEY: synthetic-management-ui-value",
    );
  await dialog
    .getByRole("button", { name: "Parse configuration", exact: true })
    .click();
  await expect(
    dialog.getByRole("textbox", { name: "MCP configuration", exact: true }),
  ).not.toHaveValue(/synthetic-management-ui-value/);
  await expect(
    dialog.getByLabel("CONNECTOR_API_KEY", { exact: true }),
  ).toHaveAttribute("type", "password");
  await dialog
    .getByRole("button", { name: "Choose approved connector", exact: true })
    .click();
  dialog = page.getByRole("dialog", { name: "Add connection", exact: true });
  await dialog.getByLabel(/^Approved connector/).click();
  await page.getByRole("option", { name: manifest.name, exact: true }).click();
  await dialog.getByLabel(/^Credential alias/).click();
  await page
    .getByRole("option", { name: "authentication", exact: true })
    .click();
  let credentialAttempts = 0;
  await page.route(
    /\/api\/brains\/[^/]+\/mcp\/connections\/[^/]+\/credentials$/,
    async (route) => {
      credentialAttempts++;
      if (credentialAttempts === 1)
        await route.fulfill({
          status: 503,
          json: {
            message:
              "Owned credential provider fixture is temporarily unavailable.",
          },
        });
      else await route.continue();
    },
  );
  await dialog
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await expect(dialog).toContainText("temporarily unavailable");
  expect(
    (await command(page, `/api/brains/${brain.id}/mcp`)).connections,
  ).toHaveLength(1);
  await dialog
    .getByLabel(/^Connection name/)
    .fill("Changed after partial save");
  await dialog
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await expect(dialog).toContainText("already saved");
  expect(credentialAttempts).toBe(1);
  await dialog.getByLabel(/^Connection name/).fill("Management proof");
  await dialog
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await expect(dialog).not.toBeVisible();
  expect(credentialAttempts).toBe(2);

  const catalogue = await command(page, `/api/brains/${brain.id}/mcp`);
  expect(catalogue.connections).toHaveLength(1);
  expect(JSON.stringify(catalogue)).not.toContain(
    "synthetic-management-ui-value",
  );
  const calls = await command(page, `/api/brains/${brain.id}/mcp/calls`);
  expect(calls.items ?? calls.calls).toHaveLength(0);
  await expect(
    page.getByRole("complementary", { name: "Connection details" }),
  ).toContainText("Configured");
  const inspector = page.getByRole("complementary", { name: "Connection details", exact: true });
  await inspector.getByRole("button", { name: "Connection actions", exact: true }).click();
  await page.getByRole("menuitem", { name: "View approved tools", exact: true }).click();
  await expect(inspector.getByRole("tab", { name: "Tools", exact: true })).toHaveAttribute("aria-selected", "true");
  await inspector.getByRole("tab", { name: "Overview", exact: true }).click();
  await inspector.getByRole("button", { name: "Close connection details", exact: true }).click();
  await expect(inspector).toHaveCount(0);
  await page.getByRole("button", { name: "View Management proof", exact: true }).click();
  await expect(inspector).toBeVisible();
  await page.getByRole("tab", { name: "Private Runners", exact: true }).click();
  await expect(page).toHaveURL(/tab=runners/);
  await page.getByRole("tab", { name: "Connections", exact: true }).click();
  await page.goto(`/brains/${brain.id}/settings?tab=privacy`);
  const row = page
    .locator(".privacy-policy-row")
    .filter({ has: page.getByText("Raw sessions", { exact: true }) });
  await expect(row).toBeVisible();
  const before = await row.boundingBox();
  const policy = await command(page, `/api/brains/${brain.id}/retention`);
  await page
    .getByRole("button", { name: "Edit privacy settings", exact: true })
    .click();
  await expect(
    page.getByLabel("Raw sessions · days", { exact: true }),
  ).toBeVisible();
  const after = await row.boundingBox();
  expect(Math.abs(before!.x - after!.x)).toBeLessThan(2);
  expect(Math.abs(before!.y - after!.y)).toBeLessThan(2);
  await page.getByLabel("Raw sessions · days", { exact: true }).fill("21");
  await page
    .getByRole("form", { name: "Privacy settings", exact: true })
    .getByRole("button", { name: "Cancel", exact: true })
    .click();
  expect(
    (await command(page, `/api/brains/${brain.id}/retention`)).change_id,
  ).toBe(policy.change_id);
  const reads: string[] = [];
  page.on("request", (request) => {
    if (
      request.url().includes("/semantic") ||
      request.url().includes("models/policy/history")
    )
      reads.push(request.url());
  });
  await page.goto(`/brains/${brain.id}/settings?tab=ai`);
  await expect(
    page.getByRole("heading", { name: "AI permissions", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("complementary", { name: "Semantic search coverage" }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "Automatic processing", exact: true }),
  ).toBeVisible();
  expect(reads).toEqual([]);
  await page
    .getByRole("button", { name: "Diagnostics", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", {
      name: "Models & search diagnostics",
      exact: true,
    }),
  ).toBeVisible();
  await expect
    .poll(() => reads.some((url) => !url.includes("summary=true")))
    .toBe(true);
  await page.keyboard.press("Escape");
  await page.goto(`/brains/${brain.id}/agents`);
  await expect(
    page.getByRole("heading", { name: "Connect your coding tool", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Copy memory-read check", exact: true }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Connectors", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Connectors", exact: true })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Brain navigation", exact: true })
    .getByRole("link", { name: "Settings", exact: true })).toHaveAttribute("href", `/brains/${brain.id}/settings`);
  await page.reload();
  await expect(page.getByRole("heading", { name: "Connectors", exact: true })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Brain navigation", exact: true })).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("explicit authenticated HTTP inspection redacts reflections and saves without calling a tool", async ({
  page,
}) => {
  const secret = "synthetic-authenticated-inspection-value";
  const methods: string[] = [];
  const fixture = createServer(async (req, res) => {
    if (req.method !== "POST") {
      res.writeHead(405);
      res.end();
      return;
    }
    if (req.headers.authorization !== "Bearer " + secret) {
      res.writeHead(403);
      res.end();
      return;
    }
    let raw = "";
    for await (const part of req) raw += part;
    const request = JSON.parse(raw);
    methods.push(request.method);
    if (!("id" in request)) {
      res.writeHead(202);
      res.end();
      return;
    }
    const value =
      request.method === "initialize"
        ? {
            protocolVersion: "2025-11-25",
            capabilities: { tools: {} },
            serverInfo: { name: "owned-fixture", version: "1" },
          }
        : request.method === "tools/list"
          ? {
              tools: [
                {
                  name: "lookup",
                  description: "Synthetic metadata reflects " + secret,
                  inputSchema: { type: "object", additionalProperties: false },
                },
              ],
            }
          : null;
    res.setHeader("Content-Type", "application/json");
    res.end(
      JSON.stringify({
        jsonrpc: "2.0",
        id: request.id,
        ...(value
          ? { result: value }
          : { error: { code: -32601, message: "Unsupported" } }),
      }),
    );
  });
  await new Promise<void>((resolve) => fixture.listen(0, "127.0.0.1", resolve));
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
      page.getByRole("heading", { name: "Your Brains", exact: true }),
    ).toBeVisible();
    const brain = await command(page, "/api/brains", {
      name: "Authenticated import proof",
    });
    const port = (fixture.address() as { port: number }).port;
    await page.goto(`/brains/${brain.id}/connections`);
    await page
      .getByRole("button", { name: "Add connection", exact: true })
      .click();
    const dialog = page.getByRole("dialog", {
      name: "Add connection",
      exact: true,
    });
    await dialog.getByLabel(/^Server name/).fill("Authenticated fixture");
    await dialog
      .getByLabel(/^MCP server URL/)
      .fill(`http://127.0.0.1:${port}/mcp`);
    await dialog.getByText("Secret", { exact: true }).click();
    await dialog.getByLabel("Bearer token", { exact: true }).fill(secret);
    expect(methods).toHaveLength(0);
    const inspected = page.waitForResponse((response) =>
      response.url().endsWith("/api/mcp/definitions/inspect-http"),
    );
    await dialog
      .getByRole("button", { name: "Inspect tools", exact: true })
      .click();
    const response = await inspected;
    expect(response.ok()).toBeTruthy();
    expect((await response.text()).includes(secret)).toBe(false);
    expect((await response.json()).tools[0].description).toContain(
      "[redacted]",
    );
    await expect(
      dialog.getByRole("heading", { name: "Review connector", exact: true }),
    ).toBeVisible();
    await dialog
      .getByRole("button", { name: "Review connection", exact: true })
      .click();
    await dialog
      .getByRole("button", { name: "Add connection", exact: true })
      .click();
    await expect(dialog).not.toBeVisible();
    expect(methods).toContain("tools/list");
    expect(methods).not.toContain("tools/call");
    const catalogue = await command(page, `/api/brains/${brain.id}/mcp`);
    expect(catalogue.connections).toHaveLength(1);
    expect(JSON.stringify(catalogue)).not.toContain(secret);
    expect(catalogue.profiles).toHaveLength(0);
  } finally {
    await new Promise<void>((resolve, reject) =>
      fixture.close((error) => (error ? reject(error) : resolve())),
    );
  }
});
