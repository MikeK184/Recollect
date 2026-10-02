# Desktop implementation and acceptance evidence

Observed: 2026-09-26
Confidence: observed-once
Implementation: uncommitted working tree; no release version.

## Sources and Method

Compared the accepted [desktop contract](../contracts/desktop-experience.md),
[answer contract](../contracts/retrieval-answers.md), [ADR 0014](../adr/0014-desktop-experience-and-answers.md)
and [approved plan](../roadmap/desktop-experience/README.md) with the actual Rust,
React, shared assets and tests. This record separates source inspection, isolated
fixture proof, real provider calls and the observed normal-stack upgrade.
It does not replace the owning execution packs or their open acceptance criteria.

The feature worker read the governing domain contracts before implementation and
used the repository CodeGraph launcher for bounded source navigation, verifying
ambiguous relationships in source. The documentation maintainer reviewed current
files and preserved the ignored Cognee and Atlas checkouts. No commit or push was
made. Test data is synthetic and held in disposable owned harness databases.

## Implemented surfaces

| Surface | Local implementation and behavior |
| --- | --- |
| Shell | `web/src/main.tsx`, `App.tsx`, `app/BrainLayout.tsx`, `app/navigation.ts`: three global and nine contextual Brain destinations, lazy route bundles, actual session/Brain boundaries and legacy Brain redirect to Ask. |
| Shared design | `design/tokens.ts`, `theme.ts`, `typography.css`, reusable components and feature CSS: one light palette, font/heading/icon conventions, readable empty/error states, drawers and dialogs. |
| Sources | `EvidencePanel.tsx`: debounced literal title search, collection/area/environment filters, import and exact version drawer; uncommon processing/learning/erasure actions are secondary. Storage permission is in Settings. |
| Memory | `ClaimsPanel.tsx`, `MemoryPage.tsx`: type tabs, assertion search, readable canonical records, exact support/history, optional review/correction, and existing handover generation. |
| Graph | `GraphPanel.tsx`, `GraphExplorer.tsx`, lazy `GraphCanvas.tsx`: bounded entry read, canvas/list alternative, exact scope, filters/paths/insights/status drawers and canonical evidence inspection. No navigation-triggered model, rebuild or analytics action. |
| Repositories | `WorkspacePanel.tsx`, `PublicationPanel.tsx`: published snapshots and files/facts/coverage/contributors/insights/receipt; environment manifests remain exact. Browser publication shows the native command; own checkouts remain private. |
| Agents | `AgentsPage.tsx`, `CapturePanel.tsx`: pair/host/read-check guidance, published captured sessions and coverage, and own working scopes. Setup is not reported as a running connection. |
| Connections | `McpPanel.tsx`, `McpConnectionDialog.tsx`: approved connector, target, credential reference, runner and review steps; saved configuration then offers **Configure profile & test**. Test execution remains an explicit independently granted profile operation. |
| Activity | `ActivityPage.tsx`: bounded admin audit timeline plus separate authorized processing, calls, model usage and data-removal tabs. No global causal order or fabricated total. |
| Settings | General, Access, AI & automation, Capture and Retention & privacy compose existing canonical editors. Installed models are read-only; answering remains separately permitted and default-off. |
| Ask | `features/ask/AskPage.tsx`, Rust `answers.rs`, retrieval bundle/gateway and migration 027: typed server-built evidence, complete validated answers, metadata-only replay/cancel lifecycle, temporary four-turn display and retained Search evidence. |

### Navigation refinements from acceptance review

The first source audit found route/tab links but missing exact resource hydration,
local-only filter state, no explicit post-save test handoff and overly prominent
raw evidence JSON. Those findings caused code fixes before closeout:

- `app/useBrainSearch.ts` validates opaque identifiers and non-sensitive scope;
  Sources binds source/version and grouping filters, Memory binds claim and frozen
  knowledge/fact criteria, Graph binds applied exact scope and bounded center /
  direction / hops. Questions, answers and free-text list searches stay out of URLs.
- Source-version-only links resolve the existing canonical evidence identity,
  then read its exact history/content. A link outside the first catalogue page
  does not fabricate a `SourceSummary`. Title/group editing still needs the real
  row metadata; direct inspection explains that limitation.
- Memory history links include the canonical revision and knowledge cutoff. A
  mismatched revision is refused rather than replaced by another version.
- Source **Explore memory linked to this version** uses an incoming exact
  `source_version` graph. Snapshot **Memory in this repository** is honestly
  repository-scoped; it does not claim every listed memory derives from that snapshot.
- Connection saving offers a profile/test next step without executing a tool.
  Raw canonical evidence metadata is under **Technical record**.
- Current failed canonical reads hide cached content; scope/epoch changes close
  affected graph inspectors. Server validity and mutation handlers remain authority.
- Nested tool inspectors suspend the parent drawer's Escape/outside-click/focus
  trap while a child is open. Explicit diagnostics close clears the selected call;
  persistent owners restore the invoking control when a conditional inspector
  closes. This fixes the reported source/call/drawer cascade and stale reopened
  call. All three runtime browser cases passed, including top-layer Escape and
  invoking-control focus; the final image contains the correction.
- A live two-node graph exposed overlapping long labels. The existing COSE
  layout now derives spring length, repulsion and component spacing from the
  145px caption width; it adds no layout dependency. The default mode summary is
  **Include uncertainties**, with existing filter semantics unchanged. The final
  live screenshot `.cache/desktop-upgrade/graph-final.png` confirms separated
  captions on that retained two-node graph.

These refinements typecheck; exact deep-link and filter-restoration browser proof
passed. Remaining domain regression acceptance is listed below.

## Asset and dependency evidence

The four self-hosted font files are pinned to Google Fonts commit
`23e54b51ddffbc7713c583748e3bd86f62b1fa4a` in
[`web/public/fonts/manifest.json`](../../web/public/fonts/manifest.json), with
source URLs, SHA-256 values and byte lengths. Newsreader and Manrope are variable
normal faces; DM Mono has regular and medium faces. All three SIL OFL notices
are in `web/public/licenses/`; `font-display: swap` supplies local fallbacks.

The original [Recollect identity](../../web/public/brand/README.md) includes a
full-color gathering mark, a one-color mark, and a portable outlined Newsreader
wordmark. The full-color symbol is the SVG favicon. SVGs have titles/viewBoxes and
no external resource or font dependency; interface icons use Lucide rather than
copies of generated concept-image icons. `FilterBar`, `ScopeSummary`,
`DetailInspector` and native-disclosure `ActionMenu` are now adopted by Sources
and Memory; their current handlers, labels and focus behavior are retained.

`npm --prefix web run check:design` is included in `typecheck` and `build`. It checks
feature color/font literals, all four font hashes and byte lengths, OFL notices,
and self-contained titled SVGs. It is a structural guard, not a rendered contrast
or full accessibility audit. `web/design.html` is a development-only Vite entry
using the real shared components/theme. It adds no production server or framework.

Successful Context7 guidance used `/websites/v8_mantine_dev` for theme variables,
Drawer focus/Escape/return behavior, `useFocusReturn`, `Tabs keepMounted={false}`
and Stepper control;
`/websites/tanstack_router` for nested routes and validated search state. Primary
references are [Mantine Drawer](https://v8.mantine.dev/core/drawer/),
[Tabs](https://v8.mantine.dev/core/tabs/), [Stepper](https://v8.mantine.dev/core/stepper/),
[focus return](https://v8.mantine.dev/hooks/use-focus-return/),
[theme variables](https://v8.mantine.dev/styles/css-variables/),
[TanStack search](https://tanstack.com/router/latest/docs/framework/react/guide/search-params)
and [Vite static delivery](https://vite.dev/guide/static-deploy).
Context7 `/cytoscape/cytoscape.js` layout guidance was checked alongside the
installed `3.34.3` COSE implementation for label dimensions and force options.
Its default 32-unit spring was insufficient for the observed long captions;
the local spacing correction uses the existing built-in layout.
The UI test worker's deferred-tool discovery did not expose Context7 tools;
it used the primary [Playwright network](https://playwright.dev/docs/network)
and [assertion](https://playwright.dev/docs/test-assertions) documentation instead.
The integrated test setup added development-only `@axe-core/playwright` with
Context7 `/websites/playwright_dev` and the official
[Playwright accessibility-testing guide](https://playwright.dev/docs/accessibility-testing).
The installation reported zero npm vulnerabilities at this observation boundary;
that is a dated dependency-audit result, not a claim of universal security.
No additional production server or runtime is required.

## Completed validation

Results below were reported by the responsible worker and, for navigation/assets,
compared with the on-disk test and measurement artifacts. The normal installed
service was rebuilt and ready on the final local image, with preserved recorded
inventory. Individual suite results below remain distinct from a full green run.

| Check | Actual result and limit |
| --- | --- |
| `npm --prefix web run typecheck` | Passed, including shared design checks after the deep-link and nested-modal refinements. |
| `npm --prefix web run build` | Passed in the final UI harness and normal product image build, including typecheck/design checks. Final image manifest-list prefix is `5126dccc`; no release version or commit was created. |
| `./scripts/validate.sh` | Governance lint and all 32 tests passed. This proves documentation structure, not application behavior. |
| `git diff --check` | Passed after feature refinements. |
| `./scripts/test-ui.sh tests/desktop.spec.ts` | All 7 cases passed across a 6-pass run in 21.8s and the repaired font-fallback/auth case in 3.4s. Covers twelve destinations, route/tab history/reload, unknown-route fallback, route consumer isolation, keyboard/focus, reduced motion, long labels, 1280/1440/1920 layout, assets, in-flight Brain/logout clearing, exact source/claim links and foreign-ID refusal, filter restoration, auth/archived/no-access/error, font fallback, 200% text and repeated measurements. The initial 7-case command was not all-green: its fallback blocker matched `.woff2` while the actual fonts are `.ttf`; the corrected targeted rerun passed. Logs: `.cache/ui-desktop-final.log`, `.cache/ui-desktop-fallback-final.log`. |
| Accessibility scans | Final axe scans found zero violations on all twelve routes and tested drawers/auth/archived/error states. Earlier scans found unnamed close controls, contrast and ARIA issues; those findings were corrected and rescanned. Automated scans and sampled keyboard/reflow checks are not a certification of every state or browser. |
| Identity and devices | `./scripts/test-oidc.sh ui tests/team.spec.ts`: 2 passed in 11.0s against owned Dex, covering local invitation/revocation and a real signed group claim. Worker drain and DB/graph/Dex cleanup passed. Device pairing UI: 2 passed. The earlier local-only Team run conditionally skipped OIDC; the owned-Dex run supplies the missing evidence. |
| MCP setup and runtime | Catalogue/wizard/independent grants/private-runner suite: 3 passed. Runtime suite: 3 passed in 1.1m, covering lost response/no duplicate effect, uncertain reconciliation, managed capture, cancellation, expiry, Use revocation and nested Escape/focus. Coding-agent guidance: 1 passed. Configured state is still distinct from a successful fixture call. |
| `./scripts/test-ui.sh tests/evidence.spec.ts` | 1 passed in 15.5s: source import/retry/history/associations/policy/reference-only and desktop overflow. |
| `./scripts/test-ui.sh tests/claims.spec.ts` | 1 passed in 9.7s through the redesigned Memory route. |
| `./scripts/test-ui.sh tests/review.spec.ts` | 1 passed in 7.5s through optional review/correction controls. |
| Other completed isolated UI journeys | `procedures.spec.ts`, `graph-combined.spec.ts`, `workspace.spec.ts`, `capture.spec.ts`: one passed each. Latest `recall-graph.spec.ts`: 1 passed in 13.3s; `investigation.spec.ts`: 3 passed in 26.4s, including scoped graph/history/comparison and stale nested evidence/late responses. Remaining corrected domain reruns are listed below. |
| `./scripts/test-ui.sh tests/ask.spec.ts` | 5 passed in 15.4s. Production paths cover default-off useful Search evidence, no-evidence and metadata replay without a model call. Positive rendering, canonical-citation inspection, protected-content clearing and cancellation use explicitly labeled transport fixtures based on real canonical Recall; those UI fixtures do not substitute for backend/model eligibility proof. |
| `cargo test -p recollect-server --test platform models::answers:: -- --ignored --test-threads=1` | 6 passed, 0 failed in 10.99s: five deterministic database/local-HTTP scenarios plus the opt-in real-provider guard returning without a call when unset. Includes literal list search, exact fragments/citations/replay, reference-only zero-call exclusion, mixed positive/invalid citation, scope/history/access/budget/expiry, policy/epoch/erasure/restart/cancellation, and separately keyed semantic embedding lifecycle. |
| Focused gateway/recall regressions | `gateway_concurrency_policy_change_failures_and_uncertain_replay`, `provider_policy_budget_roles_and_real_http_gateway`, and `canonical_recall_keeps_corrections_out_of_raw_copies_and_survives_rebuild`: one passed each. |
| Unit/generation | `cargo test -p recollect-server --lib answers::tests`: one passed. `./scripts/generate-api.sh`: 162 operations. `cargo check -p recollect-server`: passed. Whole-workspace unit suites subsequently passed with 21 passing and 3 explicitly ignored live-Vault tests; `.cache/desktop-upgrade/workspace-unit-tests.log` records the actual boundary. |
| Clippy | `cargo clippy -p recollect-server --all-targets -- -D warnings` passed in 19.05s. The subsequent whole-workspace clippy run passed in 1m32s; `.cache/desktop-upgrade/workspace-clippy.log` records that result. |
| Ask with actual graph dependencies | `cargo test -p recollect-server --test platform graph_backed_answer_preserves_exact_witness_and_suppresses_replaced_generation -- --ignored --test-threads=1 --nocapture`: one passed in 6.60s against PostgreSQL and Neo4j. Exact packed witness reached the gateway; replacement of the generation with unchanged memory epoch suppressed in-flight answer/usage. |
| Repeated local recall/answer timing | `RECOLLECT_TIMING_PROOF=1 cargo test -p recollect-server --test platform answer_and_recall_local_timing_probe -- --ignored --test-threads=1 --nocapture`: one passed in 4.59s. One first-flow and twenty warm samples per flow against PostgreSQL and a loopback HTTP model fixture; detailed environment/boundaries below. |
| Installed reflow and keyboard sample | The browser worker observed no overflow at an emulated 200% equivalent of 640×400 CSS pixels with DPR 2; skip-to-content landed on main. Native Command-plus was ineffective in the in-app browser, so native browser zoom is not claimed. This is a bounded sample, not all-route accessibility certification. |
| Normal root Compose upgrade | Final `wrap-up-build.log` built `recollect-dev:local`, manifest list `sha256:5126dccc68a04d770ea9ac98ff186ed02396de0209da5bc38d1319da3ad4295d`. `wrap-up-start.log` records API/worker-only recreation and health; live readiness is true at `127.0.0.1:8787`. Parsed `before.json` and `final-inventory.json` remain equal. Final graph/browser checks saw no JavaScript errors; Cytoscape emitted only its wheel-sensitivity advisory. |
| Explicit installed-provider probe | `RECOLLECT_REAL_PROVIDER_PROOF=1 cargo test -p recollect-server --test platform answer_installed_provider_smoke_opt_in -- --ignored --test-threads=1 --nocapture`: succeeded with installed `gpt-5.6-luna`, 843 tokens, unsuppressed. The synthetic source said Lumen uses port 8080 and did not claim verified deployment. Disposable DB/artifacts were removed; real Brain policies were not enabled. This is a provider-path sample, not universal answer quality proof. |

The deterministic answer scenarios are
`answer_cancellation_policy_epoch_erasure_and_restart_suppress_output`,
`answer_historical_revision_grant_revocation_content_budget_and_expiry`,
`answer_semantic_attempts_share_lifecycle_without_paid_replay`,
`answer_supported_fragments_policy_isolation_citations_and_replay`, and
`desktop_list_filters_before_pagination_and_treats_wildcards_literally`.


### Installed-provider answer-quality samples

The opt-in `answer_installed_provider_quality_eval_opt_in` exercised six distinct
synthetic cases through installed `gpt-5.6-luna`: supported port 8181, proposed but
unverified deployment on port 8282, unresolved conflicting 7001/7021 claims, exact
historical port 8080 without current 9090 substitution, embedded instructions
ignored while answering port 4141, and unsupported-region abstention with no
factual statements. Each case made exactly one provider call and no product write.

The initial six-case command exited 101 because its last abstention matcher did
not recognize equivalent wording (“does not specify” / “not documented”), despite
correct model abstention. After repairing the rubric, only that selected case was
rerun with `RECOLLECT_REAL_PROVIDER_EVAL=1` and
`RECOLLECT_REAL_PROVIDER_EVAL_CASE=unsupported_region`; the command passed. The
five earlier successful paid cases were not repeated. All six distinct cases
therefore passed across the initial run and selected-case rerun, not one falsely
reported all-green initial command. Seven paid quality calls totaled 7,203 tokens;
the separate smoke above used 843. Disposable fixture state was removed before
the test assertion. These samples are bounded support/abstention evidence, not
universal accuracy or independent deployment verification.

### Measurements and their limits

`.cache/ui/desktop-navigation-measurements.json` records one isolated test run.
Heading-visible navigation observations were 54–75ms for the nine Brain routes.
They are single observations, not p95, settled-data latency or a before/after
performance comparison. The per-navigation request windows end when the heading
appears, so a pending Graph view request was captured in the following Repositories
window; those arrays cannot establish final request totals for each destination.

The separate settled Sources window recorded two evidence catalogue reads and
no Graph/MCP/models/capture/tasks/jobs feed. This is useful route isolation proof,
not a universal network budget. Screenshots are
`.cache/ui/desktop-sources-{1280,1440,1920}.png`; they use synthetic content.

The later `.cache/ui/desktop-repeated-measurements.json` records twenty samples
per contextual route. Sidebar-click to heading-visible p95 ranged from 35.3 to
37.1ms across the nine routes on that run. Request counters stop at the heading
boundary and include cached navigation; they are not settled-data totals or a
pre-redesign latency comparison. Twenty completed canonical exact/lexical recall
requests over useful synthetic evidence had p95 26.1ms and no provider calls.
The live graph screenshot proves the observed small graph's readable layout;
no general canvas-interaction p95 or large-graph capacity result is inferred.

`.cache/desktop-upgrade/entry-requests.json` contains one observed entry for each
design on the same installed Brain and host. During the first three seconds from
the first Brain-detail request, the old all-in-one entry made 39 API requests and
the new Ask entry made 4. The window includes shell reads and excludes assets.
These are different implemented entry experiences; this establishes the reduced
mounted request set in that sample, not a speed, capacity or p95 improvement.

`.cache/answer-recall-timing-2026-09-26.json` records twenty warm serial samples
after one first-flow sample. The dataset has twenty synthetic sources and one
packed candidate; exact/lexical investigation uses limit 3 and an 8,192-byte
context budget. On native debug macOS arm64 with disposable local PostgreSQL,
warm nearest-rank p95 was 29.743ms for standalone recall's production handler
(28ms engine), and 135.729ms for Ask's handler (30ms retrieval, 6.657ms durable
model-attempt window). First-flow handler observations were 38.856ms and
154.664ms respectively; caches were not flushed, so these are not cold starts.
The model was a loopback HTTP fixture: 21 fixture calls, no external calls.
The window includes auth/SQL and fixture accounting, but excludes browser,
front-end network, TLS and real-LLM latency. This was a shared developer machine,
one actor and one serial synthetic query, not controlled-load capacity proof.

### Normal-stack preservation

The root Compose upgrade served the redesigned application at
`http://127.0.0.1:8787`. The preserved inventory artifacts
`.cache/desktop-upgrade/before.json` and `final-inventory.json` were independently
compared as parsed JSON and are equal: seven Brain identities, sixteen sources,
seventeen source versions, ten claims and fourteen claim revisions. Brain/group
grant, model-policy/head, retention and capture digests also match. This checks
the recorded inventory and standing policies rather than every byte of content.
No existing Brain's answering policy was enabled for demonstration. The final
image manifest-list prefix `5126dccc` includes the modal, graph and accessibility
refinements. Only API/worker were recreated for the final deployment; dependency
data was retained. The graph screenshot above was inspected from that live build.

### Preserved failed attempts and dependency recovery

The initial domain-test ledger `.cache/desktop-ui-regressions.tsv` and corrected
rerun ledger `.cache/desktop-ui-reruns.tsv` are separate. Selector/timing repairs,
the initial axe findings and the font-extension test error are not silently
relabeled as an uninterrupted green suite. The paid answer-quality rubric failure
and selected-case rerun are recorded above.

A transient Neo4j startup window around 20:18–20:20 UTC interrupted the worker /
privacy cleanup after a passing Platform browser case, then prevented the Recall
harness from checking analytic catalogue ownership before browser execution.
This was not a Recall assertion failure or a product code fix. Once Neo4j had
fully started, a fresh Query API `RETURN 1` returned HTTP 202, GDS reported
`2026.08.1`, and the catalogue count was zero. The backend worker reconciled,
graph-cleaned and dropped the two exact owned fixture databases and removed
their attributable artifact/journal paths. Normal migration preflight
`./scripts/docker.sh compose run --rm --no-deps --pull never migrate` passed
at 20:29 UTC. User Brain data was not part of fixture cleanup.

## Remaining acceptance and operating boundary

The shell and MCP setup slices have completed their owner checks and are archived.
The six remaining product/acceptance packs stay in progress; the app being live
does not declare those required checks complete. Remaining verification is:

- Finish the corrected Recall, Graph, Graph analytics, Publication, Retention and
  Operations browser runs, plus Evidence/Claims reruns after URL refinements.
  Existing individual passes retain their actual tested revision and scope.
- Complete the relocated model-policy/autonomous-learning UI checks. Those
  optional-provider suites were not run in this turn; server gateway/answer
  fixtures and older model UI evidence do not prove the new settings presentation.
- Reconcile the source/workspace, memory/settings, graph, Ask integration and
  Activity owner criteria with the final domain ledger, including the complete
  question → citation → authorized correction → subsequent recall journey.
- Keep the measurement limits above: native browser-chrome zoom was not proved,
  documented reflow/text-scaling equivalents were used; graph interaction p95,
  universal capacity and real-LLM latency are not claimed.

Installation/recovery external-proof suites were not rerun for this UI change;
their prior archived results remain historical. Current local image readiness and
preservation are freshly verified above. No pending test is reclassified as an
optional product feature in order to close the integrated acceptance pack.

## Follow-up

Use the [desktop guide](../runbooks/desktop-experience.md) for navigation,
component development and safe recovery. The
[active acceptance pack](../roadmap/execution/archive/desktop-experience-acceptance.md)
is the current source of unfinished delivery work. No external deployment,
release version, commit or push is implied. Preserve prior shipped evidence
records rather than rewriting them as redesign proof.
