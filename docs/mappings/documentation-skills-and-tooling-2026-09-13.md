# Documentation Skills and Implementation Tooling

Observed: 2026-09-13
Confidence: observed-once

## Sources and Method

- Read Recollect's documentation lifecycle, accepted foundations, governance
  ADR/contract, tooling records, product epic ownership/dependencies, local
  configuration, role definitions and checker.
- Searched the user-named `/Users/mike/private/nautilus-polymarket` and located
  the Terme material at `/Users/mike/private/terme`, not the absent historical
  `terme-flutter` path. Reads only; no edits to those projects.
- Adaptation sources are Terme's `.agents/skills/terme-doc-router/SKILL.md`
  (SHA-256 `fa62a9cf425a92be4f9cf3019607701a162045d94314a9a17665e6a59cf42cd0`)
  and `.agents/skills/terme-doc-maintainer/SKILL.md`
  (SHA-256 `c7778d90b6d6708e5c21b2bab40a7764b64019d2b49991f93e67d66482db5540`).
  No Git metadata was available for that Terme directory, so the source content
  hashes identify the inspected files. Its two UI metadata files were also read.
- The sibling Polymarket router/maintainer/writer were inspected as comparison.
  Their simplified five-document workflow, trading rules and line budget were
  not adopted. Nautilus currently supplies model-lab skills, with no doc-router
  or doc-maintainer entrypoint found in its current skill directories.
- Current [official skill documentation](https://learn.chatgpt.com/docs/build-skills)
  specifies repo `.agents/skills/` discovery, YAML entrypoints, UI metadata and
  implicit invocation. [Official MCP documentation](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)
  documents project configuration and environment forwarding.
- Context7 resolve-library-id and query-docs calls succeeded for `/openai/codex`.
  One indexed discovery excerpt used `.codex/skills`; direct official docs
  identify `.agents/skills/`, which also matches the user's repository rule.
  This illustrates why indexed snippets alone are insufficient host evidence.
- Official Markdown-page fetches returned unsupported-content-type errors;
  the corresponding HTML pages were successfully opened and used instead.

## Observations

The bootstrap had implemented the docs lifecycle, six roles, Context7 and local
CodeGraph, while explicitly deferring custom skills. The user's current request
lifts that deferral for the router and maintainer; the ADR/contract, current role
and navigation docs now reflect that scope. Archived bootstrap packs retain the
scope of their original delivery.

The adaptation keeps Terme's current-state brief, evidence citations, conflict
reporting, scoped maintenance modes and findings report. It substitutes
Recollect's accepted-source order, templates, epic/pack lifecycle and checker.
It does not introduce Terme's docsctl/index artifacts, frontmatter requirements,
SaaS settings policy or archive naming. The two skills have no MCP dependency,
delegate no work and require no personal configuration edits.

Tool checks performed during this assessment:

| Check | Observed result and limit |
| --- | --- |
| Context7 resolve/query | Successful read-only calls; this proves documentation access in this session, not a product integration |
| CodeGraph status/sync/node | Up-to-date index, 3,075 files; exact file query returned source. Broad inferred users included unrelated Cognee files, so relationships still require source verification |
| Python/Codex | Python 3.14.6 and codex-cli 0.154.0 available |
| Rust/Cargo | rustc 1.94.0 and cargo 1.94.0 available; not a pinned or built Recollect workspace |
| Node/npm | Node v26.5.0 and npm 11.17.0 available; no Recollect UI build exercised |
| Docker | Docker CLI 29.7.2 available; daemon, Compose and database services not checked |
| Skill format | Both passed the skill-creator quick validator |
| Skill references/metadata | Local links resolved; UI names/prompts/descriptions and default implicit invocation checked; no declared MCP dependency |
| Codex skill discovery | codex debug prompt-input listed both skills from the root and docs/foundation; aliased catalog paths resolved to the actual repo entrypoints, without an LLM turn |
| Repository validation | ./scripts/validate.sh passed governance lint and all 32 existing checker tests; whitespace checks included untracked files |

The existing governance checker validates docs/configuration, not skill bodies
or model decisions. Additional checks for these two entrypoints are recorded
separately from the repository suite.

The first discovery probe searched only for absolute entrypoint paths and found
none. Codex renders these through a skill-root alias; the corrected check parsed
the catalog and resolved that alias before asserting both paths. This was a
verification-method correction, not a skill-loading failure.

Manual routing and maintenance review used the actual current documents:

| Case | Route and checked boundary |
| --- | --- |
| Begin platform-bootstrap | Accepted foundations lead to Product Platform's planned slice; needs-adr/needs-contract remain, so resolve detailed authority and a pack before product code |
| Explain delivered CodeGraph navigation | Developer Tooling points to the archived codegraph-local-navigation pack and runbook; a working-tree graph is not product/runtime proof |
| Record only a foundation clarification | Update the applicable foundation from the user's decision; docs/README.md permits foundation authoring without speculative product epics/packs |
| Ask whether Vault integration is implemented | MCP Coordination's slices are planned and lack detailed contracts; report that state without installing a connector or editing records for a read-only question |
| Maintain documentation-skills closeout | Content/lifecycle mode reconciles the owner, pack and indexes; audit-only and metadata-only modes preserve their requested scope |

This is a source-based workflow review by the implementing agent, not an
independent agent evaluation or a claim that an LLM has executed every scenario.

## Translation and Limits

The [tooling assessment](../runbooks/development-tooling.md) recommends retaining
Context7, CodeGraph and existing browser/CLI capabilities. No extra development
MCP is required before platform-bootstrap. Further skills should capture proved,
repeatable service, memory/graph, MCP/Vault or UI workflows after those exist;
they cannot substitute for the missing detailed product contracts or tests.

These recommendations are inferences from current scope and available tools.
No dependency matrix, product server, browser workflow, database connection,
credential-provider integration or customer service was installed or proven.
No automatic invocation or model-compliance guarantee follows from valid skill
metadata. Recollect's initial Git branch has no commit; delivery is uncommitted.

## Follow-up

The next product assignment remains platform-bootstrap: resolve its accepted
ADR/contract details and execution pack, pin compatible dependencies, then build
and test the actual Rust/UI/storage slice. Add a development integration only
when its capability is needed and verify a real call with the authorized target.
