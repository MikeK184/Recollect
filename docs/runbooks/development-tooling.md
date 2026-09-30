# Development Tooling Assessment

Revised: 2026-09-13. The [selected stack](../foundation/techstack.md) is Rust,
React/Vite, PostgreSQL/pgvector and Neo4j/GDS Community. The MCP coordinator uses
rmcp; Cognee remains a reference. Earlier Python/FastMCP/Next.js recommendations
belonged to the superseded extension baseline.

Governance validation, Context7 calls and local CodeGraph navigation have been
exercised. The documentation router and maintainer are now repository-local
skills. Installed Rust/Node/Docker client versions have been inspected; the
product dependency matrix, UI and services remain unimplemented and unverified.

## Available development capabilities

| Capability | Choice and evidence boundary |
| --- | --- |
| Dependency documentation | Repo-local Context7 plus primary official sources; record stale/incomplete lookups |
| Source structure and callers | Repo-local CodeGraph 1.6.0, with a verified index of the Cognee reference checkout |
| Literal/configuration/docs search | rg and targeted file reads |
| Governance validation | ./scripts/validate.sh using Bash and Python 3.11+ standard library |
| Documentation routing and maintenance | [recollect-doc-router](../../.agents/skills/recollect-doc-router/SKILL.md) and [recollect-doc-maintainer](../../.agents/skills/recollect-doc-maintainer/SKILL.md), adapted from Terme |
| Git review | Git CLI and focused comparisons; preserve dirty work and separate reference checkouts |
| Browser inspection | Available browser-control tools when a real product UI is running |

Use the [CodeGraph runbook](codegraph.md) for setup, exact-symbol navigation and
refresh. Its relationships describe the indexed working tree, not published
product snapshots or runtime truth. Enola is selected as the first product
artifact adapter, subject to coverage proof; CodeGraph remains an alternative.

## Add with the owning implementation slice

Pin the Rust toolchain, Cargo dependencies and frontend lockfile during
scaffolding. Run rustfmt/clippy and focused Rust tests then; select frontend
checks against the actual React/Vite application. Do not carry Python application
constraints from Cognee into Recollect's Rust core.

Use rmcp client/server fixtures and disposable PostgreSQL/Neo4j services to prove
product interfaces, isolation, correction and recovery. Test local and private
MCP runners against explicit disposable targets, with environment/OS-store/Vault
credential delivery covered by the relevant contract. No customer service access
is required simply to author or validate documentation.

The UI uses static assets served by Rust. A Next.js-specific development MCP is
not selected. Reuse existing browser inspection before adding another integration.
Codex project development MCP registration belongs in .codex/config.toml;
OpenCode project development MCP registration belongs in root `opencode.json`;
product Brain connections and execution profiles belong in Recollect's runtime model.

## Skills assessment

Use `$recollect-doc-router` to find authority, current-state evidence and the
owning slice. Use `$recollect-doc-maintainer` for scoped authoring, audits and
pack/epic/index closeout. Both support ordinary automatic discovery. Their
instructions follow the current docs and do not copy the foundation specification.

These two skills are sufficient for the current docs and planning stage. The
existing architect, worker, qa and reviewer roles already describe their tasks;
creating equivalent skills would duplicate instructions. No separate doc-writer
skill is needed: authorized authoring is part of the maintainer's content mode.

Consider further skills only after the owning slices establish repeatable,
tested commands and fixtures:

| Candidate | When it becomes useful | Required local basis |
| --- | --- | --- |
| Rust service implementation | Repeated work after platform-bootstrap and platform-durable-work | Actual crate boundaries, Cargo checks, SQLx migration/metadata procedure and transactional command/outbox fixtures |
| Memory and graph invariant verification | Evidence, correction, retrieval and graph slices have runnable paths | Paired positive/negative Brain-isolation checks, immutable manifests, reject/rebuild and erase/restore fixtures, analytical eligibility checks |
| MCP/Vault lifecycle verification | MCP runtime and private-runner slices | Disposable stdio/HTTP targets, startup/lease/reuse/idle tests, rotation/revocation and unknown-outcome fixtures |
| Product UI workflow verification | Repeatable React/Mantine review, search and graph flows exist | Runnable UI, stable test data, browser checks for loading, error, no-access and actionable review states |

These are potential helpers, not new scope or prerequisites. Keep them in
`.agents/skills/` if later justified. Foundation requirements such as correction,
isolation, Vault support and negative evaluations still accompany their feature
slices whether a dedicated skill exists or not. Product procedural memory and
deferred automatic executable-skill learning are separate from these developer
skills.

## MCP assessment

No additional development MCP is required to begin the accepted implementation
sequence beyond the user-authorized OpenCode chrome-devtools browser control.
Keep the existing Context7 server and CodeGraph CLI. This is a tooling
recommendation based on the selected stack and available workflows, not proof
that future product integrations already work.

| Integration | Recommendation and trigger |
| --- | --- |
| Context7 | Keep; real library resolution and documentation queries succeeded in this session. Pair responses with current primary sources when incomplete or inconsistent. |
| CodeGraph MCP | No additional server now; the pinned local CLI already supplies navigation. Rust source coverage must be checked when Rust code exists. |
| OpenAI documentation MCP | Optional if sustained OpenAI-specific work warrants it; current official browsing and Context7 cover this assessment. No product model provider is selected merely by using Codex. |
| Browser/Playwright MCP | OpenCode uses secret-free local `chrome-devtools` (`chrome-devtools-mcp@latest --isolated --no-usage-statistics`) per the 2026-09-29 user request for browser control; Codex remains Context7-only. Add further browser integrations only for a demonstrated missing capability. |
| PostgreSQL, Neo4j or Docker MCPs | No extra server for scaffolding; direct clients, Compose and integration fixtures should prove the actual application paths. Client installation alone does not prove service readiness. |
| GitHub/GitLab or issue-tracker MCPs | Revisit when an actual remote/CI/review workflow needs capabilities beyond Git and the relevant CLI; no remote service was connected here. |
| Vault, Kubernetes, Terraform, Confluence or customer MCPs | These are possible product-managed connections or integration fixtures. Their presence in the vision does not require developer credentials or registration in this repository. |
| Cognee or another memory-engine MCP | Not required by Recollect's independent Rust baseline; reference material does not create a runtime dependency. |

Any later Codex development MCP belongs in `.codex/config.toml`, and any later
OpenCode development MCP belongs in root `opencode.json`, with named environment
inputs and a successful call recorded before claiming connectivity. The current
governance checker permits exactly Context7 in `.codex/config.toml`; it does not
constrain `opencode.json`. A future approved Codex addition must
update its configuration contract and focused tests together. Brain-managed
connections belong to product contracts/runtime rather than these host files.

See [dated sources and verification](../mappings/documentation-skills-and-tooling-2026-09-13.md)
for the source adaptation, installed-tool snapshot and limits. Reference-repo
agent files and skills do not govern Recollect. No source checkout was modified.

The Bash/Python governance tools remain valid and independent of the product
language. Running their checks does not prove Rust, UI, database or MCP behavior.
