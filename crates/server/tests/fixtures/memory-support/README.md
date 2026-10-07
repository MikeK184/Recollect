# Synthetic memory-support baseline

`corpus.json` contains 24 calibration and 26 held-out cases. All names, messages
and states are synthetic. The citations intentionally exist even where the
assertion is unsupported: valid bounds and a correct quote do not establish the
meaning of the complete assertion.

These are assertion-level support cases. Exact offered revision identities,
typed procedure/handover fields, parent/child eligibility, in-flight policy/scope
changes and erasure/replay require separate production-handler fixtures in Phase 1;
the textual retirement/replacement examples alone do not establish those invariants.

Validate offline:

```sh
python3 scripts/memory_support_baseline.py check
python3 -B -m unittest discover -s scripts/tests -p 'test_memory_support_baseline.py' -v
```

These offline checks also run automatically in `./scripts/validate.sh`; that
standard check does not dispatch a paid model evaluation.

The native product-handler/model harness passes only evidence/candidate fields
to the gateway. `expected`, `critical`, `category` and partition labels are scoring
data. Freeze the held-out corpus and thresholds before model evaluation; do not
tune against its observed failures. A changed study needs a new version and named
comparison rather than rewriting a failed result.

Score a bounded local result artifact with:

```sh
python3 scripts/memory_support_baseline.py score RESULT.json --partition held_out
```

Result format version 1 has `version`, `corpus`, `mode` (`fixture` or `real-model`),
`versions` (requested/returned model, prompt, schema, verifier) and `observations`.
Each observation contains `case_id`, `state` (`completed`, `failed`, `uncertain`),
`verdict` (supported/contradicted/insufficient, or null for an incomplete attempt),
`usable`, `request_count`, `charged_tokens` and `latency_ms`. Count each actual
request/charge once; the baseline runs isolated cases rather than assigning one
batch's full usage to several observations. Include replacement attempt costs.
Missing cases cannot pass; duplicate case observations are rejected.

Fixture enforcement and declared model-quality thresholds are separate outputs.
A result file does not verify actual runtime/model calls; retain and validate
canonical gateway receipts separately. Offline checks load no credentials, make
no provider calls and establish no production support-gate acceptance.

The [evaluation contract](../../../../../docs/contracts/operations-integrated-evaluations.md#automatic-memory-support-baseline-2026-10-07)
and [archived pack](../../../../../docs/roadmap/execution/archive/memory-support-evaluation-baseline.md)
own this baseline. Production scope, eligibility, staging and erasure changes
belong to the separately delivered support-verification slice.

## Opt-in native installed-model evaluation

The ignored `memory_support_installed_provider_baseline_opt_in` platform test
creates owned disposable Brains/evidence, drives the native automatic support
worker and exports observations plus safe canonical gateway receipts. Output
filenames are create-new inside the crate's `.cache`; retries never overwrite an
older outcome. Gold labels and scoring categories are not model inputs. This
assertion adapter is separate from controlled typed producer-action tests.

With the existing approved provider/runtime environment and a separately approved
synthetic allowance, run from the repository root:

```sh
RECOLLECT_REAL_SUPPORT_EVAL=1 \
RECOLLECT_REAL_SUPPORT_PARTITION=held_out \
RECOLLECT_REAL_SUPPORT_RESULTS=.cache/NEW-UNUSED-RUN.json \
cargo test -p recollect-server --test platform \
  memory_support_installed_provider_baseline_opt_in -- --ignored --nocapture
```

The explicit flag dispatches paid synthetic requests; normal validation never
does. Inspect gateway receipts independently before treating a scored result as
runtime proof. The [retained comparison](../../../../../docs/mappings/memory-source-support-staging-2026-10-07.md)
and [synthetic observations](../../../../../docs/research/automatic-memory-support-results-2026-10-07.json)
report all measured versions, failures, cost and semantic limits.
