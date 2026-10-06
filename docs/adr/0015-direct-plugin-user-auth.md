# 0015: Direct plugin and MCP user authentication without a CLI download

Status: accepted

October 1 amendment: [ADR 0018](0018-plugin-managed-agent-memory.md) supersedes the separately operated companion/capture-only plugin boundary. Automatic capture and recalled context now belong to the plugin; existing auth, sanitization, scope and privacy guarantees remain.
Date: 2026-09-29

### Browser copy-ready configuration amendment — 2026-10-05

The user approved direct-token browser setup without a required environment
export or Keychain step. The explicit Direct MCP flow may hold a newly issued
token in component memory and copy a native host configuration with its literal
Authorization header to the clipboard. The visible configuration is masked by
default; revealing or copying the credential requires an explicit action.
Closing, navigating away or changing Brain clears this transient copy. Nothing
writes it to browser storage, URLs, logs, examples, source or checked-in files.
The user saves copied configuration in their private, untracked host file.

This exception applies only to browser-created Direct MCP setup. Native plugins
keep their OS-store credential handling and static/CLI rendering remains
secret-free. Every new browser issuance has a distinct labelled name, so
same-name pairing reconciliation cannot replace a working agent credential.
Access tokens is the visible own-account credential list, including unused and
historical credentials; its metadata never returns the bearer value.

## Decision

A user on a MacBook or customer VDI connects Codex, Claude Code, or OpenCode
to a hosted Recollect server through the plugin/MCP integration alone,
Cognee-style: the plugin authenticates the user directly, selects the Brain,
and enforces Brain/environment/area/repository scope. No CLI download or
companion pairing is required. The local companion remains only for optional
automatic session capture.

No new credential type is introduced. The existing paired-device bearer
credential doubles as the plugin/MCP user token. It is issued through one of
two paths that both use the existing device-pairing endpoints and records:

- Browser Create action: the signed-in browser runs the existing
  start/approve/poll/finish pairing sequence on the user's explicit Create
  access token action and shows the token once. This path already exists in
  the coding-agent setup dialog.
- Plugin-initiated device code: any HTTPS client (the plugin skill's
  documented `curl` steps, with no Recollect binary) calls unauthenticated
  `POST /api/devices/pairings` to obtain a verification URL and user code,
  the user approves it in the browser, and the client polls and finishes to
  receive the same device identity and token.

The token is presented as `Authorization: Bearer {token}` on
`/api/brains/{brain}/mcp/agent` on every request. It acts as the user with
that user's current Brain permissions, exactly like paired companions;
execution profiles keep their separate Use/Manage/Share evaluation. Brain
selection is the URL path Brain UUID; the token itself is not Brain-scoped,
and setup text must never claim it is restricted to one Brain. Revocation
uses Devices or `revoke-self` with existing semantics: revocation denies new
calls and credential renewal, never implies remote cancellation, and never
invalidates already-issued credentials without an explicit supported
mechanism.

Outside the browser copy-ready exception above, token secrets live in the host's own secret handling (Codex
`bearer_token_env_var`/`http_headers_helper`, Claude `${VAR}` header
expansion, OpenCode `{env:VAR}` header expansion) or the OS store. Secrets
never enter plugin files, CLI-generated settings, URLs, logs, or proof output.

## Why

On 2026-09-29 the user decided the product must be fully autonomous and
plugin-first: no CLI download, no human click-ops, companion capture-only.
The verified live SWEG run already proved a browser-issued device token
drives remote MCP memory tools with no companion. The remaining gap was
authority, not transport: no accepted decision said the device credential
may serve as a plugin user token, how a plugin initiates issuance, or how it
composes with paired credentials. This ADR closes that gap by reusing the
existing pairing primitive instead of inventing OAuth, a second token table,
or a published-marketplace dependency.

Codex 0.157.1 (verified locally 2026-09-29 against its installed binary and
`codex mcp add --help`) supports remote Streamable HTTP servers with
`bearer_token_env_var`, `http_headers_helper`, and plugin-bundled MCP
servers through `.codex-plugin/plugin.json` (also discovering
`.claude-plugin/plugin.json`). Claude Code supports remote HTTP MCP servers
with `${VAR}` header expansion inline in `plugin.json` or `.mcp.json`
(Context7 `/anthropics/claude-code`, 2026-09-29). OpenCode v2 supports
remote servers with `{env:VAR}` header expansion and `oauth: false`
(Context7 `/websites/opencode_ai_v2`, 2026-09-29). All three hosts therefore
accept the same Bearer user token with no Recollect-side OAuth work.

## Consequences

The [plugin direct-auth contract](../contracts/mcp-plugin-direct-auth.md)
defines the exact wire shapes, Brain-selection behavior, and plugin file
rules. The agent crate owns secret-free remote configuration rendering and
the checked-in static plugin source under `plugins/recollect`. The web owns
the plugin-first setup text with capture as an optional advanced step.

Existing CLI-paired and direct-HTTP paths keep working unchanged; the
companion's pairing, capture, and bridge behavior is untouched. OAuth,
broader SSO, device-identity dedupe, and review-UI removal remain explicitly
deferred to their own slices. A configured endpoint still selects one Brain
per server entry; multi-Brain users add one server entry per Brain.

## Supersession

N/A: new decision. Existing paired identity ([ADR 0010](0010-agent-memory-mcp.md)),
device pairing, canonical memory, capture, and managed execution decisions
remain in force.
