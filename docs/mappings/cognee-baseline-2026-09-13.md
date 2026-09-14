# Cognee Extension Baseline Evidence

Historical evidence for the initial extension design. Later user decisions select
an independent Rust product; the translations below are not current implementation
instructions. See the [foundations](../foundation/README.md) and
[decision mapping](atlas-foundation-decisions-2026-09-13.md).

Observed: 2026-09-13
Confidence: verified

Verification here means source/configuration inspection unless a successful
runtime call is explicitly stated. No Cognee application or UI was started.

## Sources and method

- Local Cognee: clean `main`, upstream origin `https://github.com/topoteretes/cognee.git`,
  commit `c0d18c80e24b7b78918e7642c03f6f128fdd2aee`, commit date 2026-09-09.
- Inspected manifests, ACL model, graph task factory/importer, clone resolver,
  Enola installer, database-handler registration, and integration-plugin registry.
- Read public integration docs at commit
  `4e66a304bc42b3cf8f630e084de30e580bf65133` in `topoteretes/cognee-integrations`.
- Compared the user's proposal with current primary documentation. Source
  versions are pinned below where available; floating docs can change.

## Observations

| Finding | Evidence | Implication |
| --- | --- | --- |
| Cognee 1.5.4 uses Python/FastAPI/Pydantic/SQLAlchemy/Alembic | [Core manifest](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/pyproject.toml) | Extend its existing backend modules and migration tooling |
| Frontend uses Next.js 16, React 19, TypeScript, Mantine, TanStack Query, and Jest | [Frontend manifest](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee-frontend/package.json) | Extend existing UI; manifest ranges are not installed-version evidence |
| Cognee MCP uses FastMCP 3 and has a stricter Python upper bound than core | [MCP manifest](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee-mcp/pyproject.toml) | Python 3.12 fits both declared ranges; no install was run |
| `get_code_graph_tasks` accepts `snapshot_dir`; graph import reconciles/removes obsolete facts | [Importer and task factory](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/tasks/code_graph/extract_code_graph.py) | Reuse import; authenticated publication and immutable revision views need extension work |
| Repository URL ingestion clones locally under configured repo storage | [Repository resolver](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/tasks/code_graph/resolve_repo.py) | Our no-mirror policy must select a different ingestion route explicitly |
| Enola is an external compiled CLI, pinned to 0.4.12 with checksums | [Installer](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/tasks/code_graph/install_enola.py) | Reuse the binary and record extractor provenance |
| ACL references `principal_id`, `permission_id`, and `dataset_id` | [ACL model](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/modules/users/models/ACL.py) | A separate profile resource grant is needed; knowledge grants do not authorize execution |
| Dataset graph/vector handlers differ by provider | [Handler registry](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/infrastructure/databases/dataset_database_handler/supported_dataset_database_handlers.py) | Verify the selected combination and isolation behavior before VM deployment |
| Local demo defaults are SQLite, LanceDB, Ladybug | [Local Compose guide](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/docs/minimal-docker-compose.md) | Do not copy its auth-disabled single-user demo into the shared Brain service |

The upstream [Codex integration](https://github.com/topoteretes/cognee-integrations/blob/4e66a304bc42b3cf8f630e084de30e580bf65133/integrations/codex/README.md)
and [Claude Code integration](https://github.com/topoteretes/cognee-integrations/blob/4e66a304bc42b3cf8f630e084de30e580bf65133/integrations/claude-code/README.md)
document HTTP-backed hooks, per-launch connection/session state, automatic
capture, and dataset switching. Switching creates a new Cognee session and
updates the launch record. This supports reuse but does not establish safe
parallel task scopes. Upstream's shared default dataset must not become an
implicit cross-customer Brain boundary.

## Documentation checks and limits

- [NodeSets](https://docs.cognee.ai/core-concepts/further-concepts/node-sets)
  support grouping; they do not supply our structured revision/profile model.
- [Roles](https://docs.cognee.ai/core-concepts/multi-user-mode/permissions-system/roles)
  and [dataset permissions](https://docs.cognee.ai/core-concepts/multi-user-mode/permissions-system/datasets)
  document principal/role/tenant access composition. Brain mapping and grant
  propagation through ordinary API routes still need contracts and tests.
- [Code-graph guidance](https://docs.cognee.ai/guides/code-graph) states that paths
  exist within one dataset; separate datasets are searched independently.
- [Document update](https://docs.cognee.ai/python-api/update) describes
  re-ingestion and replacement of derived document content. It does not prove
  environment-aware claim revision or freshness filtering. The user's earlier
  temporal-fact wording is not treated as verified behavior here.
- The supplied deployment-options URL could not be retrieved. No writer-safety
  or VM-capacity guarantee is inferred from it. Backend writer ownership and
  bounded queues remain design requirements needing deployment validation.
- Git's [remote command](https://git-scm.com/docs/git-remote) expands URL rewrite
  configuration, while [clone documentation](https://git-scm.com/docs/git-clone)
  covers URL forms. Alias, port, fork, and move normalization still require our
  explicit identity contract.

## Verified developer connection

`codex mcp get context7 --json` discovered the repo-local npx configuration.
A stdio MCP check initialized Context7 4.1.0, listed `resolve-library-id` and
`query-docs`, resolved FastMCP, and retrieved `create_proxy` documentation.
The existing environment key was forwarded without recording its value.
The smoke-check process and its owned children were stopped afterward.

This proves that connection during this check. It does not start or connect
Recollect's proposed Cognee/runner system. Six role TOML files and registration
targets are validated locally; no subagents were spawned to test them.

## Follow-up

Use this evidence when writing the owning product ADRs and contracts. Verify
live access-control behavior, snapshot import/revision isolation, plugin capture
and scope concurrency, provider compatibility, and representative VM workloads.
Fork hosting and extension-branch provisioning await a concrete repository
destination; current remotes/checkouts were preserved.
