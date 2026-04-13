# Evaluate the installed product

## Purpose and Prerequisites

Use this opt-in synthetic harness with a named personal installation whose name
starts with `proof-integrated-`. It creates proof accounts, paired devices, Brains,
repository identities and retained evidence through existing APIs. The declared
structural graph is a synthetic committed-publication fixture. It does not scan
customer repositories. Python 3, Docker/Compose and the built application image
are required. Keep run directories directly inside the ignored `.cache/`.

The [contract](../contracts/operations-integrated-evaluations.md) defines fixed
labels, budgets and acceptance gates. The [dated report](../mappings/integrated-evaluations-2026-09-26.md)
records results, failures and the measured machine. Its small synthetic sample
does not establish customer-corpus capacity or general model superiority.

## Procedure

Initialize, configure, build and start the selected installation using the
[installation runbook](installation.md). For the opt-in quality experiment, put
the approved `OPENAI_API_KEY` only in that installation's ignored `runtime.env`
and restart the selected installation to load it. Use `gpt-5.6-luna` and
`text-embedding-3-large` at 3,072 dimensions for this declared comparison.
The provider is accessed only through the existing governed gateway.

Choose a new quality directory for each intentionally new paid experiment:

```sh
python3 scripts/evaluate-integrated.py proof-integrated-example .cache/integrated-quality-example prepare
python3 scripts/evaluate-integrated.py proof-integrated-example .cache/integrated-quality-example quality
python3 scripts/evaluate-integrated.py proof-integrated-example .cache/integrated-quality-example synthesize
```

Preparation imports seven documents and ten claims and waits for their graph and
semantic representations. Quality runs all 32 recalls before generation and then
disables automatic embedding. Synthesis generates only for results with claim
contributors. Inspect `synthetic-answers.json` against the fixed questions/facts;
the command does not automatically grade factual usefulness. Preserve annotations,
misses, uncertainty and actual provider usage alongside the retrieval report.
These files contain synthetic content; run state can also contain credentials
and must remain private.

Use a separate, new directory for each workload attempt:

```sh
python3 scripts/evaluate-integrated.py proof-integrated-example .cache/integrated-workload-example workload
```

The workload Brain disables model transmission. The harness uses 50 repository
identities, two accounts/environments, 200 documents, a 100-node/200-edge graph,
four capture producers and four recall readers, 30 operations per caller, and
four queued native GDS analyses. It tests original task attribution after default
scope changes and requires a fresh WCC result after writes settle. Finish image
builds and unrelated proof load before timing it; preserve application resource,
worker and request limits.

## Verification

Require a zero command exit and `stage: complete` in the workload state. Inspect
`workload-report.json` for recall and capture-admission p95 below two seconds,
all accepted events readable within 60 seconds, final queue drain within 180
seconds, zero scope leaks/unexpected failures and zero workload model requests.
Explicit recall-capacity refusals and their bounded caller wait remain visible.
Resource samples are periodic observations, not continuous peaks.

Quality requires all six full-fusion target records, at least four useful full
answers, correct source/revision identity and no unsupported factual completion
or authority assertions. Unsupported questions can still retrieve unrelated
semantic context; record that behavior. Actual usage is token accounting, not
an invoice or evidence of a particular cached-input discount.

## Failure and Recovery

Keep failed run directories. The harness refuses an existing workload directory;
use a new one only for an intentional new attempt after diagnosing the failure.
Do not reduce workload size or relax gates to obtain a pass.

Quality records identities before dispatch. An unresolved paid-capable attempt
stops blind resending. Inspect canonical model/run state first; received responses
can finish validation and known synthesis runs can be polled without new billing.
Use the `embed` stage only to resume an already admitted preparation. A recorded
miss is not permission for repeated generations until an answer looks better.

The separate [recovery procedure](recovery.md) handles backups/restoration; this
harness does not reset data, revoke Vault tokens/leases, or clean other services.
