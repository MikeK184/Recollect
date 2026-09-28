# Recall canonical memory and evidence

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Purpose and prerequisites

Run the API and worker with `./scripts/dev.sh`, sign in at
`http://127.0.0.1:8787`, and open an accessible Brain. Source text becomes searchable
after its processing job finishes. No model permission, embedding or provider
call is needed for exact and lexical recall.
Optional [semantic search](semantic-recall.md) adds automatic embedding readiness
and model-backed paraphrase matching under the same canonical filters.

## Procedure

Use **Ask → Search evidence** in the Brain sidebar. Enter a subject, source title, symbol,
file path or words from retained evidence. Quoted phrases, `OR` and exclusions
use PostgreSQL web-search syntax. Search is language-neutral lexical matching;
it does not infer synonyms or produce a generated answer.

Choose **Investigation** for qualified claims and unreviewed evidence,
**Accepted and current** for eligible accepted claims, **Accepted and operationally verified**
for the additional operational requirements, or **Qualified history** for explicitly
qualified retained historical/disputed evidence. Normal model learning and
acceptance still run under the standing Brain policy; these controls are optional
ways to inspect what the system knows.

Expand **Scope, time and exact lookup** to select repositories, areas,
environment, collection and an exact revision manifest. Environment-scoped
repository evidence requires the manifest; a newest Git snapshot is not proof
of deployment. Without an environment, repository recall uses the latest published
snapshot known at the selected knowledge time. Empty dimensions are Brain-wide.

Knowledge time asks what had been recorded by that instant. Fact time asks when
a claim applies. Enter UTC timestamps; raw evidence does not acquire an inferred
fact-validity interval.
Captured sources use server receipt as knowledge time and the host timestamp for
retention. A retained claim with expired raw support can still appear in
Investigation or History with that qualification; strict modes exclude it.
Exact identity accepts a claim, source-version, repository
fact or manifest-revision UUID. Older source/fact identities require History when
they are outside the selected current version. Current erasure always applies.

Results show channels, separate claim states, qualifications and original evidence
links. **Inspect claim and history** opens the existing claim lifecycle; **Inspect evidence**
opens canonical retained bytes or structural data. An administrator can erase a
source there. Changing filters or Brain clears the prior result. Erasure clears
cached content and rereads current authority.

**Copy attributed context** copies compact structured evidence with its fixed
data-only instruction and provenance. The displayed byte count includes those
fields and fits the selected 1–32 KiB budget. A fragment is at most 2 KiB; omitted
or shortened results are labeled. Copied external text is outside browser cache
invalidation and must be discarded when its scope or source authority changes.

For the native client, pair a device as described in [device pairing](device-pairing.md).
Begin a context or retrieval operation, then use its returned operation UUID:

```sh
./target/debug/recollect-agent scope begin "$BRAIN_ID" "$TASK_ID" context
./target/debug/recollect-agent scope recall "$BRAIN_ID" "$OPERATION_ID" "Vault JWT"
```

Recall uses that operation's original selection even if the task default changes.
After changing scope, begin a new operation and request fresh context. The native
command and browser call the same `POST /api/brains/{brain}/recall` handler; host
prompt injection is supplied by the later MCP integration.

## Verification

The [retrieval evidence](../mappings/retrieval-baseline-2026-09-15.md) records actual
database/API, native and browser checks and their runtime boundary. Run the focused
checks against repository-owned services:

```sh
set -a
source .env
set +a
cargo test -p recollect-server --test platform retrieval:: -- --ignored --test-threads=1
RECOLLECT_TEST_MODEL_WORKER=1 ./scripts/test-ui.sh tests/recall.spec.ts
./scripts/validate.sh
```

The browser fixture uses a live worker to process synthetic sources; its Brain
model policy is disabled. The exact/lexical handler itself creates no model request.

## Failure and recovery

- No match: check spelling, selection and knowledge time; exact/lexical retrieval
  does not promise semantic equivalence.
- Insufficient evidence: inspect qualifications and claim states. Ranking cannot
  override a correction, withdrawal, missing source or strict eligibility rule.
- Partial coverage: allow source processing to finish, retry failed processing
  in the evidence panel, or narrow the query. At most 100 ranked rows are examined.
  Large repository/manifest JSON is indexed through its first 65,536 characters;
  exact identity remains available, with normal context limits.
- Context budget reached: narrow the query or increase the explicit budget;
  provenance is not removed to squeeze in extra results.
- Missing bytes: restore only permitted retained artifacts using the applicable
  recovery procedure. Existing indexed chunk text is not a fallback source.
- Denied scope/device: inspect current Brain membership and the operation's owner
  and selection. Re-pair only if the credential has actually been revoked.
- Overload/timeout: retry after current work finishes. Four recalls per process,
  a ten-second operation bound and two-second SQL bounds protect interactive work.
- Index maintenance: generated GIN indexes derive from canonical rows. PostgreSQL
  `REINDEX` does not replace eligibility or deletion replay. Do not rebuild from
  old exported text as an alternate truth source.
