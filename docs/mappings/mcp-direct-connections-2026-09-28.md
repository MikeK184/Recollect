# Direct HTTP MCP setup, URL form and Context7 probe

Observed: 2026-09-28
Confidence: observed-once

## Sources and Method

- [Codex MCP configuration](https://developers.openai.com/codex/mcp),
  [Claude Code MCP](https://code.claude.com/docs/en/mcp), and
  [OpenAI plugin authentication](https://developers.openai.com/plugins/build/auth).
  Direct HTTP supports bearer credentials; native browser OAuth requires server
  authorization metadata/endpoints. A local capture companion is a separate
  function, not a requirement of HTTP MCP authentication.
- [Context7 clients](https://context7.com/docs/resources/all-clients) and
  [anonymous request limits](https://context7.com/docs/search-api), read live.
  The anonymous MCP endpoint is `https://mcp.context7.com/mcp`; its OAuth address
  is separate. Credentials were not attached to the anonymous test.
- Context7 lookup `/websites/rs_rmcp_rmcp` confirmed primary rmcp client lifecycle
  and pagination documentation; implementation compiled against the pinned
  rmcp 3.4 API. Source: `crates/mcp-runtime/src/discovery.rs`.
- Focused native production-handler and browser tests; new paths are owned by
  the [connection pack](../roadmap/execution/archive/mcp-direct-connections.md).

## Observations

The default Codex/Claude Code connection dialog now supplies ordinary HTTP MCP
configuration and a browser action to create a revocable access token. It uses
the existing pairing APIs and agent MCP handler. Configuration references an
environment variable instead of embedding the token. A native companion is
optional for supported session capture and local workspace discovery. Native
MCP OAuth and a published plugin are not implemented or claimed.

The token has the user's current Brain permissions; selecting a Brain endpoint
does not make it a narrowly scoped token. This is stated next to token creation.
The token is cleared when the dialog closes, never included in screenshots,
and cannot be read back after pairing is finished.

The Add connection dialog accepts a name and URL, with a Context7 preset. An
owner-initiated, bounded SDK request initializes the remote MCP server and lists
its tools. It never invokes a tool. Description whitespace is normalized before
catalogue validation; schemas remain unchanged. Edits invalidate the preview.
Saving approves a generated definition and creates the Brain connection using
existing handlers and idempotency. It does not create profile Use grants.
Manifest import and existing registered connectors remain available.

The real anonymous Context7 probe listed `resolve-library-id` and `query-docs`.
An explicit profile and owner Use grant in an isolated proof Brain then invoked
`resolve-library-id` for React through Recollect's runtime. The protocol call
completed, but Context7 returned **Monthly quota exceeded** and directed the
caller to obtain a free API key. Therefore metadata discovery and protocol
execution are proven; useful documentation access without a key is currently
quota-blocked. A configured connection or succeeded protocol receipt must not
be described as useful documentation verification.

Validation:

- Six browser cases passed in `.cache/direct-mcp-20260928/ui-final.log`:
  direct HTTP initialize/list/workspace task/memory recall, token clearing and
  revocation; anonymous form discovery/edit/save with no tool calls; actual
  Context7 quota probe; existing catalogue/use/share isolation, revocation and
  private-runner workflows.
- Owner/member/device registration/discovery boundaries passed in
  `.cache/direct-mcp-20260928/owner-boundaries.log`.
- `cargo clippy -p recollect-mcp-runtime -p recollect-server --all-targets -- -D warnings`
  passed; frontend design/typecheck/build passed during the browser harness.
- Three managed-experience browser cases passed earlier in this change.
- `./scripts/validate.sh` passed the documentation checks and all 32 governance
  tests after closeout; `git diff --check` and `cargo fmt --all -- --check` passed.
- Initial required-label selectors, multiline metadata validation, a browser
  fixture startup timeout and an overly strict Context7 result assertion were
  corrected or rerun explicitly; the final six-case run passed. The provider
  quota response remains reported as unavailable documentation, not hidden.

## Translation and Limits

Native HTTP MCP works without installing `recollect-agent`. Its current token
flow is not an OAuth authorization server or OAuth device authorization grant.
Codex sign-in to OpenAI does not independently authenticate it to Recollect.
Host support for OAuth cannot supply Recollect's missing server implementation.
No external host configuration or personal defaults were modified.

Direct memory-tool calls are integration proof; they do not establish automatic
capture from every coding host or complete product/desktop acceptance. Public
retrieval and lifecycle evidence is recorded in the
[separate benchmark report](public-memory-benchmark-2026-09-28.md).

## Follow-up

Root Compose was rebuilt and deployed as image `56202feeeabb` at
`http://127.0.0.1:8787`; migration exited zero and `/health/ready` returned true.
The live Agents page shows **Connect over MCP**, with no pairing prerequisite.
The direct configuration dialog was inspected without creating a live credential.
The new server form discovered two Context7 tools and saved the requested
anonymous connection in **Autonomous session reconciliation demo**. The UI
correctly shows **Configured · connection not checked**; the useful-documentation
quota finding comes from the isolated runtime probe above. No profile Use grant
was added by this save. No browser console errors were observed.

Parsed `.cache/direct-mcp-20260928/before.json` and `after.json` are equal:
seven Brain identities, sixteen sources, seventeen source versions, eleven
claims, seventeen claim revisions and the recorded Brain/group grants, model
policy/head, retention and capture digests. The Context7 catalogue entry is the
intentional additional configuration. Screenshots and deployment logs are under
`.cache/direct-mcp-20260928/`. All changes are local and uncommitted; version N/A.

OAuth/plugin packaging and the six earlier desktop acceptance packs remain
separate unfinished work.
