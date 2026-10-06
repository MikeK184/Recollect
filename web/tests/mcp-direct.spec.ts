import { test, expect, type Page } from "@playwright/test";
import { createServer } from "node:http";
import { randomUUID } from "node:crypto";
import { parse as parseToml } from "smol-toml";
import {
  directAgentConfig,
  directAgentCredentialName,
} from "../src/features/agents/agentConfig";

test("native direct configuration round-trips headers and distinct issuance labels", () => {
  // Generated locally only: no credential value lives in this fixture/source/output.
  const token = randomUUID();
  const url = "https://memory.example.test/api/brains/example/mcp/agent";
  const codex = parseToml(directAgentConfig("codex", url, token)) as {
    mcp_servers: {
      recollect: { url: string; http_headers: { Authorization: string } };
    };
  };
  expect(codex.mcp_servers.recollect.url).toBe(url);
  expect(
    codex.mcp_servers.recollect.http_headers.Authorization ===
      `Bearer ${token}`,
  ).toBe(true);
  const claude = JSON.parse(directAgentConfig("claude", url, token)).mcpServers
    .recollect;
  const opencode = JSON.parse(directAgentConfig("opencode", url, token)).mcp
    .servers.recollect;
  for (const server of [claude, opencode]) {
    expect(server.headers.Authorization === `Bearer ${token}`).toBe(true);
    expect(server.url).toBe(url);
  }
  expect(claude.type).toBe("http");
  expect(opencode.type).toBe("remote");
  expect(opencode.oauth).toBe(false);
  const first = directAgentCredentialName("a".repeat(90), randomUUID());
  const second = directAgentCredentialName("a".repeat(90), randomUUID());
  expect(first.length).toBeLessThanOrEqual(120);
  expect(first === second).toBe(false);
});

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
  const previousPair = await api("/api/devices/pairings", {
    name: "Codex MCP · Direct MCP proof",
    host_kind: "codex",
    integration: "mcp",
  });
  await api(`/api/devices/pairings/${previousPair.user_code}/approve`, {
    approve: true,
  });
  const previous = await api("/api/devices/pairings/poll", {
    device_code: previousPair.device_code,
  });
  await api("/api/devices/pairings/finish", {
    device_code: previousPair.device_code,
  });
  await page.goto(`/brains/${brain.id}/connections?tab=coding-agents`);
  await expect(page).toHaveURL(new RegExp(`/brains/${brain.id}/agents$`));
  await page
    .getByRole("button", { name: "Connect agent", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Connect a coding agent",
    exact: true,
  });
  let pairings = 0;
  page.on("request", (request) => {
    if (
      request.method() === "POST" &&
      request.url().endsWith("/api/devices/pairings")
    )
      pairings++;
  });
  await expect(dialog.getByTestId("agent-setup-stage-1")).toBeVisible();
  await expect(dialog.getByTestId("agent-setup-stage-2")).toHaveCount(0);
  await dialog.getByRole("button", { name: "Next", exact: true }).click();
  await expect(
    dialog.getByRole("heading", { name: "Install the plugin", exact: true }),
  ).toBeVisible();
  await dialog
    .getByLabel("Plugin package folder", { exact: true })
    .fill("/tmp/recollect-plugin");
  await expect(dialog.getByTestId("agent-plugin-command")).toContainText(
    "codex plugin marketplace add",
  );
  await page.screenshot({
    path: "../.cache/guided-setup-install.png",
    animations: "disabled",
  });
  await dialog.getByRole("button", { name: "Back", exact: true }).click();
  await dialog.getByRole("button", { name: /^Direct MCP/ }).click();
  await dialog.getByRole("button", { name: "Next", exact: true }).click();
  await expect(
    dialog.getByRole("button", { name: "Next", exact: true }),
  ).toBeDisabled();
  expect(pairings).toBe(0);
  await expect(dialog.getByLabel("Token name", { exact: true })).toHaveValue(
    "Codex MCP · Direct MCP proof",
  );
  await dialog
    .getByRole("button", { name: "Create access token", exact: true })
    .click();
  const field = dialog.getByLabel("Access token · shown once", { exact: true });
  await expect(field).toBeVisible();
  const token = await field.inputValue();
  expect(token.length).toBe(36);
  await dialog.getByRole("button", { name: "Next", exact: true }).click();
  await expect(dialog.getByTestId("agent-setup-command")).toContainText(
    "http_headers = { Authorization =",
  );
  expect(
    (await dialog.getByTestId("agent-setup-command").innerText()).includes(
      token,
    ),
  ).toBe(false);
  await expect(dialog.getByTestId("agent-credential-command")).toHaveCount(0);
  await page.evaluate(() => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: async (value: string) => {
          (window as unknown as { copiedConfig: string }).copiedConfig = value;
        },
      },
    });
  });
  await dialog
    .getByRole("button", { name: "Copy MCP configuration", exact: true })
    .click();
  const copied = await page.evaluate(
    () => (window as unknown as { copiedConfig: string }).copiedConfig,
  );
  const parsed = parseToml(copied) as {
    mcp_servers: { recollect: { http_headers: { Authorization: string } } };
  };
  expect(
    parsed.mcp_servers.recollect.http_headers.Authorization ===
      `Bearer ${token}`,
  ).toBe(true);
  expect(copied.includes("RECOLLECT_MCP_TOKEN")).toBe(false);
  await dialog
    .getByRole("button", { name: "Reveal token in configuration", exact: true })
    .click();
  expect(
    (await dialog.getByTestId("agent-setup-command").innerText()).includes(
      token,
    ),
  ).toBe(true);
  await dialog
    .getByRole("button", { name: "Hide token in configuration", exact: true })
    .click();
  await dialog.getByRole("button", { name: "Back", exact: true }).click();
  expect((await field.inputValue()) === token).toBe(true);
  await dialog.getByRole("button", { name: "Back", exact: true }).click();
  await expect(
    dialog.getByRole("button", { name: /^Direct MCP/ }),
  ).toBeDisabled();
  await dialog.getByRole("button", { name: "Next", exact: true }).click();
  await dialog.getByRole("button", { name: "Next", exact: true }).click();
  expect(pairings).toBe(1);
  const configuration = await dialog
    .getByTestId("agent-setup-command")
    .innerText();
  expect(configuration.includes("http_headers =")).toBe(true);
  expect(configuration.includes(token)).toBe(false);
  expect(configuration.includes("bearer_token_env_var")).toBe(false);
  const previousRead = await page.request.post(base + "/mcp/agent", {
    headers: {
      Authorization: `Bearer ${previous.token}`,
      Accept: "application/json, text/event-stream",
      "MCP-Protocol-Version": "2025-11-25",
    },
    data: { jsonrpc: "2.0", id: 1, method: "tools/list", params: {} },
  });
  expect(previousRead.status()).toBe(200);
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
  await dialog.getByRole("button", { name: "Next", exact: true }).click();
  const activityLink = dialog.getByRole("link", {
    name: "View recorded activity",
    exact: true,
  });
  await expect(activityLink).toBeVisible();
  await page.screenshot({
    path: "../.cache/desktop-final-setup.png",
    animations: "disabled",
  });
  await activityLink.click();
  await expect(page).toHaveURL(
    new RegExp(`/brains/${brain.id}/activity[?]tab=capture$`),
  );
  await page.goto(`/brains/${brain.id}/agents`);
  await page
    .getByRole("button", { name: "Connect agent", exact: true })
    .click();
  await expect(
    page.getByLabel("Access token · shown once", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByLabel("Keychain secret · shown once", { exact: true }),
  ).toHaveCount(0);
  const devices = await api("/api/devices");
  const device = devices.find((d: { name: string }) =>
    d.name.startsWith("Codex MCP · Direct MCP proof · "),
  );
  expect(Boolean(device?.claimed)).toBe(true);
  expect(device.host_kind).toBe("codex");
  expect(device.integration).toBe("mcp");
  expect("token" in device).toBe(false);
  await api(`/api/devices/${device.id}`, undefined, "DELETE");
  expect((await rpc("tools/list")).status()).toBe(401);
  const previousDevice = devices.find(
    (d: { name: string }) => d.name === "Codex MCP · Direct MCP proof",
  );
  await api(`/api/devices/${previousDevice.id}`, undefined, "DELETE");
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
      name: "Add connection",
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
    await page.route("**/api/mcp/definitions/inspect-http", (route) =>
      route.fulfill({
        status: 502,
        contentType: "application/json",
        body: JSON.stringify({
          error: "provider_unavailable",
          message: "Controlled discovery failure",
        }),
      }),
    );
    await dialog
      .getByRole("button", { name: "Find tools", exact: true })
      .click();
    await expect(
      dialog.getByRole("button", { name: "Add server", exact: true }),
    ).toBeDisabled();
    await expect(
      dialog.getByText("1 tools found", { exact: true }),
    ).toHaveCount(0);
    await expect(
      dialog.getByText("Controlled discovery failure", { exact: true }),
    ).toBeVisible();
    await page.unroute("**/api/mcp/definitions/inspect-http");
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
    const card = page
      .getByTestId("mcp-connection")
      .filter({ hasText: "Renamed HTTP" });
    await card
      .getByRole("button", { name: "Test connection", exact: true })
      .click();
    const check = page.getByRole("dialog", {
      name: "Test connection · Renamed HTTP",
      exact: true,
    });
    await check
      .getByRole("button", { name: "Check server", exact: true })
      .click();
    await expect(
      check.getByText("Server responded · 1 tools listed", { exact: true }),
    ).toBeVisible();
    expect(calls).toBe(0);
    // The real handshake is not recorded as a successful tool call.
    const checked = await api(base + "/mcp");
    expect(checked.connections[0].last_successful_call_at ?? null).toBeNull();
    await check.screenshot({
      path: "../.cache/actionable-connection-test.png",
      animations: "disabled",
    });
    await page.route("**/api/mcp/definitions/inspect-http", (route) =>
      route.fulfill({
        status: 502,
        contentType: "application/json",
        body: JSON.stringify({
          error: "provider_unavailable",
          message: "Controlled test failure",
        }),
      }),
    );
    await check
      .getByRole("button", { name: "Check server", exact: true })
      .click();
    await expect(
      check.getByText("Server check failed", { exact: true }),
    ).toBeVisible();
    await expect(
      check.getByText("Server responded · 1 tools listed", { exact: true }),
    ).toHaveCount(0);
    await page.unroute("**/api/mcp/definitions/inspect-http");
    await page.keyboard.press("Escape");
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
    name: "Add connection",
    exact: true,
  });
  await dialog.getByLabel("Server name", { exact: true }).fill("Context7");
  await dialog
    .getByLabel("MCP server URL", { exact: true })
    .fill("https://mcp.context7.com/mcp");
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
