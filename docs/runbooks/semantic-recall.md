# Governed semantic search

## Prerequisites

Start Recollect with `./scripts/dev.sh` and open `http://127.0.0.1:8787`.
The API, worker, PostgreSQL/pgvector and the selected provider's credential must
be available. OpenRouter setup is covered in [model policy](provider-learning.md#select-openrouter-per-brain).
Exact/lexical search remains available without model permission; see
[canonical recall](exact-lexical-recall.md).

## Enable automatic indexing once

On a Brain page, open **Edit model policy**. Allow transmission, the embedding
purpose and the content classes this Brain may send. Enable **Build semantic search
automatically** and save. Query permission is required for semantic search itself.
The initial OpenAI model is `text-embedding-3-large` with 3,072 dimensions.
OpenRouter initially selects Qwen3 Embedding 8B with 1,024 dimensions. Provider
or dimension changes require **Save and rebuild**; old vectors cannot serve a
new profile. Automatic
learning is independent; indexing does not require someone to approve each record.

**Semantic search** in the model panel shows represented, pending, blocked/failed
and removed counts, plus recent batches and coverage. Source processing must finish
before retained text can be represented. New eligible content and existing backlog
are indexed through bounded background jobs under the standing model policy.

## Search and inspect

In **Recall memory**, expand **Scope, time and exact lookup** and select
**Semantic similarity**, alone or with exact/text search. Each explicit **Recall**
may send one query embedding. Changing filters clears prior context. Policy changes,
erasure and reindex clear displayed results without silently repeating a paid query.
Failed or interrupted searches require an explicit new submit with a new attempt ID.

Repository, collection, environment, manifest and time selection follow canonical
recall. Scope applies before distance. Strict modes contain only eligible claims;
similar raw text cannot supply acceptance or operational verification. No scoped
vectors means missing coverage and no query-model call.

Results preserve source spans and claim history. Cosine similarity measures text
relevance, not truth or support for an answer. Combined search uses RRF channel ranks
while keeping exact priority. If two fragments of one source matched differently,
the displayed semantic score follows its original fragment and the other lexical
match is qualified. **Copy attributed context** includes provenance in the budget.

**Minimum semantic similarity** is an optional explicit filter, default zero. The
[fixed-corpus results](../mappings/semantic-retrieval-2026-09-15.md) show improved
paraphrase recall and unrelated returns on unsupported questions. A numerical cutoff
does not establish a reliable answer or replace inspection of evidence.

Paired native clients use their immutable operation scope:

```sh
./target/debug/recollect-agent scope recall "$BRAIN_ID" "$OPERATION_ID" \
  "How do we recover a failed rollout?" --channels exact,lexical,semantic
```

Each explicit invocation generates a separate query-attempt UUID. Reusing the context
operation does not reuse a paid request. Scope changes require a new context operation.

## Failure and recovery

- Budget/policy/class blocks: adjust the standing allowance or approved classes
  where appropriate. Pre-admission constraints can resume automatically; no paid
  request exists for a budget-blocked attempt.
- Confirmed rate/unavailable failures: at most two recorded automatic replacements
  after five and thirty minutes. An uncertain charged attempt is never silently
  resent. Writers may choose **Start new embedding attempt** for remaining eligible
  inputs; it is a new model call, not recovery of an old response.
- Missing/unreadable input: fix source processing or permitted retained artifacts.
  Indexed text is not an alternative to canonical bytes. Removed inputs stay removed.
- Profile mismatch: an administrator uses **Rebuild semantic index** with the
  currently approved installed model. A new generation starts with partial coverage;
  old vectors are immediately excluded. Rebuilding incurs new embedding work.
- Large scope: narrow the collection or repository/environment selection. Exact
  semantic scan accepts at most 5,000 scoped representations. Discovery considers
  100 inputs per pass and at most 50,000 active entries per Brain; coverage exposes
  backlog/capacity instead of discarding canonical content.
- Timeout/denial: inspect current grants, operation scope and provider status. The
  semantic operation is bounded to 90 seconds, including the provider limit;
  SQL remains bounded to two seconds. An unavailable semantic channel is an error,
  not a successful search with an undisclosed fallback.

Erasure/expiry invalidates affected vectors and in-flight publication. Restore only
through the [privacy journal procedure](retention-and-erasure.md); an older vector
export must not resurrect canonical content. Ordinary support expiry does not
delete a separately retained claim, which remains subject to current eligibility.

## Verification

With repository-owned services running:

```sh
set -a
source .env
set +a
cargo test -p recollect-server --test platform models::semantic -- --ignored --test-threads=1
./scripts/test-ui.sh tests/recall.spec.ts
./scripts/validate.sh
```

These automated fixtures use a controlled provider, with no actual OpenAI charge.
The [implementation mapping](../mappings/semantic-retrieval-2026-09-15.md) separately
records the real-model corpus, browser/native checks, usage, limits and local demo.
