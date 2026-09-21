# Invited accounts and effective team access

Status: shipped
Owning epic: `docs/roadmap/epics/product-platform.md`
Work type: product

## Summary

- Goal: Invite local teammates, optionally authenticate enrolled OIDC identities, and manage/revoke actual Brain access.
- Non-goals: Device pairing, profile authority and external identity-provider deployment.
- Delivery shape: Local Rust/SQL/browser changes with integration proof and operating documentation.

## Governing Sources

- [Team access contract](../../../contracts/platform-team-access.md)
- [Runtime ADR](../../../adr/0003-product-runtime.md)
- [Bootstrap contract](../../../contracts/platform-bootstrap.md)
- [Durable-work contract](../../../contracts/platform-durable-work.md)
- [Owning epic](../../epics/product-platform.md)

## Scope

- In scope: Invitation enrollment/recovery, account administration, direct/inherited effective grants, ownership transfer, optional OIDC and membership expiry, UI and focused proof.
- Out of scope: External Entra tenant provisioning, device credentials and MCP profile grants.
- Blockers: None; predecessor delivered and the accepted contract resolves routine choices.

## Surface and Interface Changes

- Interfaces: Team/account/invitation, auth enrollment/OIDC, Brain access/direct/group grant and owner-transfer routes and shared generated types.
- Storage: Additive account credential/issuer/subject/membership columns; invitations, OIDC flow records and group mappings. Ignored local credential file under the runtime ADR.
- Ownership: Auth/credential and team modules own identity; common database policy enforces effective roles for all API/worker consumers.

## Data and Authority

- Inputs: Authenticated actor, explicit Brain/account IDs, validated roles and normalized names; verified issuer/subject and complete signed group claims.
- Authority: Canonical accounts, grants, ownership and expiry in PostgreSQL; environment/ignored-file credentials; configured provider's verified identity.
- Blind spots: Provider configuration does not prove connectivity or external tenant behavior. Group overage has no Graph API fallback in this slice.

## States and Edge Cases

- Loading: Form pending states and bounded auth/discovery calls.
- Empty: No invitations, no shared access or OIDC not configured are explicit.
- Error: Invalid input 400, auth 401, denial 403/404, duplicate/transition 409, spent invitation 410, capacity 429 and unavailable dependencies 503.
- Blocked: Disabled accounts require owner action; unavailable provider requires retry without extending membership.
- No-access: Installation-owner and Brain-admin/owner checks, RLS and per-operation revalidation.
- Duplicate or replay: One-use locked enrollment and callback state; grant upserts are stable and ownership does not duplicate resources.
- Stale data: Five-minute maximum OIDC membership/session lifetime, current effective role queries and browser refresh/clearing.
- Reconciliation divergence: Credential file persists before account activation; database failure can leave an unused secret entry, never partial access. Expired groups do not authorize jobs.

## Integrations and Runtime Inputs

- Providers: OpenID Connect Rust library, optional single-organization provider and repository-owned isolated Dex test provider.
- Environment: RECOLLECT_CREDENTIAL_FILE and optional RECOLLECT_OIDC_ISSUER/CLIENT_ID/CLIENT_SECRET, plus existing runtime variables.
- Secrets: No hashes per ADR; secrets remain in environment/ignored file, enrollment response or HttpOnly cookie, excluded from audit/logs and list APIs.
- Failure handling: Five-second OIDC timeout, one-use five-minute callback, 24-hour invitation, five-minute membership freshness and immediate internal revocation.

## Tests and Acceptance

- Automated: Real-handler account/enrollment/access tests with negative and positive controls, worker publication denial, signed OIDC integration and browser workflow; Rust/frontend checks and governance validation.
- Manual: Inspect browser Team/Access states and local provider connection; record external tenant as unverified.
- Acceptance: Every team-access contract behavior has observable local proof before archive.

## Closeout

- Planned: Invited local identity, optional OIDC, auditable effective permissions/revocation and complete UI.
- Shipped: Invited local accounts and recovery, optional signed OIDC login, complete/fresh membership mapping, direct/inherited/owner authority, auditable revocation and browser Team/Access/enrollment workflows.
- Not shipped: External tenant deployment/verification, device credentials, profile grants and shared-operation packaging remain outside this slice.
- New blockers: None.
- Docs updated: Team contract, this archived pack, epic/indexes, team/local-development runbooks and dated proof mapping.
- Validation: [Current local proof](../../../mappings/platform-team-access-proof-2026-09-14.md): five core Rust scenarios, one real-provider OIDC scenario, three browser workflows plus mobile follow-up, authenticated startup, native worker restart, Clippy/rustfmt/frontend build, governance lint and 32 checker tests passed.
- Version: N/A: no release or strict versioning.
- Commit: uncommitted.
