# 0014: Contextual desktop experience and evidence-backed answers

Status: accepted

Amended 2026-10-01 by the user's [display and interaction priority](../contracts/desktop-experience.md#display-and-interaction-priority--2026-10-01):
normal laptops and larger desktop monitors are primary; small-screen and extra
keyboard work are optional rather than completion requirements.

## Decision

Implement the user's explicitly approved 2026-09-26
[desktop experience plan](../roadmap/desktop-experience/README.md) in the existing
React/TypeScript/Vite/Mantine/TanStack/Cytoscape frontend and Rust same-origin
service. Use nine contextual Brain routes and three global routes, with one
light-only token system, self-hosted Newsreader/Manrope/DM Mono, consistent Lucide
interface icons and an original Recollect SVG identity inspired by gathering
memories again. Feature epics retain their existing data and authority ownership.

Add read-only evidence-backed Ask through the existing canonical retrieval and
model gateway. The browser supplies questions and selection, never trusted
evidence or authority. Server retrieval creates a typed, exact-fragment bundle;
the gateway checks permission, content classes, current canonical validity and
quota before transmission and publication. Add a separate `answering` purpose,
off for existing policies. Questions and answers are temporary browser state;
the server persists bounded operational metadata, not conversation content.
Do not stream unvalidated tokens. Do not execute tools or create memory from Ask.

## Why

The user accepted the complete reviewed design and asked to implement it all,
reuse the visual system consistently and add an original SVG logo. The
[review](../mappings/desktop-experience-review-2026-09-26.md) found that the
existing single Brain screen exposes functioning backend capabilities as a large
collection of forms. Contextual navigation and task-oriented views improve that
presentation without replacing the functioning Rust authority model. Existing
Recall returns evidence, so a text box alone cannot deliver the approved Ask
experience. A separate consumer contract makes that new product behavior explicit.

The accepted [foundations](../foundation/README.md) and
[runtime ADR](0003-product-runtime.md) already provide the necessary stack.
Neither a framework replacement, production Node service, second authorization
layer nor a new mandatory external service is justified by this change.

## Consequences

The [desktop contract](../contracts/desktop-experience.md) owns navigation,
presentation, compatibility and feature placement. The
[answer contract](../contracts/retrieval-answers.md) owns new question lifecycle,
evidence, citations and temporary output. The original domain contracts retain
mutation, device, model, profile, retention and private-task authority.

Each page mounts only its own data consumers plus shell and visible-content
validity checks. Legacy Brain links continue working. New list search filters
authorized records before pagination. Activity composes only authorized bounded
feeds; aggregation does not widen visibility. Existing policy, data and grants
survive the redesign. An installed model/provider is shown read-only; configuring
permission is distinct from installing an adapter or granting tool use.

Delivery proceeds through nine slices in existing epics. Their packs specify
meaningful failure tests and actual browser/API proof. Acceptance of the design
authorizes implementation; it is not evidence of shipment or connectivity.

## Supersession

Extends ADR 0003's frontend presentation and model-consumer posture. The generated
answer deferral in the original investigation contract is superseded only by the
new answer contract; saved conversations and action-capable Ask remain deferred.
No other existing capability, permission or lifecycle contract is superseded.
