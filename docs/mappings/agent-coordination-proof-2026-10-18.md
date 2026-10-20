# Cross-machine agent coordination proof

Observed: 2026-10-18
Confidence: observed-once

## Sources and Method

Status: passed local acceptance. The final process exited 0 at
`2026-10-17T23:12:22Z` (October 18 in Europe/Vienna), after rebuilding the Linux
plugin from the final source. Its retained receipt is
`.cache/agent-coordination/proof-c00339f9482e/resources.json`.

Authority is [ADR 0023](../adr/0023-agent-to-agent-coordination.md) and the
[coordination contract](../contracts/mcp-agent-coordination.md). The
[research record](agent-coordination-a2a-research-2026-10-18.md) retains official
A2A, Context7, native-host and agentgateway findings. The
[runbook](../runbooks/agent-coordination.md) contains reproducible commands.

Local implementation uses protocol DTOs, server coordination/A2A routes and MCP
tools, migration 051, and the native plugin's local queue, SQLite inbox,
background transport and CLI. Fixtures are `scripts/test-agent-coordination.py`,
`scripts/build-agent-coordination-peer.sh` and `infra/agent-coordination/`.
The test runs a native macOS Rust server with fresh PostgreSQL/pgvector and
Neo4j/GDS containers and the real plugin inside Ubuntu 24.04. Synthetic accounts,
repositories, Brain names and OS-store credentials are isolated from normal data.

| Artifact | SHA-256 recorded by the final fixture |
| --- | --- |
| Native macOS server | `d0f44d23c48bf05190b5830505743630802a77d35eb6cf93384cab1e24538fb6` |
| Ubuntu native plugin | `1ad0a9c3584d69535e0206359eb11d5eab2ce0404d095a175266a96217683342` |
| Python fixture | `d4bae0663252916ae97b358dc7c31c0ef850c207bd69c3af298b058fd8265610` |

## Observations

The server's focused matcher/path tests passed four cases. The actual database
and HTTP test `coordination_authority_replay_negotiation_and_a2a` passed ownership,
path/privacy checks, file/topic matching, official SDK decoding, real MCP,
idempotent negotiation, cursor gaps, TTL/retained inbox, concurrent revisions,
reactivation capacity, scope change and stream revocation checks.

The final native library suite passed 54 tests, including 11 coordination tests for
intent extraction, sanitized goals, local inbox/deduplication, bindings, bounded
delivery, canonical deletion and serialized session-admission/deletion ordering.
Deletion tests exercise the canonical privacy entrypoint, confirm queued-content
canaries are erased, reject late/new writes and preserve another Brain's cache.

API generation passed with 200 unique operation IDs; web typechecking and design
validation passed. The repository
validation passed 35 governance tests and ten support-baseline tests; these are
additional repository checks, not substitutes for coordination behavior tests.

All 15 final fixture checks are true:

| Check | Observed result |
| --- | --- |
| Two accounts and canonical repository | Native macOS caller and Ubuntu peer resolve equivalent origins to one repository |
| Planned-file overlap | Both peers receive the warning; the same durable event appears through SSE and pull |
| Heartbeat, dedup, ack and privacy | No repeated unchanged warning; raw task/local paths and cross-Brain reads stay private |
| Negotiation and idempotency | Attributed proposal/acceptance, stable replay and conflicting-content rejection |
| Official A2A 1.0 wire | Advertised message binding returns a valid receipt and typed version/content errors |
| Real MCP | Tool discovery, peer reads and typed peer-message submission succeed |
| Concurrent updates | Stale revisions reject and simultaneous identical messages produce one receipt/event |
| Expiry and history gap | Server-time leases expire; reannounce and retained pull work; missing history is explicit |
| Restart and reconnect | Server restart retains inbox; SSE resumes from a durable cursor |
| Automatic context before edits | Native checkout identity and shared goal match before any file intention exists |
| Automatic intentions | Real Ubuntu native hooks publish the goal and planned repository-relative path |
| Native hook and CLI negotiation | Hook context and visible warning identify the peer; duplicate delivery is suppressed; CLI reply succeeds |
| Live watch | Readable account/host/device/environment plus incoming message; machine-readable stdout remains clean |
| Close and revocation | Closure, revoked Brain membership and revoked device deny further access/delivery |
| Model usage | Canonical `model_requests` count equals zero |

The receipt reports `passed: true`, `model_calls: 0` and `cleaned_up: true`.
Exact owned database, graph and Ubuntu containers were removed and the owned
native server stopped; no production resource was part of the fixture.
The same run's `native-hook.json` contains `PreToolUse` additional context and a
visible `systemMessage`. `native-watch.txt` shows, for example,
`mike-fixture · Codex · mike-mac-fixture · production-fixture` with overlap details
and an attributed proposal. These are synthetic test identities.

## Translation and Limits

The deterministic matcher calls no model. The final fixture measured a zero-row
count in the canonical `model_requests` ledger; it did not infer zero usage from a
constant counter. Native server acceptance, durable pull,
SSE receipt, hook output and model consumption are separate claims.

The Ubuntu fixture invokes the real native hook/CLI with documented host payloads.
It does not launch an installed Codex/Claude model or prove mid-turn injection,
an idle host wake-up, a second physical Mac, production deployment or public A2A
federation. Agentgateway is researched but not deployed or copied. A2A exposes
only the advertised synchronous JSON message skill; native SSE is separate.

Earlier partial runs exposed real fixture or implementation issues: missing
Neo4j bootstrap, a stale-revision test using invalid revision zero, native watch
and hook cursor competition, automatic topic matching before file edits, and a
grant-deletion HTTP status expectation. Corrections were retained in source;
partial receipts do not count as successful final proof.
Final source review also closed a local deletion/session-creation race with a
shared fail-fast lifecycle lock before the final binary rebuild and 15-check run.

## Follow-up

The [archived pack](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/mcp-agent-coordination.md) records
delivered scope. Version: N/A; no release was requested. Commit: uncommitted.
An installed-host or production rollout needs its own dated evidence; this
local acceptance creates no deployment or universal host-interruption claim.
