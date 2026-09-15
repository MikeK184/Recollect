# Provider gateway and learning evidence

Observed: 2026-09-14
Confidence: verified locally; no external deployment

## Sources and method

Read the accepted foundations and provider/learning, claims, review and retention
contracts. Context7 `/websites/developers_openai_api` returned current primary
[Responses migration](https://developers.openai.com/api/docs/guides/migrate-to-responses),
[response schema](https://developers.openai.com/api/reference/python/resources/responses/methods/create)
and [embedding guidance](https://developers.openai.com/api/docs/guides/embeddings).
Fetched official [Luna](https://developers.openai.com/api/docs/models/gpt-5.6-luna),
[structured outputs](https://developers.openai.com/api/docs/guides/structured-outputs)
and embedding pages. The existing Reqwest/Serde stack can use these HTTP interfaces
without introducing an SDK. No API-key helper tool is available in this session;
the user-provided ignored environment key already passed actual synthetic preflight.

The selected pair and prices remain recorded in
[the preflight mapping](openai-provider-preflight-2026-09-14.md). Default large
embeddings have 3,072 dimensions; dimension/index compatibility belongs to the
semantic retrieval slice. Gateway proof will not silently reduce dimensions.

## Local implementation and proof

Migration 011 and the shared Rust gateway implement model policy, request accounting,
bounded OpenAI calls, source learning, literal acceptance and canonical review/erasure.
The API has 101 unique operations. No SDK, content hashing or strict format gate was
introduced. Policies and input classes are checked before transmission and again
before publication. Provider data/error bodies and raw responses are not logged.

The full core integration suite passed **24 scenarios in 19.85 seconds**, excluding
optional live OIDC. Five compound model scenarios passed together in 4.18 seconds:
policy/role/content/model denial, real HTTP shape/usage, daily limits, concurrent
admission, policy changes in flight, provider failure/refusal, interrupted-call
replay, automatic learning, explicit retry, restarted state, atomic rollback,
literal acceptance, interpretation/conflict/rejection and in-flight erasure.
An additional device/foreign-input/revocation scenario passed in **1.04 seconds**,
bringing the current core scenario count to 25. It proves a real paired write
operation, reader/missing-operation denial, foreign source rejection and discarded
output after the writer loses access. Final formatting and all-target Clippy
passed after that test-only addition (2.19 seconds).

The erasure test creates an actual PostgreSQL copy before a learned claim exists.
Erasing that claim leaves its original source readable but fences model transmission.
Journal replay applies the fence to the older database even though it has no claim
row. Audit-age request detail expires while minimal accounting remains.

Test development exposed incorrect fixture URL/field/status assumptions and a
fault constraint that needed NOT VALID to avoid validating prior synthetic history.
These were corrected; failed fixture databases were deleted only after checking
their ownership comments and exact repository-local artifact paths.

## Actual OpenAI browser proof

At the isolated migration-011 runtime started at `2026-09-14T14:53:29.495208Z`,
the real-provider browser workflow passed in **13.1 seconds**. It made exactly
three fixed-synthetic calls through the product gateway: Luna connection extraction,
embedding-large at 3,072 dimensions, and Luna learning of `Amber.port = 8080`.
All three succeeded; the UI recorded 334 total tokens.

The learned claim is `model_extracted`, accepted by `literal-ports@policy-ID`,
with no reviewer ID, strict accepted eligibility and no operational verification.
The test inspected model provenance, mobile layout and a lower-budget denial that
made no additional provider call. Desktop/mobile screenshots were inspected; no
horizontal overflow or uncaught browser errors occurred. A non-searchable selector
initially blocked the test before any model call; searchable controls fixed it.
Claims/review/retention browser regression passed **three workflows in 21.5 seconds**.

## Normal local runtime and preservation

The ordinary `./scripts/dev.sh` applied migration 011 at
`2026-09-14T15:03:23.229215Z` and started the API at `15:03:32.892815Z`.
Its 101-operation browser build passed; a 739.29 kB bundle-size advisory remains
non-fatal. The existing SWEG and other Brain remained open with model transmission
disabled and zero model requests. At `15:04:51.745Z`, SWEG's original claim/revision,
repository snapshot, proposed review, zero review decisions, zero erasures and
retention policy remained unchanged; readiness was true.

A separate persistent **Model learning demo** Brain
`06b52931-7b51-4ae0-b490-14c51c8074d6` is visible at
`http://127.0.0.1:8787/brains/06b52931-7b51-4ae0-b490-14c51c8074d6`.
Its two fixed connection probes and source-learning call succeeded through the
normal native worker/gateway, with 334 total charged tokens and 3,072 dimensions.
The learned claim `1bf5bdc1-f72c-4139-87a0-937e50919ddb` is accepted under
`literal-config-demo@8e49883b-51b8-4617-80d0-0e14d03a5b06` without a reviewer.
Browser proof at `15:05:20.910Z` verified provenance, strict accepted eligibility,
no operational verification and clean mobile/desktop layout. The fixture retains
only synthetic text and metadata; no SWEG source was changed or transmitted.
An initial demo-script relative URL failed before any writes/calls and was corrected.

CodeGraph synchronized 27 changed source files. Governance and all 32 checker tests
passed after reconciliation, with `git diff --check` clean. Version N/A; uncommitted.
Persistent semantic indexes and other successor consumers remain separate slices.
