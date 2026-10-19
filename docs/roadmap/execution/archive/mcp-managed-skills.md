# Dynamic Brain-managed skills and shared instructions

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: A Brain manager publishes reusable scoped guidance; coding agents discover
  metadata, fetch only applicable instructions/resources, and optionally retain a
  temporary or persistent local copy.
- Non-goals: Native host registration/semantic AGENTS merge, executable skill
  learning, script execution, marketplace import and automatic execution grants.
- Delivery shape: Local Rust server/protocol/plugin, React manager UI, migration,
  research/authority and operator documentation; no deployment/commit implied.

## Governing Sources

- [Managed skills ADR](../../../adr/0022-brain-managed-skills.md).
- [Managed skills contract](../../../contracts/mcp-managed-skills.md).
- [MCP tools](../../../contracts/mcp-memory-and-workspace-tools.md),
  [plugin lifecycle](../../../contracts/mcp-plugin-session-memory.md),
  [workspace scope](../../../contracts/evidence-workspace-scope.md), and
  [team access](../../../contracts/platform-team-access.md).
- [Owning epic](../../epics/mcp-coordination.md).

## Scope

- In scope: Administrator bundle management through create/replace/disable, immutable
  revisions and audit/RLS, environment and advisory profile associations,
  HTTP/OpenAPI/MCP list/fetch, metadata context delivery, bridge skill, explicit
  local save/discard and Brain Agents Skills tab.
- Out of scope: Binary/script bundles, provider calls, host security override
  fields, native discovery-root writes, automatic publication from learned memory,
  external repository import and universal native reload.
- Blockers: None. User authorized research plus implementation; ADR/contract
  establish the bounded curated delivery and preserve deferred automatic learning.

## Surface and Interface Changes

- Interfaces: Contract-owned `ManagedSkillKind`, file/input/summary/bundle types;
  Brain `/skills` GET/POST and `/skills/{id}` GET/PUT; `skills.list` and
  `skills.fetch` on authenticated MCP read operations; plugin `skills
  list|fetch|save|discard` with read-operation binding and explicit task/persistent
  lifetime; HTTP entrypoint projection; Brain Agents Skills tab.
- Storage: Additive normal migration with Brain-keyed head/revision rows and
  RLS; preserve existing migration numbers and unrelated current dirty work.
  Canonical Brain deletion includes new rows. No product digest/version gate.
- Ownership: Protocol owns validation and shared types; server owns eligibility,
  history, audit and HTTP/MCP; plugin owns host metadata/cache lifecycle; web
  consumes generated OpenAPI and existing Brain/profile/environment identities.

## Data and Authority

- Inputs: Administrator-authored Markdown/plain-text files and metadata, current
  grants, canonical environment/profile IDs and immutable operation scope.
- Authority: PostgreSQL manager publications and current eligibility; explicit
  local/human/host instructions retain priority. Associations do not grant Use.
- Blind spots: Official reload documentation is dated evidence; no assumption
  about installed host versions, automatic native registration or ability to
  retract exported/conversation-held content.

## States and Edge Cases

- Loading: Bounded metadata request; UI shows loading and preserves draft input.
- Empty: Explain manager publication; no fake guidance or auto-created example.
- Error: Typed validation, network and unavailable states; no alternate version,
  external URL or environment fallback; no mutation transport replay.
- Blocked: N/A: no unresolved dependency decision; native registration is excluded.
- No-access: Reader fetch only, administrator publication, current membership and
  account/device operation checks; no stale content after denial.
- Duplicate or replay: Brain-unique name conflicts; expected-version update
  conflicts; no repeated publication silently overwrites history.
- Stale data: Recheck head enabled/access/applicability on historical and current
  fetch; persistent copies report last-fetch status, not current authorization.
- Reconciliation divergence: Cache path is not canonical authority; draft is not
  published state; deleting files does not evict model conversation context.

## Integrations and Runtime Inputs

- Providers: Existing PostgreSQL, authenticated HTTP/rmcp, native plugin hook
  adapters and same-origin web application; no new model/external registry call.
- Environment: Existing server/database/plugin configuration variable names only;
  no new secret-bearing environment variable is required.
- Secrets: Existing OS credential store/paired authentication; content must not
  contain credentials and error/audit output excludes bearer values/file bodies.
- Failure handling: Existing MCP size/concurrency/deadline boundaries; explicit
  validation/conflict/unavailable errors; hook metadata failure remains bounded
  without preventing normal work. Local saves reject symlinks and unsafe paths; session-end fences prevent
  delayed saves, while edited/unknown/crash-stage files are preserved for inspection.

## Tests and Acceptance

- Automated: Shared YAML/path/bundle validation; server database/RLS and
  create/update/disable/history/scope denials; real MCP read operations; plugin
  metadata and CLI/cache isolation/cleanup; UI typecheck/build and focused browser
  interaction where available. Regenerate OpenAPI, run governance lint,
  `git diff --check` and `./scripts/validate.sh`.
- Manual: Inspect Brain Agents management states and publication, reference fetch,
  disabled/stale behavior and temporary/persistent copy controls. Report actual
  installed-host discovery only if independently exercised.
- Acceptance: Contract acceptance is the delivery checklist. Dynamic typed fetch
  works without native hot reload; manager ownership and execution permissions
  stay separate; temporary cleanup and retained-copy limits are truthful.

## Closeout

- Planned: Curated Brain/environment skills and instructions, HTTP/MCP progressive
  consumption, plugin metadata/local copies, manager UI and operator guidance.
- Shipped: Locally implemented and validated canonical Brain/environment guidance
  with migration 050, immutable revisions, administrator-browser publication,
  current eligibility/RLS, progressive HTTP/MCP entrypoint/resource fetch, plugin
  metadata delivery, bounded temporary/persistent copies and Brain Agents Skills
  management. Operator/public-guide sources and all ownership/index records are
  reconciled. No deployment or installed package upgrade is claimed.
- Not shipped: Native registration/merging, executable skill learning and other
  explicit contract deferrals. Normal-installation migration/deployment, installed
  plugin upgrade and fresh installed-host discovery were outside this local slice.
- New blockers: None for the agreed local delivery.
- Docs updated: ADR 0022, managed-skills contract, host research mapping, scoped
  foundation amendment, operator runbook, plugin guide, owning epic and
  documentation/execution indexes, plus the public documentation guide allowlist.
- Validation: `cargo test --locked -p recollect-protocol skills --lib` passed
  six tests, including portable prefix-case and bounded/aliased/duplicate YAML
  parsing checks. `cargo test --locked -p recollect-server --test platform
  managed_skills -- --ignored --test-threads=1` passed all three fresh owned-fixture
  tests: HTTP/RLS/history/scoping/revocation/deletion, real MCP progressive fetch
  with immutable environment/ownership/kind denials, and concurrent 64-identity
  capacity. The initial plugin library suite passed 46 tests; all nine dedicated
  guidance tests passed after final cache/staged-manifest changes. Five
  route-mocked browser tests, web build/typecheck and independent screenshot
  review passed. OpenAPI regeneration passed with 188 unique operation IDs;
  final regenerated frontend typecheck and plugin binary build/`--help` passed.
  Public guide build/check passed: 28 canonical guides, 31 HTML routes,
  1,578 local references/anchors and 113 static files. Repository validation passed
  governance lint, 35 governance tests, frozen support corpus check and 10
  baseline tests; zero provider calls. Scoped diff check passed. Exact boundaries
  and commands are retained in the
  [evidence](../../../mappings/managed-skills-host-consumption-2026-10-18.md).
- Version: N/A: no release policy or release action requested.
- Commit: uncommitted.
