# Personal and shared installation

Status: delivered locally. Actual personal/shared containers, desktop/API accounts,
native HTTPS pairing/MCP, persistent state and operational failure behavior pass.
The [contract](../contracts/operations-local-and-shared.md) and
[installed acceptance](../mappings/installation-2026-09-26.md) record the evidence.

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
