import { test, expect, type Page } from "@playwright/test";
import { createServer } from "node:http";
import { randomUUID } from "node:crypto";

async function setup(page: Page, name: string) {
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
  const csrf = (await (await page.request.get("/api/auth/me")).json())
    .csrf_token;
  const api = async (
    path: string,
    body?: unknown,
    method = body === undefined ? "GET" : "POST",
  ) => {
    const response = await page.request.fetch(path, {
      method,
      data: body,
      headers: { "x-csrf-token": csrf, "Idempotency-Key": randomUUID() },
    });
    expect(
      response.ok(),
      `${method} ${path}: ${response.status()}`,
    ).toBeTruthy();
    return response.status() === 204 ? null : response.json();
  };
  const brain = await api("/api/brains", { name });
  return { brain, base: `/api/brains/${brain.id}`, api };
}

test("direct browser token works over ordinary HTTP MCP and revokes", async ({
  page,
}) => {
  const { brain, base, api } = await setup(page, "Direct MCP proof");
  await page.goto(`/brains/${brain.id}/connections?tab=coding-agents`);
  await expect(page).toHaveURL(
    new RegExp(`/brains/${brain.id}/agents\\?tab=setup$`),
  );
  await page
    .getByRole("button", { name: "Connect coding agent", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toContainText(
    "Install the Recollect plugin",
  );
  await expect(page.getByRole("dialog")).toContainText(
    "Optional execution runner",
  );
  for (const [width, height] of [
    [1280, 800],
    [1440, 900],
    [1920, 1080],
  ]) {
    await page.setViewportSize({ width, height });
    const setup = page.getByRole("dialog");
    await setup
      .getByText("Step 1 · Install the Recollect plugin", { exact: true })
      .scrollIntoViewIfNeeded();
    expect(
      await setup.evaluate(
        (element) => element.scrollWidth <= element.clientWidth + 1,
      ),
    ).toBe(true);
    await page.screenshot({
      path: `../.cache/ui/plugin-setup-${width}-top.png`,
    });
    await setup
      .getByText("Optional execution runner", { exact: true })
      .scrollIntoViewIfNeeded();
    await page.screenshot({
      path: `../.cache/ui/plugin-setup-${width}-bottom.png`,
    });
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  await page
    .getByText("Advanced · Direct MCP connection", { exact: true })
    .click();
  await page
    .getByRole("button", { name: "Set up direct MCP", exact: true })
    .click();
  const dialog = page.getByRole("dialog").last();
  await dialog.getByLabel("Store your access token", { exact: true }).click();
  await page
    .getByRole("option", { name: "Environment variable", exact: true })
    .click();
  await expect(dialog.getByTestId("agent-setup-command")).toContainText(
    'bearer_token_env_var = "RECOLLECT_MCP_TOKEN"',
  );
  await expect(dialog.getByTestId("agent-credential-command")).toContainText(
    "read -r -s RECOLLECT_MCP_TOKEN",
  );
  await expect(dialog.getByText(/It is a variable name/)).toBeVisible();
  await expect(dialog.getByText(/recollect-agent pair/)).not.toBeVisible();
  await page.screenshot({
    path: "../.cache/ui/direct-mcp.png",
    fullPage: true,
  });
  await dialog
    .getByRole("button", { name: "Create access token", exact: true })
    .click();
  const field = dialog.getByLabel("Access token · shown once", { exact: true });
  await expect(field).toBeVisible();
  const token = await field.inputValue();
  expect(token.length).toBe(36);
  await dialog.getByLabel("Store your access token", { exact: true }).click();
  await page
    .getByRole("option", { name: "macOS Keychain", exact: true })
    .click();
  const secret = await dialog
    .getByLabel("Keychain secret · shown once", { exact: true })
    .inputValue();
  // Never expose the credential in assertion diagnostics.
  expect(JSON.parse(secret).Authorization === `Bearer ${token}`).toBe(true);
  const configuration = await dialog
    .getByTestId("agent-setup-command")
    .innerText();
  expect(configuration.includes("http_headers_helper")).toBe(true);
  expect(configuration.includes("find-generic-password")).toBe(true);
  expect(configuration.includes(token)).toBe(false);
  expect(configuration.includes("bearer_token_env_var")).toBe(false);
  const credentialCommand = await dialog
    .getByTestId("agent-credential-command")
    .innerText();
  expect(credentialCommand.endsWith(" -w")).toBe(true);
  expect(credentialCommand.includes(token)).toBe(false);
  const headers = {
    Authorization: `Bearer ${token}`,
    Accept: "application/json, text/event-stream",
    "MCP-Protocol-Version": "2025-11-25",
  };
  const rpc = async (method: string, params: unknown = {}) =>
    page.request.post(base + "/mcp/agent", {
      headers,
      data: { jsonrpc: "2.0", id: 1, method, params },
    });
  const initialized = await rpc("initialize", {
    protocolVersion: "2025-11-25",
    capabilities: {},
    clientInfo: { name: "ordinary-http-test", version: "1" },
  });
  expect(initialized.status()).toBe(200);
  const list = await rpc("tools/list");
  expect(list.status()).toBe(200);
  expect(
    (await list.json()).result.tools.some(
      (t: { name: string }) => t.name === "workspace.list",
    ),
  ).toBe(true);
  const workspace = await rpc("tools/call", {
    name: "workspace.list",
    arguments: {},
  });
  expect(workspace.status()).toBe(200);
  expect((await workspace.json()).result.isError ?? false).toBe(false);
  const source = await api(base + "/sources", {
    title: "Orion MCP evidence",
    content: "Orion listens on port 8088.",
    media_type: "text/plain",
    retain_content: true,
  });
  await expect
    .poll(
      async () =>
        (
          await api(
            `${base}/sources/${source.id}/versions/${source.version.id}`,
          )
        ).version.processing,
    )
    .toBe("ready");
  const taskReply = await rpc("tools/call", {
    name: "workspace.start_task",
    arguments: {
      input: {
        label: "Direct MCP recall",
        parent_task_id: null,
        workspace_id: null,
        selection: null,
      },
      context_query: "Orion",
    },
  });
  const taskResult = (await taskReply.json()).result;
  expect(taskResult.isError ?? false).toBe(false);
  const operation = taskResult.structuredContent.context.operation_id;
  const recalled = (
    await (
      await rpc("tools/call", {
        name: "memory.recall",
        arguments: { operation_id: operation, input: { query: "Orion" } },
      })
    ).json()
  ).result;
  expect(recalled.isError ?? false).toBe(false);
  expect(
    recalled.structuredContent.context.items.some(
      (i: { id: string }) => i.id === source.version.id,
    ),
  ).toBe(true);
  const forbidden = await page.request.post(
    "/api/mcp/definitions/inspect-http",
    {
      headers,
      data: { name: "forbidden", url: "https://mcp.context7.com/mcp" },
    },
  );
  expect(forbidden.status()).toBe(403);
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Connect coding agent", exact: true })
    .click();
  await expect(
    page.getByLabel("Access token · shown once", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByLabel("Keychain secret · shown once", { exact: true }),
  ).toHaveCount(0);
  const devices = await api("/api/devices");
  const device = devices.find(
    (d: { name: string }) => d.name === "Codex MCP · Direct MCP proof",
  );
  expect(Boolean(device?.claimed)).toBe(true);
  await api(`/api/devices/${device.id}`, undefined, "DELETE");
  expect((await rpc("tools/list")).status()).toBe(401);
});

test("name and URL form discovers without calling tools, invalidates edits and saves", async ({
  page,
}) => {
  let calls = 0;
  const fixture = createServer(async (req, res) => {
    if (req.method !== "POST") {
      res.writeHead(405);
      res.end();
      return;
    }
    let raw = "";
    for await (const part of req) raw += part;
    const request = JSON.parse(raw);
    if (!("id" in request)) {
      res.writeHead(202);
      res.end();
      return;
    }
    let result;
    if (request.method === "initialize")
      result = {
        protocolVersion: "2025-11-25",
        capabilities: { tools: {} },
        serverInfo: { name: "fixture", version: "1" },
      };
    if (request.method === "tools/list")
      result = {
        tools: [
          {
            name: "lookup",
            description:
              "Fixture lookup\nwith ordinary multiline documentation",
            inputSchema: {
              type: "object",
              properties: {},
              additionalProperties: false,
            },
          },
        ],
      };
    if (request.method === "tools/call") calls++;
    res.setHeader("Content-Type", "application/json");
    res.end(
      JSON.stringify({
        jsonrpc: "2.0",
        id: request.id,
        ...(result
          ? { result }
          : { error: { code: -32601, message: "Unsupported" } }),
      }),
    );
  });
  await new Promise<void>((resolve) => fixture.listen(0, "127.0.0.1", resolve));
  try {
    const address = fixture.address() as { port: number };
    const { brain, base, api } = await setup(page, "MCP form proof");
    await page.goto(`/brains/${brain.id}/connections?tab=connections`);
    await page
      .getByRole("button", { name: "Add connection", exact: true })
      .click();
    const dialog = page.getByRole("dialog", {
      name: "Add an MCP connector",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Server name", exact: true })
      .fill("Fixture HTTP");
    await dialog
      .getByRole("textbox", { name: "MCP server URL", exact: true })
      .fill(`http://127.0.0.1:${address.port}/mcp`);
    await dialog
      .getByRole("button", { name: "Find tools", exact: true })
      .click();
    await expect(
      dialog.getByText("1 tools found", { exact: true }),
    ).toBeVisible({ timeout: 30000 });
    expect(calls).toBe(0);
    await dialog
      .getByRole("textbox", { name: "Server name", exact: true })
      .fill("Renamed HTTP");
    await expect(
      dialog.getByRole("button", { name: "Add server", exact: true }),
    ).toBeDisabled();
    await dialog
      .getByRole("button", { name: "Find tools", exact: true })
      .click();
    await expect(
      dialog.getByRole("button", { name: "Add server", exact: true }),
    ).toBeEnabled();
    await dialog
      .getByRole("button", { name: "Add server", exact: true })
      .click();
    await expect(dialog).toHaveCount(0);
    const catalogue = await api(base + "/mcp");
    expect(
      catalogue.connections.some(
        (c: { name: string }) => c.name === "Renamed HTTP",
      ),
    ).toBe(true);
    expect(catalogue.profiles).toHaveLength(0);
    expect(calls).toBe(0);
  } finally {
    await new Promise<void>((resolve) => fixture.close(() => resolve()));
  }
});

test("Context7 anonymous documentation probe through Recollect", async ({
  page,
}) => {
  test.skip(
    process.env.RECOLLECT_CONTEXT7_PROOF !== "1",
    "Explicit external proof only",
  );
  test.setTimeout(120000);
  const { brain, base, api } = await setup(page, "Anonymous Context7 proof");
  await page.goto(`/brains/${brain.id}/connections?tab=connections`);
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Add an MCP connector",
    exact: true,
  });
  await dialog
    .getByRole("button", { name: "Use Context7 · no API key", exact: true })
    .click();
  const inspected = page.waitForResponse((r) =>
    r.url().endsWith("/api/mcp/definitions/inspect-http"),
  );
  await dialog.getByRole("button", { name: "Find tools", exact: true }).click();
  const inspection = await inspected;
  expect(inspection.status(), JSON.stringify(await inspection.json())).toBe(
    200,
  );
  await expect(
    dialog.getByRole("button", { name: "Add server", exact: true }),
  ).toBeEnabled({ timeout: 35000 });
  await page.screenshot({
    path: "../.cache/ui/context7-discovered.png",
    fullPage: true,
  });
  await dialog.getByRole("button", { name: "Add server", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  const catalogue = await api(base + "/mcp");
  const connection = catalogue.connections.find(
    (c: { name: string }) => c.name === "Context7",
  );
  const profile = (
    await api(base + "/mcp/profiles", {
      name: "Documentation",
      description: "Public anonymous Context7 proof",
      environment_id: null,
      connection_ids: [connection.id],
      enabled: true,
      base_revision: null,
    })
  ).profile;
  await api(
    `${base}/mcp/profiles/${profile.id}/grants`,
    {
      username: process.env.RECOLLECT_OWNER_USERNAME,
      rights: { use_profile: true, manage: true, share: true },
    },
    "PUT",
  );
  const pairing = await api("/api/devices/pairings", {
    name: "Context7 MCP host proof",
  });
  await api(`/api/devices/pairings/${pairing.user_code}/approve`, {
    approve: true,
  });
  const issued = await api("/api/devices/pairings/poll", {
    device_code: pairing.device_code,
  });
  await api("/api/devices/pairings/finish", {
    device_code: pairing.device_code,
  });
  // Use the host-facing MCP transport, not the browser's managed-call endpoint.
  const rpc = async (method: string, params: unknown) => {
    const response = await page.request.post(base + "/mcp/agent", {
      headers: {
        Authorization: `Bearer ${issued.token}`,
        Accept: "application/json, text/event-stream",
        "MCP-Protocol-Version": "2025-11-25",
      },
      data: { jsonrpc: "2.0", id: 1, method, params },
    });
    expect(response.status()).toBe(200);
    const reply = await response.json();
    expect(reply.error).toBeUndefined();
    expect(reply.result.isError ?? false).toBe(false);
    return reply.result;
  };
  await rpc("initialize", {
    protocolVersion: "2025-11-25",
    capabilities: {},
    clientInfo: { name: "context7-through-recollect-proof", version: "1" },
  });
  const tool = async (name: string, args: unknown) =>
    (await rpc("tools/call", { name, arguments: args })).structuredContent;
  const task = await tool("workspace.start_task", {
    input: {
      label: "Public documentation lookup",
      selection: { repository_ids: [], area_ids: [], environment_id: null },
    },
    context_query: "React useState documentation",
  });
  const operation = await tool("workspace.begin", {
    id: task.task.id,
    input: { kind: "tool" },
  });
  const run = await tool("mcp.call", {
    operation_id: operation.id,
    input: {
      request_id: randomUUID(),
      profile_id: profile.id,
      connection_id: connection.id,
      tool_name: "resolve-library-id",
      arguments: {
        libraryName: "react",
        query: "React useState documentation",
      },
      client_session_id: randomUUID(),
      timeout_seconds: 60,
    },
  });
  const callId = run.call?.id ?? run.id;
  let complete;
  await expect
    .poll(
      async () => {
        complete = await tool("mcp.status", {
          operation_id: operation.id,
          id: callId,
        });
        return complete.call?.state ?? complete.state;
      },
      { timeout: 85000, intervals: [500, 1000] },
    )
    .toBe("succeeded");
  const result = complete.call?.result ?? complete.result;
  const content = JSON.stringify(result).toLowerCase();
  const quotaLimited = content.includes("quota exceeded");
  expect(
    content.includes("react") || quotaLimited,
    "Expected documentation or an explicit provider quota response",
  ).toBe(true);
  console.log(
    quotaLimited
      ? "Context7: anonymous tool discovery and protocol call succeeded; documentation unavailable because the provider reports monthly quota exceeded."
      : "Context7: anonymous discovery and React documentation lookup succeeded through Recollect.",
  );
});
