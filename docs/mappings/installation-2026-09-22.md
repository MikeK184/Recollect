# Installation interface and runtime evidence

Observed: 2026-09-22
Confidence: verified component evidence; historical pending items completed in
[September 26 installed acceptance](installation-2026-09-26.md).

The operational slice's actual predecessors are durable work, team access and
device pairing, all shipped. It can proceed independently of slice-25 integration.
No external deployment or new mandatory provider is authorized.

Context7 `/docker/compose` cites actual Compose implementation/tests for
`service_healthy` and `service_completed_successfully`. Official
[startup documentation](https://docs.docker.com/compose/how-tos/startup-order/)
confirms that a started container alone does not establish readiness. The
[service reference](https://docs.docker.com/reference/compose-file/services/)
documents resource limits, logging, stop signals and persistent volumes. The
locally installed Compose CLI reports v5.5.0. During initial research the Docker
daemon timed out; the approved recovery below restored connectivity.

Context7 `/websites/caddyserver_caddyfile` and the official
[TLS](https://caddyserver.com/docs/caddyfile/directives/tls),
[reverse proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
and [global options](https://caddyserver.com/docs/caddyfile/options) references
support supplied certificate/key files, HTTP upstream Host preservation and
disabling certificate automation/admin API. Native component proof now passes
below; the container installation remains unverified.

Source review confirms the existing application has distinct migration and
non-owner runtime roles, the privacy startup barrier, JSON tracing without raw
request paths, and bounded worker/MCP lanes. At initial research, native processes
waited for SIGINT only and Compose started only development databases. This slice
adds shared SIGTERM/SIGINT handling and full application packaging; the existing
development volumes and running processes have been preserved.

The Docker Hub tag endpoint could not be opened by the browser tool. A direct
read-only HTTPS API lookup then confirmed `caddy:2.11-alpine` is active and lists
amd64/arm64 images. This is availability evidence, not a pulled/running image.
Context7 `/seanmonstar/reqwest` documents additive PEM roots; installed reqwest
0.12.28 uses `add_root_certificate`, while 0.13.5 supports `tls_certs_merge`.
Both retain certificate/hostname verification. The native service client and
MCP transport share a bounded file reader and accept only explicitly selected CA
files; generated host settings retain its absolute path.

Implemented locally: application Dockerfile with generated API/web build, personal
and shared Compose files, private named installation configuration, explicit
migration startup, persistent data/receipt/journal paths, optional provider
overrides, resource/log bounds, diagnostics, native bundle, owner desktop/API
metrics and shared SIGINT/SIGTERM handling. Runtime remains on its previous native
binary; no product-image installation acceptance was claimed during the outage.

Actual independent proof:

- Two installed-Compose configuration/ownership cases pass, including both
  profiles, repeated initialization, credential separation, required storage,
  ports/resources and rejection of foreign data volumes. The final ownership/
  resource-guard run took 0.602 s in `.cache/installation-config-final.log`.
- Real native health and MCP TLS calls accept the selected CA, reject an untrusted
  chain and reject a hostname mismatch. Host settings carry only the CA path.
  Debug proof took 7.412 s; the complete release bundle repeated it in 6.192 s
  (`.cache/installation-release-tls-proof.log`). These use an owned HTTPS protocol
  fixture, not a claimed live application installation or personal Keychain.
- Three actual HTTP/cancellation/native-signal cases pass in 0.05 s, with one
  internal helper explicitly skipped by ordinary test discovery. They prove fixed
  telemetry fields, interrupted-request accounting and both OS signals through
  the shared handler, including the nonstandard-method canary
  (`.cache/installation-http-signal-current.log`). API/worker draining with real
  jobs remains open.
- The release companion and both sibling helpers built in 61 s and are installed
  together under `.cache/packages/operations-20260922/`. They are macOS ARM64
  artifacts; the build-host record is not a portability or release claim.
- API generation exposes 156 unique operations. The separate desktop bundle
  latest build takes 5.05 s, retaining the existing main-chunk warning
  (`.cache/installation-web-current.log`). No browser acceptance
  is inferred. All-target Clippy passed in 7.12 s
  (`.cache/installation-clippy-current.log`). The full workspace suite passes
  25 tests with 142 explicitly skipped fixture cases
  (`.cache/installation-workspace.log`). All 32 governance cases and
  `git diff --check` pass (`.cache/installation-governance-final.log`).

The desktop `operations.spec.ts` case is prepared for actual owner login,
loading, removal of a previous observation after a failed poll, successful retry,
and a real invited member's hidden control/API denial. Only the transient failure
is injected; account authority and successful diagnostics use the real API.
Standalone TypeScript checking and Playwright discovery pass. The browser case
subsequently passed after Docker recovery, as recorded below. Its first standalone
compile found missing Node declarations;
the Node-22 packaging baseline now includes dev-only `@types/node` 22.20.4.
The initially requested per-package README returned 404; the upstream
[DefinitelyTyped guidance](https://github.com/DefinitelyTyped/DefinitelyTyped#npm)
and actual npm metadata establish the declaration package. No runtime dependency
or product host-version gate was added.

The official [Caddy release](https://github.com/caddyserver/caddy/releases/tag/v2.11.4)
provides the macOS ARM64 binary used only under `.cache/installation-caddy/`.
`RECOLLECT_TEST_CADDY="$PWD/.cache/installation-caddy/caddy" python3 -B -m unittest
discover -s scripts/tests -p 'test_caddy.py' -v` runs the actual Caddyfile with only
fixture certificate paths/listeners/upstream substituted. Actual validation and
HTTPS calls prove trusted access, untrusted-chain/IP denial, preservation of both
trusted and hostile Origin values and Host including its port, upstream-failure
502 without request-canary logging, and active-request SIGTERM draining before
exit. The test initially expected a client hostname exception for IP access;
Caddy instead rejects certificate selection earlier with a TLS alert. The
assertion now records that actual denial without weakening TLS verification.
No certificate automation, autosaved configuration, personal trust-store change
or global installation was required. See `.cache/installation-caddy-proof.log`.
This is native proxy component evidence, not the complete shared installation.

The obsolete failed Linux target cache was removed after verifying that its
owned host-build scripts had stopped. This reclaimed about 4 GiB; the release
build completed with approximately 20 GiB still free. Sources, databases,
credentials, normal binaries and Vault processes were not removed by that cleanup.

After explicit user approval, Docker Desktop was stopped using its supported
forced-stop command when ordinary restart could not exit stuck helpers, then
started normally. Engine 29.7.2 and both normal database containers are healthy.
Fresh normal inventory, renewing central lease, readiness 200 and actual SWEG
recall pass without new paid model requests or customer repository scans. The
original API/worker remain running; no reset, prune, volume removal or Vault
revocation occurred. Evidence: `.cache/docker-recovery-20260922/`.

Actual owner/RLS diagnostics now pass against PostgreSQL in 0.70 s
(`.cache/installation-rls-proof.log`), including exclusion of inaccessible Brain
jobs and the explicit-grant control. The owner loading/failure/retry and member
denial browser flow also passes alongside MCP guidance (two tests, 19.0 s), and
both desktop screenshots were visually inspected
(`.cache/mcp-tools-installation-desktop-proof.log`, `.cache/installation-desktop.png`).
These are native fixture proofs; container installation acceptance remains open.

The actual product image `recollect-product:local` now builds successfully using
the locked Rust and Node-22 dependencies. Release server compilation takes 2m10s;
generated API/web assets are built inside the image, retaining the frontend chunk
warning. `.cache/installation-product-image.log` records the build. The new owned
personal instance `proof-personal-20260922` starts at `http://127.0.0.1:18787`:
migration exits zero, PostgreSQL/Neo4j/API are healthy, the worker is running, and
actual liveness/readiness both return 200. Installer diagnostics report resource
observations and `ready:true`. Evidence: `.cache/installation-personal-start.log`,
`.cache/installation-personal-diagnostics.json`. Its private `.data/install/` files
and dedicated volumes are separate from the normal development service. Startup
does not yet prove the remaining account, HTTPS, pairing, failure or recovery flows.

Open acceptance: actual shared container-proxy startup; personal and two-user
shared installation UI/API/paired-device flows; migration-failure
ordering, real queued-work restart/drain and dependency outage diagnostics.
No incomplete requirement is shipped or replaced by fixture-only TLS/configuration
results. These historical pending items are now complete in the
[installed acceptance](installation-2026-09-26.md) and
[archived pack](../roadmap/execution/archive/operations-local-and-shared.md).
