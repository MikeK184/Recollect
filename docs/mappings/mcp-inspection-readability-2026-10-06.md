# Readable connection metadata and equivalent Exa inspection

Observed: 2026-10-06
Confidence: verified for local source/build/invariants; observed-once for public Exa response

## Sources and Method

- User's five marked Connections comments on the existing SWEG — test Brain.
- Governing [desktop contract](../contracts/desktop-experience.md),
  [catalogue contract](../contracts/mcp-catalogue-and-profiles.md),
  [runtime contract](../contracts/mcp-runtime-and-credentials.md).
- Current React Markdown guidance through successful Context7 `/remarkjs/react-markdown`
  lookup; current TanStack Query retry guidance through Context7. Existing safe
  Markdown/lowlight/TanStack APIs reused; no dependency or remote grammar addition.
- Source: `web/src/McpPanel.tsx`, shared `ToolDescription`, `CodeBlock`, lexical
  `jsonFormatting`, `features/connections/toolDescription`, `McpCallDialog`, and
  server `mcp/definitions/equivalent_schema.rs` called only during HTTP inspection.
- Pure checks: `npx playwright test tests/json-formatting.spec.ts tests/tool-description.spec.ts --reporter=line`
  passed five tests with no browser fixtures. Actual numeric lexemes, string
  escapes, invalid/oversized/deep input and fenced/indented command preservation
  are covered. `npm run build` passed type/design checks and production build.
- Backend worker ran `cargo test -p recollect-server --lib equivalent_schema -- --nocapture`
  (three passed), and the ignored exact platform test
  `mcp::inspection::equivalent_http_inspection_is_owner_only_and_never_approves_or_calls`
  against a Harness-created PostgreSQL database (one passed; owned DB removed).
  `cargo clippy --locked -p recollect-server --lib -- -D warnings` and owned-file
  rustfmt passed. No normal installation write was needed for fixture proof.
- Root/worker `./scripts/validate.sh`: governance and 32 tests passed;
  `git diff --check` and final CodeGraph sync/status passed. Seven existing
  connection/management regressions were parsed with `--list`, not executed
  against the user's real Brain. No browser automation ran outside CUA.

## Local delivery

The standard clean-build disk requirement was not met. No guard was bypassed,
cache purged, data removed or unrelated container modified. A bounded offline
incremental Linux release build reused immutable Recollect builder
`sha256:0c6ae209db45421bd10f4b65b1e6b2b8f8585cc7980e921118e37a7c5e8f6bb3`.
Read-only source hashes proved exactly two production files differed:
`definitions.rs` and its new `equivalent_schema.rs` helper. The cached binary's
SHA-256 matched the running pre-change API. The incremental release compiled
only recollect-server (1m09s); generated OpenAPI stayed byte-identical, SHA-256
`da8f96fbd05cb6e53e3235795e52454eb7ac7b25a2d48a4ac6c8b870da77eff9`.

The freshly compiled binary and final local web build were layered on immutable
existing runtime base `sha256:7de277c25678cd9773a4fd7710aaad3c2e80169212093937183cfd491d1f5df7`.
Final image is `sha256:5c10ac7b8b8857bac433dff449791fc348b4407ef3143765ad0deb42db418ac6`.
Normal `./scripts/stack.sh up` recreated migrate/API/worker; API/worker were
healthy and readiness returned `{"ready":true}`. Running server binary SHA-256:
`34d6ceb86a7335a8e9b62427dd8325d371e72706af4802d2f833999a0cb53fb2`.
Logs/context are ignored `.cache/mcp-inspection-*`; no fresh clean-build claim.
Version N/A; work uncommitted.

## Observations

1. Actual connection environment/placement now appears as compact badges.
   Brain-wide/Recollect service were observed for existing Context7 and Exa;
   paired/private placement and named environment labels remain derived from
   actual DTO values rather than inferred provider state.
2. In a separate temporary CUA tab, 900ms read latency showed Loading approved
   tools and no fabricated zero. Tab-only blocked definition URLs showed
   Unavailable / Tool metadata unavailable / Reload tools; restoration and
   explicit reload returned both approved tools. Transient retry is bounded to
   two retries; denied/invalid requests stop immediately. All test network
   settings and viewport overrides were reset; temporary tab closed. The user's
   original tab, route and dialogs/drafts were preserved.
3. Exa's explicit anonymous server check returned Server responded · 2 tools
   listed at 11:16:25 local time. Test initializes/lists metadata, runs no tool,
   grants no permission, persists no approval and promises no future health.
   The prior false failure arose from its Draft 7 declaration meeting the strict
   2020-12 candidate validator. The supported audited assertion subset now gets
   only the root declaration converted before strict validation; all other
   values/assertions remain exact. Unsupported/dialect-dependent forms still
   fail; direct raw Draft 7 approval remains rejected.
4. Expanded Exa descriptions render their lead once; the hidden preview computed
   display:none. Labelled indented vendor prose renders as prose (zero pre blocks).
   Genuine command/fence examples remain exact. Shared JSON Format/Compact
   controls work. Final inner code computes white-space:pre, with 26/22 schema
   lines and 74/60 highlight tokens. Pixel QA found and corrected an old
   inspector inline-code rule that had collapsed line breaks despite pre text.
5. Actual historical call `d0ec9c5f-420f-47eb-8fc3-685f262a093e` shows structured
   runner/scope/request/timeout/start/finish/result rows; identifiers and scope
   retain their technical detail. Tool output is explicitly Expired; no removed
   content is exposed. Existing reader knowledge-write restriction and absent
   published observation remain visible. No prior call was replayed.

Read-only PostgreSQL comparisons before/after the live check and UI proof show
identical approved-manifest digests and three SWEG calls. The owned handler
fixture additionally proved owner-only inspection before network access,
unchanged approved definitions/mutation audit and zero tool dispatches.

## Screenshots and independent review

Actual PNGs and invariant ledger are in
[`output/mcp-inspection-review-2026-10-06/`](../../output/mcp-inspection-review-2026-10-06/):

- [Connections badges](../../output/mcp-inspection-review-2026-10-06/connections-badges.png)
  and [user's 2504×1314 layout](../../output/mcp-inspection-review-2026-10-06/connections-wide.png).
- [Loading](../../output/mcp-inspection-review-2026-10-06/tools-loading.png),
  [failed read](../../output/mcp-inspection-review-2026-10-06/tools-read-error.png),
  [Exa check](../../output/mcp-inspection-review-2026-10-06/exa-check-success.png).
- [Readable Exa descriptions/JSON](../../output/mcp-inspection-review-2026-10-06/exa-tools-pretty.png)
  and [wide proof](../../output/mcp-inspection-review-2026-10-06/exa-tools-wide.png).
- [Call summary](../../output/mcp-inspection-review-2026-10-06/recorded-call.png)
  and [whole call/expiry/capture](../../output/mcp-inspection-review-2026-10-06/recorded-call-wide.png).
- [Before/after manifest hashes and call count](../../output/mcp-inspection-review-2026-10-06/runtime-invariants.json).

Independent reviewer inspected final source and actual replacements, including
widths, the loading/failure states, multiline JSON and recorded-call authority.
Four source edge cases and the CSS whitespace issue were corrected before final
acceptance. Final review passed all five user comments with no remaining blocker.

## Translation and Limits

Approved tool discovery remains a PostgreSQL metadata read, not a remote list or
permission-to-execute promise. Use-only groups retain canonical scoped discovery
and 20-tool paging; revision/scope changes and out-of-range pages reset safely.
Authority loss hides tools/results and retains existing backend denials. New
reader paging/access behavior was source-reviewed; no extra login/grants were
created in this follow-up. JSON controls format original lexemes; they do not run
jq expressions or code. Lazy grammars remain installed/bounded.

The schema conversion is inspection-only and intentionally narrow, not a general
Draft 7 migration. Test remains anonymous; an authenticated server can reject it
although a separately approved credentialed tool call may succeed. Exa was
observed once; remote availability may change. No new Brain/policy/connection/
account/credential writes, external tool executions or release publication.
Separate OpenCode native-token acceptance remains open in its existing pack.

## Follow-up

Both current execution packs are archived as locally shipped with evidence. No
remaining action is required for these five Connections comments. Recheck remote
responses on a future explicit Test; keep unrelated native-host work separate.
