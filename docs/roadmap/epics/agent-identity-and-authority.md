# Agent Identity and Authority

Status: planned

## Purpose

Give agents their own narrow, short-lived identities so headless agents, for
example jobs running in a cluster, can work through Recollect safely: non-human
principals owned by a person, credentials limited to one Brain, environment and
set of profiles, workload identity exchange instead of stored secrets, runners
that are not a person's device, and configuration changes agents may only
propose. Part 3 of the five-part improvement sequence in the [roadmap](index.md#improvement-sequence).

## Governing Sources

- [Vision](../../foundation/vision.md) (deployment and access model, MCP coordinator)
- [Team access](../../contracts/platform-team-access.md),
  [device pairing](../../contracts/platform-device-pairing.md) and
  [plugin direct authentication](../../contracts/mcp-plugin-direct-auth.md)
- [Catalogue and profiles](../../contracts/mcp-catalogue-and-profiles.md) and
  [Vault and private runners](../../contracts/mcp-vault-and-private-runners.md)
- [ADR 0015: direct plugin user auth](../../adr/0015-direct-plugin-user-auth.md) and
  [ADR 0009: Vault and private execution](../../adr/0009-vault-and-private-execution.md)

## Dependencies and Boundaries

Starts after [memory quality and proof](memory-quality-and-proof.md); independent
of [claim truth and freshness](claim-truth-and-freshness.md), so the two can swap
order. One identity ADR covering principals, scoped credentials, workload
exchange and runner identity comes before any slice; a proposed draft is being
prepared separately.

Today only human accounts exist, a device token acts as its whole account across
every accessible Brain for 30 days, and a private runner must be a Brain
administrator's paired device. Configuration changes stay browser-only by design;
agents gain at most the right to propose. `agent-scoped-credentials`,
`agent-workload-identity-exchange`, `agent-runner-identities` and
`agent-config-proposals` are authorization changes and each needs a
decision-complete pack. A hosted multi-tenant offering may follow later; the
vision still excludes it, so the ADR should only leave room for an organization
boundary above Brains.

**Open decisions for the owner.**
- May agent principals belong to a team or group as well as to one account, and who may create them?
- Which configuration changes may an agent propose (connections, profile grants, skills), and must approval happen in the browser or may an approving human use the CLI?

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `agent-identity-principals` | planned | needs-adr | inline: non-human agent principals are owned by an account (or team, if decided), created and disabled by authorized people, and attributed in audit, capture and coordination separately from human devices; validation: native tests for creation, ownership, disable, audit attribution and denial after owner removal | Foundation for headless and cluster agents; the identity ADR comes first |
| `agent-scoped-credentials` | planned | needs-adr, needs-contract | inline: an agent credential is limited to one Brain and optionally one environment and named profiles, expires quickly and is revocable, while human device tokens keep their current behavior; validation: admission tests proving out-of-scope Brains, environments and profiles are denied on every surface | Replaces account-wide 30-day tokens for agents; authorization change, pack required |
| `agent-config-proposals` | planned | needs-adr, needs-contract | inline: agents may propose catalogue, connection, skill or grant changes through MCP or the plugin as inert drafts, and only an authorized human approves them, so no agent can grant itself tools; validation: tests showing proposals cannot execute, grant or publish before approval and that approval rechecks current authority | Gives "configure through MCP or plugin" without moving the authority boundary; pack required |
| `agent-workload-identity-exchange` | planned | needs-adr, needs-contract | inline: a Kubernetes projected service-account token or CI OIDC token from an allowlisted issuer and audience is exchanged for a short-lived scoped agent credential, with no long-lived secret in the workload; validation: fixture issuers covering valid, expired, wrong audience, wrong issuer and replay | Lets cluster jobs authenticate without stored secrets; pack required |
| `agent-runner-identities` | planned | needs-adr, needs-contract | inline: a private runner can be registered to a runner principal instead of an administrator's paired device, keeps outbound-only connections and the existing dispatch, claim and pre-send rechecks; validation: runner tests for registration, revocation mid-call and cross-Brain denial | Lets a runner be a cluster deployment; pack required |
