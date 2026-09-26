# Installed personal/shared product acceptance

Observed: 2026-09-26
Confidence: verified through the owned Docker installations, browser and native clients.

This completes the runtime evidence started in the
[September 22 mapping](installation-2026-09-22.md). It does not claim external
deployment, backup recovery or integrated capacity acceptance.

## Runtime and failure proof

The actual release image `recollect-product:local` contains the API and generated
desktop bundle. The independent saved installations are
`proof-personal-20260922` at `http://127.0.0.1:18787` and
`proof-shared-20260922` at `https://localhost:8443`. Each has its own persistent
PostgreSQL, Neo4j and application volumes. The shared proxy is actual Caddy.
Neither installation selects a paid model key or a Vault credential.

`scripts/test-installation-runtime.py NAME [--ca FILE]` requires an explicitly
named `proof-` installation. Both actual runs passed owner login, invitation and
enrollment, mutual private-Brain denial, permitted reader access, owner-only
diagnostics and invalid-Origin denial. The shared run also rejects an untrusted
CA through an ordinary verifying HTTP client. Its first run correctly rejected
the fixture CA's missing key-usage extension; the fixture was corrected without
weakening TLS verification.

The script holds a real database lock while a worker processes an accepted job,
sends SIGTERM, releases the lock and verifies successful completion before exit.
It similarly blocks an accepted API write, sends SIGINT and verifies that write
commits before an exit-zero shutdown. It waits for actual process exit before
restart; the first fixture attempt raced that exit and was corrected. Stopping
the selected Neo4j dependency produces readiness 503, liveness 200 and an explicit
disconnected graph observation. Restoring it restores readiness. A deliberately
invalid migration credential makes installer startup fail with API/worker/proxy
stopped. Restoring the original private operator file and restarting preserves
the accepted sources and authenticated sessions. Model request counts remain zero.

Passing private reports and logs:

- `.cache/installation-runtime/proof-personal-20260922-4376ed26c7/report.json`
  and `.cache/installation-personal-runtime-current.log`.
- `.cache/installation-runtime/proof-shared-20260922-1ab5b1fae9/report.json`
  and `.cache/installation-shared-runtime-current.log`.

On September 26 both fixtures and the ordinary development databases were
observed stopped. Both saved proof installations were started again successfully;
their retained source records remained readable. Their expired disposable
certificate was replaced with a fresh seven-day fixture certificate, including
critical CA/key-usage constraints. No machine trust store changed. The ordinary
development containers and the existing Cognee services were left as observed.
See `.cache/installation-{personal,shared}-resume-20260926.log` and corresponding
`diagnostics-20260926.json` files: both installations report `ready:true` with
actual HTTP checks and resource observations.

## Desktop and actual native proof

`web/tests/installation.spec.ts` reads only the selected ignored private fixture
input. It signs in as the actual owner and invited member, verifies private Brain
isolation and the shared reader UI, reads retained source evidence and opens live
owner diagnostics. The final personal case passes in 2.6 s. The first test attempted to
open installation diagnostics from a Brain page; it was corrected to use their
actual home-page location. Desktop screenshots were inspected at 1440 by 960.
Mobile remains explicitly deferred.

The shared case passes in 20.8 s. It starts the actual Linux companion bundle in
an isolated Secret Service session, pairs through the delivered browser approval
flow, and runs `installed_https_native_pairing_and_scoped_memory`. Fresh native
processes read the real persisted credential for identity and Brain discovery.
The actual stdio bridge then connects through Caddy's HTTPS endpoint, discovers
tools, starts an immutable task scope and returns its permitted source through
`memory.recall`. This is application/SDK/OS-store proof, without an HTTP protocol
substitute or a manually planted device token. Untrusted native health/MCP and
the wrong native TLS hostname are denied. The expected untrusted bridge error
appears in the passing log. There is no new macOS Keychain proof claim.

The test browser first rejects the untrusted fixture normally. Its positive UI
run uses Chromium's exception for the disposable certificate's exact public key
in that temporary browser process. This is solely browser fixture access:
[Chromium's verifier](https://chromium.googlesource.com/chromium/src/+/HEAD/services/network/ignore_errors_cert_verifier.h)
documents that the exception skips validation for the selected key. It is not CA
or hostname-validation proof; the independent Python/native calls above establish
those. Context7 `/microsoft/playwright` documents isolated contexts and temporary
browser profiles. No general `ignoreHTTPSErrors`, personal browser change or
system CA installation is used. The certificate identifier is test-only and
does not introduce product content hashes or a version gate.

Evidence: `.cache/installation-personal-desktop-20260926.log`,
`.cache/installation-shared-desktop-20260926.log`, and the shared report directory's
`native.log`, `owner-desktop.png`, `member-desktop.png` and
`native-pairing-desktop.png`. Rebuildable Linux test binaries were rebuilt after
the previous cache had been removed; `.cache/installation-native-build-20260926.log`
records that build. The separately tested macOS release bundle remains documented
in the earlier mapping.

## Boundaries and completion checks

Actual API/worker container inspection confirms no administrative DB or Vault-root
environment variable, positive memory/CPU/PID limits and 10 MiB/three-file log
rotation. Actual API/worker/proxy logs contain neither the private input canaries
nor the selected fixture credentials. Only booleans are written to
`.cache/installation-runtime-boundaries-20260926.json`. The scripted runtime test
now repeats the same private log check. The September 22 recovery inventory is
historical preservation evidence, not a claim that the stopped normal application
is currently serving. No Vault token/lease was revoked.

Current standalone TypeScript and all 32 governance cases pass. The workspace
suite has 25 passed, zero failed and 144 explicitly skipped fixture cases;
the required actual installation/native proofs are recorded separately above.
All-target Clippy passes in 36.82 s. Evidence is in
`.cache/installation-{workspace,clippy,governance}-20260926.log`. The earlier mapping
retains actual configuration, RLS, HTTP counter, proxy, API generation and web
build evidence. The installation slice is shipped locally under its
[archived pack](../roadmap/execution/archive/operations-local-and-shared.md);
the overall 29-slice goal remains incomplete.
