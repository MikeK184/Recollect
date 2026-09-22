# MCP catalogue interfaces and reference findings

Observed: 2026-09-22
Confidence: verified interfaces, real database/API/browser proof and local runtime.

## Primary interfaces

The [current official tools specification](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
separates listing from calls, permits per-request authorization filtering, recommends
deterministic ordering, and treats annotations as hints. Input schemas describe
objects; output schemas can describe other JSON types. Aggregators must disambiguate
same-named tools. Recollect's slice 22 is an application catalogue API, not an MCP
wire server; actual protocol support is verified with rmcp in slice 23. The older
2025-11-25 page was also inspected and is not the current protocol baseline.

Context7 resolved `/modelcontextprotocol/modelcontextprotocol` and
`/stranger6667/jsonschema`. Registry metadata reports `jsonschema 0.57.0` (Rust 1.85
minimum) and `rmcp 3.4.0`. Only the validator is needed here. Its default features
enable file/HTTP resolution; select `default-features = false` and `offline()`.
Build validates schema syntax as well as instances. Validate with installed source
and actual tests before claiming integration. See the
[official validator](https://docs.rs/jsonschema/0.57.0/jsonschema/).

## Reference findings

The user-requested read-only Atlas/Cognee agent inspected the coordination boundary.
Cognee's `cognee-mcp/src/tool_registry.py` derives tiers from registration metadata,
which supports one source for cached advertisements and dispatch configuration.
Its `server.py` tool mode deliberately leaves hidden tools callable (also proven in
`tests/test_tool_search.py`); discovery filtering therefore cannot authorize calls.
Its `client.py:31` starts and initializes a provider before listing; this does not
provide Recollect's dormant-provider catalogue behavior. Its omitted-dataset
fallback in `server.py:725` is not reused as authority. Dataset share enforcement
is useful reference, but creator-wide grants cannot replace independent profile
Use. Atlas's explicit destination, scope key and pluggable provider patterns
support retaining one application authority across adapters.

Existing Recollect `workspace.rs` tool operations bind scope only. Brain access and
operation bindings cannot grant profile use. The foundation explicitly separates
use/manage/share, keeps targets/runner/credentials outside model arguments, and
requires approved cached discovery without launching every backend. Reuse existing
account/Brain transaction locks, OIDC membership expiry, device authority and audit.

The [accepted contract](../contracts/mcp-catalogue-and-profiles.md) records the
concrete ownership, API, limits and successor boundary. Separate reference checkouts
remain read-only. No actual external MCP call or credential connection is claimed.

## Local proof

Five integrated PostgreSQL tests passed in 4.82 seconds through the actual API and
non-owner RLS role (`crates/server/tests/platform/mcp.rs`). They exercise:

- Independent Use/Manage/Share, admin without Use, foreign IDs, raw RLS negative
  and authorized controls, private configuration redaction and Brain revocation.
- Definition import/update/disable/idempotent audit, malformed/offline schemas,
  configuration validation, duplicate tool names across connections, deterministic
  24-tool pagination, changed schema invalidation and disabling incompatible config.
- Environment/profile membership, protected environment deletion, immutable device
  operations, exact device ownership, scope changes, device mutation denial,
  revoked devices, foreign Brains and archived Brains.
- Exact issuer/current group membership, expired/empty/wrong-issuer negative cases,
  direct-grant positive controls, disabled accounts and last-self-share removal.
  Group scenarios use trusted membership fixtures; they do not claim a new live
  OIDC-provider exchange beyond the earlier identity slice's proof.
- Concurrent idempotent creates, stale edits, changed replay, capacity boundaries
  and atomic audit. A sentinel executable and bound loopback listener demonstrate
  zero provider launches/connections during cached discovery; no model/job is made.

Three affected existing workspace/team/evidence scenarios passed individually
(1.23s, 0.81s, 1.40s). Thirteen workspace tests and workspace all-target Clippy
passed. Generated API validation reports 136 unique operations. Desktop build and
typecheck passed; Vite retains a non-blocking main-bundle size warning (867.62 kB).
This is focused validation, not a fresh run of all 107 platform cases.

Two integrated desktop scenarios passed in 30.3 seconds (`web/tests/mcp.spec.ts`),
including a separately invited reader context. They configure scoped connections
and profiles, reject malformed settings, require explicit self-Use, render literal
hostile text inertly, inspect a non-object output schema, clear revoked cached
tools, close a remotely changed draft and hide failed detail until reload. The
reader independently exercises delegated Manage and Share and loses the Brain
after revocation. The fixture is imported only into the UUID-owned UI database;
the native worker drained its queue. Initial failures were readiness assertions
at the exact five-second polling boundary and a required-label selector; test
waits now allow observed polling without changing the product's interval.

Inspected desktop images are `.cache/mcp-catalogue-tools.png` and
`.cache/mcp-catalogue-desktop.png`. Normal runtime was restarted with migration
021 and the new bundle after a private pre-migration dump. Actual browser/API proof
(`.cache/mcp-catalogue-runtime.mjs`) confirms honest empty catalogue state and
disabled Add connection, with no fixture imported into the normal database. The
retained SWEG exact fact still returns one item / 1,630 bytes / 61 ms. No new model
request or customer filesystem read was needed. Normal inventory preserves the
same seven Brain UUIDs, 33 model requests and zero active jobs; migrations advance
20 to 21, and all three new catalogue inventories remain empty. Proof and dump are
under `.cache/mcp-catalogue-proof/`; runtime image/JSON have the corresponding
`.cache/mcp-catalogue-runtime` prefix. API and worker are running locally.

Governance lint/32 checker cases, formatting and whitespace checks passed at
closeout. Version N/A; all changes remain uncommitted. No MCP provider execution,
credential resolution, Vault connection or private-runner health is claimed by
this slice. Those remain required successor work.
