export type CodingHost = "codex" | "claude" | "opencode";

/** Browser-only direct setup. Callers keep the credential in transient memory. */
export function directAgentConfig(host: string, url: string, token: string) {
  const headers = { Authorization: `Bearer ${token}` };
  if (host === "codex")
    return `[mcp_servers.recollect]\nurl = ${JSON.stringify(url)}\nhttp_headers = { Authorization = ${JSON.stringify(headers.Authorization)} }`;
  const server =
    host === "opencode"
      ? { type: "remote", url, oauth: false, headers }
      : { type: "http", url, headers };
  return JSON.stringify(
    host === "opencode"
      ? { mcp: { servers: { recollect: server } } }
      : { mcpServers: { recollect: server } },
    null,
    2,
  );
}

/** Same-name device pairing rotates credentials; each new browser Create is distinct. */
export function directAgentCredentialName(label: string, issuanceId: string) {
  return `${label.trim().slice(0, 80)} · ${issuanceId}`;
}
