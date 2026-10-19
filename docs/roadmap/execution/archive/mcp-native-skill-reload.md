# Native managed-skill installation and host reload

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Manager-published skills/instructions become actual native Codex, Claude
  Code and OpenCode skills with owned temporary/retained lifetimes and verified
  discovery, update, activation and removal behavior.
- Non-goals: Executable learning/marketplace, global installation and editing
  existing AGENTS/CLAUDE instruction hierarchies.
- Delivery shape: Local native filesystem engine, CLI/plugin adapters, lifecycle
  and real installed-host fixtures/proof; no normal deployment/release implied.

## Governing Sources

- [ADR 0022 amendment](../../../adr/0022-brain-managed-skills.md).
- [Native contract](../../../contracts/mcp-native-skill-reload.md) and
  [canonical managed-skills contract](../../../contracts/mcp-managed-skills.md).
- [Workspace scope](../../../contracts/evidence-workspace-scope.md),
  [plugin lifecycle](../../../contracts/mcp-plugin-session-memory.md) and the
  [owning epic](../../epics/mcp-coordination.md).

## Scope

- In scope: Install/sync/reload/uninstall/status/leases CLI, owned workspace
  roots/transforms, workspace-wide scope ledger across host kinds, per-task refs,
  explicit persistence, automatic hook/scope synchronization, instructions skill
  wrappers and real Codex/Claude/OpenCode discovery/activation/update/remove proof.
- Out of scope: Global roots, local instruction hierarchy rewrite, script execution,
  skill auto-learning, arbitrary remote import and impossible context retraction.
- Blockers: None for implementation. Host capability differences are explicit
  observable states/proof requirements. Claude 2.1.302 is installed only in the
  repository-owned ignored fixture directory; personal/global host setup is
  preserved.

## Surface and Interface Changes

- Interfaces: Contract CLI commands and host-specific native roots; OpenCode
  skill transforms/reload; actual supported Codex/Claude refresh or explicit
  reload/restart action. Distinguish placement/catalog/activation statuses.
- Storage: Workspace `.recollect/managed-skills-native/` registry/lock, private
  ownership/lease manifests and exact-byte receipts with owned staging;
  N/A server migration because canonical versioned bundles remain unchanged.
- Ownership: Agent crate owns filesystem/CLI/state; host plugin owns refresh and
  automatic lifecycle; server remains canonical eligibility authority; docs own
  official research, actual host versions and delivery evidence.

## Data and Authority

- Inputs: Existing canonical bundles and immutable operation scope, current grants,
  canonical workspace identity and installed host capabilities/version.
- Authority: Existing authenticated fetch plus explicit local ownership. Native
  exports require a fresh scoped canonical check before applying cached content.
  Local/human/host rules and tool Use remain independent.
- Blind spots: Workspace files are visible to unrelated local programs; active
  context cannot be withdrawn. Latest docs may exceed installed API schema.

## States and Edge Cases

- Loading: Bounded automatic sync under the existing hook deadline; explicit
  partial results and retry when downloading/reloading is incomplete.
- Empty: Remove/deactivate only eligible unchanged owned exports; preserve local
  skills and separately retained canonical copies.
- Error: Native file, transport and refresh errors identify the safe next action;
  no implicit global install or foreign target fallback.
- Blocked: Scope conflict returns native_scope_busy without writes; native host
  unable to reload reports exact reload/restart action and MCP remains available.
- No-access: Current fetch denies disabled/revoked/wrong-scope content; exports do
  not grant tool or server permissions and cannot claim filesystem confidentiality.
- Duplicate or replay: Stable source aliases, exact receipts, locks and task refs
  prevent duplicate adoption and one session removing another's copy.
- Stale data: Canonical eligibility checked on sync/activation; stale native state
  has visible reload/unavailable status and no false loaded-success claim.
- Reconciliation divergence: Preserve modified/unknown/symlink/crash-stage files;
  validate staged bytes before success; ended tasks fence delayed install work.

## Integrations and Runtime Inputs

- Providers: Existing HTTP/MCP and native Codex/Claude/OpenCode interfaces; use
  owned synthetic provider fixtures for proof. No paid model call required.
- Environment: Existing private plugin/fixture configuration only; no new secret
  environment variable needed. Record host/platform/version in evidence.
- Secrets: Existing OS credentials and fixture secrets remain outside source,
  CLI arguments, generated native content and diagnostic output.
- Failure handling: Existing bounded hook deadline, explicit incomplete/reload
  state, no mutation replay or arbitrary host process injection. Preserve receipts
  and exports when ownership/content cannot be verified safely.

## Tests and Acceptance

- Automated: Temporary/retained/combined copy tests, multi-task and cross-host
  scope leases, scope change/recovery, stale ended-task fences, modified/foreign/
  symlink safety, wrapper/relative resources, exact host interface regressions.
- Manual: Actual installed host fixtures prove native discovery and content
  activation before/after add/update/remove, recording any required reload or
  restart. Verify existing local instructions/skills remain intact.
- Acceptance: All native contract acceptance, actual host proof rather than copy
  inference, focused tests and `./scripts/validate.sh`; retain first slice history.

## Closeout

- Planned: Native adapter commands, automatic sync, shared scope/ref lifetimes,
  actual installed-host reload/activation proof and current operator guidance.
- Shipped: Locally implemented and validated native adapters with completed
  installed-host acceptance and final review.
  Native install/sync/reload/uninstall/status/leases, automatic task/context/scope
  synchronization, cross-host workspace leases, retained pins, offline cleanup,
  instructions wrappers and all-three-host discovery/activation are present.
- Not shipped: Normal deployment or installed user-plugin update; global roots,
  instruction hierarchy rewrites, executable learning and other explicit exclusions.
  These are not unmet acceptance for the bounded native successor.
- New blockers: None. The feedback contract's Explicit Deferrals section is now
  present, and the authorized 2026-10-18 repository-wide validation rerun passed.
- Docs updated: ADR amendment, canonical/native contracts, native proof mapping,
  operator runbook, bundled skill, plugin README, pack archive, owning epic,
  execution indexes and current roadmap. First shipped pack history is preserved.
- Validation: Final full agent library suite passed 94 tests; nine JavaScript
  adapter tests passed. These include cache/native/CLI lifetimes, cross-host scope
  leases, delayed fetch/denial/metadata generation fences, both interrupted
  transition recovery phases and merged hook-output bounds. Final review found no
  blockers. Actual installed Codex 0.162.1, Claude Code 2.1.302 and OpenCode 2.0.22
  passed same-process/session v1/v2 body/reference activation, temporary two-task
  references, retained instruction/source and explicit uninstall proof. Additional
  OpenCode late compatibility-root deduplication, removed activation HTTP 404 and
  real reload counters 1 through 7 passed. The actual freshly built plugin completed eight offline local CLI maintenance
  actions with a mismatched global/workspace Brain and no credential slot/server;
  local instructions were preserved. The updated bundled skill passed
  frontmatter/command checks and exact native Codex catalog discovery. Targeted
  Rust format, JavaScript syntax and diff checks passed. Earlier post-edit
  `./scripts/validate.sh` passed governance lint, 35 governance tests and 10
  support-baseline tests; an intervening global attempt failed on unrelated
  `docs/contracts/retrieval-attributed-feedback.md` missing Explicit Deferrals.
  After that section was present, the authorized 2026-10-18 full rerun passed
  governance lint, all 35 governance tests, the frozen support baseline and all
  10 support-baseline tests. The earlier failure is resolved.
  Fresh standalone governance/support suites passed 35+10 tests. After archive,
  public guide build/check passed 31 HTML routes, 1,579 local references/anchors
  and 113 static files. Scoped native filled-pack, metadata, accepted-authority,
  links and all execution/epic index comparisons passed through the existing
  validator. Whole-repository validation now passes as well. See exact proof
  boundaries and current checks in the
  [mapping](../../../mappings/native-skill-reload-host-proof-2026-10-18.md).
- Version: N/A: no release policy or release action requested.
- Commit: uncommitted.
