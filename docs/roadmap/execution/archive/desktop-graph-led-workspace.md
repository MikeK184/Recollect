# Graph-led Brain workspace

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Four clear daily destinations, practical 3D graph and readable intelligent retrieval.
- Non-goals: Invented relationships, private workspace exposure, Ask writes, ambient TV closeout.
- Delivery shape: Local implementation and existing local stack verification; no commit/release.

## Governing Sources

- [ADR 0017](../../../adr/0017-desktop-knowledge-and-ask-experience.md).
- [Desktop knowledge](../../../contracts/desktop-knowledge-surface.md).
- [Graph exploration](../../../contracts/graph-exploration.md).
- [Answers](../../../contracts/retrieval-answers.md).

## Scope

- In scope: Dashboard/Ask/Graph/Explore navigation, secondary Manage, optional memory kinds, Markdown/code/tables, calmer evidence notes, opt-in automatic retrieval, 3D/2D and bounded exact contribution perspective.
- Out of scope: New graph authority, unbounded loading, paid fallback providers, source writeback or routine human review.
- Blockers: None; user authorized implementation and routine design choices.

## Surface and Interface Changes

- Interfaces: Optional RecallRequest strategy auto/manual; SnapshotDetail observed_at/valid_until; legacy routes preserved.
- Storage: No migration or persistent conversation/graph state.
- Ownership: Rust recall chooses strategy; frontend displays exact authorized records with renderer adapters.

## Data and Authority

- Inputs: Canonical graph, recent pipeline and explicitly selected snapshot contributions.
- Authority: Existing Brain grants, native path qualification, exact source/version and snapshot identities.
- Blind spots: Recent contribution feed is bounded; no inferred authorship, causality, online agents or current deployment claims.

## States and Edge Cases

- Loading: Bounded loading states; no illustrative data.
- Empty: Clear scoped empty state.
- Error: Clear affected content and report actual failure; WebGL offers 2D.
- Blocked: Report real prerequisite or policy blocker.
- No-access: Clear content on permission loss.
- Duplicate or replay: Existing stable request handling; a new explicit submission gets a fresh attempt.
- Stale data: Deadlines, memory epoch, Brain/scope changes and failed refresh clear selections/results.
- Reconciliation divergence: Preserve qualifications and channel coverage; no fabricated associations.

## Integrations and Runtime Inputs

- Providers: Existing approved gateway only; auto channels respect current policy.
- Environment: Existing ignored local configuration.
- Secrets: No values copied or exposed.
- Failure handling: Existing bounded queries and renderer cleanup; no automatic paid retries.

## Tests and Acceptance

- Automated: Existing web typecheck/build and relevant tests, focused Rust recall checks/clippy, governance validation and dependency audit.
- Manual: Desktop real graph/controls, Explore deep links, Markdown/citations, retrieval defaults, WebGL failure and expiry/access checks; independent UI reviewer.
- Acceptance: Main Graph with real 3D/2D, readable selected relationships and exact provenance boundaries; fewer daily menus; all original inspection/mutation actions reachable.

## Closeout

- Planned: Above authorized scope.
- Shipped: Dashboard/Ask/Graph/Explore sidebar and secondary Manage; one bounded Explore browser; optional memory kinds; lazy 3D/2D canonical graph with explicit neighborhood focus, orbit/zoom and exact recorded contribution perspective; safe formatted answers and natural evidence retrieval with qualified automatic channels. Final API and web image deployed to the existing local stack at `http://127.0.0.1:8787`.
- Not shipped: TV acceptance and overall epic closeout remain separate.
- New blockers: None.
- Docs updated: Governing ADR/contracts, owning epic/index, active/archive indexes and handoff; this pack carries the dated evidence without an additional mapping or hashes.
- Validation: Web production build/typecheck/design, Rust check/clippy/format, existing five Ask lifecycle cases, focused Recall/formatted-answer cases and scoped native recall test passed. Dependency audit reports zero production vulnerabilities. Independent source and live screenshot review has no material blocker. CodeGraph sync and final `validate.sh`/whitespace checks passed. Live browser proof and limits are recorded below.
- Version: N/A; no release bump.
- Commit: Uncommitted.

## Dated evidence — 2026-10-03

- Dependency evidence: Context7 checked react-force-graph, React Markdown and
  Lowlight APIs; official package documentation and installed source were
  inspected. The library's filtered bounding box still includes directional
  arrow geometry; explicit focus therefore fits finite selected-node coordinates
  through its camera API, without forking the graph engine or changing traversal.
- Performance boundary: 500 entities/2,000 relationships maximum; 3D is a lazy
  chunk (about 373 KB gzip), finite simulation settling, pixel ratio capped at
  1.5, hidden rendering paused, no perpetual particles/auto-orbit. Markdown parser
  and bounded syntax grammars load separately. There is no universal device FPS
  guarantee. Existing initial shell chunk size warning remains.
- Deployed evidence: SWEG knowledge reads 111 entities/86 directed relationships;
  selected exact repository snapshot reads 99/183. Contributions shows the actual
  30-input window and one snapshot publication, 36/57. Blocking snapshot refresh
  removes its publication portion immediately (33/54); networking was restored.
  Mismatched repository/snapshot links show a correction and withhold publication.
  Contribution perspective, exact selection and canonical repository name survive
  reload, including a repository outside the first list page.
- Interaction evidence: Real WebGL loss switches the same dataset to 2D, with
  an unobstructed notice. Explicit focus closes inspection, retains valid selection
  and enlarges the neighborhood; 2D focus and reopening the same entity pass.
  Real orbit drag moves the camera while preserving its target. Desktop 1440/1920
  bounds have no horizontal overflow. All temporary browser overrides/network
  blocks were reset; the retained user-facing tab shows the ordinary application.
- Inventory and cleanup: All original 16 Brains (one archived) remain unchanged.
  Existing test suites used disposable databases and cleaned their resources;
  focused UI proof made no provider calls and created no live Brain fixtures.
- Proof files: ignored `.cache/graph-led-3d.jpg`, `graph-led-overview.jpg`,
  `graph-led-find.jpg`, `graph-led-mismatch.jpg`, `graph-led-fallback.jpg` and
  `ask-formatted-proof.png`; build/deploy/validation logs and before/after
  inventory are under `.cache/graph-led-*`. Formatted-answer screenshot is
  synthetic test rendering; graph/finder/mismatch/fallback screenshots are real
  local runtime. No external deployment, release, commit or push is claimed.
