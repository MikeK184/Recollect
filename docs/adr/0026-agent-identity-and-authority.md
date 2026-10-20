# 0026: Agent identity and authority

Status: proposed
Date: 2026-10-20

This record sets out the decision for the owner; it does not make the decision.
Each topic lists its options and trade-offs and marks one **Recommendation**.
The [open decisions](#open-decisions-for-the-owner) belong to the owner. Until
the owner accepts this record, it governs nothing. The slices of the agent
identity and authority epic (`docs/roadmap/epics/agent-identity-and-authority.md`,
Part 3 of the improvement sequence) stay `needs-adr` until then.

## Decision

### Boundaries every option keeps

- Recollect owns principals, roles and grants
  ([vision](../foundation/vision.md#deployment-and-access-model)). Every new
  credential goes through the single [admission order](../contracts/platform-team-access.md#admission-order)
  on every surface. No option adds a bypass.
- Human device tokens keep their current behavior. Browser or plugin pairing
  issues them. Each one acts as its account across every Brain that account can
  access, lasts 30 days, and an account may have at most 20 active
  ([plugin direct auth](../contracts/mcp-plugin-direct-auth.md)).
- Profile Use stays explicit and separate from knowledge roles, and
  administrators never inherit it ([catalogue](../contracts/mcp-catalogue-and-profiles.md)).
- Configuration, grants, the catalogue, skill publication, policies and runner
  registration can still be changed only in the browser. Agents may at most
  propose changes (topic 5).
- Revocation denies new calls and renewals. It does not cancel work in
  progress, and Recollect never replays a dispatched call
  ([ADR 0008](0008-managed-mcp-runtime.md)).
- A Recollect credential never reaches a connector or backend. Connector
  credentials never reach a model ([ADR 0009](0009-vault-and-private-execution.md)).
- These limits apply only to operations Recollect manages. They do not sandbox
  a host's shell or other credentials on the same machine.

### 1. Agent principals

| Option | Trade-off |
| --- | --- |
| A. A human-style account per agent (the [ADR 0009](0009-vault-and-private-execution.md) workaround) | No new model, but it has a password and a browser session, looks human in audit, receives account-wide device tokens and has no accountable owner |
| B. Mark a paired device as an agent | Smallest change, but a device is a credential, not a principal: it carries its human's whole authority and cannot hold grants of its own |
| C. A separate agent principal owned by one account | Adds a principal kind and an admission branch. The agent is distinct in audit, cannot sign in, and is bounded by and accountable to its owner |
| D. An agent principal owned by a team or group | Survives one person leaving, but Recollect has no team record, and OIDC group snapshots exist only for interactive sign-in, so no current human authority stands behind the agent |

**Recommendation: C, leaving room for D once the owner decides what a team
is.** An agent principal has a UUID, a display name, an owning account, a
creator and created and disabled times. It has no password, OIDC subject,
browser session or enrolment. It therefore never passes a browser-only surface,
never owns a Brain, is never the installation owner and never holds profile
Manage or Share.

Its knowledge role in a Brain is the lower of two roles: its owner's current
role and the role its Brain binding sets. When the owner is disabled, removed
from the Brain or downgraded, the agent's next operation is narrowed or denied.

The owner creates and disables their own agents in the browser. They can bind
an agent only to Brains they can access, and only up to their own role. This
gives less authority than the device token the same person can already create.
Brain administrators see the agents bound to their Brain in Brain Access and
can disable a binding there. The installation owner can disable any agent.
Disabled agents keep their identity for attribution, as accounts do.

Profile Use for an agent requires a grant naming that agent, made by someone
with Share on the profile. The owner's own direct Use is also a ceiling: the
agent cannot exceed it.

The ceiling for an OIDC owner needs a rule of its own. Device authentication
for OIDC principals requires an identity snapshot no older than five minutes
([pairing](../contracts/platform-device-pairing.md)). A headless agent cannot
refresh that snapshot. The recommended ceiling therefore counts only the
owner's ownership and direct grants. It ignores roles inherited through groups
and does not need a fresh snapshot. Disabling the account still denies the
agent at once. The trade-off: Recollect sees a removal in the identity provider
only when the installation owner disables the account. The vision already
warns that SSO removal does not revoke independently granted access.

### 2. Scoped agent credentials

| Option | Trade-off |
| --- | --- |
| A. Add an optional Brain to device tokens | Reuses pairing, but changes the human credential, mixes two meanings and keeps the 30-day lifetime |
| B. A separate opaque agent credential, checked in PostgreSQL on every request | One more credential record and lookup. Revocation takes effect immediately, through the same stateless per-request check device tokens use |
| C. Self-contained signed tokens issued by Recollect | Can be verified offline, but revocation needs deny lists and signing keys. Admission already reads PostgreSQL on every request, so offline verification gains nothing |

**Recommendation: B.** An agent credential belongs to one agent principal and
records a scope: exactly one Brain, at most one environment, optionally a set
of named profiles, and optionally a role ceiling (reader or writer) below the
Brain binding. The named profiles must be in that Brain and among the agent's
Use grants. An environment profile must match the credential's environment.
The scope is frozen at issuance.

- If the request path names any other Brain, the answer is 404 at the
  visibility step, never 403. That neither widens access nor reveals whether
  the Brain exists.
- An environment fixes every operation, capture binding, coordination
  announcement and profile discovery to that environment, and Brain-wide
  selection is denied. Environments are not document security boundaries
  ([vision](../foundation/vision.md#deployment-and-access-model)), so this limits
  scope but does not guarantee confidentiality. Setup text must say so.
- Named profiles limit discovery and calls. The agent's current Use grants
  still apply.
- Lifetime. A credential issued by workload exchange (topic 3) lasts at most the
  issuer's configured maximum (proposed default: one hour), and never longer
  than the workload token it replaced. A credential issued in the browser, for a
  job that has no workload issuer, is by nature a stored secret. The recommended
  default for those is 24 hours. The owner may choose a longer lifetime, up to
  the device token's 30 days. A stricter cap would push unattended jobs back to
  broader device tokens.
- Renewal means a new exchange or a new issuance. There are no refresh tokens.
- Revocation works per credential, from the browser or by possession (like
  `revoke-self`). Disabling an agent revokes all its credentials. Removing a
  binding denies that Brain. Each agent has a bounded number of active
  credentials.
- Admission step 1 names the bearer kinds each surface accepts. Browser-only
  surfaces refuse every bearer kind. A surface refuses agent credentials unless
  its contract lists them. Candidate surfaces are the Brain MCP endpoint, the
  device-facing HTTP API, capture, coordination (including SSE and A2A) and
  skill fetch. Runner routes refuse agent credentials (topic 4).

### 3. Workload identity exchange

| Option | Trade-off |
| --- | --- |
| A. RFC 8693 token exchange at a Recollect token endpoint | Standard request and response, usable with `curl` and OAuth libraries. It carries the requested scope and leaves room for actor delegation and a later MCP authorization server. Recollect must validate the JWT strictly |
| B. RFC 7523 JWT bearer grant | Also standard and simpler, but it has no requested token type and no actor model |
| C. A custom JSON endpoint | Simplest, but no standard tooling supports it, and later OAuth would need a second endpoint |
| D. The workload signs in to Vault, which issues Recollect credentials | Reuses Vault's Kubernetes and JWT methods, but makes Vault mandatory for headless agents (the vision keeps Vault optional) and needs a Recollect secrets engine |
| E. Store a long-lived credential in a Kubernetes Secret or a CI variable | Works today with device tokens, but this is the long-lived, account-wide secret this work is meant to remove |

**Recommendation: A, as a narrow RFC 8693 profile.**

- **Request.** `grant_type=urn:ietf:params:oauth:grant-type:token-exchange`,
  with the workload JWT as `subject_token` and
  `subject_token_type=urn:ietf:params:oauth:token-type:jwt`. The requested
  Brain, environment and profiles go in `resource` and `scope`; the contract
  fixes their encoding. The first delivery has no `actor_token`.
- **Response.** The opaque agent credential as `access_token`, with
  `issued_token_type=urn:ietf:params:oauth:token-type:access_token`,
  `token_type=Bearer`, `expires_in`, and `scope` when the grant was narrowed.
- **Issuer allowlist.** Operator configuration, like today's OIDC variables
  and credential-reference file. Each entry gives:
  - a kind (`kubernetes`, `github_actions` or `gitlab_ci`)
  - the exact issuer URL
  - one Recollect-specific accepted audience
  - a key source: discovery, or a static JWKS file for clusters whose issuer
    is not reachable
  - the permitted algorithms
  - a maximum token lifetime and a maximum issued lifetime

  Default audiences are rejected. Kubernetes defaults to the API server,
  GitHub to the repository owner's URL and GitLab to the instance domain.
  Accepting a default would let Recollect take tokens meant for other
  services. Discovery and key fetches use the shared outbound client with
  the OIDC limits: a five-second timeout, no redirects and 1 MiB responses.
- **Trust bindings.** The agent's owner creates them in the browser. A binding
  maps an allowlisted issuer and exact claim values to one agent principal and
  a maximum scope. Values must match exactly; wildcards are not allowed. Each
  kind requires stable identifiers:
  - GitHub: `repository_id` and `repository_owner_id`
  - GitLab: `project_id` and `namespace_id`
  - Kubernetes: the namespace and service account name, optionally its UID

  For CI kinds, a protected ref or an environment condition is recommended.
  Anyone who can run a matching workload acts as that agent.
- **Validation.** Recollect checks the signature against the allowlisted keys.
  `iss` must match exactly and `aud` must contain the configured audience.
  `exp`, `nbf` and `iat` are checked with a small clock skew. The lifetime must
  not exceed the maximum, and the kind's required claims must be present.
- **Replay rules depend on the kind.** For CI kinds, each `jti` can be exchanged
  once. A Kubernetes pod reuses its projected token, and re-reads it after a
  container restart, until the kubelet rotates it. Single use would break those
  restarts. So an unexpired Kubernetes token may be exchanged again. Each
  exchange is rate-limited and audited and bound to the pod UID claim, and the
  credential never outlives the token.
- **Kubernetes validation.** Kubernetes recommends its TokenReview API because
  validation through OIDC keeps a deleted pod's token valid until it expires.
  TokenReview would require Recollect to hold a cluster credential and reach the
  API server, a long-lived secret on Recollect's side. The recommendation is to
  start with discovery and keys plus a short cap on token lifetime, and to add
  TokenReview later as an optional issuer setting.
- **Residual risk.** Exchange has no client authentication, because the workload
  token itself is the proof. RFC 8693 warns that without client authentication a
  stolen token can be exchanged. The Recollect-only audience, short lifetimes,
  the replay rules and exact bindings limit that risk; they do not remove it.
- **Client.** An exchange helper in `recollect-agent` reads the token file or
  CI variable and keeps the issued credential in memory. The pod spec and CI
  configuration carry no secret.

### 4. Runner identities

| Option | Trade-off |
| --- | --- |
| A. Keep device-bound runners; a cluster runner pairs the device of a dedicated human account | No code changes, but a 30-day account-wide device token sits in a cluster Secret and needs browser re-pairing every month |
| B. A runner principal sponsored by an administrator's account | A clear accountable human, but the runner stops when that person leaves or loses admin, which a cluster deployment experiences as an outage |
| C. A runner principal owned by its Brain registration | Survives staff changes and any current Brain admin controls it; the accountable human is recorded only as its creator |

The [private-runner contract](../contracts/mcp-vault-and-private-runners.md)
specifies a recheck of the registering account's current Brain admin role.
Pairing requires a fresh five-minute OIDC snapshot. Together these mean that, as
specified, a device-bound runner of an OIDC administrator authenticates only
shortly after that administrator signs in to the browser. Option C avoids this.

**Recommendation: C, with today's device-bound registrations unchanged.**

- In the browser, a Brain admin registers either one of their own devices, as
  today, or a new runner principal.
- A runner principal has no knowledge role, no profile Use and no MCP agent
  tools. Its credential authenticates only the runner routes (`register`,
  `heartbeat`, `claim`, `start`, `defer`, `complete`, `instance` and runner
  `credentials`), and only for its registration's Brain.
- **Credential.** For clusters, the recommended source is workload exchange,
  with the trust binding naming the runner principal. Elsewhere, a one-time
  enrolment approved in the browser, like pairing, follows the lifetime rule
  for browser-issued agent credentials.
- **Rechecks.** Dispatch, claim, startup, credential acquisition and renewal,
  and the pre-send check keep their checks. One check changes: "exact device,
  active paired account and current Brain admin role" becomes "exact runner
  principal and credential, registration enabled, Brain open". The caller's own
  scope, Use and frozen configuration checks stay as they are.
- Connections stay outbound only. The outbox identity becomes the service URL
  plus the runner principal.
- Each registration keeps a single live epoch. A Deployment runs one replica per
  registration; for more capacity, add registrations.
- Connector credentials still come from the runner's environment, mounted files
  or Vault Proxy. The operator authenticates the Proxy with Vault's own
  Kubernetes method, and Recollect still implements no Vault login.

### 5. Configuration by proposal

| Option | Trade-off |
| --- | --- |
| A. No proposals | Agents describe changes in prose and humans retype them |
| B. Inert typed drafts proposed through MCP or the plugin, approved in the browser by an authorized human | Keeps the authority boundary, but adds a proposal record, a review view and a recheck at approval |
| C. The agent applies a change after confirmation inside the host | Recollect does not authenticate host prompts, so an agent holding the credential could confirm its own change |
| D. Approval through the CLI | Convenient, but the CLI uses the same device token an agent on that machine holds, so Recollect cannot tell a human's approval from an agent's |

**Recommendation: B, with approval in the browser.** The CLI may list
proposals and print the browser approval link, as pairing prints its
verification URL.

- **Content.** A proposal is a typed draft of exactly one change the browser
  could make directly. It records the base revision, the reason, the proposer's
  identity (topic 6) and an expiry. It is validated against the same schemas as
  the direct change and carries credential aliases, never secret values.
- **Inert.** A proposal executes, grants, publishes, starts, connects and
  resolves nothing. Discovery and skill fetch never see it.
- **Approval.** The approver needs the authority the direct browser change
  would need, rechecked at approval. A stale base revision is a conflict. The
  change goes through the same canonical command, and audit names both the
  proposer and the approver. Rejection, withdrawal and expiry are final. Agents
  cannot approve, and no bearer credential can approve.
- **Who may propose.** Proposing requires writer access within the
  credential's ceiling, so readers cannot propose. Proposals are bounded per
  agent and per Brain.
- **Visibility.** A proposal is visible to the people who could approve it and
  to the proposer's owner. The proposing agent sees only the status of its own
  proposals. A grant that would benefit the proposer or its owner is clearly
  marked.
- **Recommended first kinds:** skill and instruction bundle drafts and
  revisions, profile membership, connections created or edited from already
  approved definitions, and profile grants.
- **Recommended exclusions:** definition imports (executables), Brain access
  grants and ownership, capture, model and retention policy, runner
  registrations, agent principals, trust bindings, workload issuers and Brain
  deletion.

### 6. Identity fields for attribution

Each authenticated request resolves to one identity derived by the server.
Receipts (Part 4), the capture API, managed observations, coordination and
audit can freeze that identity. Receipts themselves are out of scope here; this
topic defines only what identity must provide.

| Field | Meaning |
| --- | --- |
| `principal_kind`, `principal_id`, display name | `account`, `agent` or `runner`, with its UUID and server-side name |
| `accountable_account_id` | The account itself, an agent's owner, or none for a runner, which acts only for its callers |
| `credential_kind`, `credential_id` | `browser_session`, `device`, `agent_issued`, `agent_exchanged` or `runner`, and the credential's UUID |
| `credential_scope` | Brain, environment, profiles and role ceiling frozen at admission; empty for account-wide device tokens |
| `workload_attestation` | For exchanged credentials only: the issuer configuration key, `iss`, `sub` and a fixed set of non-secret claims per kind, such as repository and run identifiers, project, pipeline and job identifiers, or namespace, service account and pod. Never the raw token |
| Host and operation | Today's `host_kind`, `integration`, host session, task, operation and immutable scope, unchanged |
| Executing runner | The registration and runner principal that ran a managed call, kept separate from the caller |
| `authenticated_at`, `credential_expires_at` | When admission happened and when the credential expires |

The server derives these values; client display strings never become identity,
as in coordination today. Existing rows map to `account` and `device`. Queued
evidence keeps its original authority. Disabled agents and their credentials
keep tombstones for attribution. These fields describe what happened and grant
nothing: each later request rechecks current state.

### 7. Out of scope

- **MCP OAuth.** The owner deferred MCP OAuth for Recollect's endpoint on
  2026-10-20. This design leaves room for it in four ways:
  - A Brain-bound credential corresponds to an RFC 8707 resource.
  - The endpoint could later publish RFC 9728 metadata and accept OAuth access
    tokens that resolve to the same identity record.
  - The token endpoint from topic 3 could become an authorization server's
    token endpoint.
  - The MCP specification's audience binding and its ban on passing tokens
    through already match the rules above.

  Nothing here implements metadata, client registration or authorization-code
  flows, and hosts keep `oauth: false`.
- **Multi-tenant hosting.** The vision excludes it. Principals, issuers, trust
  bindings and proposals are scoped to the installation and keyed by Brain and
  account, so a later organization identifier could sit above Brains without
  changing the rules at Brain level. Nothing adds an organization record,
  federation or an owner bypass.
- Also out of scope: receipts (Part 4), sandboxing the shell, putting human
  plugin sessions on agent principals, and other workload issuers such as cloud
  instance identity or SPIFFE. Each issuer kind needs its own verification.

## Why

On 2026-10-20 the owner set the direction for verified agent work: every
agent's work should leave a receipt linking its claims to evidence, and
headless agents, for example cluster jobs, should later work through Recollect.
A receipt is worth only as much as its record of which agent acted, under whose
authority, and within which scope.

The accepted sources describe the current state:

- Only human accounts exist ([team access](../contracts/platform-team-access.md)).
- A device token acts as its whole account across all Brains for 30 days
  ([ADR 0015](0015-direct-plugin-user-auth.md)).
- Nothing exchanges workload identity.
- A private runner must be a Brain admin's paired device
  ([ADR 0009](0009-vault-and-private-execution.md)).
- Configuration changes are deliberately browser-only, and agents can at most
  read and discover ([ADR 0010](0010-agent-memory-mcp.md), the
  [catalogue](../contracts/mcp-catalogue-and-profiles.md) and
  [skills](../contracts/mcp-managed-skills.md)).
- The identity fields that capture and coordination record are tied to a
  device ([session capture](../contracts/evidence-session-capture.md),
  [observations](../contracts/mcp-observation-capture.md),
  [ADR 0023](0023-agent-to-agent-coordination.md)).

On 2026-10-20 these external interfaces were checked:

| Source | Checked | Limits |
| --- | --- | --- |
| [RFC 8693](https://www.rfc-editor.org/rfc/rfc8693.html) (fetched) | Grant type, request parameters, token type URNs, response fields, `act` and `may_act`, the client-authentication warning | Standards Track, January 2020 |
| [RFC 7523](https://www.rfc-editor.org/rfc/rfc7523.html) (fetched) | `jwt-bearer` grant, `assertion`, rules for `iss`, `sub`, `aud` and `exp`; `jti` replay tracking is optional | Considered as option B |
| Kubernetes documentation (Context7 `/websites/kubernetes_io`) | Projected `audience` and `expirationSeconds` (default one hour, minimum 600 seconds; the kubelet rotates after 80% of the lifetime or 24 hours), `/.well-known/openid-configuration`, `/openid/v1/jwks`, `--service-account-jwks-uri`, `kubernetes.io` claims, TokenReview recommended over OIDC validation | No cluster was exercised |
| [GitHub OIDC concepts](https://docs.github.com/en/actions/concepts/security/openid-connect) and [reference](https://docs.github.com/en/actions/reference/security/oidc) (fetched) | Issuer `https://token.actions.githubusercontent.com` and its discovery document, `id-token: write`, request URL and token variables, the `audience` parameter, the default audience (repository owner URL), the claims, and an immutable `sub` with owner and repository IDs for repositories created after 2026-07-15 | Token lifetime not stated (the example shows 300 seconds); Enterprise Server issuers not checked; `jti` uniqueness per request not confirmed |
| [GitLab ID tokens](https://docs.gitlab.com/ci/secrets/id_token_authentication/) (fetched) | `id_tokens` with `aud`, `iss` as the instance URL, `exp` set to the job timeout or five minutes, the claims, the configurable `sub`, guidance to use `project_id` and `namespace_id`, and `CI_JOB_JWT_V2` deprecated | The page names no discovery or JWKS location; self-managed issuers not checked |
| MCP authorization 2026-07-28 (Context7 `/websites/modelcontextprotocol_io_specification_2026-07-28`) | RFC 9728 metadata, the RFC 8707 `resource` parameter, audience validation, the ban on passing tokens through | Only checked to leave room for OAuth |

Context7 was not tried for GitHub or GitLab, whose official pages were fetched
directly. None of these lookups failed. No issuer, cluster or CI system was
exercised, so every external statement above is documentation evidence, not
proof of runtime behavior.

## Consequences

- Because this record is proposed, it governs nothing. The epic's slices stay
  `needs-adr`. Acceptance may take some recommendations and reject others.
- Under the [governance guide](../README.md), implementing scoped credentials,
  workload exchange, runner identities and configuration proposals is risky
  work. Each changes security, credentials or authorization in a way no
  accepted contract specifies, adds schema, and, for exchange, adds an external
  integration. Each needs a contract and a decision-complete execution pack
  before code. Agent principals add a principal kind to admission and a
  migration, so they meet the pack criteria too, even though the epic plans
  them inline.
- Contracts to amend or replace on acceptance:
  - [team access](../contracts/platform-team-access.md) (admission step 1 and
    principal kinds)
  - [pairing](../contracts/platform-device-pairing.md) and
    [plugin direct auth](../contracts/mcp-plugin-direct-auth.md) (to state that
    device tokens are unchanged)
  - [memory tools](../contracts/mcp-memory-and-workspace-tools.md)
  - [catalogue](../contracts/mcp-catalogue-and-profiles.md) (grant targets and
    proposals)
  - [skills](../contracts/mcp-managed-skills.md)
  - [runtime](../contracts/mcp-runtime-and-credentials.md) and
    [private runners](../contracts/mcp-vault-and-private-runners.md)
  - [session capture](../contracts/evidence-session-capture.md) and
    [observations](../contracts/mcp-observation-capture.md)
  - [coordination](../contracts/mcp-agent-coordination.md)

  Workload exchange and proposals each probably need a new contract.
- **Compatibility.** Human device tokens, the pairing and plugin flows,
  device-bound runners and existing attribution rows keep working unchanged.
  The recommended agent ceiling for OIDC owners deliberately differs from
  device authentication for OIDC users, and the contract must state that
  difference.
- **Residual risks the owner accepts with the recommendations:**
  - Exchange has no client authentication.
  - Validating through keys keeps a deleted pod's token valid until it expires.
  - A trust binding gives the owner's delegated authority to anyone who can run
    the bound workload.
  - Environment limits restrict scope but do not protect confidentiality.
  - An IdP-side removal is seen only when the account is disabled.
- **Foundations.** The recommendations need no foundation change. Team-owned
  principals, or environments that become confidentiality boundaries, would
  need one.

## Open decisions for the owner

1. May agent principals be owned by a team or group as well as by one account?
   If so, what is a team in Recollect: an OIDC group mapping, a new record, or a
   Brain? Who answers for the agent when membership changes?
2. Who may create agent principals? The recommendation is that any enabled
   account may create them for itself. The alternatives are Brain admins only
   or the installation owner only. May Brain admins disable another account's
   agent in their Brain? The recommendation is yes.
3. Does an agent need its own profile Use grant from someone with Share, with
   the owner's direct Use as a ceiling (recommended)? Or may the owner delegate
   their own Use by naming profiles on a credential?
4. For an OIDC owner, may an agent's ceiling ignore the owner's five-minute
   identity snapshot and roles inherited through groups (recommended)? This
   accepts that an IdP-side removal takes effect only when the account is
   disabled.
5. Which configuration changes may agents propose? The recommended first set is
   skills, profile membership, connections from approved definitions and
   profile grants. Should the recommended exclusions be permanent?
6. Must approval happen in the browser (recommended), or may an approving human
   use the CLI? If the CLI is allowed, how does Recollect tell that human from
   an agent holding the same device token?
7. May a proposing agent's own owner approve its proposals? The recommendation
   is yes, with an optional later Brain policy requiring a different approver.
   Or must another person always approve?
8. What maximum lifetime should credentials have? The recommendation is at
   most one hour for exchange-issued credentials, never beyond the workload
   token. Browser-issued ones would default to 24 hours, with a cap no higher
   than the device token's 30 days.
9. Who configures the workload issuer allowlist: operator configuration
   (recommended) or the installation owner in the browser? Who creates trust
   bindings: the agent's owner (recommended) or a Brain admin?
10. Does a runner principal belong to its Brain registration until a Brain admin
    disables it (recommended)? Or should it lapse, as device runners do today,
    when the admin who created it loses that role?
11. Should agent principals get an execution pack, as recommended above, rather
    than the inline row the epic plans?

## Supersession

N/A: this is a new proposed decision. If accepted, it would extend ADRs
[0010](0010-agent-memory-mcp.md), [0015](0015-direct-plugin-user-auth.md) and
[0023](0023-agent-to-agent-coordination.md). It would also amend
[ADR 0009](0009-vault-and-private-execution.md)'s rule that a private runner is
a Brain admin's paired device. Those records stay in force unchanged until then.
