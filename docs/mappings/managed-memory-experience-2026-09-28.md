# Managed autonomous memory implementation and local proof

Observed: 2026-09-28
Confidence: verified

## Sources and Method

The user's annotated Recollect screens and Cognee screenshots supplied the
operating-model correction: connect once, let agents contribute and retrieve,
and let the existing memory workers maintain evidence. The governing decision
is [managed experience](../contracts/memory-managed-experience.md), owned by the
[memory epic](../roadmap/epics/memory-lifecycle.md). Local Atlas capture and
decay/reinforcement patterns were reference evidence, not runtime dependencies.

Inspected the existing dirty checkout, vision, domain contracts, shared handlers
and CodeGraph relationships. No new dependency or external API was introduced;
the installed Mantine components and existing companion commands were reused.
Private command logs and synthetic screenshots are in
`.cache/managed-memory-20260928/` and `.cache/ui/managed-*.png`. They contain no
credential values and are not a substitute for the committed tests.

## Observations

### Implemented behavior

- Browser-created Brains request one managed preset: installed model selection,
  Ask, learning, automatic embeddings, supported sanitized capture, bounded
  resource accounting and the existing autonomous maintenance workers.
- Existing Brains have one adoption action in Ask or Settings. Shared Rust
  persistence applies model/capture changes atomically with revision checks,
  audit and outbox; retention, capture exclusions and other Brains are preserved.
  Older API callers and saved create receipts keep their previous behavior.
- Ordinary settings show status and lifecycle explanations. Model/capture/storage
  overrides are under advanced controls. Recall budgets remain agent API inputs;
  the browser uses bounded defaults. A manual memory contribution is a plain
  evidence note, with structured authoring available separately.
- Five primary navigation links and a secondary workspace group replace the
  expanded nine-section sidebar. Connections starts with Codex/Claude Code cards
  and a three-step pairing/capture/verification flow.
- An empty MCP catalogue has owner-browser manifest registration using the
  existing validator. Migration 028 grants owner-scoped insertion through RLS.
  Registration is inert; existing independent tool-use grants remain required.
  Profiles are explained as tool groups. Arbitrary MCP configuration files are
  not automatically converted to approved Recollect connector manifests.

### Validation

- Two new native managed-memory tests passed, including absent provider, stale
  policy rejection, atomic replay, old receipt identity, role/device/archive
  denial, exclusion/retention preservation and inert owner-only connector import.
- Existing autonomous catch-up/revision/handover/retirement/erasure proof passed
  with its controlled HTTP model fixture. Existing provider budget/authority and
  MCP independent-grant/RLS regressions also passed; these are not paid-provider
  quality measurements.
- Workspace clippy, 21 unit cases (three explicitly ignored live-Vault cases),
  generated schema with 165 unique operations, frontend typecheck/design/build
  and the repository governance checks passed during implementation.
- Three new browser cases passed: managed new Brain/compact navigation/agent
  wizard, existing Ask activation/plain note, and empty-catalogue registration.
  The updated coding-agent setup case also passed. Existing Ask five and Claims
  one passed in the first focused batch.
- Ten of eleven cases passed in the follow-up desktop/retention/MCP batch:
  six desktop cases, three catalogue/private-runner cases and the retention
  erasure/replay case. The remaining repeated-navigation measurement reached
  its recall stage but received HTTP 503. Its targeted rerun passed with the
  fixture worker present: 20 samples for each of nine routes and 20 useful
  exact/lexical recalls. The desktop harness now starts that worker, as the
  other canonical-recall suites already did.
- Three investigation cases passed with collapsed optional filters and real
  canonical responses: comparison/history/source/graph navigation, bounded
  copied context and late responses, and stale nested-evidence invalidation.
  The bounded-result fixture asks the server for one item through the request
  harness instead of relying on a removed browser numeric field.

The initial runs were not uninterrupted all-green suites. Required-field
selectors, delayed disclosure mounting, a removed erased-claim label and the
collapsed sidebar required test updates. One cold Graph route transition missed
the five-second assertion; the unchanged navigation assertion passed on rerun.
The original repeated-navigation service error remains in the log; only its
subsequent completed run establishes the measurement result. Capture and Recall
also exposed obsolete selectors: the new setup explanation, a disclosure absent
from supporting/captured-evidence modals, and the collapsed refine-search group
after navigation. Their final two-case rerun passed in 11 seconds, and the native
worker drained the queue. In total, 26 distinct browser cases passed across the
documented runs; this was not one uninterrupted full-suite run.

### Local deployment

`./scripts/stack.sh up` deployed image `0dce46b9b9e3` after the release/image build.
Migration 028 completed, API and worker started, PostgreSQL/Neo4j remained healthy,
and `/health/ready` returned `{"ready":true}`. Parsed before/after inventories
are equal: seven Brain identities, sixteen sources, seventeen source versions,
ten claims and fourteen claim revisions, including all recorded policy and grant
digests. Existing Brain policies were not adopted merely for testing.

Reloaded the user's in-app browser against port 8787 and observed the five primary
links, live Connections cards, graph evidence and the direct **Enable autonomous
memory** action in Ask. No JavaScript errors were reported. The existing graph
wheel-sensitivity advisory remains. Live screenshots are
`.cache/managed-memory-20260928/live-connections.png` and `live-ask.png`.

## Translation and Limits

These changes make the existing autonomous lifecycle the normal setup path.
They do not establish a universally best memory system or measured optimal
budgets. The preset's ceiling is an engineering bound. Supported capture still
requires the companion/host setup, and a configured provider is not a successful
call. Reference-only content, unavailable providers and revoked access remain
honest states. Raw expiry and evidence retirement are distinct from deleting
durable knowledge merely because it is old.

The previous six desktop/combined acceptance packs remain open independently.
Graph interactions/analytics, publication, activity, remaining evidence workflows,
real-provider UI learning and the integrated question → citation → correction →
subsequent recall journey are not signed off by this slice. No existing Brain's
private sources were sent to a provider for these changes. Local work remains
uncommitted; no push, release or external deployment was requested.

## Follow-up

The [managed experience pack](../roadmap/execution/archive/memory-managed-experience.md)
is closed against this evidence. Continue the separate desktop acceptance list
in [CONTINUE_HERE.md](../../CONTINUE_HERE.md); retain its remaining scope rather
than treating these focused checks as full product sign-off.
