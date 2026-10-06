# Actionable Brain management

Observed: 2026-10-04
Confidence: verified

## Sources and Method

- User's nine annotated Connections/Settings comments and explicit implementation authorization.
- Existing desktop, managed/autonomous memory, MCP catalogue/runtime/private execution contracts.
- Local Atlas patterns: memory as an editing surface, trust state machine and scope as a first-class key. They inform presentation; Recollect contracts govern behavior.
- Context7 Mantine documentation lookup for the installed 8.3 family succeeded; no dependency was installed.
- Existing browser journeys run through `scripts/test-ui.sh` against disposable databases and an owned anonymous HTTP fixture. No installation provider calls.
- Independent reviewer inspected source and actual screenshots. Read-only live browser inspection used the existing SWEG Brain and preserved its settings and inventory.

## Observations

- The Add connection form's Context7 shortcut was hardcoded. It is removed; the existing saved Context7 connection remains real installation data.
- The installation catalogue contains owner-approved connector definitions, distinct from per-Brain connection configurations and independent tool-group grants. The chooser now displays actual names/counts and their origin.
- Explicit Test performs the existing owner-only anonymous HTTP handshake/tool listing. The owned fixture returned one tool and recorded zero tool calls; failed checks removed prior success. Authentication, custom settings and private execution direct to a named authorized tool group, filtered to the chosen connection.
- Privacy is edited inline, one section at a time, with Save/Cancel. Actual defaults remain 30-day raw sessions/output, durable evidence/memory until erased, and repository file references unless explicitly permitted. Existing custom policies remain intact. Capture and retention keep revision/idempotency fencing. Storage editors detect observed external boolean changes.
- Access is a compact member list with focused add/edit/remove/group/ownership popups. Effective roles and owner/group/direct grant sources remain available. The existing access-loss browser journey passed.
- Managed autonomy owns routine learning/reconciliation/acceptance. Legacy literal-only learning controls appear only when autonomy is disabled in the explicit editor. No live model policy or transmission permission was silently changed.
- Optional environment creation/rename lives under Settings → General. The disposable browser journey created and renamed an environment, preserving its existing description. An environment scopes knowledge and tool routing and grants no permissions. Private-network execution is separate from ordinary plugin/MCP memory access.
- The real local stack was rebuilt and readiness returned `ready: true`; the browser still lists the same 16 Brains. Privacy, member list/popups, neutral setup and correctly named environment popup were inspected without saving a live policy or grant.
- Privacy editors load on demand within existing files. The entry bundle remains about 607 KB (191 KB gzip); no overall route speedup is claimed from that entry measurement alone.

## Translation and Limits

Anonymous inspection is neither a successful tool call nor proof of authenticated/private runtime or persistent availability. Remote services are not processes Recollect can stop. A successful metadata check never updates last successful tool-call time or grants Use access.

Boolean storage APIs have no atomic revision parameter. The inline editor prevents overwriting a change observed since it opened; an unseen simultaneous update remains an existing API limitation. No claim of atomic compare-and-swap is made for those endpoints.

All work remains local/uncommitted, without version bump, commit, push or external release. Earlier visual acceptance remains historical; this subsequent correction has its own pack and checks.

## Validation

- Web typecheck/design and production build pass; no dependency or API/schema change.
- Existing `mcp-direct.spec.ts` name/URL case passes discovery, save, real metadata Test, zero tool calls and failed/repeated-inspection invalidation.
- Existing `mcp.spec.ts` desktop catalogue case passes environment creation/rename, description preservation, scoped connection setup, tool groups, cached metadata and explicit use controls.
- Existing `team.spec.ts` invitation/revocation case passes focused membership popups and effective access loss.
- Existing `retention.spec.ts` lost-response case passes inline retention save, single-section editing, observed external storage change protection and retained excerpts/erasure.
- Existing `managed-memory.spec.ts` new-Brain case passes managed defaults, conditional legacy overrides, compact navigation and staged plugin setup.
- Its separate empty-catalogue case passes owner manifest registration and the transition to actual installation catalogue setup without a pre-imported fixture.
- The environment case initially stopped on an exact required-label locator. Its dialog accessible-name assertions passed; semantic textbox locators corrected the test without weakening those assertions. The complete case subsequently passed.
- Live read-only screenshots show actual Privacy summary/inline editing and members. An independent UI reviewer found no remaining source or visual blockers.
- Six unique existing journeys pass across the targeted runs. Owned databases and credential files were cleaned up, with no installation provider calls or live grant/policy mutations. Final `./scripts/validate.sh`, `git diff --check` and CodeGraph synchronization pass; the rebuilt installation reports ready.

## Follow-up

None within the accepted correction. The [archived pack](../roadmap/execution/archive/desktop-actionable-management.md), owning epic and indexes are reconciled. Authenticated/private testing and stronger boolean-policy concurrency would require separate interface work; the limits above remain explicit.
