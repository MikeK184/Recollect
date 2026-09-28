# Desktop experience review and design concepts

Observed: 2026-09-26
Confidence: observed-once

Historical scope: this records the planning pass. The user subsequently approved
the complete proposal and requested implementation; the
[authority handoff](desktop-experience-authority-2026-09-26.md) records that later
instruction, accepted contracts and active packs. Baseline observations below
remain evidence from before those implementation changes.

## Sources and Method

The user requested a critical multi-agent UI/automation review, a detailed plan,
an Atlas-inspired reusable light frontend, contextual Brain navigation, and an
image for every proposed submenu view. Desktop only; no application
implementation was requested in this pass.

- Recollect worktree at HEAD `fbb92e2d2bff7f89a360a64154c5b8b5384eff6b`.
  Existing root-Compose/docs/script edits were present and preserved.
- Read applicable agent guide, doc router/maintainer skills, documentation
  lifecycle, accepted foundations, ADR 0006 and relevant domain contracts.
- Three independent read-only agents audited frontend/UX, Rust/API/MCP/Ask,
  and Atlas/Cognee/design/dependency evidence. Two reviewed the consolidated
  proposal for consequential errors.
- Repository CodeGraph status/sync was current; structural results were checked
  in source. The prior UI inventory and the user's screenshots establish the
  long-page context; this review did not execute product mutations.
- Atlas local commit `7eca7f7abd934c2e44bc096fc1dd0cbf94275b99`;
  Cognee local commit `c0d18c80e24b7b78918e7642c03f6f128fdd2aee`.
  Both reference checkouts were read-only.
- Published [Atlas](https://neoneye.github.io/agent-memory-atlas/) opened.
  The design reviewer checked official Mantine/TanStack/Vite, Hindsight and
  W3C/font sources and successfully queried Context7 for Mantine and Router.
  Exact sources/limits are in the [design spec](../roadmap/desktop-experience/design-system.md).
- Built-in image generation produced a separate desktop concept per proposed
  destination. All selected outputs were inspected; Devices received a targeted
  correction to remove an incorrect Brain-management menu.

## Observations

The current app uses React/TypeScript/Vite/Mantine and four top-level routes.
BrainDetail mounts the feature panels together. Autonomous memory is already
accepted and implemented. The remote agent catalogue has 22 tools; the native
bridge adds metadata-only workspace discovery.

Current Recall returns bounded attributed evidence, not a generated answer.
The existing model gateway is infrastructure, not a delivered question-answering
product. Its full-source resolver cannot simply replace recall's permitted
fragments. Ask needs a new canonical consumer, separate answering consent,
citation behavior, temporary-session rules and negative/positive proof.

Current model identities are deployment-selected/read-only in UI. Memory/source
list endpoints lack text-query filtering. Generated handovers accept exact
claim/decision/procedure revisions, not raw sources. The second review identified
these three distinctions; the final plan explicitly records them.

## Translation and Limits

[The proposal](../roadmap/desktop-experience/README.md) recommends nine Brain
routes, three global pages, content-first exploration, secondary diagnostics,
simpler setup, the existing stack and an Atlas-derived token system.

The function audit accounts for current source capabilities; it is not a fresh
runtime acceptance test. External competitor documentation is not a live
comparison. Mockups contain fictional names, content, counts, policy, endpoints,
credential references and timestamps. Their rendering is conceptual; written
tokens and future acceptance govern implementation.

New behavior is labelled separately: Ask, answering purpose, bounded graph
auto-load, list search and any necessary authorized aggregates. Saved
conversations, Ask-triggered tools/writes, arbitrary browser-installed connector
definitions and additional provider administration are not silently included.

At this review's baseline, no accepted ADR/contract governed a complete Ask consumer.
The plan describes the contract work required before its implementation. It
does not mark the redesign accepted/shipped or reopen completed historical packs.
Proposed slices name existing capability owners; register them and create
decision-complete packs when the reviewed scope is selected for implementation.

## Artifacts

- [Plan and page specifications](../roadmap/desktop-experience/README.md)
- [Complete current-function mapping](../roadmap/desktop-experience/current-ui-audit.md)
- [Agent inventory and proposed Ask contract](../roadmap/desktop-experience/agent-and-ask-design.md)
- [Tokens, components, framework and competitor evidence](../roadmap/desktop-experience/design-system.md)
- [Browsable image gallery](../roadmap/desktop-experience/index.html)
- [Generation prompts](../roadmap/desktop-experience/prompts.json)
- [Image review](../roadmap/desktop-experience/image-review.md)

The gallery is a standalone local static artifact. It contains no calls to the
product API. A local preview may be started from the repository with:

```sh
python3 -m http.server 8790 --bind 127.0.0.1 --directory docs/roadmap/desktop-experience
```

Then open [the gallery](http://127.0.0.1:8790/). Port 8787 remains the product.

## Validation

- `./scripts/validate.sh`: passed governance lint and all 32 checker tests.
  The first lint run rejected links into excluded reference checkouts; replacing
  them with commit-specific upstream links resolved it. The checker also required
  percent-encoded parentheses in the Cognee URL.
- `git diff --check`: passed. Existing root-Compose changes remain present;
  this task added only proposal/evidence artifacts and two index links.
- Focused artifact check: all 12 PNGs have valid PNG headers, landscape dimensions
  and widths of at least 1400 pixels; total approximately 12.9 MiB. All proposal
  local Markdown links resolve; all 12 view prompts are recorded.
- In-app browser: gallery loaded, Ask/Memory/Graph/Devices page selection and
  keyboard navigation worked, the 12-thumbnail overview expanded, hash-selected
  Graph survived reload, and the corrected Devices view was present. No browser
  console errors were observed in the checked log. Gallery left open at Ask.
- Two independent proposal reviews identified handover input, list-search and
  installed-provider distinctions; all three were corrected in the saved plan.
- No product regression/provider test was run: this task changed no application
  implementation. These checks prove proposal integrity and gallery behavior,
  not delivery of the redesigned application or new Ask feature.

## Follow-up

The original next action was to review information architecture, visual direction
and Ask boundaries, then register domain-owned slices and contracts. The user's
subsequent explicit approval completed that decision step; see the authority
handoff for implementation entry. This mapping's deliverables remain the planning
documents and images, not application shipment. Version N/A; changes uncommitted.
