import { test, expect } from "@playwright/test";
import {
  parseMcpConfig,
  safeConfigPreview,
} from "../src/features/connections/mcpConfig";

test("JSON, TOML and YAML imports extract secrets into masked credential drafts", () => {
  const fixtures = [
    JSON.stringify({
      mcpServers: {
        docs: {
          url: "https://example.com/mcp",
          headers: { Authorization: "Bearer synthetic-value" },
        },
      },
    }),
    '[mcp_servers.docs]\nurl = "https://example.com/mcp"\n[mcp_servers.docs.http_headers]\nAuthorization = "Bearer synthetic-value"',
    "mcpServers:\n  docs:\n    url: https://example.com/mcp\n    headers:\n      Authorization: Bearer synthetic-value",
  ];
  for (const source of fixtures) {
    const drafts = parseMcpConfig(source);
    expect(drafts).toHaveLength(1);
    expect(drafts[0].target).toBe("https://example.com/mcp");
    expect(drafts[0].secrets[0].value).toBe("synthetic-value");
    expect(safeConfigPreview(drafts[0])).not.toContain("synthetic-value");
    expect(safeConfigPreview(drafts[0])).toContain("${AUTHORIZATION}");
  }
});
test("multi-server imports keep command and environment references inert", () => {
  const drafts = parseMcpConfig(
    JSON.stringify({
      mcpServers: {
        remote: {
          url: "https://example.com/mcp",
          headers: { Authorization: "Bearer ${DOCS_TOKEN}" },
        },
        local: {
          command: "/usr/bin/false",
          args: ["serve"],
          env: { CONNECTOR_API_KEY: "synthetic-local-value" },
        },
      },
    }),
  );
  expect(drafts).toHaveLength(2);
  expect(drafts[0].secrets[0]).toMatchObject({
    variable: "DOCS_TOKEN",
    value: "",
  });
  expect(drafts[1]).toMatchObject({
    transport: "stdio",
    command: "/usr/bin/false",
    arguments: ["serve"],
  });
  expect(safeConfigPreview(drafts[1])).not.toContain("synthetic-local-value");
});
test("ambiguous transport, routing headers, shell destinations, unsafe YAML and deep payloads fail without reflecting secrets", () => {
  const secret = "synthetic-sensitive-text";
  const cases = [
    JSON.stringify({ url: "https://example.com/mcp?token=" + secret }),
    JSON.stringify({ command: "/usr/bin/false", type: "http" }),
    JSON.stringify({
      url: "https://example.com/mcp",
      command: "/usr/bin/false",
    }),
    JSON.stringify({
      url: "https://example.com/mcp",
      headers: { Host: secret },
    }),
    JSON.stringify({
      command: "/usr/bin/false",
      env: { NODE_OPTIONS: secret },
    }),
    JSON.stringify({
      url: "https://example.com/mcp",
      configuration: { token: secret },
    }),
    "servers: &server\n  docs:\n    url: https://example.com/mcp\nother: *server",
    '{"__proto__":{"url":"https://example.com/mcp"}}',
    '{"url":"' + secret + '",',
    "x".repeat(131073),
  ];
  for (const source of cases) {
    let error: Error | undefined;
    try {
      parseMcpConfig(source);
    } catch (value) {
      error = value as Error;
    }
    expect(error).toBeTruthy();
    expect(error?.message).not.toContain(secret);
  }
});
