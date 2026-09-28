# Direct Codex credential handoff repair

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: fix the user's failed SWEG Codex MCP connection and prevent the same setup mistake.
- Non-goals: OAuth authorization server, published capture plugin, customer infrastructure changes.
- Delivery shape: narrowly scoped local credential/configuration repair, setup UI and actual installed-host proof.

## Governing Sources

[Agent MCP](../../../contracts/mcp-memory-and-workspace-tools.md),
[engineering principles](../../../foundation/engineering-principles.md),
[technology stack](../../../foundation/techstack.md) and the
[owning epic](../../epics/mcp-coordination.md) permit standard HTTP bearer access
and OS-store/environment credential delivery. The user's reported failure
authorizes repairing only SWEG's Recollect integration outside this repository.

## Scope

- In scope: remove the access-token value mistakenly placed in `bearer_token_env_var`; use Codex's standard HTTP-header helper with macOS Keychain; give other hosts explicit environment setup steps; test the installed Codex CLI against the actual SWEG endpoint.
- Out of scope: changing other MCPs, customer repositories, global Codex configuration, Brain grants or automatic capture policy.
- Blockers: none. The existing token authenticates successfully to the live MCP endpoint.

## Surface and Interface Changes

- Interfaces: existing HTTP MCP and browser pairing endpoints, no API changes.
- Storage: one named Keychain generic-password item for the selected MCP URL; no product schema changes.
- Ownership: setup UI in web; project-scoped host configuration in SWEG is the explicitly requested repair.

## Data and Authority

- Inputs: selected Brain URL and browser-created token; never put the token in copied TOML, arguments, shell history, evidence or logs.
- Authority: existing device and current Brain permissions remain authoritative.
- Blind spots: an HTTP test does not prove Codex initialization; a connected tool catalogue does not prove useful memory exists.

## States and Edge Cases

- Loading: existing token creation remains disabled while pending.
- Empty: offer token creation or reuse, with concrete credential storage instructions.
- Error: missing Keychain item, locked store, unset environment or revoked token remains a connection failure.
- Blocked: clients without header-helper support use explicitly documented environment authentication.
- No-access: preserve existing server authorization and revocation.
- Duplicate or replay: update only the named credential when requested; preserve other servers and settings.
- Stale data: instruct restart/resume after changing host configuration; never imply an existing failed session was reloaded.
- Reconciliation divergence: report live-host results separately from isolated browser tests and previous native-bridge fixtures.

## Integrations and Runtime Inputs

- Providers: installed Codex 0.157.1 and local Recollect at port 8787; synthetic loopback model fixture may drive read-only host tools without paid inference or customer data leaving the machine.
- Environment: macOS `security` CLI with Codex `http_headers_helper`, or `RECOLLECT_MCP_TOKEN` for environment-based clients.
- Secrets: Keychain stores the authorization header JSON; helper outputs it only to Codex. The browser copy action is explicit and state is cleared on close.
- Failure handling: no unrequested daemon restarts, credential revocation or broad shell-profile edits.

## Tests and Acceptance

- Automated: setup UI assertions, token/revocation tests, installed Codex consumes repaired credential configuration and invokes `workspace.list`; focused web checks and `./scripts/validate.sh`.
- Manual: current SWEG connection diagnosis and normal-process restart instructions; deploy the setup correction without deleting data.
- Acceptance: a normal Codex process can authenticate without a Recollect launcher or shell exports; copied configuration contains no token; documented evidence distinguishes initialization, tool call, and empty knowledge.

## Closeout

- Planned: credential/configuration repair, concrete setup instructions and installed-host proof.
- Shipped: SWEG's Recollect config now reads its credential from macOS Keychain through Codex's standard helper. Installed Codex 0.157.1 completed a real workspace read. The setup dialog gives Keychain and explicit environment instructions and is deployed locally as image 385793c54044. [Evidence](../../../mappings/mcp-auth-repair-2026-09-28.md).
- Not shipped: OAuth and published capture plugin.
- New blockers: none.
- Docs updated: Agent MCP contract/runbook, dated evidence, continuation handoff, epic and execution indexes.
- Validation: Three focused browser cases, installed-host workspace read, design/typecheck/build, documentation checks and 32 governance tests passed. Live readiness and equal recorded seven-Brain inventory confirmed. An initial test locator and fixture trust loading were corrected; the unrelated opt-in Context7 probe was skipped.
- Version: N/A.
- Commit: uncommitted.
