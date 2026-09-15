# Team access dependency and local proof

Observed: 2026-09-14
Confidence: verified

## Sources and Method

The [team contract](../contracts/platform-team-access.md) governs acceptance.
Context7's first generic Rust OIDC lookup returned unrelated Rust documentation;
the exact `ramosbugs/openidconnect-rs` lookup then resolved successfully. Its
query confirmed typed custom claims, async discovery/code exchange and token
verification. Primary sources checked:

- [OpenID Connect Rust 4.0.1](https://docs.rs/openidconnect/4.0.1/openidconnect/)
- [Entra ID-token claims and group overage](https://learn.microsoft.com/en-us/entra/identity-platform/id-token-claims-reference)
- [Dex 2.45.1 release](https://github.com/dexidp/dex/releases/tag/v2.45.1)
- [Dex configuration](https://github.com/dexidp/dex/blob/v2.45.1/examples/config-dev.yaml)
- [Dex synthetic connector](https://github.com/dexidp/dex/blob/v2.45.1/connector/mock/connectortest.go)

The initially guessed Dex mock source paths returned cache misses. GitHub's
directory API resolved the actual `connectortest.go` source. The local fixture
uses that documented synthetic identity, never a real user's credentials.

Executed `./scripts/test-platform.sh`, `./scripts/test-oidc.sh`,
`./scripts/test-oidc.sh ui`, generated TypeScript/Vite build, Clippy with warnings
denied, rustfmt and `./scripts/validate.sh`. A final focused browser run verified
the corrected mobile navigation and inspected screenshots. The API generator now
rejects duplicate operation IDs; two discovered collisions were fixed before proof.

## Observations

| Behavior | Evidence |
| --- | --- |
| Local identity | Concurrent acceptance creates one session/credential; duplicate enrollment, expired/revoked invitations and unauthorized account administration fail. Password file corruption returns 503. Reset revokes the former password and activates the new one. |
| Effective access | Ownership and direct/group grants are independent. Tests transfer a Brain, remove a direct owner grant, remove a mapped role while preserving direct reader access, and reject unrelated Brain reads. |
| Internal revocation | Disable waits for an already authorized transaction, then revokes sessions and prevents worker publication. Re-enable does not resurrect an old session. Direct removal becomes visible in an open teammate browser. |
| OIDC protocol | A real isolated Dex container supplies discovery/JWKS and signed code exchange. An unknown subject, wrong callback cookie, replay, expired flow and wrong bound nonce are denied. Correct enrolled issuer/subject succeeds. |
| External membership bound | Signed `authors` membership supplies a real mapped role. Advancing stored deadlines tests the five-minute bound; inherited access and queued publication cease, while independent direct access remains. Missing/malformed/Entra-overage claims produce no inherited groups. |
| Provider outage | Failed discovery redirects to a safe sign-in state; local owner operations remain usable. Provider tokens and query strings are excluded from runtime traces. |
| Browser | Three workflows passed: owner/Brain/durable work, teammate enrollment and revocation, organization sign-in with a real mapped group. Desktop/mobile Team and Access screenshots were inspected. A focused follow-up proves mobile navigation and no horizontal overflow. |
| Local startup | `./scripts/dev.sh` migrated existing local data and started API plus worker. A real owner login and `/api/team` read succeeded; readiness was true. Normal startup correctly reports OIDC unconfigured. |

Five core Rust scenarios and one dedicated live OIDC scenario passed. Three full
browser workflows passed; the final mobile-navigation follow-up passed. Native
worker restart drained persisted test jobs. The generated API has 31 unique
operations. Clippy, rustfmt, frontend build, governance lint and all 32 governance
checker tests passed. Whitespace inspection included untracked source files.

## Translation and Limits

This is local product delivery under the user's no-hashing development instruction.
No external Entra tenant, customer federation or shared deployment was configured
or verified. Group expiry was advanced in the test database rather than waiting
five wall-clock minutes. The optional local Dex service was removed after tests;
the normal installation has no synthetic identity provider enabled. Provider
signature validation uses the maintained library; no application hash/version
contract was introduced. Local teammate passwords use the ignored credential file.

## Follow-up

Continue with device pairing. Shared HTTPS packaging and whole-installation
backup/recovery remain required operational slices; preserve the credential file
alongside the database for local account recovery.
