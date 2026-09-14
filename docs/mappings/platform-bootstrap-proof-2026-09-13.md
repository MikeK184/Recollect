# Platform bootstrap local proof

Observed: 2026-09-13
Confidence: verified

## Sources and Method

Current repository code, `./scripts/dev.sh`, `./scripts/test-platform.sh`,
`./scripts/test-ui.sh`, `cargo clippy --workspace --all-targets -- -D warnings`,
`./scripts/validate.sh`, live readiness/agent calls and direct screenshot inspection.
Dependency interfaces were checked through successful Context7 and primary-source
calls recorded in [dependency evidence](platform-dependencies-2026-09-13.md).

## Observations

| Requirement | Direct evidence |
| --- | --- |
| Runnable selected stack | Rust workspace builds; TypeScript/Vite production build passes; one-command dev startup reuses existing credentials/data, migrates, generates API types, builds UI and serves on 8787 |
| Actual integration | Readiness and native agent health succeed; PostgreSQL 17.10, pgvector 0.8.6, Neo4j Community 2026.08.1 and GDS 2026.08.1 answer SQL/vector/Query API calls |
| Owner lifecycle | Real-handler tests cover generic failed login, valid login, persisted session across router reconstruction, expiry, logout, owner bootstrap replay and recovery revoking existing sessions |
| Brain isolation and access | Two persisted actors create separate Brains through production handlers; direct ID/list/audit/update paths deny forbidden scope with positive controls; reader access cannot administer; revocation leaves independent ownership usable |
| Database enforcement | Tests connect as non-owner/non-bypass role; unbound Brain queries return zero rows and actor-bound queries return only that actor's Brain |
| Transactional mutation/audit | An injected audit constraint failure makes the production create request fail and rolls back its Brain; a subsequent authorized request succeeds; updates and archives retain history |
| Browser behavior | One isolated Chrome workflow passes real login, empty view, create, rename, archive/reopen, audit, network-error recovery, mobile layout without overflow, logout and reload; no page errors |
| Visible UI | Inspected login, empty, Brain list, Brain detail and 390px mobile screenshots in `.cache/ui/`; readable, no clipping/overlap; unavailable knowledge tools are labeled |
| Governance and code quality | Governance lint and all 32 existing checker tests pass; Rust format/build/Clippy and frontend generation/type/build pass; untracked-file whitespace and credential-value checks are clean |

The Rust proof comprises two focused scenarios. The UI proof uses a disposable
database and separate local API on 8788; all test databases were removed after
the successful runs. No user Brain was added by UI testing. CodeGraph status/sync
and exact `actor_tx` caller navigation succeeded after source changes.

## Translation and Limits

This is local platform proof, not the full product or external deployment.
Knowledge, jobs, invited/OIDC users, paired devices, graph projections/analytics,
retrieval and MCP/Vault remain in their named slices. Basic local credentials
follow the user's no-hashing development override; no hardened shared access claim.

The initial PostgreSQL bind mount failed during initialization due to macOS
ownership handling; Compose health had incorrectly accepted a merely listening
server. A project-owned named volume and actual database query health check fixed
the cause, and startup/test reruns succeeded. The failed bind-mount data was left
untouched and is no longer mounted. Public image pulling initially hung in the
desktop credential helper; the repository-local anonymous Docker wrapper fixed it.

The Browser plugin's JavaScript worker referenced missing version `26.901.51231`
despite installed `26.903.61454`; bootstrap failed again after a session reset.
No personal plugin file was changed. The permitted fallback was an isolated
Playwright Chrome test. The first UI test also caught a real query-cache reset
bug after login; preserving the active session query while clearing other user
data fixed login/logout notification, and the complete workflow then passed.

## Follow-up

Proceed to platform-durable-work, then the remaining platform and domain slices.
Run the named proof commands after behavior changes; these observations are
dated and do not substitute for later runtime verification.
