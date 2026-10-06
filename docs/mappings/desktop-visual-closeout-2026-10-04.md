# Ambient TV and final desktop polish acceptance

Observed: 2026-10-04
Confidence: verified

## Sources and Method

- User-authorized final TV acceptance, Graph/Connections/setup/Privacy polish and measured loading, with no file-size refactoring.
- [Desktop contract](../contracts/desktop-experience.md), [visual epic](../roadmap/epics/desktop-visual-experience.md), TV and final-polish execution packs.
- Existing disposable browser harness, real retained sources and claim revisions, controlled transport failures/deadlines/access withdrawal, and the existing canonical retention test.
- Frontend build/typecheck/design, clippy, governance validation, whitespace check and CodeGraph sync. Evidence logs and captures are ignored local `.cache/desktop-final-*` files, not published credentials or payloads.

## Observations

TV displays the bounded authorized recent claim page, preserves exact revision
selection across inserted items, and clears both transition cards and the feed
timestamp on failed refresh or expiry. Canonical `ClaimPage.expires_at` includes
claim content and supporting evidence whose retained labels were emitted, even
if a deadline passes during response construction. Already-sanitized expired or
erased support does not make otherwise retained claim content expire forever.
Review, freshness, operational and conflict/rule qualifications remain visible.

Seven focused browser journeys passed across the final targeted runs: TV lifecycle
and empty state; flat Graph chrome and keyboard entity picker; canonical lazy
Knowledge/evidence inspection with focus return and accessibility; searchable
connection catalogue with explicit Pause/Enable use; direct HTTP MCP setup and
its recorded-activity destination; Privacy direct retention editing, nested save,
lost-response erasure and cleanup. The latter six affected journeys are verified
in `.cache/desktop-final-ui-rerun.log` and `.cache/desktop-final-ui-last.log`;
the empty TV case passed in `.cache/desktop-final-ui.log`. Initial failures were
transport timing and expectations for superseded UI labels/controls; acceptance
was rerun after correction. No unrelated suite or new test file was added.

The existing backend `retention_deadlines_excerpt_independence_and_policy_authority`
test passes with assertions for support-owned and claim-owned deadlines, plus
sanitized expired support. Clippy and frontend build/typecheck/design pass.
The second UI reviewer approved current TV, Graph, Connections, final setup and
lineage captures at laptop/desktop widths; source review also checked deadline
membership and permission-aware Privacy actions.

The entry JavaScript shrank from 842,411 to 607,350 bytes, about 28%. Vite reports
191.27 KB gzip in the local fixture and 191.64 KB in the deployed Linux build,
down from the previous 256.24 KB. Actual owned browser requests
in `.cache/desktop-final-browser-assets.json` show the Brain chooser loads
755,589 JavaScript bytes across 14 chunks before the final 27-byte title fix;
KnowledgeSurface, ClaimsPanel and
ClaimReviewDialog are absent from that initial fetch. A fresh Memory route and
subsequent evidence inspection still incur their feature dependencies. This is
an entry-size and deferred-loading result, not an equivalent speed or total
network-savings claim. Existing files retain their responsibilities; no file was
split by line count and no dependency was added.

## Translation and Limits

Local implementation and focused acceptance are verified. The local stack was
rebuilt with `./scripts/stack.sh up --build`; readiness is true and the served
entry is 607,350 bytes. Fresh owner browser verification preserves all 16 Brains
and displays actual SWEG retained memories with proposed/conflict qualifications.
Live TV cycling and explicit exit, connection name filtering and direct Privacy
retention navigation were verified without saving or creating credentials. The
first cold graph read after restart reached its time limit; explicit retry
loaded 111 entities and 86 directed relationships. Warm retry is observed once,
not a cold-query performance guarantee. The TV exit title race found in live
verification was corrected by making the existing shell own the title; the deployed exit now restores the correct Activity title, and both TV
cases were rerun successfully in `.cache/desktop-final-tv-title.log`.
The live TV capture is `.cache/desktop-final-tv-live.png`; no new production
memories or configurations were created for this verification.
The current cream/ink/sage palette and flat graph remain the accepted design.
Setup completion links to Brain-wide recorded activity; it cannot identify or
certify the resulting plugin device. Search covers the loaded authorized
connection catalogue, not an unbounded inventory. Hidden-tab pause was tested
through a controlled visibility event; deadlines continue to govern display.

## Follow-up

Both accepted packs are archived, with the complete epic, indexes and handoff
reconciled. Final governance, whitespace and CodeGraph checks pass.
The research backlog, SSE, LongMemEval, commit/push and external publication
remain separate work. Version N/A; the working tree is uncommitted.
