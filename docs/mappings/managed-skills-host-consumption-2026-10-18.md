# Dynamic skills: format, harness consumption and reload boundaries

Observed: 2026-10-18
Confidence: official-documentation; locally validated; deployment and installed-host upgrade not tested

## Sources and Method

Read the following primary documentation on October 18. This records documented
behavior, not an installed-host or deployed-service test.

- [Agent Skills specification](https://agentskills.io/specification) and
  [client integration guide](https://agentskills.io/client-implementation/adding-skills-support).
- [OpenAI skills](https://developers.openai.com/codex/skills/), redirected to
  [Build skills](https://learn.chatgpt.com/docs/build-skills), and
  [AGENTS instructions](https://developers.openai.com/codex/guides/agents-md/).
- [Claude Code skills](https://code.claude.com/docs/en/skills).
- [OpenCode v2 skills](https://opencode.ai/v2/docs/skills).
- Context7 resolved OpenCode to `/websites/opencode_ai_v2`; its query confirmed
  HTTP catalog shape, versioned cache refresh and metadata-only model-step lists.
  The initial Agent Skills resolve returned third-party skill collections rather
  than the format specification; no collection was treated as standard authority.
- Local reads: `AGENTS.md`, `docs/README.md`, accepted foundations/ADRs/contracts,
  `crates/server/src/mcp/agent.rs`, its catalogue/dispatch modules, and plugin/UI
  paths. These established an existing scoped MCP/host context delivery route,
  not an already implemented dynamic manager-skills registry.

## Observations

A skill is a file bundle rather than an independently running service. The
standard requires a frontmatter-bearing `SKILL.md`; optional supporting files
are read as needed. Its common consumption sequence is metadata discovery,
instruction activation, then resource loading. The integration guide explicitly
supports remote registries/APIs and a dedicated activation tool when local file
access is unavailable. That permits fetching skill instructions into a running
agent conversation without native disk discovery.

| Consumer | Documented consumption and update behavior |
| --- | --- |
| Codex | Skills are advertised with name, description and path, then read when selected. `.agents/skills` is scanned across repository ancestors, with user/admin/system roots too. Changes are detected automatically with restart as fallback. Duplicate names are not merged. Plugin distribution can pair skills and connectors. |
| Codex AGENTS.md | The host builds its instruction chain at run/session start. Files follow hierarchical scope; editing/downloading a file does not prove the running instruction chain was rebuilt. |
| Claude Code | Skills load on explicit invocation or relevance. Existing personal/project/additional-directory skill roots are watched live; a newly created unwatched root requires `/reload-skills`. Plugin component changes have a separate reload path. Host invocation includes features such as dynamic command expansion and tool permissions, which are unsafe to infer from ordinary text delivery. |
| OpenCode v2 | A `skills` array accepts local roots or HTTP catalogs. Catalog entries identify a name, version and files; the version controls cache refresh. The model sees metadata and activates content with its skill tool. The documented remote-root `SKILL.md` ID caveat favors named Markdown for a native v2 catalog adapter. |

These are host-specific mechanisms. None establishes a universal API through
which an arbitrary MCP server can silently replace the host's own system/project
instructions. Nor does native filesystem discovery imply semantic AGENTS merging.

## Translation and Limits

[ADR 0022](../adr/0022-brain-managed-skills.md) and the
[contract](../contracts/mcp-managed-skills.md) adopt authenticated progressive
fetch as the shared baseline. The plugin's existing context adapter can advertise
scoped metadata; a stable bridge skill and MCP tools activate the selected body
and exact resources. Native HTTP catalogs and native skill-root registration
remain separate adapters with host/version-specific authentication/reload proof.

Manager-authored instructions need a typed channel distinct from recalled memory.
Environment applicability does not grant execution; MCP profile references remain
advisory. Text is still active guidance: frontmatter permissions and dynamic
command interpolation require explicit exclusion in this bounded initial format.
Temporary files and persistent exports have different disk lifetimes; neither
can be recalled from a conversation after delivery. No installed Codex/Claude/
OpenCode native reload, external registry authentication, or deployed Recollect
availability was verified by this documentation research.

## Local implementation checkpoint

The working tree now contains protocol bundle types/validation, canonical
server storage and HTTP/MCP handlers, native plugin metadata delivery and managed
local-copy lifecycle, plus the Brain Agents Skills management tab. These source
reads establish local implementation, separately from test/deployment acceptance.

The backend implementation selected `yaml_serde` 0.10.7 (YAML-org maintained fork,
with `libyaml-rs` 0.3.0 in Cargo.lock) after Context7 `/yaml/yaml-serde` and
[official parser documentation](https://docs.rs/yaml_serde/0.10.7/yaml_serde/)
verification. Its typed YAML deserialization supports block scalars and rejects
unsupported host frontmatter fields; no handwritten YAML parser is introduced.

Metadata summaries are individually bounded to 12 KiB serialized JSON and the
Brain catalog to 64 identities. HTTP `entrypoint_only=true` projects the requested
entrypoint in SQL. MCP and CLI fetch default to that projection, while explicit
resource paths fetch one file and local save fetches the complete bundle. CLI
stream/receipt limits allow 2 MiB escaped JSON for valid 512 KiB raw text.

Temporary copies are keyed by the validated operation's task ID. Supported end
hooks/closure cleanup take the cache lock and fence delayed saves. Changed,
unowned, symlinked or crash-stage content stays available for deliberate
inspection. Persistent copies, native registration and conversation-held context
remain distinct from this cleanup behavior.

`./scripts/validate.sh` passed locally on October 18: governance lint, 35
governance tests, frozen support corpus validation and 10 support-baseline tests.
The baseline used zero provider calls and measured no semantic quality. These
checks prove documentation/fixture consistency, not feature runtime behavior.

## Local acceptance evidence

The implementation workers and parent recorded the following same-working-tree
results on October 18. Database tests use the repository-owned disposable
PostgreSQL fixture; browser tests mock routes. No paid provider run or deployed
normal-installation migration is part of this proof.

| Surface | Recorded local result | Meaning and limits |
| --- | --- | --- |
| Protocol | Final `cargo test --locked -p recollect-protocol skills --lib`: six tests passed | Valid block-scalar YAML, bounded text and unsafe/host-override rejection; final suite also covers consistent path-component spelling and aliased/duplicate YAML metadata bounds |
| HTTP/storage | `managed_skills_http_permissions_versions_scopes_revocation_and_deletion` passed | Real handler/database publication, history, grants, environment and current eligibility/deletion checks |
| Capacity | `managed_skills_catalogue_capacity_is_explicit_and_counts_disabled` passed | Concurrent creation respects the 64-identity limit, counting disabled entries |
| Agent/plugin | 46 library plugin tests passed; the nine managed-guidance tests passed again after staged-manifest reconciliation | Cache isolation/containment, cleanup fences, escaped JSON bounds and synthetic HTTP recall-to-context behavior; this shared-tree suite includes concurrent coordination tests |
| Manager browser | `web/tests/managed-skills.spec.ts`: five tests passed in 18.1 seconds; build/typecheck passed | Route-mocked create/edit/read/denial/conflict behavior; not a live browser-to-server end-to-end proof |
| Visual | `.cache/managed-skills-ui.png` independently reviewed as passing | Local mocked management view in the established light design; no deployment or live-data claim |
| Repository | `./scripts/validate.sh` passed; scoped `git diff --check` passed | 35 governance tests, frozen support corpus check and 10 baseline tests; zero provider calls |
| API schema | OpenAPI regeneration passed with 188 unique operation IDs and `entrypoint_only` in generated types; final regenerated frontend typecheck passed | Regeneration/typecheck snapshot includes the shared-tree API; no deployed client upgrade is implied |
| Plugin executable | Binary build and `--help` passed | New CLI surface is built locally; installed host packages were not changed |
| Public guide build | `npm --prefix site run build` and `npm --prefix site run check` passed | 28 canonical guides, 31 HTML routes, 1,578 local references/anchors and 113 static files; no publication/deployment |
| MCP | `managed_skills_real_mcp_progressive_fetch_and_immutable_scope` passed | Real MCP initialization/discovery and progressive entrypoint/resource fetch, immutable environment and account/device/operation-kind denial proof |

The final owned-database command
`cargo test --locked -p recollect-server --test platform managed_skills -- --ignored --test-threads=1`
passed all three tests (233 unrelated tests filtered), after loading fixture
configuration without printing credentials. It covers the HTTP, capacity and real
MCP rows above. Test fixtures were cleaned after the successful run.

The [archived pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-managed-skills.md) records this
completed local delivery. No installed plugin upgrade,
native skill-root registration/reload, customer Brain data or deployed Recollect
availability was tested by these focused runs.

## Follow-up

The user subsequently authorized native install/reload and complete lifetime
proof. Completed local behavior is recorded in the [native successor](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-native-skill-reload.md)
and its [host evidence](native-skill-reload-host-proof-2026-10-18.md); this first
local typed-fetch delivery does not infer native results. Normal installation
migration/deployment remains a separate operational scope.
