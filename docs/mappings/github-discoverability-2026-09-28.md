# GitHub README presentation and discoverability

Observed: 2026-09-28
Confidence: observed-once

## Sources and Method

Read the live GitHub repository pages and authenticated REST metadata/README
responses for the projects below. Compare their About descriptions, actual
repository topics, opening layout, use cases, setup paths and evidence links.
These are presentation observations, not comparative product benchmarks.

| Primary source | Observed README blob SHA |
| --- | --- |
| [Cognee](https://github.com/topoteretes/cognee) | `fba455594a0ce7b02e5f3db1123488039b193836` |
| [Graphiti](https://github.com/getzep/graphiti) | `a426f3482b963b0986577a96732e1896ca82b871` |
| [Mem0](https://github.com/mem0ai/mem0) | `9a81e74effacbd7433e91b6c1fcc907f110d4773` |
| [Letta entry point](https://github.com/letta-ai/letta) | `ebac9f5e9d8da416ca7f95987520137b2fd4fdf3` |
| [Letta Code, current source linked by Letta](https://github.com/letta-ai/letta-code) | `e4ac82d36ba13b2c9ab4cfd644384d8bc17b4360` |

GitHub's official [topic guidance](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/classifying-your-repository-with-topics),
[repository search documentation](https://docs.github.com/en/search-github/searching-on-github/searching-for-repositories)
and [CLI edit reference](https://cli.github.com/manual/gh_repo_edit) establish
the metadata behavior. Context7 `/websites/github_en_rest` also confirmed the
repository update and topic read/write endpoints. No external lookup failed.

For Recollect, read `gh repo view MikeK184/Recollect --json
nameWithOwner,description,repositoryTopics,url,homepageUrl,isPrivate,defaultBranchRef,viewerPermission,licenseInfo`
and the README response from `gh api repos/MikeK184/Recollect/readme`.
Rechecked the local README, accepted foundations, existing runbooks and current
epic/execution indexes before writing capability or maturity claims.

## Observations

| Project | Presentation and explanation | Topic approach | Useful pattern for Recollect |
| --- | --- | --- | --- |
| Cognee | Centered logo, purpose statement, navigation and badges; use cases and a starting-point table lead into a runnable example. | 19 topics mix memory, agents, knowledge graphs and community terms. | Explain the purpose immediately and route readers to their first useful action. |
| Graphiti | Centered identity, real workflow/release badges and a graph animation; explains temporal graphs and provides installation, MCP and service paths. | Four broad topics: agents, graph, llms and rag. | Explain the graph's practical role and connect claims to technical guidance. |
| Mem0 | Branded banner, navigation, package badges and benchmark tables; offers separate library, server, cloud and CLI starts. | 14 topics cover agents, memory, RAG, language and related concepts. | Make installation choices obvious; preserve the distinction between managed results and open-source behavior. |
| Letta / Letta Code | The old entry point now redirects readers to current source. Letta Code uses a short introduction, package/community badges, a demo and a feature table. | Four broad topics on the entry point; eight on current source, including agent-memory and codex. | Use a concise feature table and link to the actual integration/setup guide. |

At inspection, Recollect was already public, had an empty About description and
no topics, and GitHub recognized Apache-2.0. Its published README blob was
`07d1fd9821f03c7b31bb981f75c4e33d924281f1`. The working README already contained
uncommitted root Compose setup changes; those are preserved.

The material discovery gap was empty metadata. GitHub's default repository
search includes names, descriptions and topics. README content is available
through `in:readme`; making the README longer alone does not fill the default
search gap. Topic filters provide another route to related repositories.

## Translation and Limits

Keep Recollect's positioning specific: engineering memory for coding agents,
across repositories and sessions, plus governed MCP execution. Use the existing
original brand, a light-background SVG header that also reads on dark GitHub
pages, factual license/stack/integration badges, use cases, a feature table and
direct setup/documentation links. The header reuses the outlined wordmark from
`web/public/brand/recollect-logo.svg` without changing the original assets.

Do not copy peer popularity, benchmark, package-release or CI badges: those
would imply evidence or distribution channels this change does not establish.
Recollect's current desktop acceptance stays explicitly open. Cognee remains
a design reference, not a runtime dependency or a repository topic.

Selected GitHub About description:

> Self-hosted AI memory and MCP coordination for coding agents. Persistent context across repositories and sessions, with knowledge graphs, hybrid search, and a desktop web UI. Built in Rust for Codex and Claude Code.

Selected topics, grouped by the actual product concepts they describe:

| Purpose | Topics |
| --- | --- |
| Memory and audience | `agent-memory`, `ai-memory`, `long-term-memory`, `ai-agents`, `coding-agents`, `context-engineering` |
| Retrieval | `knowledge-graph`, `graph-rag`, `rag`, `hybrid-search` |
| Integration | `mcp`, `mcp-server`, `model-context-protocol`, `claude-code`, `codex` |
| Deployment and stack | `self-hosted`, `rust`, `pgvector`, `neo4j` |

All 19 names meet GitHub's lowercase/hyphen rules and 50-character limit, within
its maximum of 20 topics. These are relevant categories, not a mandatory or
optimal-count formula. Better classification is an informed discoverability
improvement; search ranking, indexing time, traffic and growth remain unmeasured.
No homepage is invented when a dedicated product website does not exist.

## Follow-up

The description and all 19 topics were applied with `gh repo edit` and verified
through a fresh REST read. The repository's name, public visibility, default
branch, archive/disabled state and empty homepage match the pre-change read.
This is live metadata evidence, not a claim that search indexing has completed.

The README passed 28 local file/anchor checks and SVG parsing. GitHub's Markdown
API rendered it successfully; its rendered HTML was inspected in local light and
dark page styles, including the header, all four loaded badges and feature table.
This preview is not a published README or an exact reproduction of GitHub's CSS.
The existing local setup section is unchanged, and fingerprints confirm that
207 unrelated previously changed files retain their contents. Governance lint,
all 32 checker tests and `git diff --check` passed.

Local comparison/metadata snapshots and rendered previews are retained under
ignored `.cache/readme-discovery-2026-09-28/`. The bounded documentation slice is
recorded in the owning [governance epic](../roadmap/epics/repository-governance.md).

README, asset and governance changes require a later commit/push to appear on
GitHub. Existing desktop/Compose work is outside this publication change; the
current assignment does not publish that work or establish runtime health.
