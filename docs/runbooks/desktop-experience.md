# Use and maintain the desktop experience

## Purpose and prerequisites

This guide describes the implemented contextual desktop UI. The
[managed setup follow-up](../mappings/managed-memory-experience-2026-09-28.md)
records the simpler agent-first flow deployed on 2026-09-28. The earlier dated
[implementation evidence](../mappings/desktop-experience-implementation-2026-09-26.md)
separates completed checks and the verified final local image from remaining
domain and integrated acceptance. The final local image is serving this interface
at port 8787; its recorded Brain data and standing policies were preserved.
Start the authorized root stack through the [installation procedure](installation.md)
and open `http://127.0.0.1:8787`. Sign in using the existing account. Do not change
policy merely to make a status look connected. Desktop is the supported design
scope; dark mode and mobile/small-tablet redesign remain deferred.

## Find a function

```text
Workspace
├── Brains                 Create, find and open your knowledge spaces
├── Team                   Installation accounts and invitations (owner only)
└── Devices                Pair, approve or revoke your native identities

Inside a Brain
├── Ask                    Ask a supported question or Search evidence
├── Memory                 Claims, decisions, procedures, handovers and history
├── Sources                Original documents, references, versions and grouping
├── Graph                  Recorded relationships, paths and evidence
├── Connections            Coding agents, MCP servers, tool groups and runners
└── Workspace & settings   Expand for these secondary destinations
    ├── Repositories       Published snapshots and exact environment selections
    ├── Agents             Captured sessions and your working scopes
    ├── Activity           Processing, model/tool outcomes and data removal
    └── Settings           Autonomous setup, access and optional advanced overrides
```

Each Brain destination has `/brains/<brain-id>/<destination>` as its own path.
Opening the old `/brains/<brain-id>` path selects Ask. The Brain switcher clears
incompatible context; **All Brains** returns to the workspace. Browser Back,
forward and reload use real routes. Tabs are shareable, for example
`/brains/<brain-id>/settings?tab=privacy`; unsupported values use the default tab.
Opaque exact identifiers and selected scope may be shareable; questions, answer
text, free-text searches, private paths and credentials never belong in the URL.

## Read knowledge and evidence

**Ask → Ask a question** sends the current self-contained question and selected
scope only after the Brain's answering policy permits it. Each question retrieves
fresh evidence. At most four completed turns remain in browser memory; reload,
new conversation, navigation/Brain change, logout or invalidation clears them.
No tool is executed and no answer is automatically saved as memory.

New Brains created in the browser include the managed autonomous preset. Existing
Brains show **Enable autonomous memory** in Ask and **Settings → AI & automation**.
That one action authorizes the installed models for eligible retained Brain
content and turns on learning, semantic indexing, supported capture and Ask.
Existing exclusions and retention are preserved. No numerical tuning is required.
The preset has a 1,000,000-token daily accounting ceiling, two concurrent calls,
a 32 KiB input bound and 4,096 output tokens. These are engineering defaults,
not an optimal-cost claim. Advanced controls remain available for exceptions.

**Search evidence** works without a provider. **Refine evidence search** reveals
optional scope/time/channel filters; ordinary users do not choose result counts
or context budgets. The agent-facing MCP still accepts bounded task-specific
retrieval settings. Installed provider/model configuration is not connectivity;
Activity shows actual outcomes.

**Memory** shows All memory, Claims, Decisions, Procedures and Handovers. Search
matches recorded assertion fields, not every raw document. Open a record for
exact support, independent review/freshness/assessment and **Version history**.
**Add memory** accepts a plain note, retained as source evidence and learned
under the Brain policy. **Structured entry** remains an optional expert path.
**Review and corrections** is optional human intervention. **More memory actions**
holds erasure. The Handovers tab keeps the exact-contributor generation workflow;
it does not reuse Ask text as evidence.

**Sources → Add source** imports supported text files, pasted text or a reference.
Use **Filters** for collection/area/environment and **Manage views** for the
underlying group definitions. Search matches current titles. Open a source for
readable content and exact **Version history**; **More source actions** holds
explicit learning, excerpts, reprocessing and erasure. A reference-only or expired
source does not pretend to have readable raw content. Storage permission moved to
**Settings → Retention & privacy**. Exact direct links outside the current list
remain readable; edit title/grouping from the real source row when that metadata
is not on the current page.

**Graph** begins with a bounded knowledge view when no exact scope is selected.
Use Knowledge/Repository/Combined and **Filters → Apply graph filters** to change
it. Repository/combined views require explicit snapshots/manifests; no latest
revision is guessed. **List** is the keyboard alternative to the canvas. Open an
entity or relationship for canonical evidence, use **Find path** for textual paths,
and run **Insights** or **Graph status → Rebuild graph** only when needed. Position
and reachability describe recorded relationships, not operational impact.

**Repositories** contains published repositories and private **Your checkouts**.
Snapshots expose Files/Facts/Coverage/Contributors/Insights/Receipt. **Environments**
contains exact desired/observed revision manifests and history; it does not deploy
anything. **Publish from companion** supplies the native command because the
browser cannot scan arbitrary checkouts. Repository-scope memory links are wider
than one snapshot; inspect each memory's actual support.

## Configure automation once

**Agents → Connect an agent** provides direct HTTP MCP configuration, an access
token and a verification prompt for an actual scoped memory read. A setup card is not a
live connection. **Captured sessions** shows deliberately published evidence;
**Your working contexts** contains only the current account's tasks and checkouts.
Normal agents manage scope and retrieve/contribute memory themselves.

**Connections** starts with Codex and Claude Code cards. Ordinary memory access
uses native HTTP MCP; the companion and its hooks are optional for session
capture/local discovery. Native MCP OAuth and a published plugin are not yet
available. **MCP servers → Add connection** takes a name and anonymous server
URL, discovers metadata and prepares a connection, with a Context7 preset.
**Use registered connector** keeps the existing credential/runner setup, and
**Import manifest** accepts an operator-authored definition. Saving calls no
tool and does not grant execution rights. Continue to Tool groups to include
the connection, grant Use separately,
inspect cached tools, review actual tool inputs/effects and explicitly run it.
Manage, Share, Brain administration and installation ownership do not imply Use.
Runtime diagnostics and Activity retain cancel/reconcile/resolve/release semantics;
an uncertain side effect must not be retried blindly.

Within diagnostics, **Inspect call → Inspect captured evidence** opens nested
inspectors. Escape closes the top inspector and returns focus to its opening
control: evidence returns to the call, and the call returns to diagnostics.
Closing diagnostics clears its selected call,
so reopening the drawer starts with the activity view.

**Settings** separates General, Access, AI & automation, Capture and Retention &
privacy. The normal view explains automatic behavior; model/capture/retention
and storage overrides are collapsed. The preset configures the distinct backend
permissions together. Existing policies are preserved until explicit adoption.
Raw session/tool records default to 30 days; durable knowledge follows its evidence
lifecycle and does not disappear merely because it is old. **Activity** explains exceptions without becoming a mandatory work
queue: Processing, Tool calls, Model usage and Data removal retain their own
permissions and canonical recovery controls; the audit timeline is admin-only.

## Develop the shared interface

Use `web/src/design/tokens.ts` and Mantine theme/component defaults for all colors,
fonts and geometry; `app/navigation.ts` is the contextual icon map. Feature pages
compose the existing canonical typed API and inspectors. Keep visible protected
content checks; do not mount unrelated polling panels behind hidden tabs.

For the development-only component reference, start Vite from the repository:

```sh
npm --prefix web run dev
```

Open the printed local Vite origin at `/design.html` (normally
`http://127.0.0.1:5173/design.html`). This uses the real theme/components, adds no
production Node server, and is not a service-health test. The normal product
continues to use the Rust-served build at port 8787.

Fonts and licenses live under `web/public/fonts/` and `web/public/licenses/`.
The [font manifest](../../web/public/fonts/manifest.json) pins bytes/checksums and
upstream revision. The [SVG identity guide](../../web/public/brand/README.md)
describes the mark, monochrome variant and portable wordmark. Do not replace them
with generated screenshot icons or introduce a runtime external-font request.

## Verification

```sh
npm --prefix web run typecheck
npm --prefix web run build
./scripts/test-ui.sh tests/desktop.spec.ts
./scripts/validate.sh
git diff --check
```

Typecheck/build include `check:design`, which verifies shared font/color usage,
pinned font files/notices and self-contained SVG assets. The desktop test uses an
isolated stack/database; it does not modify the seven existing normal Brains.
Automated accessibility checks use development-only `@axe-core/playwright`;
their results complement keyboard, rendered visual and reflow inspection.
Run the affected domain/answer tests for behavior changes. Reference the dated
mapping for actual outcomes rather than treating this command list as proof.

## Current handover boundary

The shared shell and MCP setup slices are complete. The final local image,
seven desktop cases, real identity/invitation flows, Ask display/fallback,
MCP configuration/runtime and the recorded backend/provider checks are verified.
Some relocated domain workflows still need their final corrected regression run;
model-policy/autonomous-learning UI and the complete citation/correction/recall
journey remain part of acceptance. Read the maintained
[remaining verification list](../mappings/desktop-experience-implementation-2026-09-26.md#remaining-acceptance-and-operating-boundary)
before treating the full redesign as closed. Test fixtures and live readiness
remain separate from general model quality or production-capacity claims.

## Failure and recovery

- A disabled Ask policy is a supported state. Use Search evidence; an explicit
  retry after failure is a new accounted request, never automatic replay.
- When access, evidence, policy or expiry changes, protected payload disappears.
  Reload the exact canonical view after the prerequisite recovers. Do not keep
  cached text beside an error or broaden scope to make the error disappear.
- Empty and forbidden are different. Profile grants, Brain membership and private
  task ownership are independent; fix the actual authority rather than UI flags.
- For visual regressions, use a previous compatible frontend/image through the
  installation runbook. Do not restore old Brain grants, retention state or erased
  data just to undo a layout. Keep exact/text search available while diagnosing Ask.
- Inspect `./scripts/stack.sh status` and readiness after an authorized stack
  restart. Never use volume deletion as a UI restart or rebuild procedure.
