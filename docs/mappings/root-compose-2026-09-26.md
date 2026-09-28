# Root Compose workflow — 2026-09-26

## Scope and method

User-requested packaging follow-up for the existing local app. Inspected current
Compose labels/mounts, native process working directories, actual HTTP readiness,
the installation contract and current database counts. The screenshot and earlier
handoff were discovery hints, not runtime authority.

Root project initially contained only PostgreSQL and Neo4j; the UI/API and worker
were native processes. Separate personal/shared/recovery/integrated proof projects
accounted for most containers. Cognee belongs to another project.

## External interface evidence

Context7 `/docker/compose` and current official Docker documentation were read on
2026-09-26. No failed lookups. Compose's
[dependency conditions](https://docs.docker.com/compose/how-tos/startup-order/)
support healthy databases and successful one-shot migration gating.
[Unresolved environment keys](https://docs.docker.com/reference/compose-file/services/#environment)
are omitted from containers, allowing absent OIDC settings.
[Stop](https://docs.docker.com/reference/cli/docker/compose/stop/) retains containers;
[down](https://docs.docker.com/reference/cli/docker/compose/down/) retains named
volumes unless volume removal is requested. Tested Compose version: 5.5.0.

## Local evidence

Before conversion: actual readiness passed, seven Brains, one account, 33 model
request rows, 26 migrations, no active jobs and no configured MCP connections.
Native API/worker process working directories matched this repository. A private
database dump and inventory were saved under `.cache/root-compose-20260926/`.

Confidence: verified locally on macOS/Docker Desktop with Compose 5.5.0.

- Built `recollect-dev:local` from the existing product Dockerfile, including
  locked Rust build, API generation, TypeScript and Vite production assets.
- The original native API/worker drained after exact PID/executable/working-directory
  verification. Four root services now run; migration exits successfully with 0.
- Actual Chrome owner login, seven-Brain dashboard, runtime diagnostics and HTTP
  readiness pass. Browser screenshot/report are in the private evidence directory.
- Exact Brain/account identities and migration/model/MCP counts match before,
  after conversion and after complete stop/start. All 33 existing model-request
  rows remain, with no new model calls. API and worker can read existing artifacts
  and deletion/analytics journals under the selected host UID/GID.
- A deliberate migration exit 42 leaves API and worker stopped. Normal startup
  subsequently recreates the migration role, succeeds and restores readiness.
  This is a startup-gating probe, not a new schema-failure or recovery evaluation.
- Container and native launchers reject a concurrent API. Effective API/worker
  environments omit administrative DB and Vault-root inputs and unset OIDC keys.
  Compose inspection can retain bare unset key names; effective process environment
  was checked instead of treating those metadata markers as configured values.
- Stopped six verified saved proof installations and three labeled standalone
  Vault-database/SFTP fixtures after checking no proof scripts were active. Kept
  every container, volume and saved fixture input. No external Vault changes or
  revocations occurred. Cognee remains untouched. Only the four normal application
  services are running after completion.
- Three focused setup/configuration tests, shell syntax, `./scripts/validate.sh`
  (32 governance tests), standard Compose rendering and `git diff --check` pass.

Private evidence: `pre-compose.dump`, `before.json`, `after-conversion.json`,
`after-restart.json`, `verification.json`, `browser.json`, `browser.png`, startup/
refusal/restart logs and `stopped-fixtures.json` under `.cache/root-compose-20260926/`.
The image build log is `.cache/root-compose-build.log`. Recheck runtime health
after subsequent changes; this observation does not promise continuous uptime.

## Subsequent authorized fixture deletion

After the stack was verified, the user explicitly requested deletion of unused
containers and unnecessary data. Using fresh stopped-state observations, saved
installation identities, fixture labels, local Vault boundary records and exact
mounts, removed **50 Recollect proof containers, 49 exclusively owned volumes,
9 networks and 10 corresponding saved fixture directories**. No retained container
shared a selected volume or deleted bind directory. Deletion used explicit IDs
and names without force or a global prune. The local operation made no external
Vault call and did not revoke tokens/leases.

Docker-reported container storage fell from 3.688 GB to 1.033 GB and local-volume
storage from 9.276 GB to 2.946 GB: approximately 8.985 GB less logical Docker data.
This is not a claim about host filesystem free space or sparse disk compaction.
Historical proof reports, source code, images and unknown/unrelated data remain.
The separate stopped `cognee-local` project is outside this cleanup.

Post-deletion: four normal services running and the successful migration role
retained; readiness 200; the same seven Brain IDs, account, migration/model/MCP
inventory. Private manifest and result: `cleanup-plan.json`, `cleanup-result.json`
and `cleanup.log` under `.cache/root-compose-20260926/`. The deleted proof runtime
data/configuration no longer exists; earlier records of those running fixtures
are historical evidence, not restartable installations.

## Limits

This follows the existing personal development topology; no shared HTTPS,
off-machine deployment, new model requests or optional provider execution is
claimed. Named installations retain their existing separate storage and runtime
proof. UI changes are a subsequent user task. Version N/A; uncommitted.
