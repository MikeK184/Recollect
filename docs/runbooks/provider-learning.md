# Model policy and autonomous memory

Status: active

## Prerequisites

Start `./scripts/dev.sh` and sign in at `http://127.0.0.1:8787`. The installed
OpenAI adapter reads `OPENAI_API_KEY` from the environment. This installation uses
`gpt-5.6-luna` and `text-embedding-3-large` with 3,072 dimensions, configured by
`RECOLLECT_TEXT_MODEL`, `RECOLLECT_EMBEDDING_MODEL` and
`RECOLLECT_EMBEDDING_DIMENSIONS`. The ignored `.env` is preserved by setup.

Key presence does not prove a successful call. Each Brain starts with transmission
disabled; its policy independently grants model, purpose and content-class access.
New policy forms select autonomous maintenance. Existing saved policies preserve
their previous mode until an admin changes them. No fallback model/provider is
selected automatically.

## Policy and connectivity

1. Open **Model learning → Edit model policy** as a browser Brain admin.
2. Enable transmission and keep **Autonomous memory** selected for normal operation.
   Permit extraction, synthesis and claim content, plus the actual source classes
   to learn, such as document. Query permission supports handover titles. The fixed
   connectivity check additionally needs embedding permission.
3. Set the daily token allowance, concurrent calls and input/output limits.
   Unknown outcomes keep their conservative reservation; known usage replaces it.
4. Save once. The native worker catches up eligible retained current sources and
   responds to later evidence changes. It checks every ten seconds and resumes
   queued work after restart. Existing unscoped sources use Brain scope; the system
   does not guess a task or environment. Source contributor provenance is retained.
5. Optionally use **Check selected models**. Success confirms the actual model pair
   and vector shape through the shared gateway using only fixed synthetic text.

Policy history records immutable application revisions. If deployment model choices
change, use **Use installed models** and save an explicit new policy; old permissions
do not silently switch models. The model request list separates provider completion,
discarded output, failure and uncertainty.

## Automatic learning, revision and forgetting

Supported claims, explicitly evidenced decisions and procedures become accepted
by policy without a human reviewer. Changed evidence can revise the same canonical
memory, retain alternatives as uncertain, or retire an obsolete assertion. Revisions
keep evidence and model provenance. A changed contributor automatically refreshes
its existing machine-maintained handover; see [procedures and handovers](procedures-and-handovers.md).

The worker selects at most twelve compatible current machine-maintained targets
from the same source lineage, scope and manifest. This is bounded reconciliation,
not universal semantic matching across every source. Human corrections, rejection
rules and withdrawal survive later extraction. Ambiguity remains qualified without
creating a mandatory human review task or stopping unrelated processing.

Model output starts with unknown fact time and declared operational status.
Procedures have no test observations until actual evidence is recorded; a model
cannot verify deployment or authorize execution. Logical forgetting excludes or
qualifies obsolete evidence. Class deadlines and the deletion journal perform
physical removal automatically; see [retention and erasure](retention-and-erasure.md).
Durable knowledge is not erased merely because it has not been retrieved recently.

## Optional explicit learning and human controls

Open retained **Source evidence → Learn from this source**. Select repositories,
areas, environment and an exact manifest when applicable, then **Queue learning**.
Source text is bounded; retain an explicit smaller excerpt if necessary. Learning
runs in the model worker lane and does not block evidence capture.

Results link to **Inspect learned claim** and the optional review/correction controls.
Explicit source requests still use the Brain's current acceptance policy.

With autonomous memory disabled, legacy learning produces proposals by default.
A named literal configuration rule can accept exact lines such as
`Amber.port = 8080` when the source class, collection and property are permitted.
The accepted claim records the rule/policy ID and has no reviewer. Interpretations,
conflicts and rejected values remain qualified. The separate legacy automatic-source
switch learns only newly processed sources; it does not backfill existing evidence.
Neither mode can override rejection rules or modify an existing human decision.

## Failure, retention and recovery

- Policy/role/model/content denial occurs before transmission. Adjust the actual
  Brain policy or scope; capture permission alone is insufficient.
- Provider failures have safe codes; no response body is logged. Autonomous work
  can create at most two separately recorded retries for known rate-limit or
  availability failures, after five and thirty minutes. A timeout or other uncertain
  outcome keeps its reservation and does not automatically spend again. After
  diagnosing the service, **Start new learning attempt** records an explicit new
  attempt. This is service recovery, not approval of individual memories.
- A repeated interrupted command uses its original request key. Gateway calls are
  not implicitly repeated when a response was lost or a process restarted.
- Permission, policy, lease or input changes during a call discard output. Already
  transmitted data cannot be recalled from the provider.
- Erasure cancels dependent work. Erasing a source-supported claim also fences its
  exact supporting source versions from further model transmission, while retained
  source evidence remains manually inspectable. The preview reports this effect.
  The [erasure journal](retention-and-erasure.md) carries those opaque source IDs so
  restoring a database from before the claim existed cannot regenerate it.
- Request detail follows audit retention; minimal accounting/fence identities remain.
  Claim-owned derivation provenance follows claim retention.

## Verification

The core suite includes controlled local HTTP fixtures and actual older-database
restore/replay, without sending customer data:

```sh
./scripts/test-platform.sh
cargo clippy --workspace --all-targets -- -D warnings
./scripts/validate.sh
```

Opt in to three small real OpenAI calls through a disposable browser-test Brain:

```sh
RECOLLECT_TEST_OPENAI=1 RECOLLECT_TEST_MODEL_WORKER=1 \
  ./scripts/test-ui.sh tests/models.spec.ts
```

This starts a native model worker during the UI test, verifies the selected pair,
learns the fixed synthetic literal and checks policy acceptance, provenance, token
limits and mobile layout. Without opt-in, the real-provider browser test is skipped.
Its database and artifacts are removed by the owned test fixture. See the
[provider proof](../mappings/provider-learning-proof-2026-09-14.md).

The autonomous closed-loop browser proof uses four synthetic Luna calls:

```sh
RECOLLECT_TEST_OPENAI=1 RECOLLECT_TEST_MODEL_WORKER=1 \
  ./scripts/test-ui.sh tests/autonomous.spec.ts
```

It checks learning, same-identity revision and handover refresh without individual
review. Controlled HTTP/database tests cover retirement, bounded retries, expiry,
in-flight erasure and older-database replay. The [maintenance proof](../mappings/procedures-and-handovers-proof-2026-09-14.md)
also records the persistent synthetic demo and its visible provider timeout.
Persistent semantic indexes, graph inference and host capture remain later consumers.
A successful embedding call does not establish semantic retrieval or approximate
index support for the chosen dimensions.
