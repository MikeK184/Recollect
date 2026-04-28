# 0004: Exact committed repository publication

Status: accepted

## Decision

The Rust companion reads a locally available commit using Git object plumbing,
materializes permitted regular-file bytes in its own isolated staging directory
and invokes Enola as an external extraction adapter. It never extracts from the
ordinary working copy or uses Git archive substitutions as canonical bytes.
Recollect's full origin and repository UUID remain authoritative; Enola labels,
upstream digests and its shortened remote spelling do not establish identity.

Use Enola facts.jsonl, insights.json and receipt.json behind Recollect's typed
publication envelope. Retain immutable UUID-identified artifacts, exact recorded
commit, adapter/settings, optional permitted file text, coverage and contributor
history in the shared service. Compare normalized values for duplicate admission;
do not introduce custom hashes or strict format/version handshakes.

Run the extractor with an explicit configuration and minimal child environment,
without provider credentials, personal HOME, update checks, external fact
providers, history or incremental persistence. Output and temporary paths stay
inside the owned staging tree. A missing personal home skips Enola's optional
global receipt without preventing extraction, as verified in the fixture.

Repository artifacts are evidence. Static code references, remote module names,
heuristic insights, intended revisions and recorded deployment observations keep
their distinct meanings. The first adapter provides verified Rust, TypeScript
and Terraform coverage; Kubernetes YAML and other unsupported inputs retain file
inventory/availability and explicit coverage gaps. It does not fabricate parsers
or runtime relationships for those formats.

## Why

The [foundations](../foundation/techstack.md#extraction-capture-and-models) select
Enola subject to fixture proof. The user authorized the complete product and
routine architecture decisions under the [development posture](0003-product-runtime.md).
The [adapter experiment](../mappings/enola-extraction-proof-2026-09-14.md) verifies
the native artifact interface and exposes archive, personal-state and language
coverage constraints before implementation. The shared service must remain useful
when a contributor or its ordinary source checkout is offline.

## Consequences

The [publication contract](../contracts/evidence-repository-publication.md) owns
bounded extraction, source policy, admission, immutable artifacts, duplicate
handling, task attribution, durable processing and revision manifests. PostgreSQL
is canonical; Neo4j projection and repository linking remain in graph intelligence.
Optional retained text uses the same artifact authority and later erasure rules
as other evidence, rather than exposing arbitrary files on a contributor's device.

The backend validates the envelope and current authority but cannot independently
prove that an authenticated contributor's upload came from its asserted remote
commit. The companion's exact-input proof and attributable contribution are
distinct from remote authenticity or deployed-state verification. Unsupported
formats remain usable as permitted retained evidence for later memory processing;
their absence from the static fact graph remains visible.

## Supersession

N/A: new decision within the accepted independent Rust baseline and ADR 0003.
