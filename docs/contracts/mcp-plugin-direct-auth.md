# Plugin/MCP direct user authentication

Status: accepted

## Source

[ADR 0015](../adr/0015-direct-plugin-user-auth.md), the user decisions of
2026-09-29 (plugin authenticates the user directly; auth-and-connect first),
[Agent memory MCP ADR](../adr/0010-agent-memory-mcp.md),
[Agent memory/workspace tools](mcp-memory-and-workspace-tools.md),
[Runtime and credential contract](mcp-runtime-and-credentials.md),
[Companion pairing and device authority](platform-device-pairing.md), and the
[Vision deployment and access model](../foundation/vision.md#deployment-and-access-model).
Host transport shapes verified 2026-09-29: installed Codex 0.157.1
(`codex mcp add --help`, plugin manifest strings in the shipped binary),
Context7 `/openai/codex`, `/anthropics/claude-code`, and
`/websites/opencode_ai_v2` documentation queries.

## Contract

### Token transport and fixed destination

`POST /api/brains/{brain}/mcp/agent` serves rmcp Streamable HTTP with legacy
session mode disabled, exactly as the memory/workspace tools contract
specifies. Every request requires `Authorization: Bearer {token}` where the
token is a claimed, unrevoked, unexpired device credential owned by an
enabled principal, plus current Brain read access before
discovery/initialization/dispatch. The token acts as the user with that
user's current Brain grants; execution profiles keep separate Use/Manage/Share
evaluation. Reading production knowledge never implies production execution.
Missing/invalid bearers never fall back to a browser cookie. Bearer calls do
not require browser CSRF.

The `{brain}` path segment selects exactly one Brain for that server entry.
The token itself is not Brain-scoped: it inherits the account's current
grants across Brains. Setup text and plugin skills must state this plainly
and must never claim the token is restricted to the selected Brain. Users
needing several Brains add one server entry per Brain.

### Issuance paths

Both paths mint the same device record and bearer credential with a 30-day
expiry, the 20-device account limit, and existing audit entries. No new
tables, migrations, or credential kinds exist.

- Browser Create action: the signed-in browser runs start/approve/poll/finish
  on the user's explicit Create access token action, shows the token once in
  component memory, and never persists it. This reuses the existing pairing
  endpoints with a browser session and CSRF for approval.
- Plugin-initiated device code: any HTTPS client with no Recollect binary
  calls `POST /api/devices/pairings` with a name of 1–120 characters and
  receives a private `device_code`, an eight-character public `user_code`, a
  verification URL, expiry, and poll interval. The user approves the public
  code in the browser. The client polls `POST /api/devices/pairings/poll`
  no faster than the returned interval, then calls
  `POST /api/devices/pairings/finish` to claim the device. A minimal
  reference flow using only `curl` and the browser is:

```sh
open_url=$(curl -s -X POST "$RECOLLECT_URL/api/devices/pairings" \
  -H 'content-type: application/json' \
  -d '{"name":"Codex plugin · personal laptop"}')
echo "$open_url"
# Open verification_url, compare user_code, approve in the browser.
# Then poll with {"device_code":"..."} until state is approved, and finish.
```

Revocation uses Devices (`DELETE /api/devices/{id}`) or possession-based
`POST /api/devices/revoke-self`. Revocation denies new calls and credential
renewal per existing semantics. Repeat revocation is harmless.

### Host configuration shapes

All rendered configuration is secret-free and `configured_only`: it proves
nothing until a live tool call succeeds. The literal token never appears in
plugin files, generated settings, URLs, commands, history, logs, or proof
output. Only the variable name `RECOLLECT_MCP_TOKEN` appears. The Brain UUID
and absolute service origin are ordinary configuration, not secrets.

- Codex remote (`codex-remote`): TOML with the fixed Brain endpoint URL and
  `bearer_token_env_var = "RECOLLECT_MCP_TOKEN"`, or equivalently
  `codex mcp add recollect --url {origin}/api/brains/{brain}/mcp/agent --bearer-token-env-var RECOLLECT_MCP_TOKEN`.
  macOS Keychain remains a supported alternative through
  `http_headers_helper` reading authorization-header JSON, as the existing
  setup dialog documents.
- Claude remote (`claude-remote`): `{"mcpServers":{"recollect":{"type":"http","url":"{origin}/api/brains/{brain}/mcp/agent","headers":{"Authorization":"Bearer ${RECOLLECT_MCP_TOKEN}"}}}}`.
  The token is exported into the terminal that starts Claude; already running
  agents do not receive it.
- OpenCode remote (`opencode-remote`): `{"mcp":{"servers":{"recollect":{"type":"remote","url":"{origin}/api/brains/{brain}/mcp/agent","oauth":false,"headers":{"Authorization":"Bearer {env:RECOLLECT_MCP_TOKEN}"}}}}}`.
  `oauth: false` is required because this server authenticates exclusively
  with the header credential; Recollect does not implement MCP OAuth
  discovery/authorization, and host OAuth support does not supply that
  missing server behavior. Do not label this setup as OAuth.

### Plugin packaging rules

The checked-in static plugin source under `plugins/recollect` carries memory
skills and setup guidance only. It grants no authority, scope, or execution
rights. Its fixed contents are:

- A Codex marketplace root with a local-source marketplace manifest and a
  `recollect-memory` plugin (`plugin.json` with the `skills` key, memory
  `SKILL.md` with valid name/description frontmatter). The plugin bundles no
  MCP server entry because the endpoint URL is deployment-specific; the skill
  gives the exact `codex mcp add` command instead.
- A Claude-compatible manifest for the same plugin directory plus an
  `.mcp.json` template using the `${RECOLLECT_MCP_TOKEN}` placeholder.
- An OpenCode remote snippet using the `{env:RECOLLECT_MCP_TOKEN}`
  placeholder.
- A README with install, token, and verify steps per host.

Static plugin files are validated secret-free: no `Bearer ` literal with a
value, no UUID token, no `http_headers_helper` output. Regenerating or
re-copying the bundle never touches personal host profiles; standalone
rendering commands print configuration without writing host files.

### Scope and fresh context

Scope enforcement is unchanged from the memory/workspace tools contract:
every memory, graph, write, and managed tool operation uses an explicit
immutable operation ID of the existing required kind and same
Brain/principal/device. Models choose the task; the server derives selection
from its operation. Environment/area/repository selection continues through
the existing workspace operations. Brain-wide search is denied unless
permitted; in-Brain scope is the default. A user with no Brain grants sees an
explicit no-access state, never an empty Brain silently.

## Acceptance

Prove with a real rmcp client over HTTP using a browser-issued device token
and no companion binary installed: discovery without provider startup;
`workspace.list` confirming the Brain; a scoped write operation plus
`memory.contribute` and `memory.recall` returning the contribution with
provenance; revocation followed by denied discovery on the next request;
wrong-Brain and reader-writer denials. Prove the three rendered remote
configurations are byte-shaped per this contract, carry no secret, and parse
with the host's own tooling (`codex mcp add --help` flags for Codex; JSON
parse for Claude/OpenCode). Prove the static plugin bundle carries the
`skills` key, parseable `SKILL.md` frontmatter (name of at most 64
characters plus description), the placeholder-only `.mcp.json`/OpenCode
snippet, and no secret literal. Run focused Rust/frontend checks and
`./scripts/validate.sh` before archive.

## Explicit Deferrals

No MCP OAuth discovery/authorization endpoints; no published public
marketplace (the plugin installs from the checked-in local source); no
device-identity dedupe (one record per issuance stands until the successor
slice); no review-UI removal or Ask/Search simplification (successor slices);
no OpenCode capture hooks; no change to what a model may do with an approved
tool; no customer deployment of the public server itself.
