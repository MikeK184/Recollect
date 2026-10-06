# Readable management content

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Display readable Markdown and highlighted exact code in management surfaces.
- Non-goals: Runtime CDN scripts, interpreting remote HTML or fetching Markdown images.
- Delivery shape: Local bundled UI and bounded discovery text preservation.

## Governing Sources

- [Desktop](../../../contracts/desktop-experience.md)
- [Catalogue](../../../contracts/mcp-catalogue-and-profiles.md)
- [Owner](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Reuse bounded lowlight CodeBlock for JSON/TOML/YAML/shell, schemas,
  tool descriptions and agent configuration; preserve Markdown newlines on discovery;
  compact captured agent activity and hide automatic context/folder management.
- Out of scope: Recovering formatting already discarded in old cached metadata.
- Blockers: None; approved user corrections amend the catalogue contract.

## Surface and Interface Changes

- Interfaces: Shared CodeBlock supports exact explicit copy distinct from masked display.
- Storage: Existing text columns/manifest JSON; 2,000-character description limit retained.
- Ownership: Shared safe renderer; setup/activity workers consume the same component.

## Data and Authority

- Inputs: Authorized metadata and transient masked configuration presentation.
- Authority: Existing read gates; rendering never grants access or tool use.
- Blind spots: Old flattened metadata remains readable until explicit refresh.

## States and Edge Cases

- Loading: Exact plain code fallback while lazy grammar loads.
- Empty: Omit absent descriptions; explicit empty metadata states remain.
- Error: Clipboard feedback; unknown/failed grammars fall back to exact text.
- Blocked: N/A; rendering requires no provider.
- No-access: Existing sensitive view clearing on failed authority refresh.
- Duplicate or replay: N/A; renderer has no mutation except explicit clipboard write.
- Stale data: Props change cancels old highlight completion.
- Reconciliation divergence: Source code copied exactly; no recovered formatting claims.

## Integrations and Runtime Inputs

- Providers: Existing bundled lowlight/react-markdown; rmcp metadata inspection.
- Environment: N/A; no added runtime environment input.
- Secrets: Actual setup token never fed to highlighter; masked display and explicit copy differ.
- Failure handling: Highlight only labelled ≤16,384-character/500-line blocks; inert remote HTML/images.

## Tests and Acceptance

- Automated: Requested languages, exact copy, malicious Markdown, unknown/oversized fallback,
  discovery text preservation; focused build/typecheck and `./scripts/validate.sh`.
- Manual: Independent actual desktop visual review coordinated by parent.
- Acceptance: Consistent light highlighted code and readable descriptions with no active remote content.

## Closeout

- Planned: Shared safe code/Markdown and compact agent inspection.
- Shipped: Bounded bundled CodeBlock/CodeSyntax, exact explicit copy, preserved
  discovery Markdown/newlines, readable first-paragraph tool previews, compact
  selected-agent timeline with expandable evidence and hidden automatic-context
  controls. Fenced code does not inherit inline-code borders.
- Not shipped: Recovery of already-flattened old metadata remains out of scope.
- New blockers: None.
- Docs updated: Desktop/catalogue amendments, pack, epic/index, evidence and handoff.
- Validation: Runtime description/control/2,000-character unit proof passes.
  CUA renders JSON/TOML/YAML/bash, exact copy including newline, inert script
  and tracking-image content, and existing captured activity. Independent final
  visual review and frontend production build/type/design checks pass.
  Browser regression sources are authored, not executed as a browser suite.
- Evidence: [Coordinated delivery and limits](../../../mappings/desktop-browser-management-2026-10-05.md).
- Version: N/A; no release requested.
- Commit: Uncommitted.
