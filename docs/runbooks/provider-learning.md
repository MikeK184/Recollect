# Model policy and autonomous memory

For the current route/menu map, see the [desktop guide](desktop-experience.md).

Status: active

The normal setup now uses **Enable autonomous memory** in Ask or Settings;
new browser-created Brains include the preset. Detailed overrides below are
under **Advanced model controls**. See the [desktop guide](desktop-experience.md).

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

1. Open **Settings → AI & automation → Edit model policy** as a browser Brain admin.
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
do not silently switch models. **Activity → Model usage** separates provider completion,
discarded output, failure and uncertainty.

## Automatic learning, revision and forgetting

Supported claims, explicitly evidenced decisions and procedures become accepted
by policy without a human reviewer. Changed evidence can revise the same canonical
memory, retain alternatives as uncertain, or retire an obsolete assertion. Revisions
keep evidence and model provenance. A changed contributor automatically refreshes
its existing machine-maintained handover; see [procedures and handovers](procedures-and-handovers.md).

The worker freezes at most twelve compatible current machine-maintained targets
from the same source lineage or a sole exact assertion family found after
extraction across sessions. Selection, nullable manifest, fact-time meaning,
support and current human rules must match. This is bounded exact discovery,
not universal semantic matching. Unsupported replacement/retirement preserves
the old head; faithful same-value reuse creates no duplicate revision. Human
corrections, rejection rules and withdrawal survive later extraction. Ambiguity
records omitted coverage without a mandatory user task or stopping other work.

Model output starts with unknown fact time and declared operational status.
Procedures have no test observations until actual evidence is recorded; a model
cannot verify deployment or authorize execution. Logical forgetting excludes or
qualifies obsolete evidence. Class deadlines and the deletion journal perform
physical removal automatically; see [retention and erasure](retention-and-erasure.md).
Durable knowledge is not erased merely because it has not been retrieved recently.

## Automatic support, session digests and later context

The ordinary flow is **capture → extract → check evidence → publish supported
memory → consolidate a settled session → recall in later work**. All steps use
standing permissions and the same gateway budget. A matching citation alone
cannot establish support: the independent assessment checks the complete
assertion and must quote its exact admitted evidence. Direct unreviewed
contributions and older unchecked memory receive automatic audits before use.
Withheld outcomes remain inspectable; routine operation needs no approval queue.

A closed task or five quiet receipt minutes schedules a digest after learning
settles. Late uploads/corrections refresh its current coverage; unchanged useful
inputs need no repeated synthesis. Each digest stays within its exact
binding/task/session/child, environment and nullable manifest. Missing capture
remains missing. A digest describes supported knowledge, not proof of deployment.

With both retained-content and support-excerpt permission, the worker keeps only
minimum exact verified spans. Original class/speaker/scope still govern model
transmission. These copies can support memory after raw expiry under their own
retention; original erasure removes copies and descendants. They cannot start
another automatic extraction loop.

Normal plugin recall includes a compact supported project brief and, when
unambiguous, a current digest for the authenticated session/predecessor. It adds
no brief model call and retains the existing 8 KiB/eight-second hook limit.
Several applicable episodes omit continuation instead of guessing which is meant.
See [current local implementation and proof](../mappings/memory-source-support-staging-2026-10-07.md)
for deployment and model-quality boundaries.

## Optional explicit learning and human controls

Open retained **Sources → Source evidence → More source actions → Learn from this source**. Select repositories,
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
  availability failures and rejected incomplete/malformed results, after five
  and thirty minutes. A daily budget denial makes no provider call and waits
  until the next UTC reset within the same replacement limit. A timeout or other uncertain
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

## Processing limits and native document workflow

Settings → AI permissions → Processing limits exposes maximum output tokens,
maximum input bytes, daily token allowance and concurrent calls in the main form
below Automatic processing. Choose Edit AI permissions, adjust values, then
Save AI permissions.
Output tokens default to 4,096 for new policies; existing policies retain their
value until saved. Output exhaustion rejects a response rather than publishing
partial memory. Known invalid/incomplete results receive at most two automatic
replacements with 5/30-minute backoff; failed history stays visible.
The daily allowance resets at midnight UTC. Budget rejection does not charge a
provider call; eligible automatic learning resumes after the reset within its
two-replacement limit. Raising output tokens can require a larger per-call
reservation even when recent actual responses were smaller.

The complete primary source and provenance must fit the input byte limit.
Reconciliation adds only whole compatible memory revisions that fit the remaining
space, up to twelve. It does not truncate source evidence. `model_input_too_large`
means the primary input itself needs a larger approved limit or a deliberately
smaller retained excerpt.

The native plugin now provides `source.import`, `source.list` and `source.inspect`
for authorized documents, exact IDs and retained citation spans. Use immutable
write/read operations on the supplied task. Recollect derives the import scope
and keeps it during automatic learning and recall. The bundled memory skill
describes the workflow; no Rust repository path or private credential lookup is
part of normal memory use.


### Processing limit and protected input reasons

The dashboard distinguishes source processing from learning. Daily limit
reached means a request cannot fit the remaining conservatively reserved
allowance; it does not mean capture failed. Automatic learning can resume
after midnight UTC within its bounded retries. Semantic indexing waits for
that reset or a changed policy rather than rescheduling every minute.

Sensitive input means the existing credential/private-key detector blocked
model transmission (`model_input_sensitive`). Code excerpts containing
private-key markers can trigger this guard too. Keep the original evidence;
import a separate sanitized excerpt if model learning is wanted. Historical
`invalid_input` outcomes retain their original code and need individual inspection.

Settings → AI permissions → Processing limits keeps its explanatory text
behind the info icon next to the heading, accessible by pointer and keyboard.
