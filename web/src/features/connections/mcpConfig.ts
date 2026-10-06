import { parse as parseToml } from "smol-toml";
import { parse as parseYaml } from "yaml";

export type ConfigFormat = "auto" | "json" | "toml" | "yaml";
export type DraftSecret = {
  destination: string;
  variable: string;
  value: string;
  prefix: string;
  kind: "header" | "environment";
};
export type ServerDraft = {
  name: string;
  target: string;
  transport: "streamable_http" | "stdio";
  command: string | null;
  arguments: string[];
  configuration: Record<string, unknown>;
  secrets: DraftSecret[];
};
type RecordValue = Record<string, unknown>;
const object = (value: unknown): value is RecordValue =>
  !!value &&
  typeof value === "object" &&
  !Array.isArray(value) &&
  !(value instanceof Date);
function fail(): never {
  throw new Error(
    "Use a supported MCP configuration under 128 KiB. Check the format, server fields and credential destinations.",
  );
}
const text = (value: unknown, max = 2048): string =>
  typeof value === "string" &&
  value.length <= max &&
  !/[\u0000-\u001f]/.test(value)
    ? value
    : fail();
const variable = (value: string) =>
  value.match(/^\$\{([A-Z][A-Z0-9_]*)\}$/)?.[1] ??
  value.match(/^\{env:([A-Z][A-Z0-9_]*)\}$/)?.[1];
const headerAllowed = (name: string) =>
  /^[a-z0-9!#$%&'*+.^_`|~-]+$/i.test(name) &&
  !/^(mcp-|proxy-|x-forwarded-)/i.test(name) &&
  ![
    "host",
    "forwarded",
    "x-original-url",
    "x-rewrite-url",
    "cookie",
    "set-cookie",
    "content-type",
    "content-length",
    "accept",
    "connection",
    "transfer-encoding",
    "origin",
    "referer",
    "last-event-id",
    "upgrade",
    "te",
    "trailer",
  ].includes(name.toLowerCase());
const environmentAllowed = (name: string) =>
  /^[A-Z_][A-Z0-9_]*$/.test(name) &&
  !/^(RECOLLECT_|DYLD_|LD_|DATABASE_)/.test(name) &&
  ![
    "PATH",
    "HOME",
    "BASH_ENV",
    "ENV",
    "NODE_OPTIONS",
    "NODE_PATH",
    "RUBYOPT",
    "RUBYLIB",
    "PERL5OPT",
    "PERL5LIB",
    "PERLLIB",
    "JAVA_TOOL_OPTIONS",
    "JDK_JAVA_OPTIONS",
    "_JAVA_OPTIONS",
    "GEM_HOME",
    "GEM_PATH",
    "SHELLOPTS",
    "BASHOPTS",
    "IFS",
    "CDPATH",
    "PYTHONPATH",
    "PYTHONHOME",
    "RUST_LOG",
    "POSTGRES_PASSWORD",
  ].includes(name);

function bounded(value: unknown, depth = 0): void {
  if (depth > 16) fail();
  if (object(value)) {
    if (Object.keys(value).length > 128) fail();
    for (const [key, child] of Object.entries(value)) {
      if (["__proto__", "constructor", "prototype"].includes(key)) fail();
      bounded(child, depth + 1);
    }
  } else if (Array.isArray(value)) {
    if (value.length > 128) fail();
    value.forEach((child) => bounded(child, depth + 1));
  } else if (typeof value === "string" && value.length > 32768) fail();
}
export function parseConfigDocument(
  source: string,
  format: ConfigFormat,
): RecordValue {
  if (!source.trim() || new TextEncoder().encode(source).length > 131072)
    fail();
  const candidates = format === "auto" ? ["json", "toml", "yaml"] : [format];
  for (const candidate of candidates) {
    try {
      const parsed: unknown =
        candidate === "json"
          ? JSON.parse(source)
          : candidate === "toml"
            ? parseToml(source)
            : parseYaml(source, {
                maxAliasCount: 0,
                merge: false,
                uniqueKeys: true,
              });
      if (!object(parsed)) continue;
      bounded(parsed);
      return parsed;
    } catch {
      /* Never reflect parser excerpts: they can contain credentials. */
    }
  }
  return fail();
}
export function parseMcpConfig(
  source: string,
  format: ConfigFormat = "auto",
): ServerDraft[] {
  const document = parseConfigDocument(source, format);
  const group = document.mcpServers ?? document.mcp_servers ?? document.servers;
  const entries: [string, unknown][] =
    group !== undefined
      ? object(group)
        ? Object.entries(group)
        : fail()
      : [
          [
            typeof document.name === "string" ? document.name : "MCP server",
            document,
          ],
        ];
  if (!entries.length || entries.length > 32) fail();
  return entries.map(([name, input]) => {
    if (!object(input)) return fail();
    const allowed = [
      "name",
      "url",
      "serverUrl",
      "type",
      "transport",
      "command",
      "args",
      "arguments",
      "headers",
      "http_headers",
      "env_http_headers",
      "bearer_token_env_var",
      "env",
      "enabled",
      "configuration",
      "description",
      "startup_timeout_sec",
      "tool_timeout_sec",
    ];
    if (Object.keys(input).some((key) => !allowed.includes(key))) fail();
    const remote = input.url ?? input.serverUrl;
    if (remote !== undefined && input.command !== undefined) fail();
    const transport = remote !== undefined ? "streamable_http" : "stdio";
    if (
      input.type !== undefined &&
      !["http", "streamable-http", "streamable_http", "stdio"].includes(
        text(input.type),
      )
    )
      fail();
    if (
      input.transport !== undefined &&
      !["http", "streamable_http", "stdio"].includes(text(input.transport))
    )
      fail();
    if (
      remote === undefined &&
      [input.type, input.transport].some(
        (v) => v !== undefined && v !== "stdio",
      )
    )
      fail();
    if (
      (input.type === "stdio" || input.transport === "stdio") &&
      remote !== undefined
    )
      fail();
    const command = transport === "stdio" ? text(input.command) : null;
    const target = transport === "stdio" ? "" : text(remote);
    if (target) {
      let url: URL;
      try {
        url = new URL(target);
      } catch {
        return fail();
      }
      if (
        !["http:", "https:"].includes(url.protocol) ||
        url.username ||
        url.password ||
        url.search ||
        url.hash
      )
        fail();
    }
    const args = input.args ?? input.arguments ?? [];
    if (!Array.isArray(args) || args.length > 128) fail();
    const secrets: DraftSecret[] = [];
    const headers = input.headers ?? input.http_headers ?? {};
    const envHeaders = input.env_http_headers ?? {};
    if (!object(headers) || !object(envHeaders)) fail();
    for (const [destination, supplied] of Object.entries(headers)) {
      if (!headerAllowed(destination) || transport !== "streamable_http")
        fail();
      const raw = text(supplied, 16384);
      const prefix = raw.startsWith("Bearer ") ? "Bearer " : "";
      const value = raw.slice(prefix.length);
      secrets.push({
        kind: "header",
        destination,
        prefix,
        variable: variable(value) ?? destination,
        value: variable(value) ? "" : value,
      });
    }
    for (const [destination, supplied] of Object.entries(envHeaders)) {
      const name = text(supplied, 120);
      if (
        !headerAllowed(destination) ||
        !/^[A-Z_][A-Z0-9_]*$/.test(name) ||
        transport !== "streamable_http"
      )
        fail();
      secrets.push({
        kind: "header",
        destination,
        prefix: "",
        variable: name,
        value: "",
      });
    }
    if (input.bearer_token_env_var !== undefined) {
      const name = text(input.bearer_token_env_var, 120);
      if (!/^[A-Z_][A-Z0-9_]*$/.test(name) || transport !== "streamable_http")
        fail();
      secrets.push({
        kind: "header",
        destination: "Authorization",
        prefix: "Bearer ",
        variable: name,
        value: "",
      });
    }
    if (input.env !== undefined) {
      if (!object(input.env) || transport !== "stdio") fail();
      for (const [destination, supplied] of Object.entries(
        input.env as RecordValue,
      )) {
        if (!environmentAllowed(destination)) fail();
        const raw = text(supplied, 16384);
        secrets.push({
          kind: "environment",
          destination,
          prefix: "",
          variable: variable(raw) ?? destination,
          value: variable(raw) ? "" : raw,
        });
      }
    }
    if (
      secrets.length > 32 ||
      new Set(secrets.map((s) => s.destination.toLowerCase())).size !==
        secrets.length
    )
      fail();
    const configuration = input.configuration ?? {};
    if (!object(configuration)) fail();
    const encoded = JSON.stringify(configuration);
    if (
      new TextEncoder().encode(encoded).length > 16384 ||
      /"(?:password|passwd|secret|token|access[_-]?token|refresh[_-]?token|api[_-]?key|authorization|private[_-]?key|client[_-]?secret|credentials)"\s*:/i.test(
        encoded,
      ) ||
      encoded.includes("-----BEGIN") ||
      encoded.includes("Bearer ") ||
      encoded.includes("sk-proj-")
    )
      fail();
    return {
      name: text(input.name ?? name, 120),
      target,
      transport,
      command,
      arguments: (args as unknown[]).map((value) => text(value)),
      configuration: configuration as RecordValue,
      secrets,
    };
  });
}

export function safeConfigPreview(server: ServerDraft): string {
  return JSON.stringify(
    {
      name: server.name,
      ...(server.transport === "streamable_http"
        ? { url: server.target }
        : { command: server.command, args: server.arguments }),
      ...(server.secrets.length
        ? {
            [server.transport === "stdio" ? "env" : "headers"]:
              Object.fromEntries(
                server.secrets.map((secret) => [
                  secret.destination,
                  secret.prefix +
                    "${" +
                    secret.variable.replace(/[^A-Z0-9_]/gi, "_").toUpperCase() +
                    "}",
                ]),
              ),
          }
        : {}),
      ...(Object.keys(server.configuration).length
        ? { configuration: server.configuration }
        : {}),
    },
    null,
    2,
  );
}
