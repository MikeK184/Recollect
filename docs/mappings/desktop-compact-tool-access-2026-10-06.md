# Compact Tool access acceptance

Observed: 2026-10-06
Confidence: verified

## Sources and Method

- User-approved concept: `output/imagegen/2026-10-06-tool-access-compact/tool-access-v2.png`.
- Authority: [desktop contract](../contracts/desktop-experience.md),
  [independent grants](../contracts/mcp-catalogue-and-profiles.md),
  [archived pack](../roadmap/execution/archive/desktop-compact-tool-access.md).
- Working-tree source: `McpProfileCard`, `McpPermissionIcons`,
  `member-permissions.ts` and scoped `inline-management.css`; initial baseline
  `c5ec05e` on `main`. Context7 Mantine guidance and installed Mantine source
  checked Popover focus, dismissal and portal behavior.
- Actual CUA browser journeys against the ready local stack, focused pure tests,
  non-secret canonical API comparisons and independent source/pixel review.

## Observations

### Layout and permission presentation

The compact header uses a 320×32 px name field, small environment picker and
Enabled switch. A description field appears only when present or requested.
The environment label remains visible during Share-only editing and when no
picker exists. The filter and outer tab strip align with bounded cards.
MCPs and People share two columns with a thin separator. Each person has one
adjacent Use/Manage/Share icon set; green/red states retain accessible labels.
Admin/Group/Direct chips open a small source popover. It preserves explicit
direct-grant editing and cleanup without repeating all effective/direct rows.
Read cards keep Tools & testing in the MCP column and omit the access footer.

Effective inheritance remains green and locked when a direct grant is cleared.
Brain administration supplies Manage/Share, never Use. Disabled or departed
members retain denied effective access while stored direct grants can be cleared.
Non-sharers see only their authoritative You row; withheld identities stay hidden.
Pending/archive guards include portal pickers. Same-name retained OIDC group
grants keep authoritative member and individual-grant rights; ambiguous group
editing does not guess which issuer is active.

### Browser and canonical command proof

On SWEG — test, the actual owner edited name, Enabled, MCP selection, a draft
person and direct permissions, inspected inherited locks, then cancelled all
drafts. Source-popover Escape returned focus and left no duplicate editor.

A real reversible description save staged unchanged existing demo/owner grants.
A tab-scoped controlled 503 failed the owner grant after configuration and demo
acknowledgments. The UI displayed partial-save receipts and retained that edit.
Retry sent only owner: attempts were `demo`, `owner`, `owner`. The description
was restored. Canonical snapshots confirm identical final group configuration,
grants/IDs, effective rights, 16-Brain count and three recorded tool-call IDs.
Audit/revision records naturally changed from these actual canonical writes.
No permission expansion, tool invocation or permanent fixture was added.

The existing demo account was checked through a task-owned loopback proxy using
its actual session, preserving the user's owner browser session. Actual Manage
and Share writes returned 403. Its UI showed Use allowed, Manage/Share denied,
no edit controls and no other people. Tools & testing loaded both approved Exa
tools. The proxy initially blocked this read-only discovery POST; allowing only
the exact Brain discovery route resolved that test-harness limitation. No tool
was executed. The proxy and temporary tabs were removed afterward.

### Validation and independent comparison

- `npm run build`: typecheck, design guards and production build passed.
- Eight focused pure permission tests passed: admin/direct separation, overlapping
  group inheritance, authoritative denial, staged group removal, independent
  powers, issuer-name collisions, disabled accounts and missing Brain access.
- Existing inline browser expectations were updated and listed successfully;
  those two fixture-based browser cases were not executed. Actual CUA journeys
  above supply running UI proof for this change.
- `./scripts/validate.sh`: governance checks and 32 tests passed.
- CodeGraph sync/status and `git diff --check` passed.
- Actual 1384×1473 and 2504×1314 browser layouts have no horizontal overflow.
  Wide content is capped at 1200 px; name control remains 320×32 px. Tab-scoped
  CDP metrics and full PNG capture replaced the in-app viewport API's stale or
  physically clipped screenshot results; overrides were cleared afterward.
- Independent `/root/ui_review` compared the approved image, final source,
  laptop and genuine full-width PNGs. All findings were fixed; final review
  reports no remaining UI or source blocker. Its checks included inheritance,
  inactive grant cleanup, portal guards, withheld readers, issuer collisions,
  header scope fallback and final alignment/spacing.

Evidence is in `output/tool-access-compact-review-2026-10-06/`:
`edit-final-1384.png`, `read-final-1384.png`, `edit-final-2504.png`,
`read-final-2504.png`, `source-final-2504.png`, `reader.jpg`,
`reader-tools.jpg`, `partial-save.jpg`, `viewport-proof.json`,
`save-proof.json`, `reader-api-proof.json`, `runtime-before.json` and
`runtime-after.json`. Earlier JPEG captures use `.jpg`; incorrect wide captures
were removed. Build/deploy/validation logs use `.cache/tool-access-*`.

### Local runtime

The final frontend-only image is
`sha256:2c12972776391aa4e69b87cca2b03d83dcf866713b57f8937f4b4291f13c5752`.
It layers current `web/dist` over inspected immutable runtime base
`sha256:5c10ac7b8b8857bac433dff449791fc348b4407ef3143765ad0deb42db418ac6`;
the existing backend binary is unchanged. Normal `./scripts/stack.sh up`
installed this image; `/health/ready` returned `{"ready":true}`. No clean
backend rebuild, external deployment, cache/data purge or release is claimed.

## Translation and Limits

This proves local implementation, actual canonical editing/retry and independent
visual acceptance of the approved compact layout. External OIDC inheritance is
covered by projection fixtures and canonical-source review, not a newly
configured live identity provider. Existing native-token host acceptance remains
with its separate active pack. Version: N/A. Commit: uncommitted.

## Follow-up

No remaining work for this slice. Refresh an already-open Tool access page to
load the new frontend assets. Existing native-host acceptance and research
backlog remain separate.

## Header badge follow-up

The subsequent explicit quick correction uses the small-fix exception: only
the existing metadata header/CSS changed, with the same draft commands and gates.
Environment and MCP count now have their own compact badges beneath the title.
During editing, the environment picker replaces its badge beside the MCP count;
Enabled stays on the right. Share-only/no-environment label fallbacks remain.

Actual CUA at 1280 px verified two separate badges in each read card, picker
alignment with the title's left edge and MCP badge's top, 26 px controls and no
horizontal overflow. Selecting Staging in the draft and cancelling restored
the user's existing Local test environment. No settings save was performed.
Read/edit PNGs are in `output/tool-access-header-badges-2026-10-06/`.
Build/type/design, CodeGraph, whitespace and required governance32 passed.
Normal local stack startup installed frontend-only image
`sha256:4c425ae34df3a0ad2ed9d86d176f84bfe4cea196e261669cfce0faca797ce4eb`;
readiness passed and the test tab was closed. Original user tab/drafts preserved.
No additional independent agent review was requested for this isolated follow-up;
the preceding compact-layout independent review remains historical proof.
