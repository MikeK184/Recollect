# Copy-ready direct agent configuration and Access tokens

Status: in-progress
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Browser-created direct MCP credentials copy into valid native host
  configuration without a terminal export, and their safe metadata is easy to find.
- Non-goals: Change native plugin OS-store handling, static plugin templates,
  CLI rendering, authentication authority or credential lifetime.
- Delivery shape: Local web/API changes, focused checks and documentation.

## Governing Sources

- [Direct authentication ADR](../../../adr/0015-direct-plugin-user-auth.md),
  including the accepted 2026-10-05 browser exception.
- [Direct authentication contract](../../../contracts/mcp-plugin-direct-auth.md).
- [Device pairing contract](../../../contracts/platform-device-pairing.md).
- [Desktop contract](../../../contracts/desktop-experience.md).
- [Owning epic](../../epics/mcp-coordination.md).

## Scope

- In scope: Labelled unique browser issuance, native TOML/JSON literal-header
  generation, masked display/reveal/explicit exact copy, visible Access tokens,
  and safe reported host/integration metadata in the own-account list.
- Out of scope: Automatic testing, host file writes, token persistence/reveal
  from history, new credential table or changed grants.
- Blockers: None; the user's explicit decisions resolve the browser exception.

## Surface and Interface Changes

- Interfaces: `McpAgentSetup` Direct MCP wizard; own-account Access tokens list;
  compatible optional `host_kind` and `integration` fields on Device reads.
- Storage: No migration; existing device markers are read. Tokens remain
  transient browser component/mutation state until explicit copy.
- Ownership: Web agent setup/list; protocol Device metadata; server devices read.

## Data and Authority

- Inputs: Selected host/Brain/label, current service origin, newly issued
  credential and authorized device-list metadata.
- Authority: Existing browser pairing/finish and current Brain/account grants.
- Blind spots: Copying configuration is not a host connection. Unknown markers
  remain unreported; list responses never contain the secret.

## States and Edge Cases

- Loading: Creation disables closing/navigation and duplicate issuance.
- Empty: No tokens gives a connect-agent action; no use shows an explicit empty last-use value.
- Error: Existing bounded pairing cancellation/error recovery remains;
  failed clipboard copy shows an error without the credential.
- Blocked: Account limit and invalid/expired auth report the existing API error.
- No-access: Device list remains account-scoped; Brain permission checks remain canonical.
- Duplicate or replay: Unique suffix per browser Create prevents rotating an
  existing same-label credential; Back/Next does not issue again.
- Stale data: Current list refresh failures hide prior payload; navigation,
  Brain switch/unmount or closing drops the one-time credential.
- Reconciliation divergence: A failed finish directs users to Access tokens
  before retry; no claim of verified connection is generated.

## Integrations and Runtime Inputs

- Providers: Codex native TOML, Claude native JSON, OpenCode v2 native JSON;
  current interfaces verified through Context7/official documentation.
- Environment: No required token environment variable in browser direct setup.
- Secrets: Mask by default. Highlight only masked config; reveal uses plain
  local text. Explicit Copy alone puts exact secret-bearing config on clipboard.
  No source/config/test fixture or proof output contains a real secret.
- Failure handling: No automatic external/provider test or retry; existing
  pairing retry/cancel semantics and clipboard error state remain explicit.

## Tests and Acceptance

- Automated: Parse all generated native formats using temporary synthetic
  values, assert exact copy/masking/clear-on-close, no export, unique issuance
  preserving a previously working credential, metadata/history/revocation and
  existing real HTTP memory-read checks. Run frontend typecheck and required validator.
- Manual: Inspect the actual masked wizard and Access tokens entry without
  screenshots of revealed credentials. Host runtime proof uses an isolated disposable credential only.
- Acceptance: A user can create/copy/start their host normally; tokens are
  visible before first use and cannot be re-revealed from metadata; existing
  credentials remain usable; native plugin/storage behavior remains intact.

## Closeout

- Planned: Copy-ready config, unique issuance and discoverable credential metadata.
- Shipped: Local implementation generates masked copy-ready native configuration,
  explicit reveal/copy, labelled unique issuance, transient secret clearing and
  safe host/integration metadata; deployed to the ready local installation.
- Not shipped: Final fresh native-host acceptance remains open. Installed OpenCode
  timed out waiting for its background service; fresh Claude launch was not
  exercised. Native config recognition and HTTP protocol proof are separate.
- New blockers: The installed OpenCode background service does not start within
  the bounded native CLI probe; no unidentified host process was terminated.
- Docs updated: ADR 0015/direct-auth amendment, this pack, epic/index and handoff.
- Validation: Synthetic native TOML/JSON parsing and unique issuance pass.
  UI-created disposable token copied from the masked wizard; Codex native CLI
  recognizes its literal-header configuration. The same copied header completes
  actual initialize, tools/list, workspace.list and memory.recall requests on
  the isolated Brain. Empty recall returns zero items, not invented evidence.
  Access tokens shows host/integration, paired/last-used/expiry metadata without
  the secret. Roster/pairing/revocation tests, schema generation, frontend build,
  independent masked-wizard visual review and local readiness pass. All owned
  token/config/database fixtures were removed after privacy/graph cleanup.
- Next action: Repeat the isolated native-host launch once the installed OpenCode
  service starts; retain this pack in active until that acceptance is recorded.
- Evidence: [Coordinated delivery and limits](../../../mappings/desktop-browser-management-2026-10-05.md).
- Version: N/A; no release requested.
- Commit: Uncommitted.
