# Development tooling

Reviewed: 2026-10-19 against the repository configuration at `f1b6bb3`.
Recollect's product is Rust with React/Vite, PostgreSQL/pgvector and
Neo4j/GDS. The [technology baseline](../foundation/techstack.md) and accepted
[runtime ADR](../adr/0003-product-runtime.md) govern implementation. This
runbook is the single record of the developer tooling installed in the
repository, how to set it up and how to verify it. It replaces ADRs 0002 and
0019, their contracts and the tooling slices; see [History](#history).

## What is installed

| Task | Tool and boundary |
| --- | --- |
| Rust definitions, references, types, diagnostics | Rust Analyzer MCP (`zeenix/rust-analyzer-mcp` 0.4.0 over rust-analyzer 2026-10-05); allow indexing to finish before trusting results |
| Repository context and call traces | Graft MCP (`@nanonets/graft` 0.21.1, npm-locked); automatic refresh is structural only |
| Literal, configuration and documentation lookup | `rg` and targeted file reads |
| Current dependency guidance | Context7 and primary official interfaces; record failed or stale lookups |
| Authority and current-state routing | [recollect-doc-router](../../.agents/skills/recollect-doc-router/SKILL.md) |
| Documentation edits and lifecycle reconciliation | [recollect-doc-maintainer](../../.agents/skills/recollect-doc-maintainer/SKILL.md) |
| Browser and native product checks | Existing browser-control tools and the repository's Playwright/native fixtures |

CodeGraph (ADR 0002) was retired on 2026-10-19: Rust Analyzer and Graft are
the two navigation tools, and `rg` covers literal lookup. Graft is an upstream
Node developer dependency; it is not linked into the product or a deployed service. Developer graphs do not prove product runtime
behavior and do not create Brain topics. The product's plugins, Brain-managed
MCP connections and private runner are separate from these developer MCPs.

## Host configuration

`AGENTS.md` is the only shared instruction prose. Role definitions live once, in
`.claude/agents/*.md` (explorer, architect, worker, qa, reviewer; explorer and
reviewer disallow edit tools); the main session is `default`. Each other host
keeps only what it requires:

| Host | Files | Contents |
| --- | --- | --- |
| Codex | `.codex/config.toml`, `.codex/agents/*.toml` | Six role registrations, stub role files pointing at `.claude/agents/`, and the Context7, Rust Analyzer and Graft stanzas |
| Claude Code | `CLAUDE.md`, `.mcp.json`, `.claude/settings.json`, `.claude/agents/`, `.claude/skills/` | Imports `AGENTS.md`; the same three MCP command stanzas as Codex; denies `rust_analyzer_format`, `code_actions`, `rename` and `set_workspace`; skills are links to `.agents/skills/` |
| OpenCode | `opencode.json`, `.opencode/agents/*.md` | Context7 and an isolated chrome-devtools server; stub role files pointing at `.claude/agents/` |

When a role changes, edit `.claude/agents/<role>.md` only. When an MCP command
changes, update `.codex/config.toml` and `.mcp.json` together; the linter does not
compare them. Personal models, permissions and UI settings stay outside project
configuration, and specialist agents run only when delegation is requested.
`CONTEXT7_API_KEY` is optional and forwarded by name only. Claude Code asks once
to approve the project's `.mcp.json` servers; that approval stays personal.
Workspace rebinding and mutating refactors are never exposed.

## Rust Analyzer MCP and Graft

Verified host: Apple Silicon macOS, Rust 1.94.0 with rust-src, Node 26.5.0,
npm 11.17.0, Git, curl, gzip, shasum and `/usr/bin/sandbox-exec`. No model key
is needed. Setup downloads public dependencies; runtime launchers deny
networking and writes outside this checkout. Other hosts fail explicitly. The
rust-analyzer download is checked against a pinned SHA-256 and discarded on a
mismatch before it is decompressed or run (see [Supply-chain pins](#supply-chain-pins)).

```bash
./scripts/setup-rust-navigation.sh
codex mcp get rust_analyzer
codex mcp get graft
```

Setup installs the pinned bridge, rust-analyzer and Graft under `.codex/tools/`
and keeps downloads, caches and indexes in ignored `.cache/`. It uses an owned,
compiler-specific copy of the standard-library sources and caches dependencies
for offline analysis; a compiler change selects a different cache, so rerun
setup if it is missing. It does not modify home rustup or Codex defaults, run
Graft `init`, add hooks, attach a remote Brain or use `--deep` summaries.
Launchers disable telemetry, point dotenv loading at `/dev/null`, strip model and
Brain credentials, and run under the shared repository-write-only,
network-denied sandbox policy. Reopen the project or start a new session to
expose newly registered tools.

Rust positions are zero-based lines and UTF-16 offsets. An empty or
`content modified` response while loading is not an absence result. Diagnostics
can run Cargo checks and may be incomplete after the upstream timeout. Graft's
launcher selects `.cache/rust-navigation/graft-graph` for CLI and MCP calls.
Ordinary queries refresh structural data only; after source edits, refresh
compiler-resolved edges explicitly:

```bash
./.codex/tools/graft/graft build --lsp --no-follow-nested-repos --no-follow-submodules
./.codex/tools/graft/graft check
```

A failed refresh can return the previous graph: inspect freshness and confirm
ambiguous relationships in source. `graft_check_freshness` prints `NO GRAPH`
for the missing deep manifest; the separate `graph check: OK` verifies the
structural index. Do not run paid summarization to clear it.

## Verification

A registration or tool listing is not a connection. The Rust proof client loads
the actual project registrations, launches both servers from `crates/server`,
checks the sandbox (a home-config write-open and a loopback connection are
denied while a repository canary write succeeds and home bytes stay equal),
resolves a cross-module Rust definition, hover and references, and calls real
Graft file, caller, context and freshness tools:

```bash
CARGO_HOME="$PWD/.cache/rust-navigation/cargo" \
CARGO_TARGET_DIR="$PWD/.cache/rust-navigation/proof-target" \
cargo run --offline --manifest-path .codex/tools/rust-navigation-proof/Cargo.toml
./scripts/validate.sh
```

Expect isolation, Rust MCP and Graft MCP PASS. Receipts stay under
`.cache/rust-navigation/` and contain only local code navigation. The
2026-10-09 proof resolved `db::preparation_tx` across modules and built a
Graft graph of 4,424 nodes, 12,128 edges and 447 files (3,003 LSP-resolved
edges); later runs resolved graph, semantic-queue and support-discovery call
sites. Graft caller output is approximate for generic names. Exposure in a
currently open host session is separate from these stdio calls. For Claude Code
check `claude mcp list`, `/memory` and `/agents`, then make a read-only call
such as a Context7 library lookup.

## Checks for current implementation

```sh
cargo fmt --all --check
cargo check --workspace --all-targets
cargo test --workspace --lib
npm --prefix web run typecheck
python3 scripts/check-pins.py
./scripts/validate.sh
```

After API changes run `./scripts/generate-api.sh` and rebuild the web client.
Use the [local development runbook](local-development.md) for PostgreSQL/Neo4j
and focused native/browser tests; a fast unit suite does not prove ignored
native paths. For documentation or public-guide changes also run
`npm --prefix site run build` and `npm --prefix site run check`. The
[Pages workflow](../../.github/workflows/pages.yml) builds the site and runs
governance validation; it publishes static documentation, not the app.

## Duplicate Rust dependencies

`cargo tree -d --workspace -e normal,build --target all` lists every crate that
resolves to more than one version. Reviewed 2026-10-19: after the
`platform-oidc-shared-tls` and `platform-dependency-dedupe` slices, 30 crate names
(63 versions) remain, down from 32 (68). The slices removed reqwest 0.12 with its
bundled roots (OIDC now shares the workspace's reqwest 0.13 client), hashlink 0.9,
hashbrown 0.14 and, from `Cargo.lock`, flume, sha1, sqlx-mysql and sqlx-sqlite. Every
duplicate left is pinned by an upstream crate rather than by a Recollect direct
dependency, so the next pass starts here:

| Duplicated crates | Held back by | Unblocks when |
| --- | --- | --- |
| base64 0.21, thiserror 1 | openidconnect 4.0.1 and oauth2 5.0.0, both the latest releases | a new openidconnect, or replacing it |
| rand 0.8, rand_core 0.6, the older RustCrypto line (sha2, digest, hmac, hkdf 0.10 to 0.12, cpufeatures 0.2) | openidconnect/oauth2 (p256, rsa, ed25519) and sqlx-postgres 0.8.6 (SCRAM, md-5); both must move, and the newer line that argon2 0.6 and keyring's secret-service use is not the cause | openidconnect and sqlx 0.9 |
| getrandom 0.2 | ring 0.17 (rustls) and oauth2 | those crates moving to getrandom 0.3 or later |
| hashbrown 0.15, foldhash 0.1, webpki-roots 0.26 (with its 1.0 re-export), windows-sys/targets 0.48, redox_syscall 0.9 | sqlx 0.8.6 (sqlx-core, sqlx-postgres through etcetera and whoami) | the separate sqlx 0.9 slice (about 3,137 call sites); sqlx also verifies database TLS with bundled webpki roots, not the system store |
| getrandom 0.3 and r-efi 5 | jsonschema 0.57 to 0.59 through ahash 0.8 | ahash or jsonschema moving to getrandom 0.4 |
| miniz_oxide 0.8 and 0.9 | png 0.18 (image 0.25.10): its own and flate2's | a png release on one miniz_oxide |
| syn 2 and syn 3, darling 0.24 | async-trait, rmcp, zbus and darling are on syn 3; most other macros on syn 2 | the ecosystem finishing the syn 3 move |
| base64 0.22 and 0.23 | axum and hyper-util on 0.22; reqwest, rmcp and a2a on 0.23 | axum and hyper-util moving to 0.23 |
| windows-sys 0.52 and 0.61 (Windows only) | ring, rustls and the platform verifier on 0.52 | those crates moving to 0.61 |

Two constraints shaped the last pass:

- rusqlite 0.33 and later share `links = "sqlite3"` with sqlx-sqlite 0.8, whose
  `libsqlite3-sys ^0.30` cannot coexist with rusqlite's newer one. Cargo admits
  sqlx-sqlite to the lockfile resolution through the facade's weak features
  (`sqlx-sqlite?/uuid`, `?/chrono`, `?/json`) even though SQLite is never built, so
  the workspace enables `uuid`, `chrono` and `json` on `sqlx-core` and
  `sqlx-postgres` directly. Keep that split until sqlx moves: sqlx-sqlite 0.9 accepts
  `libsqlite3-sys <0.38`, so with the facade features restored the highest compatible
  rusqlite is 0.39, not 0.40.
- rusqlite uses `default-features = false, features = ["bundled"]`. The statement
  cache and hashlink arrive only with its `cache` feature, and no Recollect code
  calls `prepare_cached`.

## Supply-chain pins

Everything the repository pulls from outside is pinned, and `scripts/check-pins.py`
(offline, standard library only) fails on a gap. `./scripts/validate.sh` runs it with
its unit tests, and the `pins` job of the [code workflow](../../.github/workflows/code.yml)
runs it on every push and pull request.

| Reference | Required form |
| --- | --- |
| Dockerfile `FROM` and `COPY --from`, Compose `image:`, third-party images started by `scripts/` and `infra/` | `name:tag@sha256:<multi-arch index digest>` on one line, so Renovate can track the tag |
| GitHub Actions `uses:` | `owner/repo@<40-hex commit SHA> # tag` |
| Downloaded tool archives | A SHA-256 table or constant in the setup script. A mismatch stops before anything is extracted or executed: age in `setup-recovery-tools.py`, Enola in `setup-enola.py`, rust-analyzer in `setup-rust-navigation.sh` |

Locally built images (`*:local`, `${VAR:-name:local}`), Dockerfile stage aliases and
local `./` actions are exempt. The checker proves the form of a pin, not that it is
current or that the digest exists; it never touches the network.

To refresh an image, resolve its tag to the index digest (not one platform's
manifest) and replace the tag and digest together everywhere `rg 'NAME:TAG'` finds
them, then rebuild the affected images. On Docker Desktop, `./scripts/docker.sh`
passes a private `--config` that hides the buildx plugin, so run this read-only
lookup with `docker` directly:

```sh
docker buildx imagetools inspect NAME:TAG
```

To bump a downloaded tool:

- **age** (`scripts/setup-recovery-tools.py`): verify the release's `.proof` once with
  sigsum-verify (the age repository's `SIGSUM.md`), then update the `SHA256` table, the
  `AGE_*` build arguments in `infra/product/Dockerfile`, `scripts/update-local.py`,
  `scripts/test-recovery-runtime.py` and [recovery](recovery.md) together. age publishes
  no checksum file, so the values come from the GitHub asset digests.
- **Enola** (`scripts/setup-enola.py`): copy the four values from the release's
  `enola-<version>-<system>-<arch>.sha256` assets into `SHA256`.
- **rust-analyzer** (`scripts/setup-rust-navigation.sh`): update the release URL and
  `RECOLLECT_RUST_ANALYZER_SHA256` (the asset digest GitHub shows) together.

`scripts/tests/test_check_pins.py` covers the checker and guards the repository itself;
`scripts/tests/test_download_pins.py` proves with synthetic archives that a mismatching
download is rejected before extraction or execution and a matching one installs.

## Scope and recovery

Keep setup, caches and launchers inside this repository. When caches are
missing, rerun the setup script; there is no global fallback and no home-level
configuration. Missing rust-src is reported as a prerequisite. Preserve ignored
reference checkouts and unrelated running services. Add another tool or skill
only when a concrete task needs it.

## History

ADRs [0002](../adr/0002-local-codegraph-navigation.md) and
[0019](../adr/0019-rust-semantic-developer-navigation.md) are superseded by
this runbook. Their contracts, the developer-tooling and governance slice packs
(Rust navigation, Claude Code, OpenCode, CodeGraph, documentation skills,
bootstrap, documentation cleanup and the product docs site), the CodeGraph
launcher, setup script and runbook and the Rust
navigation and public-site mappings were removed on 2026-10-19. They remain
readable at commit
[`f1b6bb3`](https://github.com/MikeK184/Recollect/tree/1e49d772360b1ea12ddf198a5c3dbbed0f714b1b/docs).
