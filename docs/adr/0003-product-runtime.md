# 0003: Product runtime and local development posture

Status: accepted

## Decision

Implement the accepted Rust/React/PostgreSQL/Neo4j product in this repository.
Use a Cargo workspace with protocol, server and agent crates. Axum serves a
same-origin Vite/React/Mantine application; SQLx owns relational persistence and
Reqwest talks to Neo4j's Query API. PostgreSQL is authoritative; graph data is a
rebuildable projection. Repository-owned Compose services persist in a project-labelled PostgreSQL volume
and ignored `.data/` graph/artifact directories.

The user's implementation instruction of 2026-09-13 explicitly prioritizes speed
and says not to hash or introduce strict versioning now. This overrides the
foundation's hash, exact pin, content-addressing and password-hashing requirements
for this local development delivery. Use ordinary compatible dependency ranges,
normal package-manager lockfiles, UUID artifact identities, recorded source
revisions and append-only history. Do not introduce custom digest checks,
mandatory version handshakes or release gates. Dependencies may internally use
hashes; no product hash contract is introduced.

Local owner authentication compares an environment-provided password and issues
an opaque, revocable, server-backed session. No password is saved to the database.
The initial local stack binds host ports to loopback. Team credential enrollment
and shared HTTPS operation remain in their named slices. This override does not
remove Brain access checks, source provenance, transactional integrity, retention,
revocation or actual integration proof.

## Why

The user authorized all 29 roadmap slices and their routine ADR/contract decisions,
with no per-slice approval. The [foundations](../foundation/README.md) select the
architecture; [dependency evidence](../mappings/platform-dependencies-2026-09-13.md)
records the interfaces used. This decision resolves the current instruction's
conflict with the earlier strict dependency and hashing posture before coding.

## Consequences

Bootstrap uses a separate database migration principal and a non-owner application
principal with row-level Brain policies. Mutations and audit commit together;
durable outbox/worker support follows in platform-durable-work. Local setup is
re-runnable without deleting data or resetting credentials. No reference checkout
is modified. Shared exposure, hardened credentials and operational guarantees
must be assessed in the operations slice under the user's current constraints.

## Supersession

Overrides only the hashing and strict version/pinning portions of the accepted
foundations for this authorized development goal. The rest of the product
baseline and all 29 slice obligations remain in force.
