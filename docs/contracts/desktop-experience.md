# Contextual desktop experience

Status: accepted

The [managed experience amendment](memory-managed-experience.md) governs the
2026-09-28 managed setup, progressive disclosure and owner connector approval
changes; earlier explicit APIs and stored policies remain compatible.

## Source

The user explicitly approved the complete desktop proposal on 2026-09-26 and
requested implementation, consistent fonts/icons/SVGs and an original SVG logo.
[ADR 0014](../adr/0014-desktop-experience-and-answers.md), the accepted
[plan](../roadmap/desktop-experience/README.md),
[design tokens](../roadmap/desktop-experience/design-system.md) and existing
[foundations](../foundation/README.md) govern this work. Concept images illustrate
layout; invented labels, counts and generated icon variations are not API data.

## Contract

### Routes, identity and state

Global routes are Brains `/`, Team `/team` and Devices `/devices`. Team remains
installation-owner only. Inside `/brains/{brain}`, the contextual sidebar groups
Ask, Memory, Sources, Graph; Repositories, Agents; Connections, Activity, Settings.
Their suffixes are `/ask`, `/memory`, `/sources`, `/graph`, `/repositories`,
`/agents`, `/connections`, `/activity`, `/settings`. The Brain switcher and All
Brains link replace a second full global menu. `/brains/{brain}` resolves to Ask;
Search evidence remains functional when answering is disabled or unavailable.

Every destination has a real route and usable Back/forward, reload and direct
link behavior. Validated URL state holds non-sensitive tabs/filters and exact
same-Brain identifiers. Unknown section/tab values fall back to the documented
default; foreign identifiers fail through canonical reads. Questions, answers,
retrieved bodies, private checkout paths, secrets and tokens never enter URLs.
Resource inspection preserves exact revisions and the return location.

Mount only the current route's feature queries/polling, shared shell metadata
and mandatory checks for currently displayed protected content. Lazy-load feature
bundles. Brain/selection changes cancel superseded requests and clear incompatible
state; late responses cannot replace the new selection. Logout, current access
failure, observed epoch change and advertised expiry clear affected payloads.
Do not retain old protected data next to a failed refresh. Existing bounded
validity polling remains until an equally strong replacement is proven.

### Shared visual language

Use the accepted primitive, semantic and component token layers. The canvas is
`#f7f4ec`, surfaces `#fffdf8`, light sidebar `#eee9de`, ink `#101b2a`, secondary
ink `#2f3d4f`, accent `#3d6b59`, separators `#d7d0c2`. Preserve the full palette,
contrast roles, spacing and geometry in the design-system document. Newsreader
500 page headings are 36–40px/1.1 and section headings 24px/1.2; Manrope controls
14px/1.45 and reading text 16px/1.55; DM Mono metadata 12px/1.5. Self-host licensed
font files and notices; no runtime external font request. Lucide icons use one
shared size/stroke convention and navigation map rather than per-page variants.

Create an original vector Recollect mark expressing remembered fragments gathering
together, plus a reusable wordmark/compact icon and SVG favicon. It must remain
recognizable at navigation size, use the same tokens, and have appropriate text
alternatives or decorative semantics. A raster image is not a substitute.

Shared PageHeader, FilterBar, ScopeSummary, DetailInspector, StatusBadge,
AsyncState, SettingsSection and ActionMenu own recurring behavior. Keep forms,
dialogs, tables and graphs on that system. No dark mode or mobile redesign.
Prove desktop widths 1280, 1440 and 1920, browser zoom, keyboard/focus return,
reduced motion, readable long labels and non-color status meanings. Auth,
invitation, OIDC error/recovery and expired-session views use the same system.

### Page responsibilities

| Destination | Required behavior and boundary |
| --- | --- |
| Ask | Question/citations/inspector governed by [answers](retrieval-answers.md); existing Matches, Disagreements, Sources, Copy context and advanced exact/mode/time/budget criteria remain available as Search evidence. Applied restrictions stay visible. |
| Memory | Readable claim/decision/procedure/handover list and exact detail/history/support. Optional correction, review, withdrawal, conflict resolution and erasure retain canonical handlers. Handover generation selects exact claim revisions, never silently saves Ask text. |
| Sources | Current paste/text-file/reference import, list/version content, collections/areas/environment filters and one Manage views editor. New-version editing and contextual reprocess/learning/erase retain their effects. Show reference-only/unavailable bytes honestly. Storage policy lives in Settings. |
| Graph | Canvas first, compact kind/scope toolbar, filters/insights/status drawers and canonical inspector. Provide keyboard/list selection and textual paths. Default bounded knowledge overview uses current Brain-only investigation selection when no explicit selection exists; it invokes the existing read endpoint once, without model/rebuild/analytics. Explicit historical/repository/manifest selections are never guessed or reset. Missing exact inputs ask for selection. Existing bounds and oversize refusal remain. |
| Repositories | Published repositories, snapshots, files/facts/coverage/contributors/insights/receipt; Environments tab owns exact revision manifests/history. Browser publishing opens accurate native instructions. Own checkout paths remain account/device-private. Committed, desired and observed revisions stay distinct. |
| Agents | Supported host onboarding and real memory-read check, published sessions/capture coverage and advanced own task/scope controls. Setup is not an active connection. Capture policy has one editor in Settings. Browser metadata retry and native discovery are distinct. Other accounts' tasks remain private even from Brain admins. |
| Connections | Connections/Profiles/Runners; approved connector → target/environment → credential reference → runner → review → explicit test wizard. Independent Use/Manage/Share grants remain. Test may have effects and never implicitly retries uncertain work. No arbitrary executable installation. |
| Activity | Timeline/Processing/Tool calls/Model usage compose existing bounded authorized feeds, with canonical detail links. No fabricated counts or total causal ordering. Admin audit, profile outputs and private contexts retain independent authorization. |
| Settings | General/Access/AI & automation/Capture/Retention & privacy, with rare repair/history actions secondary. Installed provider/model identity read-only; permission, content classes, purposes and quotas editable by actual authorized roles. Capture, model transmission, answering and retention remain separate decisions. |
| Brains | Create/open, personal/shared, archived and search views with supported metadata; installation health is a small affordance with owner diagnostics. |
| Team | Existing account/invitation/local/OIDC/disable/reset/audit functions, installation-owner only; Brain access remains a different resource. |
| Devices | Own identities and native pairing instructions, pending approval/deny/revoke, observed last-seen only when available; approval stays explicit. |

Normal autonomous learning, capture and agent scope/tool work require no new
mandatory human clicking. Existing capability controls remain reachable in their
owning page or contextual inspector. Hiding a control does not change its default
or permission. Counts/statuses need actual response support; unavailable data is
not replaced with sample/mock values.

### Literal list search

Extend authorized source and claim list endpoints with optional `q`, trimmed,
at most 200 UTF-8 bytes. Empty means the existing unfiltered list.
Search is case-insensitive literal containment: `%`, `_` and escapes are ordinary
characters. Source search covers the current source title; Memory search covers
displayed assertion fields: subject, predicate, value and rationale.
The UI labels this scope. This first list search does not scan raw retained source
bodies, use a model, replace Recall or imply exhaustive evidence retrieval.

Apply current Brain, role, lifecycle and existing list filters first and apply
`q` before LIMIT/OFFSET. Preserve stable existing ordering and pagination. No
match is distinct from an empty Brain. Changing search or filters resets offset;
the browser consumes cancellation and stale pages cannot appear under a new query.
Existing exact detail, historical inspection and read/erase fences are unchanged.

### Permissions and failure behavior

Disabled answering leads to exact/text search, not a fake answer. Configured
credentials/models/agents/tools are not reported connected without a successful
corresponding call. Explicit model/tool checks show cost/effect implications.
Never infer profile Use from Brain membership or installation ownership.

Every page handles loading, empty, partial, failed, forbidden, archived and stale
states. Preserve safe unsaved input on recoverable errors, but clear invalid
protected payload. Destructive or effectful actions retain actual previews,
permissions, version checks and uncertainty. This redesign does not reset data,
enable model transmission or change capture/retention policy during migration.

## Acceptance

Prove all twelve destinations with real route navigation, direct links, Back/
forward, deep detail and role-appropriate actions. Focused backend tests prove
search before pagination, literal special characters, foreign Brain isolation,
empty/limit behavior and no model call. Existing mutation/domain regressions
must pass from their new locations. Graph proof covers an automatic bounded read,
no hidden expensive action, keyboard inspection and invalidation. Test private
tasks, profile grants, disabled/unconfigured states, expiry and in-flight Brain
switches with positive useful controls. Measure request volume/performance rather
than claiming improvement by layout alone. Verify the actual desktop assets,
fonts/logo and browser behavior; run frontend checks and `./scripts/validate.sh`.

## Explicit Deferrals

Mobile/small-tablet redesign, dark mode, arbitrary MCP executable approval,
browser provider installation, broader administrative MCP parity, new source
formats/connectors and general-purpose agent hosting. Saved or action-capable
Ask requires separate authority. No external deployment, automatic version bump,
commit or push is implied by this desktop delivery.
