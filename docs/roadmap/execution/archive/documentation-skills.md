# Recollect Documentation Skills

Status: shipped
Owning epic: `docs/roadmap/epics/developer-tooling.md`
Work type: governance

## Summary

- Goal: Supply discoverable documentation router and maintainer skills adapted
  from Terme, and assess further skills/MCPs against the accepted Recollect stack.
- Non-goals: Product implementation, additional integrations, external writes,
  generic skill installation, or changes to personal execution preferences.
- Delivery shape: Local skills, reconciled governance documentation and dated
  tooling evidence; no release or deployment.

## Governing Sources

- [Repository governance ADR](../../../adr/0001-repository-governance.md)
- [Repository governance contract](../../../contracts/repository-governance.md)
- [Selected stack](../../../foundation/techstack.md)
- [Developer Tooling epic](../../epics/developer-tooling.md)

## Scope

- In scope: Read the user-named private project skills; adapt Terme's router
  and maintainer to Recollect; supply UI metadata; remove current skill-deferral
  contradictions; assess tooling and verify entrypoints, references and discovery.
- Out of scope: Importing Terme's docsctl, SaaS policy, metadata schema or archive
  layout; changing reference checkouts; adding product contracts or code; adding
  an MCP merely because the product will support that integration.
- Blockers: None. The explicit request lifts the two skills' bootstrap deferral.

## Surface and Interface Changes

- Interfaces: `$recollect-doc-router` and `$recollect-doc-maintainer`, with
  automatic selection allowed by task-specific descriptions.
- Storage: Markdown under `.agents/skills/` and UI metadata in each skill's
  `agents/openai.yaml`; N/A for application data or migrations.
- Ownership: Recollect owns the adapted skills; docs remain the source of
  authority. Existing repo roles remain roles, with no delegation implied.

## Data and Authority

- Inputs: Current user scope, Recollect docs/code/config evidence, Terme source
  skills, current official Codex docs, Context7 responses and local tool checks.
- Authority: Accepted Recollect sources and current user decisions; reference
  skills and mappings are evidence, not imported governing instructions.
- Blind spots: No application handlers or runtime exist to validate future
  Rust/UI/database/MCP workflows. Static checks cannot prove model compliance.

## States and Edge Cases

- Loading: Read only relevant docs via existing indexes and targeted rg searches;
  use relative repository links rather than external-checkout dependencies.
- Empty: Missing authority is reported with exact topic and owning slice;
  absent product code is not reported as a missing MCP connection.
- Error: Record lookup/validation failures and use source inspection where useful;
  never claim that configured or discoverable tools were successfully called.
- Blocked: Mark needs-adr/needs-contract for dependent work and continue
  independent authorized work. Routine document maintenance adds no approval gate.
- No-access: Scope all edits to Recollect; credentials and private service access
  are not required for these skills.
- Duplicate or replay: Reuse existing document ownership and slice IDs; retain
  shipped history instead of creating competing current specifications.
- Stale data: Preserve intended, implemented, validated and deployed distinctions;
  recheck current sources and do not automatically rewrite accepted decisions.
- Reconciliation divergence: Update pack, epic and indexes together; unmet
  acceptance remains active/blocked and is never converted into shipment.

## Integrations and Runtime Inputs

- Providers: Existing Context7 for dependency docs, official sources for host
  behavior, and local CodeGraph CLI for source navigation; no new registration.
- Environment: CONTEXT7_API_KEY is forwarded by existing configuration. Skill
  execution itself needs only repository reads and the existing validation tools.
- Secrets: Never persist credential values; inspect configuration and environment
  presence without emitting secrets.
- Failure handling: Record unavailable source paths and failed lookups; use the
  found Terme source pair and official HTML when a Markdown fetch is unsupported.

## Tests and Acceptance

- Automated: Run the skill-creator quick validator for both skills, verify local
  links/UI metadata, check host discovery without an LLM turn, inspect whitespace
  including new untracked files, and run `./scripts/validate.sh`.
- Manual: Walk routing cases for platform-bootstrap, a shipped navigation slice,
  a foundation-only request and a missing contract; audit the final docs for
  skill deferrals and incompatible imported conventions. Review maintainer mode
  boundaries and closeout using the actual changed records.
- Acceptance: Both skills are valid and found by Codex; all repo references
  resolve; no foreign workflow is required; governance passes; the assessment
  distinguishes available tools, future slice needs and unverified runtime.

## Closeout

- Planned: Two adapted skills and a scoped assessment of further skills/MCPs.
- Shipped: Terme-derived recollect-doc-router and recollect-doc-maintainer with
  UI metadata and default implicit invocation; current governance/role guidance
  reconciled; a stack-specific tooling assessment and dated evidence delivered.
- Not shipped: Additional skills/MCP servers, product code, dependency installation,
  remote/CI integration, deployment and model-based skill evaluation are outside
  this governance slice. Existing reference skills/checkouts were not edited.
- New blockers: None for this slice. Product contracts and runtime proof remain
  with their already-planned owners, beginning with platform-bootstrap.
- Docs updated: Root agent guide, docs guide, Codex setup/default role, governance
  ADR/contract, developer-tooling assessment/mapping, epics and execution indexes.
- Validation: Both skill-creator quick checks passed; local references/UI metadata
  passed; Codex 0.154.0 catalog discovery resolved both entrypoints from root and
  docs/foundation without an LLM turn. Manual routing and maintenance cases are
  recorded in the [evidence mapping](../../../mappings/documentation-skills-and-tooling-2026-09-13.md).
  Governance lint and all 32 existing tests passed; whitespace checks included
  untracked files. Actual Context7 resolve/query and CodeGraph status/sync/node
  calls succeeded. These checks do not establish product or model behavior.
- Version: N/A: no application release.
- Commit: uncommitted.
