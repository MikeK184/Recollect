# Personal and shared installation

Status: delivered locally. Actual personal/shared containers, desktop/API accounts,
native HTTPS pairing/MCP, persistent state and operational failure behavior pass.
The [contract](../contracts/operations-local-and-shared.md) and
[installed acceptance](../mappings/installation-2026-09-26.md) record the evidence.

## Existing local app: one Compose group

Run from the repository root with Docker/Compose and Python 3.11+:

```sh
./scripts/stack.sh up --build
./scripts/stack.sh status
```

Open **http://127.0.0.1:8787** and sign in with `RECOLLECT_OWNER_USERNAME` and
`RECOLLECT_OWNER_PASSWORD` from the ignored `.env`. The UI is built into the API
image; a separate frontend container is unnecessary. Docker Desktop groups
`postgres`, `neo4j`, `api`, `worker` and the completed `migrate` role under
**recollect**. A successfully exited migration container is expected.

```sh
./scripts/stack.sh stop          # Stop the whole normal app, preserving data
./scripts/stack.sh up            # Restart the same data and credentials
./scripts/stack.sh logs          # Follow application logs
./scripts/stack.sh update        # Encrypted checkpoint, rebuild, migrate and verify
./scripts/stack.sh down          # Remove containers/network, preserving data
```

The root `compose.yaml` is also a complete standard Compose file. After setup,
`docker compose up -d`, `docker compose stop`, `docker compose ps --all` and
`docker compose down` work directly. The repository wrapper is equivalent:
`./scripts/docker.sh compose ...`. Prefer `stack.sh update` for an existing app:
it retains a checkpoint and verifies the rebuilt API and worker. Direct
`compose restart` does not build changed code or provide that update sequence.
Never use `down --volumes` to stop an installation whose data you want to keep.

### Backup-backed current-build update

For the already running root app, install the pinned recovery tools once and run:

```sh
python3 scripts/setup-recovery-tools.py
./scripts/stack.sh update
curl -fsS http://127.0.0.1:8787/health/ready
```

The update requires the owned root API/worker, at least 12 GiB free for the build
and sufficient recovery staging space. It acquires a single update lock, records
the currently installed immutable image, and creates an encrypted checkpoint in
`.data/backups/root-local/` with the existing recovery format. A missing private
local age identity is generated once under `.data/install/root-local/` with mode
0600. Preserve that identity separately; the default checkpoint/key are local,
so this does not prove recovery after losing this machine.

The command builds this checkout, drains application roles, applies missing
migrations, and checks dependency readiness, the expected build time/revision,
the latest migration in this checkout, and matching API/worker images. It pins
Compose project, directory, environment file and configuration; inherited
`COMPOSE_PROJECT_NAME`/`COMPOSE_FILE` cannot redirect it. A failed checkpoint
prevents the build. A failed migration prevents serving. Failures retain data and
the checkpoint; they do not roll an old image back over a migrated database.

Readiness exposes `build`, `migration`, and `schema_current` without credentials.
The image contains current code and UI; the persistent database retains existing
user memory and still needs migration history. Backup/restore does not remove
that requirement. Restore uses the existing fresh named-target workflow below,
with compatible schema and the current independent erasure journal. Root restore
over the existing app is refused. The first-batch
[proof](../mappings/memory-quality-first-batch-proof-2026-10-08.md) records the
isolated database restore and inventory comparison; it is not a new full
off-machine recovery drill. `stack.sh up --build` remains the lower-level build/
migrate command and creates no checkpoint by itself. No unattended update
schedule or release publication is implied.

The root stack retains `recollect_postgres_data`, `.data/neo4j/`,
`.data/artifacts/` and `.data/erasure-journal/`. Runtime account credentials live
under `.data/runtime/`; setup preserves a legacy `.data/credentials.json` when
present and updates only that path in `.env`. It records your UID/GID so files
stay writable from either runtime. The application mounts these three narrow
directories, not the repository, `.env` or other installation credentials.
MCP receipts retain their existing location under artifacts. PostgreSQL and Neo4j
retain loopback development ports 55432 and 57474 for native tooling.

Native development remains available with `./scripts/dev.sh` after stopping the
container stack. It starts only database containers and runs the API/worker in
the terminal. Ctrl+C drains both native processes. Both launchers reject an
occupied port 8787; stop the previous API **and** worker before switching.
Old detached native processes require an explicit graceful stop after checking
their repository ownership. Neither launcher kills arbitrary host processes.

Optional model credentials remain governed by Brain policy. Custom account-file,
MCP-provider/outbox or journal-mirror paths require explicit container mounts;
the convenience launcher refuses those unsupported native-path configurations.
Use a named installation below for another origin, shared HTTPS or provider
mount customization. Native companions and private runners remain native.

Projects named `recollect-install-proof-*` and labeled Vault/SFTP fixtures are
independent test datasets from earlier acceptance runs. They are not required
by the normal app and are not included by root stack commands. Stop only verified
inactive fixtures; keep their saved installation files/volumes unless deletion
is separately intended. Do not restart failed-upgrade proof stacks blindly.
On 2026-09-26 the user explicitly authorized deletion of the old local proof
stacks and their disposable data. Those saved installations must be recreated
to rerun their tests; the normal root stack and historical reports remain.

Focused configuration/setup verification: `python3 scripts/test-root-compose.py`.
Current local conversion evidence is recorded in the
[root Compose mapping](../mappings/root-compose-2026-09-26.md).

## Prepare one installation

Use Docker/Compose and Python 3.11 or later. Image builds need at least 12 GiB of
free workspace disk; runtime memory/CPU budgets are listed in the resolved config.
The commands operate only on their saved installation name and preserve volumes
on stop. They do not attach the existing native development stack.

Prepare loopback access on an unused local port:

```sh
./scripts/install.py init personal-demo personal --port 8788
./scripts/install.py config personal-demo
```

The generated `.data/install/personal-demo/runtime.env` contains the owner login
and application/provider inputs. Initialization does not print credentials or
overwrite existing files. `config` renders actual Compose privately and prints
only a bounded service/port/resource summary. It does not contact a running API.

For shared access, supply a certificate/key matching a hostname already routed
to the selected machine:

```sh
./scripts/install.py init team shared \
  --origin https://memory.example.internal \
  --cert /path/to/certificate.pem --key /path/to/private-key.pem
./scripts/install.py config team
```

The key is mounted read-only by the proxy. Recollect does not issue a certificate,
change system trust, create Vault resources or deploy to another machine.
The shared profile publishes HTTPS only; PostgreSQL, Neo4j and the API stay on
the installation's Compose network.

## Build and operate

The verified command sequence is:

```sh
./scripts/install.py build personal-demo
./scripts/install.py start personal-demo
./scripts/install.py diagnose personal-demo
./scripts/install.py stop personal-demo
```

`start` drains an existing API/worker/proxy, waits for the databases and reruns
the explicit migration role before starting application services. A failed
migration leaves application roles stopped; it does not claim rollback or
automatically serve an older schema. `stop` preserves the installation's data.
Use the delivered [backup and recovery procedure](recovery.md) for checkpoints,
interrupted restores and rollback into a fresh compatible installation.

The generated `compose.env` can select explicit resource limits and an application
image. `overrides.yaml` can add an operator-owned provider mount or custom image.
After edits, rerun `config`; required data volumes must remain in the saved project,
and API/worker environments cannot contain administrative database or Vault-root
credential names. Build custom MCP executables into a selected image before
starting it; the normal `build` command builds this repository's application image.

No provider is enabled simply by preparing the installation. Set an OpenAI key
only when using the existing governed model features, then configure Brain policy.
Vault remains optional for each MCP connection. `mcp-bindings.json` stores only
credential references and optional Proxy sources; its parent directory is private
and the file is readable through its single explicit container mount. Put secret
values in selected runtime environment inputs or the chosen credential provider.
Central container tools require their binaries and any selected Proxy socket to
be available there. Native local/private runners retain their own OS credentials
and paths; an unused provider is never required for another connection.

## Native companions and private certificate trust

Build a bundle for the current host without editing its PATH or global settings:

```sh
./scripts/package-companion.sh .cache/packages/my-companion
```

Keep `recollect-agent`, `recollect-mcp-runner` and `recollect-mcp-bridge` together.
The bundle records its build host; a macOS binary is not a Linux binary.

For a private CA, select a client-local PEM bundle before health, pairing, capture
or MCP configuration. Publicly trusted certificates need no additional CA file.

```sh
export RECOLLECT_URL=https://memory.example.internal
export RECOLLECT_CA_FILE=/path/to/trusted-ca.pem
.cache/packages/my-companion/recollect-agent health
.cache/packages/my-companion/recollect-agent pair
```

The companion adds the selected roots to normal certificate and hostname checks;
it never disables verification. Generated coding-host settings include the
absolute CA-file path. Initial OS-store permission still belongs to that native
host. See [pairing](device-pairing.md) and [session capture](session-capture.md).

## Diagnose current state

`diagnose NAME --ca /path/to/trusted-ca.pem` checks the saved shared origin using
that CA only for this command. It reports actual container state, bounded resource
observations and HTTP liveness/readiness without dumping environment or raw logs.
Missing resource observations do not overwrite an otherwise successful Docker
connection observation. A stopped worker or unhealthy dependency prevents a
ready installation report. An unreachable Docker engine remains explicit.

The installation owner can open **Runtime diagnostics** beside desktop
installation health. The owner-only `/api/operations` reports process request
counts, response-header latency, DB-pool state and job counts under that caller's
existing Brain access. Counters reset on API restart and are observational;
Brain mutation and job history remain authoritative. Request/query/header/source
payloads are excluded, including nonstandard HTTP method text.

Local component checks are available through `./scripts/test-installation-local.sh`.
Set `RECOLLECT_TEST_CADDY` to an explicitly installed Caddy binary to include the
actual HTTPS proxy fixture; otherwise that one test reports itself skipped.
It uses temporary repository-local certificates and a loopback upstream, preserves
Host/Origin, checks trust/error/logging behavior and drains an active request.
These component checks do not replace full personal/shared installation proof.

For an explicitly owned `proof-` installation, run
`python3 scripts/test-installation-runtime.py NAME` (add `--ca FILE` for shared
HTTPS). This exercises real account isolation, accepted-work draining, dependency
failure and failed-migration recovery; use only disposable proof data. Its ignored
private report directory contains inputs for `web/tests/installation.spec.ts`.
Run that desktop case with `RECOLLECT_TEST_INSTALLATION_CONFIG` set to the absolute
`browser-credentials.json` path. The shared native fixture requires the Linux
host-test image/binaries and its saved localhost:8443 installation. It uses an
isolated OS store and browser pairing, not personal host configuration. See the
mapping for exact tested prerequisites and the browser-only certificate exception;
normal native clients always verify their selected CA and hostname.
