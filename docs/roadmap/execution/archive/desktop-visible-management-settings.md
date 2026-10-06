# Visible management settings and concise navigation

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Address all seven marked navigation, description, AI and Privacy follow-ups.
- Non-goals: New paid retries, changed capture/retention policies or external MCP refresh.
- Delivery shape: Local frontend correction with source recovery audit, CUA and independent review.

## Governing Sources

- [Desktop follow-up](../../../contracts/desktop-experience.md)
- [Managed experience](../../../contracts/memory-managed-experience.md)
- [Semantic recovery](../../../contracts/retrieval-semantic.md)
- [Capture policy](../../../contracts/evidence-session-capture.md)
- [Owning epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Bottom Team nav; remove roster footer and extra connection metadata;
  readable legacy/source tool paragraphs; visible AI/Privacy fields; hide normal
  coverage/review card and empty read-view exclusions; align provider glyph.
- Out of scope: New backend/schema, token/native-host acceptance, policy changes.
- Blockers: None; explicit user follow-up supplies presentation authority.

## Surface and Interface Changes

- Interfaces: Existing routes, canonical Edit/Save/Cancel and diagnostic commands.
- Storage: N/A; original descriptions and saved exclusion arrays stay unchanged.
- Ownership: Desktop chrome, roster, connection renderer and settings presentation.

## Data and Authority

- Inputs: Current authorized roster, descriptions, policy and provider metadata.
- Authority: Existing installation-owner/admin, revision and privacy boundaries.
- Blind spots: Old flattened text has no exact original whitespace; only explicit
  display list boundaries are restored. Recovery source audit is not live provider proof.

## States and Edge Cases

- Loading: Existing skeleton/read errors; persistent bottom navigation.
- Empty: Omit empty read-view exclusions; editable optional privacy rules remain.
- Error: Existing safe drafts and revisioned errors; diagnostics remain optional.
- Blocked: Existing semantic policy/budget recovery; other failures stay idle safely.
- No-access: Team remains installation-owner-only; policy inputs remain read-only.
- Duplicate or replay: No new command; existing idempotency and exact save payload.
- Stale data: Existing fresh-authority/error clearing and draft conflict handling.
- Reconciliation divergence: Presentation never drops stored exclusion rules or claims
  unknown paid work succeeded; independent exact/lexical retrieval remains available.

## Integrations and Runtime Inputs

- Providers: Existing bundled Markdown/lowlight only; no new remote fetch/library.
- Environment: Existing local origin and build inputs; no added setting.
- Secrets: Existing redaction and hidden credentials; no secrets enter formatting.
- Failure handling: Preserve code bytes; safe Markdown and inert HTML/images;
  existing semantic bounded recovery reviewed without changing retry authority.

## Tests and Acceptance

- Automated: Focused render-only description checks, frontend build/type/design,
  whitespace and `./scripts/validate.sh`.
- Manual: CUA actual bottom nav, roster, tools, AI and Privacy views; capture limit
  and exclusions visible in Edit, Cancel restores; independent final screenshots.
- Acceptance: Seven points visibly corrected; no normal coverage review prompt or
  collapsible AI/Privacy fields; configured exclusions and real policy unchanged.

## Closeout

- Planned: Seven visible management corrections and recovery source audit.
- Shipped: Persistent bottom Team nav, concise roster/connection preview,
  render-only legacy description lists and existing JSON highlighting, aligned
  provider branding, visible AI/Privacy controls and optional coverage diagnostics.
  Local stack rebuilt and independent final source/screenshot review accepted.
- Not shipped: Unrelated direct-token native-host acceptance remains separately open.
- New blockers: None.
- Docs updated: Desktop, managed and semantic amendments, epic and active/archive
  indexes, [dated evidence](../../../mappings/desktop-visible-management-settings-2026-10-06.md),
  mappings index, this archived pack and handoff.
- Validation: Two pure formatting tests, production build/type/design, final
  governance/whitespace and CodeGraph checks; CUA actual seven surfaces, cancelled
  Capture/connection edits and unchanged saved revision/inventory. Source audit
  confirms existing bounded 5/30-minute replacements and one-minute pre-request
  block recovery; no runtime provider probe performed. Independent source and
  final screenshot review found no remaining blocker.
- Version: N/A; no release requested.
- Commit: Uncommitted.
