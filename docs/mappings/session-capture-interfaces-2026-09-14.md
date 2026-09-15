# Session capture interface evidence

Observed: 2026-09-14; native host integration refreshed 2026-09-15
Confidence: verified native, actual host, real API/database, browser and normal local runtime behavior

## Sources and method

Read the installed `codex --version`, help and relevant repository source, then
current official [Codex hooks](https://learn.chatgpt.com/docs/hooks) and
[Claude hooks](https://code.claude.com/docs/en/hooks). Context7 resolved and queried
`/llmstxt/learn_chatgpt_llms-full_txt` and `/websites/code_claude`. Local Codex is
0.154.0; Claude was absent from the macOS PATH. npm reported Claude Code 2.1.270;
the later isolated compatibility proof below exercised that release.

Hooks supply prompt/reply/tool/lifecycle data, with coverage and attribution limits.
Codex subagent events share a parent session ID; turn identity must be preserved.
Its transcript format is explicitly unstable and hosted tools are outside ordinary
tool-hook coverage. Claude's documented prompt ID requires a recent host; agent ID
distinguishes child events. No transcript scraping or unconditional coverage is inferred.

Context7 `/rusqlite/rusqlite` documents bundled SQLite and the transaction/connection
interfaces. Candidate rusqlite 0.40.1 failed Cargo resolution because its native
SQLite link conflicts with SQLx's existing `libsqlite3-sys` 0.30.1 dependency.
Selected [rusqlite 0.32.1](https://docs.rs/crate/rusqlite/0.32.1/source/Cargo.toml)
uses that compatible native binding and compiled successfully. Cargo's
[native links rule](https://doc.rust-lang.org/cargo/reference/resolver.html#links)
explains why the optional SQLx SQLite dependency still affects resolution.
The companion does not link a server
PostgreSQL driver. SQLite remains an implementation detail behind native capture.

## Cognee reference requested by the user

The separate local Cognee checkout is clean at
`c0d18c80e24b7b78918e7642c03f6f128fdd2aee`. Its catalogue entries for Codex/Claude
point to `topoteretes/cognee-integrations`; the actual hook scripts live there.
Read local `modules/integrations/plugins.py`, `plugin_status.py` and
`cognee-mcp/src/tool_registry.py` plus the MCP README. Registry identity/activity
and compact memory-tool discovery are useful patterns for later MCP slices.
Configuration/key presence is not adopted as Recollect's successful-call proof.

Fetched selected integration files at GitHub revision
`f1f92a50c4db307041ef0cf91114c2003e113abb` into ignored
`.cache/capture-research/upstream/`; neither reference checkout was modified.
Inspected both hook manifests, `store-to-session.py`, Codex `store-user-prompt.py`
and `_capture_policy.py`. Reusable patterns are event-specific adapters, redaction
before truncation, sensitive-path exclusions, explicit pending prompt identities,
buffering failed uploads and background lifecycle work. Recollect implements these
through its own immutable scope, native inbox and existing deletion authority.

The [Cognee Claude documentation](https://docs.cognee.ai/integrations/claude-code-integration)
describes automatic capture and background distillation. The direct Codex docs URL
returned a web-tool internal error; its pinned source manifest/scripts were read
instead. No Cognee service, external account or model call was started by this research.

## Native implementation and validation

The shared capture protocol and sanitizer now normalize supported event shapes,
redact recognizable/configured credentials before truncation, exclude sensitive
tool inputs and retain explicit coverage/outcome fields. The native SQLite inbox
pins known turns and tool calls to immutable capture bindings, marks unattributed
content unavailable, deduplicates pending native identities and preserves queued
content across restart. Cached policy expiry, bounded capacity/retry, tighter class
deadlines, acknowledgments and erasure use transactional storage. Deleted payload
bytes were checked in the actual database file and rollback journals.

`cargo test -p recollect-protocol -p recollect-agent --lib` passed **seven** tests:
four agent tests in 0.48s and three protocol tests in 0.01s, including five new
compound capture scenarios. Evidence: `.cache/capture-library-all.log`.
These cover six concurrent hook writers, changed defaults with delayed tool output,
missing child identity, conflicting duplicate content, restart, denied profile
preservation, queue pressure, retry readiness, stale policy, shorter retention,
acknowledgment arriving after erasure and physical payload removal. Positive
independent events remain available. Synthetic secret fixtures are not credentials.

The native checkpoint passed Clippy after four equivalent nested conditionals
were simplified; the final full-workspace result is recorded below.
Cargo now resolves rusqlite 0.32.1 with libsqlite3-sys 0.30.1 and regex 1.13.1.
The native inbox uses `secure_delete=ON`, full synchronous commits and DELETE
journaling, following [SQLite's documented controls](https://sqlite.org/pragma.html).
This is removal from controlled application files, not forensic media sanitization.

The authority/index validator passed all **32** tests before implementation.
Final in-progress validation and CodeGraph sync accompany this checkpoint.

## Server and cross-system integration

Migration 014 adds independent Brain capture policy, device/operation bindings,
content-free event metadata/receipts and opaque deletion fences. Accepted text is
stored through the existing source artifact/chunk/processing lane. Both autonomous
and legacy automatic learning inherit the source's captured selection. Publication
rechecks current device/grant/scope and capture permission; model permission remains
independent. Replay compares retained content without hashing and preserves the first
admission's policy snapshot until expiry. Native cleanup includes the capture inbox
before acknowledging a server deletion position.

Two compound real API/PostgreSQL scenarios passed in **2.40s**, using an isolated
HTTP provider fixture (`.cache/capture-server-tests.log`). Proof includes:

- Paired writer attribution, browser/foreign-device denial, policy conflicts,
  grant downgrade/device revocation, redaction at server admission and receipt
  replay after a lost response or changed policy. A closed task's original binding
  drains without adopting its newer scope.
- A source processed and learned under the standing policy actor retained the
  writer's original area selection. The resulting claim was policy-accepted with
  no reviewer or review decision. Zero calls occurred before model permission.
  A subsequent source version automatically revised the same claim while retaining
  that area selection; source-version UUID changes cannot silently widen its scope.
- A pending native body was erased before privacy acknowledgment while an independent
  body remained. A late success acknowledgment and repeated host key could not undo
  erasure. The actual SQLite file no longer contained the removed sentinel.
- The durable journal was replayed into a real PostgreSQL database copy predating
  another event and binding. Normal binding re-creation followed by upload returned
  a removed receipt and created no source/event. Thirty-day expired arrivals created
  no artifact; tighter raw/tool limits removed existing source and lifecycle details.

This restore scenario exposed an existing serialization mismatch: optional empty
`model_input_sources` arrays were present in canonical SQL closures but omitted by
the typed journal. Migration 014 normalizes only the known optional empty arrays
during replay comparison. Non-empty fences and installation identity remain exact.
The initial native sync fixture also used a path containing `..`; the fixture now
uses the existing normalized project-root helper. Both failures were resolved.

All **35** platform scenarios passed in **32.62s**, including existing retention,
access, model and publication regressions (`.cache/capture-platform-all.log`).
Full-workspace Clippy passed in **3.50s** (`.cache/capture-clippy.log`). The seven
native library scenarios also passed after inbox privacy integration. No real
OpenAI call or customer content was involved in these integration checks.
API generation validated **111** unique operations and frontend type checking
passed (`.cache/capture-api-generation.log`, `.cache/capture-ui-types.log`).

The normal application still returns `ready: true`; it has not been restarted or
migrated for this capture checkpoint. Its current UI remains available at
`http://127.0.0.1:8787`. Cognee was rechecked clean at the recorded revision.

## Intermediate checkpoint boundary

The accepted [capture contract](../contracts/evidence-session-capture.md) governs
implementation. Native normalization/inbox/restart/expiry, server admission/central
evidence, original-scope learning and cross-system deletion/restore are locally proved.
Native setup/upload and both actual host adapters are now exercised. Browser
controls/status/source inspection are implemented; final erasure-dialog proof and
installation into the normal local runtime remain open.
At that checkpoint twelve of 29 product slices were shipped;
session capture was active. The completed validation below supersedes this boundary.

## Native launch and actual hosts, 2026-09-15

Followed the user's Cognee reference through its pinned Codex README, local
marketplace and plugin manifest, not only the hook payload scripts. Cognee registers
the plugin with `codex plugin marketplace add` and `codex plugin add`; its plugin
hooks are subsequently discovered by the host. Neither raw hook command-line
overrides nor marketplace enablement overrides alone caused Codex 0.154.0 to run
the generated hooks. An absolute `--profile` path was also rejected: profiles take
plain names under the host home. Those unsuccessful candidates were removed.
The current official Codex configuration schema and plugin/CLI help were read.

Recollect now prepares a native-command plugin in its own storage and registers it
through the actual Codex installer on explicit managed launch. Claude accepts the
generated settings file directly. The generic Codex command has no global Brain
destination: it requires the per-launch `RECOLLECT_CAPTURE_SETUP` environment
binding, and succeeds without capture when absent. The launcher preserves host
arguments/terminal/exit status, drains in the background, and retains interrupted
uploads. Host upgrades refresh the binding automatically against the original
operation rather than imposing a pinned host version or adopting a changed scope.

`./scripts/test-capture-hosts.sh` builds an owned Rust 1.94/Node 22 container with
Codex **0.154.0** and Claude Code **2.1.270**. Package engine metadata required
Node 22 for that Claude release. After dependency compilation the test container
runs with networking disabled, using local synthetic Responses/Messages streams.
No personal home, authentication, customer checkout or real model key is mounted.
The host calls the actual compiled native hook, not a payload recorder or stand-in.

The compound host proof passed in **11.65s** (`.cache/capture-host-test.log`): both
hosts emitted prompt, reply, tool result, PreToolUse and session lifecycle events
that entered the native inbox. Codex's installed plugin stayed inactive for a real
unbound host session. The supervisor preserved a child exit code of 17. Unsupported
coverage and lifecycle deduplication limits remained explicit. The generated plugin
manifest passed the Plugin Creator validator; the inspected copy is
`.cache/capture-hosts/work/verified-capture-plugin`.

A real native hook process passed in **491 ms**, including quoted paths,
configured/recognizable redaction, transcript non-reading, duplicate identity,
malformed/oversized input and credential-free local status. A current-thread
runtime avoids unnecessary executor-thread startup for short companion commands.
An earlier contended build run narrowly exceeded the one-second fixture target;
the final process check passed (`.cache/capture-hook-process-test.log`).

The new setup/uploader API scenario proves configured-only setup, server commit
followed by a lost response, UUID-preserving recovery, deletion before retry,
independent publication and offline expiry. It also proves automatic host-version
rebinding to the original operation and reuse of that refreshed setup. Paired-device
reports now expose actual Brain queue/denied counts, device-wide gaps and safe issue
codes; browser impersonation and arbitrary error text are rejected, while a paired
reader can report its own denied upload. Server-confirmed last publication remains
separate from the companion's acknowledgment. All **36** platform scenarios passed
in **41.09s**; the three capture scenarios also passed together in **6.95s**.
Full-workspace Clippy passed. API generation now exposes **113** unique operations.

The capture panel provides standing-policy controls, setup guidance, configured,
queued, partial, denied and stale companion states, published activity and canonical
source viewing. The mobile source was inspected at 390 px: content fits and the
synthetic credential is redacted. Browser testing caught an incorrect source-version
erasure target and a whole-Brain query reset that closed the progress dialog. Both
were fixed: erasure targets the canonical source and resets content queries while
preserving the Brain shell and mounted progress control.

## Browser and normal runtime closeout, 2026-09-15

`./scripts/test-ui.sh tests/capture.spec.ts tests/retention.spec.ts` passed both
scenarios in **21.9s** (`.cache/capture-browser-test.log`). The tests exercise
capture policy, configured-only/queued/partial/denied/stale device reports,
redacted canonical source inspection, erasure preview/fences and completed
erasure progress, plus existing retention recovery after a lost response.
Desktop and 390 px mobile screenshots were inspected; content fits without
horizontal overflow. No source HTML is rendered.

Stopped only the verified owned prior development launcher and its two children.
`./scripts/dev.sh` preserved `.env`, PostgreSQL/Neo4j data and applied migration
**014_session_capture** at **07:08:55 UTC**. It generated 113 API operations,
built the UI and began serving at **07:09:18 UTC**. The ready endpoint returned
`true`; the application serves the new capture bundle. Evidence:
`.cache/capture-runtime-start.log`.

The ordinary application proof at **07:12:54 UTC** created the explicitly synthetic
Brain `83938975-751d-4b7d-a550-9ae601f0ecc8` (Session capture demo). A temporary
native companion paired through the real approval UI, generated setup using the
installed Codex version, and reported configured only before delivery. Two
identical native hook invocations retained one event; native drain confirmed one
publication with zero pending bytes, denials or failures. The browser displayed
the retained `Amber.port = 8080` evidence with the synthetic credential redacted.
Source `e49f2e7e-78e3-4f6b-af3a-3bec344eaa4c` and version
`f501f0ec-4ca3-4160-8e36-f849067a2a1d` remain inspectable at
`http://127.0.0.1:8787/brains/83938975-751d-4b7d-a550-9ae601f0ecc8`.
The task was closed, temporary device revoked and its OS-store credential removed.
Its later stale/offline report is expected. No personal host plugin was installed.
Proof/state/screenshots: `.cache/capture-runtime-proof/`.

All four prior Brains retain their IDs and model policies. Existing usage remains
zero for SWEG and the other initial Brain, three requests for the legacy model
demo, and six for the autonomous demo. Capture stays disabled in those Brains.
SWEG's exact claim/evidence/manifest, zero review decisions, thirty-day raw policy,
unlimited repository/claim policy and zero erasure requests were rechecked through
the browser/API (`.cache/capture-preservation-sweg.log`). No customer file content
or additional OpenAI request was used for capture runtime verification.
The two existing model demonstrations also passed their read-only preservation
paths: the legacy policy-accepted claim, autonomous 9090 revision, untested
procedure and automatically refreshed handover remain eligible with no human
reviewer. Their request histories remain three and six respectively. Today's
token usage is zero; earlier measured usage belongs to the previous UTC day.
Evidence: `.cache/capture-preservation-models.log` and
`.cache/capture-preservation-autonomous.log`.

Final `./scripts/validate.sh` passed all **32** checks in **1.412s**, and
`git diff --check` passed (`.cache/capture-final-validation.log`). CodeGraph was
synchronized after the source changes; both separate reference checkouts remain
outside the implementation write boundary, with Cognee rechecked clean.

Thirteen of 29 product slices are delivered locally. The capture pack is archived;
exact/lexical retrieval is the next eligible slice. The session-capture runbook,
README and lifecycle indexes describe this delivered boundary. Version is N/A;
changes remain uncommitted and no external deployment is claimed.
