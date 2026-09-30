# Direct plugin auth without CLI download

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: A user on a MacBook or customer VDI connects Codex, Claude Code, or OpenCode to a hosted Recollect server through the plugin/MCP integration alone (Cognee-style): the plugin authenticates the user directly via API token or browser device code, selects the Brain, and enforces Brain/environment/area/repo scope. No CLI download or pairing is required. The local companion remains only for optional automatic session capture.
- Non-goals: removing the existing CLI-paired and direct-HTTP paths (they keep working); OAuth/broader SSO; device-identity dedupe and review-UI removal (recorded successor slices, decided 2026-09-29, specified separately); OpenCode capture hooks; any change to what the model may do with an approved tool.
- Delivery shape: server auth path plus plugin/MCP client configuration, focused proof, runbook and setup-text updates.

## Governing Sources

- [Vision deployment and access](../../../foundation/vision.md#deployment-and-access-model) (same service locally or on a shared server; companion pairing yields revocable device credentials)
- [Vision MCP coordinator and Vault](../../../foundation/vision.md#mcp-coordinator-and-vault-integration) (connections, execution profiles, runner placement)
- [Agent memory MCP ADR](../../../adr/0010-agent-memory-mcp.md)
- [Direct plugin user auth ADR](../../../adr/0015-direct-plugin-user-auth.md)
- [Plugin direct-auth contract](../../../contracts/mcp-plugin-direct-auth.md)
- [Agent memory/workspace tools](../../../contracts/mcp-memory-and-workspace-tools.md)
- [Runtime and credential contract](../../../contracts/mcp-runtime-and-credentials.md)
- [Autonomous memory ADR](../../../adr/0006-autonomous-memory.md) (routine operation without human approval)
- [Owning epic](../../epics/mcp-coordination.md)
- Atlas stacks: single-user tool starts with scope plus explicit write destination; correctable stack builds scope, evidence, gateway, tombstone in that order (`agent-memory-atlas/content/patterns/index.md`, 2026-09-29).

## Scope

- In scope: a server-issued user token (or device-code browser approval) that a plugin/MCP client presents per Brain; Brain selection by ID with Brain-wide search only when permitted, otherwise in-Brain scope; environment/area/repository selection through existing workspace scope; UI and setup text that present plugin-first connection with capture as an optional advanced step.
- Out of scope: changing scope enforcement, execution-profile permission evaluation, capture behavior, or review/lifecycle rules; device-row dedupe; Ask/Search simplification; review-UI removal.
- Blockers: None. The ADR delta ([0015](../../../adr/0015-direct-plugin-user-auth.md)) and contract delta ([plugin direct auth](../../../contracts/mcp-plugin-direct-auth.md)) landed 2026-09-29, closing the recorded `needs-adr`/`needs-contract` gap.

## Surface and Interface Changes

- Interfaces: `recollect-agent mcp-config codex-remote|claude-remote|opencode-remote --brain UUID` renders secret-free remote HTTP MCP configuration (Codex TOML with `bearer_token_env_var`, Claude `mcpServers` JSON with `${RECOLLECT_MCP_TOKEN}` headers, OpenCode `mcp.servers` JSON with `{env:RECOLLECT_MCP_TOKEN}` headers and `oauth:false`); checked-in static plugin source under `plugins/recollect` (local marketplace root, `recollect-memory` Codex/Claude manifests, memory skill with auth setup, Claude `.mcp.json` template, OpenCode example snippet, README); coding-agent dialog gains the OpenCode host shape; pairing endpoints unchanged and reused for both issuance paths.
- Storage: N/A with reason — no schema or migration change; issuance reuses the existing device/pairing records per ADR 0015.
- Ownership: the agent crate owns remote rendering (`mcp_host::configuration_remote`) and static-bundle validation (`plugin::validate_bundle`); web owns the dialog host shape; the contract owns the wire shapes.

## Data and Authority

- Inputs: fixed Brain ID, user identity, existing Brain grants and execution-profile grants. Scope selection continues through the existing workspace operation.
- Authority: the issued token acts as the user with that user's current Brain permissions (same rule as paired companions); execution profiles keep separate Use/Manage/Share evaluation. Reading production knowledge never implies production execution.
- Blind spots: rendered configuration is `configured_only` until a live call succeeds; public-server deployment itself remains excluded from this repo's scope.

## States and Edge Cases

- Loading: N/A: issuance and rendering are synchronous local operations besides the browser approval wait, which reuses the existing pairing-approval UX.
- Empty: a user with no Brain grants sees an explicit no-access state, never an empty Brain silently.
- Error: invalid/expired/revoked tokens fail explicitly with re-auth guidance; secrets never appear in plugin files, generated settings, or proof output.
- Blocked: N/A beyond the recorded ADR/contract blockers.
- No-access: Brain-wide search is denied unless permitted; in-Brain scope is the default.
- Duplicate or replay: re-issuing for the same user and machine updates one credential record (anticipates the decided dedupe rule; specified in the successor slice).
- Stale data: revocation denies new managed calls per existing revocation semantics; already-issued credential handling follows the explicit supported mechanism.
- Reconciliation divergence: N/A: no new mutable server state beyond the credential record the ADR will specify.

## Integrations and Runtime Inputs

- Providers: installed Codex, Claude Code, OpenCode v2; plugin/skill schema verified per host at implementation time via Context7 plus host docs.
- Environment: variable names only; never values. Existing `RECOLLECT_URL` and `RECOLLECT_DEVICE_PROFILE` patterns apply; new names (if any) come from the contract delta.
- Secrets: the user token lives in the host's own secret handling or the OS store; never in plugin files, generated settings, URLs, or proof output.
- Failure handling: N/A with reason — transport retry/timeout behavior reuses the existing runtime contract; new behavior (if any) is specified in the deltas.

## Tests and Acceptance

- Automated: agent unit proof for all three remote shapes (URL, placeholder-only auth, `oauth:false` for OpenCode, secret-free, `configured_only`) plus rejection of unknown hosts and missing Brains; static-bundle proof (manifest names, `skills` key, skill frontmatter, placeholder-only templates, no bearer value); existing agent/server unit tests; all-target Clippy; web typecheck; `./scripts/validate.sh`; `git diff --check`.
- Manual: N/A beyond the documented commands; live-host verification is recorded acceptance, not a manual claim.
- Acceptance: a plugin-only Codex/Claude/OpenCode client authenticates as the user with no CLI installed, lists the Brain, and performs a scoped recall plus an evidence-backed write; revocation denies new calls; capture stays optional and off by default. The scoped HTTP tool proof already exists in the (PG-gated) server suite; this slice adds the issuance/rendering/bundle proof plus live boundary checks.

## Diagnosis feeding this slice (2026-09-29, verified same day)

- The `.env` `OPENAI_API_KEY` is valid: `GET /v1/models` returns 200. The Codex live-run failure (`workspace out of credits`) is not this key: `~/.codex/auth.json` carries `auth_mode: chatgpt`, so that run billed the ChatGPT workspace, a different wallet from the $10 API balance. No product defect on the token path.
- Repeated macOS Keychain prompts come from two compounding causes: the SWEG direct-HTTP entry shells out to `/usr/bin/security find-generic-password ... -w` on every MCP call (one prompt per call unless the keychain is unlocked/approved), and every CLI/bridge process reloads the companion credential separately. `cargo run` rebuilds also rotate the ad-hoc-signed binary identity that trusted-app matching relies on. Mitigations (stable install path, stdio bridge instead of per-call `security` helper, session `security unlock-keychain`, single-load handoff) belong to implementation after the ADR delta.
- Ask "doesn't work" on the near-empty SWEG-test Brain is correct `no_evidence` behavior, not a defect: almost no eligible accepted claims exist there. The defect is UX — it reads as breakage.
- Devices shows 8 rows for ~2 real companions because every `pair` call mints a new device record (two "Codex live plugin test" rows are two separate pairings). Dedupe rule decided: one identity per user+machine; successor slice.
- Review/proposal/overwrite UI removal decided 2026-09-29 (autonomous learning log instead); Ask/Search read-only simplification decided; both are successor slices after this one.

## Closeout

- Planned: slice row, this pack, ADR delta, contract delta, implementation, proof, runbook/setup-text updates.
- Shipped: ADR 0015 and the plugin direct-auth contract (accepted 2026-09-29); `mcp-config codex-remote|claude-remote|opencode-remote` rendering with unit proof; static plugin source under `plugins/recollect` with bundle proof; OpenCode host shape in the coding-agent dialog; runbook and setup-text updates; live boundary proof (pairing start/poll/cancel, MCP 401/403, Codex `mcp add`/`get` in a scratch home); full live-token acceptance 2026-09-29 with the owner login (local dev stack only): plugin-initiated device-code start, browser approve/poll/finish, Bearer-token MCP initialize with 22 tools, `workspace.list`, evidence-backed `memory.contribute` (`device_authored`/`proposed`), `memory.recall` returning the contribution with provenance, owner device revocation followed by 401 denial. Proof script (loads `.env` in-process, prints statuses only) at the scratch path noted in the evidence mapping; test Brain `Plugin live-token proof` and its claim remain on the dev stack, the proof device stays revoked.
- Not shipped: successor slices only (device dedupe, review-UI removal, Ask/Search simplification), explicitly out of scope.
- New blockers: None.
- Docs updated: ADR 0015, plugin direct-auth contract, ADR/contract README indexes, memory/workspace tools contract pointer, owning epic slice map/status/dependencies/governing sources, active/archive indexes, this pack, agent-memory-tools runbook, coding-agent dialog text, evidence mapping.
- Validation: `cargo test --locked -p recollect-agent --lib` 17/17 (3 new); `cargo clippy --locked --all-targets -p recollect-agent` clean; web `typecheck` clean; `./scripts/validate.sh` 32/32; `git diff --check` clean. Live stack `ready:true`; no secrets in proof output.
- Version: N/A (no release policy).
- Commit: uncommitted.
