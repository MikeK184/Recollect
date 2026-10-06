# claude-mem Study — Research Notes for Recollect

Status: research mapping (evidence only, not implementation authority)
Date: 2026-10-03

## Source and method

- **Repo**: `references/claude-mem` (read-only study checkout; not modified).
- **Version**: 13.28.0 (`package.json` line 3), Apache-2.0 (`LICENSE`, `README.md`).
- **Head at study time**: commit `039c6160` (2026-10-01), TypeScript/Node ≥20 + Bun runtime.
- **Method**: read-only file reading of README, docs, and source. No builds, installs,
  or network calls. All claims below cite paths relative to `references/claude-mem`.
- **Scope note**: claude-mem is a single-user, per-machine Claude Code plugin
  (plus ports for Codex, Cursor, OpenCode, Grok Bot, etc.). It is a study
  reference for agent-memory UX and plugin mechanics; it does not govern
  Recollect implementation.

## Architecture summary

Layered system (`docs/architecture-overview.md`):

```
Claude Code (host)  — hook system + MCP client
CLI Layer (Bun)     — bun-runner.js (Node→Bun bridge), hook-command.ts orchestrator, handlers/
Worker Daemon       — Express HTTP API on per-user port 37700+(uid%100)
                      SessionManager, SDKAgent (Claude Agent SDK), SearchManager,
                      ProcessRegistry, ChromaSync, SSEBroadcaster
Storage             — SQLite (~/.claude-mem/claude-mem.db) + ChromaDB (chroma.sqlite3)
MCP Server          — stdio server exposing search tools to the host agent
```

Key evidence:

- Port default `37700 + (uid % 100)` (`src/shared/SettingsDefaultsManager.ts` line 350,
  `src/services/worker/README.md`).
- Hook wiring is a single JSON manifest with bash wrappers that locate the plugin
  cache and invoke `bun-runner.js worker-service.cjs hook <ide> <handler>`
  (`plugin/hooks/hooks.json`). Hooks: `Setup` (version-check), `SessionStart`
  (worker start + context injection; matcher `startup|resume|clear|compact`),
  `UserPromptSubmit` (session-init, 15 s timeout), `PostToolUse` (observation,
  async, 120 s), `PreToolUse` on `Read` (file-context, async), `Stop`
  (summarize, async, 120 s), `SessionEnd` (session-end, async).
- Hook exit-code discipline: transport errors (ECONNREFUSED, timeout, 5xx) →
  exit 0 so the host agent is never blocked; client bugs (4xx, TypeError) →
  exit 2 (`docs/architecture-overview.md`, "Graceful Degradation").
- Two session-ID namespaces: `contentSessionId` (host-invariant) and
  `memorySessionId` (SDK-agent side, changes on worker restart); conversion is
  handled by SessionStore for FK integrity (`docs/architecture-overview.md`).
- Reliability patterns: `PendingMessageStore` queue with binary parser contract
  (`{valid:true,…}` clears the session's pending rows; `{valid:false}` leaves
  them), and a generator restart loop (1 s/2 s/4 s backoff, stop after 3
  consecutive crashes) (`docs/architecture-overview.md`).
- The MCP server auto-starts the worker on demand before serving search tools
  (`src/servers/mcp-server.ts` around line 440).
- An optional "server" runtime exists (Docker pg+redis, API keys, Postgres-backed
  observations) selected at install time (`--runtime server`); the local
  "worker" runtime is the default single-machine path
  (`src/npx-cli/index.ts`, `docs/public/hosted-server.mdx`).

## How it works: capture → observations → compression → storage → recall

### Capture

1. Every `PostToolUse` event is forwarded to the worker, which stores the raw
   tool I/O in a `tool_uses` table keyed by
   `UNIQUE(content_session_id, tool_use_id)` with idempotent upsert on replay
   (hook retry / transcript re-scan refreshes the row via COALESCE) and 64 KB
   payload truncation with an explicit `…[truncated: N bytes]` marker
   (`src/services/sqlite/tool-uses.ts`).
2. The same event is enqueued to a per-session pending queue; a long-lived
   Claude Agent SDK "observer" session consumes the queue live
   (`docs/architecture-overview.md`, data-flow diagram).
3. Dedup at write time: `SHA256(memory_session_id + title + narrative)[:16]`
   content hash; if the hash exists within a 30 s window the existing ID is
   returned, no insert (`docs/architecture-overview.md`). A second, tiered
   near-duplicate classifier runs on titles: exact normalized-title match →
   safe silent auto-merge; IDF-weighted cosine above threshold with an
   IDF-veto check → "candidate" persisted for review, **never** a silent merge
   (`src/services/dedup/nearDuplicate.ts`).

### Observations (the atomic memory unit)

One belief at a time. The observer model is instructed to emit one or more
`<observation>` XML blocks (or a `<skip_summary reason="noise" />` sentinel).
The schema for the default `code` mode (`plugin/modes/code.json`):

- `type`: exactly one of 9 — `bugfix`, `feature`, `refactor`, `change`,
  `discovery`, `decision`, `security_alert`, `security_note`, `sensitive`.
- `title` (short), `subtitle` (one sentence, max 24 words).
- `facts`: concise self-contained statements, no pronouns, specific details.
- `narrative`: full context — what was done, how it works, why it matters.
- `concepts`: 2–5 from a fixed 7-keyword set — `how-it-works`, `why-it-exists`,
  `what-changed`, `problem-solution`, `gotcha`, `pattern`, `trade-off`.
- `files_read` / `files_modified`: full paths.

**Honesty note**: v13.28.0 has **no confidence field** on observations and no
confidence scores in search results (grep for `confidence` across `src/` finds
nothing product-relevant). Ranking is recency/relevance only. Any
"confidence-scored recall" in Recollect would be our own addition, not a copy.

The observer is a separate, tool-less SDK session explicitly framed as
"SILENT BY DESIGN … one-way recorder: observations in, XML out" — it must not
contact or influence the session it watches (`plugin/modes/code.json`,
`observer_role`). Prompt construction lives in `src/sdk/prompts.ts`:

- Per-field 16 k-char budget with 60 % head / 30 % tail slicing and an explicit
  `<elided chars=… original_size_chars=… />` marker so the model knows what was
  cut and does not fabricate the missing range (`OBS_PROMPT_FIELD_MAX_CHARS`).
- Image/base64 payloads are stripped by *shape* (Anthropic `image` blocks,
  Claude Code `Read` image files, MCP image results, OpenAI `image_url`, data:
  URLs) rather than by tool name, including inside nested JSON strings
  (`stripImagePayloads`).
- A one-shot schema re-reminder is appended if the previous reply drifted off
  the XML schema (`OBSERVATION_SCHEMA_REMINDER`, issue #3461).
- Session summaries use a distinct `<summary>` root with `request / investigated
  / learned / completed / next_steps / notes` fields, requested at `Stop`
  (`buildSummaryPrompt`).
- Modes are JSON config packs (`plugin/modes/`: `code.json`, `code--zh.json`,
  `law-study.json`, `meme-tokens.json`, …) that swap observation types,
  concepts, prompt text, and output language.

### Compression / summarization pipeline

There are no offline tiers; compression is live:

1. **Per tool use** → observer emits observations (or skip). This is the
   "raw notes → compressed memory" step, happening during the session.
2. **At Stop** → one `<summary>` checkpoint per user request.
3. **At SessionStart** → the context injector compiles stored rows into a
   compact *index* (see recall below) — this is the persistent compression
   surface: titles + timestamps + type icons + per-item token cost, grouped by
   date and file path (`docs/public/progressive-disclosure.mdx`,
   `src/services/context/`).
4. **Curated-answer layer**: a "knowledge agent" renders matching observations
   into a markdown corpus file, primes a dedicated SDK session with it, and
   answers questions conversationally against that corpus
   (`src/services/worker/knowledge/KnowledgeAgent.ts`, MCP tools
   `build_corpus` / `prime_corpus` / `query_corpus` in `src/servers/mcp-server.ts`).

### Storage

- **SQLite** (`~/.claude-mem/claude-mem.db`): `sdk_sessions`, `observations`,
  `session_summaries`, `user_prompts`, `pending_messages`, `observation_feedback`,
  `tool_uses`; a long hand-written migration chain in `schema_versions`
  (`src/services/sqlite/SessionStore.ts`).
- **ChromaDB** for vectors: each observation expands to multiple documents
  (`obs_{id}_narrative`, `obs_{id}_fact_0`, …); accessed through a separate
  `chroma-mcp` stdio subprocess managed by `ChromaMcpManager`
  (`docs/architecture-overview.md`, `src/services/sync/ChromaMcpManager.ts`).
- **Server runtime** (optional): Postgres schema with `projects`,
  `server_sessions`, `agent_events`, `memory_items` (kind: observation /
  summary / prompt / manual), `memory_sources`, `teams`, `api_keys`,
  `audit_log`, plus FTS5 mirrors (`src/storage/sqlite/schema.ts`,
  `src/storage/postgres/schema.ts`).

### Retrieval and recall presentation

- **Hybrid search**: SQLite metadata filter (project, type, date, file) →
  Chroma vector ranking over the candidate set → intersect → hydrate full rows;
  falls back to pure metadata ordering when vectors are unavailable
  (`src/services/worker/search/strategies/HybridSearchStrategy.ts`).
- **MCP recall is a 4-layer "progressive disclosure" workflow**
  (`src/servers/mcp-server.ts`, `plugin/skills/mem-search/SKILL.md`,
  `docs/public/progressive-disclosure.mdx`):
  1. `search` → compact index with IDs (~50–100 tokens/result).
  2. `timeline(anchor|query, depth_before, depth_after)` → chronological
     context around an anchor.
  3. `get_observations(ids=[…])` → full details only for filtered IDs
     (~500–1000 tokens each; always batched).
  4. `get_tool_uses(ids=[…])` → raw unsummarized tool I/O, last resort
     (up to 64 KB/row).
  A no-op meta-tool `important_workflow` exists purely to teach the agent the
  pattern ("10x token savings").
- **SessionStart context injection** renders the index with a legend of emoji
  type markers, per-item `~N tokens` cost column, date + file grouping, and a
  "context economics" header; an opt-in ACT-R-style reinforcement ranking
  (`CLAUDE_MEM_REINFORCE_ALPHA > 0`) scores rows as
  `ln(1 + age_created^-d + α·Σ age_reinforcement^-d)` so re-confirmed older
  knowledge climbs back into the window (`src/services/reinforcement/rank.ts`).

## Plugin and terminal UX mechanics

### Install flow (`npx claude-mem install`)

- Interactive TUI built on `@clack/prompts`: spinners with elapsed-time ticks,
  `select` / `multiselect` for IDEs (pre-selected from detection), provider
  choice (cmem host observer / claude / codex / gemini / openrouter / host),
  and runtime choice (worker vs server-Docker) (`src/npx-cli/commands/install.ts`).
- Non-interactive/CI paths are first-class: explicit flags, implicit defaults
  for non-TTY, no account prompts (`src/npx-cli/index.ts`, README "skip the
  sign-in" note).
- Setup installs Bun + uv if missing, runs `bun install` in the plugin cache,
  writes an `.install-version` marker; a sub-100 ms `version-check.js` Setup
  hook compares markers on every host startup and prints a "run `npx claude-mem
  repair`" hint on mismatch — always exit 0 (`docs/architecture-overview.md`).
- Command surface: `install / repair / update / uninstall / start / stop /
  restart / status / doctor / search / mcp / hook / transcript watch / project
  merge / adopt / cleanup` (`src/npx-cli/index.ts` help text).
- Skills ship inside the plugin (`plugin/skills/`): `mem-search`, `cloud-sync`,
  `handoff`, `timeline-report`, `weekly-digests`, `standup`,
  `knowledge-agent`, … — each a SKILL.md that teaches an agent workflow over
  the MCP tools.

### Terminal / TUI patterns worth imitating

- **Animated ASCII banner** at install (`src/npx-cli/banner.ts`,
  `banner-frames.ts`): 192 frames of 128×36 art rendered *offline from a webm
  video* into a luminance ramp, stored gzip-deflated + base64 (22 ms/frame).
  Rendering details: truecolor when `COLORTERM=truecolor`, else 256-color
  fallback; `NO_COLOR` → monochrome glyphs with zero SGR sequences; disabled on
  non-TTY / CI / `CLAUDE_MEM_NO_BANNER`; nearest-neighbor downsample to the
  actual terminal size (never upsamples); cursor hide/restore; abort on window
  resize; then a 14-step wordmark reveal, a 6-step tagline typewriter
  ("persistent memory across sessions"), and a 3-step brightness pulse.
  Fail-open: if frame decoding fails the banner is skipped, never breaking the CLI.
- **Live worker status**: `status`/`doctor` commands, log tailing
  (`scripts/worker-logs.cjs --follow`), and a viewer URL printed on startup
  (README "Web Viewer UI" feature list).
- **Viewer micro-animations**: SSE-driven feed with `isProcessing` +
  `queueDepth` indicators, skeleton loaders, a spinning favicon while the
  worker is active (`src/ui/viewer/hooks/useSSE.ts`,
  `useSpinningFavicon.ts`).

### Web viewer / visual language

- React 19 SPA served by the worker; live updates over SSE
  (`src/services/worker/SSEBroadcaster.ts`, `src/ui/viewer/hooks/useSSE.ts`).
- **Warm dark design tokens** (`src/ui/viewer-template.html`): backgrounds
  `#1a1916` / cards `#252320`; per-category accents — observations blue
  `#79b8ff`, summaries gold `#d4b888`, prompts purple `#8e7cbc`; badge tints at
  12–15 % alpha; monospace stack `Monaco, Menlo, Consolas, Courier New`.
- **Terminal card component** (`src/ui/viewer/components/TerminalPreview.tsx`):
  macOS traffic-light chrome (red/yellow/green dots), ANSI→HTML rendering via
  `ansi-to-html` sanitized with DOMPurify, word-wrap/scroll toggle, deep shadow
  — the "terminal card" pattern for showing agent output in a web UI.
- **Observation TV** (`src/ui/tv.html`): fullscreen ambient display mode —
  near-black `#0d0d0c`, single accent `#c15f3c`, one observation at a time with
  900 ms fade + scale transitions; a "wall screen" presentation of the memory
  stream.

## What Recollect already has equivalently

| claude-mem concept | Recollect equivalent | Honest differences |
| --- | --- | --- |
| Host hook capture (PostToolUse per tool use) | Coding-agent plugins (Codex/Claude Code/OpenCode) posting session transcripts to the server via hooks; canonical capture with per-Brain admission policy | claude-mem ingests *per tool call*; Recollect ingests *session transcripts* and lets the learning worker extract — coarser capture, richer extraction. |
| SDK observer agent (live XML extraction) | Learning worker extracting memories/claims/decisions into Postgres + Neo4j | Same idea (separate LLM pass over captured activity). claude-mem's observer is a persistent per-session SDK process with a pending queue; Recollect's worker is a durable Rust job pipeline. |
| Observation schema (9 types, 7 concepts, facts, files) | Memory items: claims, decisions, memories with provenance and Brain scoping | claude-mem's taxonomy is a fixed enum in a mode JSON; Recollect has a richer evidence/claim/decision model with time and corrections. **No confidence field in claude-mem** — our confidence-scored recall would be novel vs. it. |
| SQLite + Chroma hybrid search | Hybrid retrieval: vector + keyword + graph (Postgres + Neo4j) | Recollect has the graph leg claude-mem lacks; claude-mem keeps raw `tool_uses` as a 4th disclosure layer, which we can map to retained permitted documents. |
| MCP search tools (search/timeline/get_observations/get_tool_uses) | MCP recall tools + "Ask" chat surface | claude-mem's explicit 4-layer token-cost workflow and `important_workflow` meta-tool are more deliberate than our current single-surface recall. |
| SessionStart context injection (indexed, cost-annotated) | Session capture/priming via plugin hooks | We should verify how much of a cost-annotated index we inject today; claude-mem's per-item token column + legend is a concrete pattern. |
| Worker daemon + web viewer + SSE | Rust server + React/Mantine/TanStack web UI (Docker stack) | Our UI is functional but plain; theirs has the terminal-card/TV/SSE-feed language we want to move toward. |
| MCP coordinator (single-user local, auto-start worker) | MCP coordinator with Brains, device keycards, direct MCP tokens, placement central/local | We are far ahead: multi-Brain, team, auth. claude-mem is single-user per machine. |
| Cloud sync to cmem.ai hub | None (local-first; shared private server supported) | Theirs is vendor-specific; the *pattern* (sync-on-write, DB-as-queue) is reusable if we ever add multi-device sync. |
| `npx claude-mem install` TUI + skills | Plugin-managed capture/recall packaging (ADR 0018) with Rust runner | Their installer UX (spinners, animated banner, doctor, repair) is a strong model for our plugin/runner CLI presentation. |

## Adoption recommendations

### (a) Product / mechanics ideas

Prioritized highest-first for our vision and tech stack.

1. **Adopt the 4-layer progressive-disclosure recall contract.**
   - *What*: index (IDs + titles + cost) → timeline/context → full details by ID
     batch → raw evidence last resort; expose per-layer token/cost estimates in
     tool responses; add a no-op "workflow" meta-tool that teaches agents the
     pattern. Evidence: `src/servers/mcp-server.ts`,
     `docs/public/progressive-disclosure.mdx`.
   - *Why*: directly serves our Ask/MCP surfaces with token economics and agent
     autonomy; our graph leg makes the timeline layer stronger than theirs.
   - *Effort*: medium (protocol + MCP tool shaping over existing retrieval).
   - *Surface*: Rust core (protocol, server, mcp-runtime).
2. **Tiered near-duplicate reconciliation for the learning worker.**
   - *What*: exact normalized-title match → auto-merge; fuzzy tier (IDF-weighted
     cosine + rare-token veto) → candidate into a review queue, never silent
     merge. Evidence: `src/services/dedup/nearDuplicate.ts`.
   - *Why*: gives our `memory-capture-reconciliation` contract a concrete,
     validated algorithm (they tested against a 7,651-observation DB).
   - *Effort*: small–medium.
   - *Surface*: Rust core (worker).
3. **Raw-evidence tier with stable identity and truncation markers.**
   - *What*: store raw tool I/O / transcript excerpts keyed by a stable
     `(session, tool_use_id)` with idempotent upsert, 64 KB-style truncation
     carrying an explicit marker, and late linkage to extracted memories.
     Evidence: `src/services/sqlite/tool-uses.ts`.
   - *Why*: supports our "evidence" memory form and the layer-4 disclosure;
     makes summaries auditable ("show me the raw output").
   - *Effort*: medium (schema + capture path).
   - *Surface*: Rust core (server/worker, protocol).
4. **Hook exit-code discipline + async capture in our plugins.**
   - *What*: transport/availability failures → non-blocking success; genuine
     client bugs → blocking error code; make observation hooks async so capture
     never stalls the agent host. Evidence: `docs/architecture-overview.md`,
     `plugin/hooks/hooks.json` (async flags, timeouts).
   - *Why*: cheap reliability win for our Codex/Claude Code/OpenCode plugins.
   - *Effort*: small.
   - *Surface*: plugin CLI (Rust runner + hook wiring).
5. **Observer/extractor prompt hardening.**
   - *What*: skip sentinel; one-shot schema re-reminder after drift; head/tail
     elision with explicit `<elided>` markers instead of silent truncation;
     shape-based stripping of base64/image payloads from inputs.
     Evidence: `src/sdk/prompts.ts`, `plugin/modes/code.json`.
   - *Why*: these are battle-tested failure modes (oversized tool outputs,
     screenshot floods, schema drift) our learning worker will hit too.
   - *Effort*: small.
   - *Surface*: Rust core (worker prompts).
6. **Cost-annotated session-start index with type legend and date/file grouping.**
   - *What*: inject a compact index (ID, time, type icon, title, ~tokens)
     grouped by date and file path, plus an emoji/legend key and retrieval-cost
     guidance. Evidence: `docs/public/progressive-disclosure.mdx`,
     `src/services/context/`.
   - *Why*: better agent priming at near-zero cost; pairs with recommendation 1.
   - *Effort*: small–medium.
   - *Surface*: Rust core (worker/protocol) + plugin CLI.
7. **Opt-in reinforcement ranking for context/recall ordering.**
   - *What*: ACT-R-style score `ln(1 + age^-d + α·Σreinforcement_age^-d)` where
     re-confirmation events (retrieval hits, user corrections, re-cites) are
     "presentations". Evidence: `src/services/reinforcement/rank.ts`.
   - *Why*: durable knowledge stays current without manual curation; fits our
     memory-review/corrections contracts as a signal source.
   - *Effort*: medium (needs reinforcement event plumbing).
   - *Surface*: Rust core (retrieval + worker).
8. **Curated-answer corpus layer ("knowledge agent").**
   - *What*: render a Brain's relevant memories into a markdown corpus, prime an
     agent session against it, answer conversationally; expose build/prime/query
     tools. Evidence: `src/services/worker/knowledge/KnowledgeAgent.ts`.
   - *Why*: matches our "managed experience"/synthesis direction; gives Ask a
     deeper mode than ad-hoc RAG.
   - *Effort*: medium–large.
   - *Surface*: Rust core (worker) + web UI (Ask).
9. **Sync-on-write blueprint for future multi-device (do not build now).**
   - *What*: record the pattern — every row carries `synced_at` (NULL = not in
     log), writes nudge a debounced flusher, batches of ≤500 ops / 4 MB,
     canonical JSON ops with content hashes, tombstones, cursor-based pull lane,
     optional advisory WebSocket speed layer. Evidence:
     `docs/public/cloud-sync.mdx`, `src/services/sync/CloudSync.ts`.
   - *Why*: if we ever add multi-device sync for a Brain, this is a proven
     offline-first design that keeps the write path unblocked. Keep it out of
     scope until a product decision exists.
   - *Effort*: n/a (documentation only).
   - *Surface*: future Rust core.

### (b) Visual / UX language ideas (animated desktop UI modernization)

1. **Terminal-style card component for agent output surfaces.**
   macOS traffic-light chrome, ANSI→HTML rendering (sanitized), wrap/scroll
   toggle, monospace 12 px/1.6, deep shadow. Evidence:
   `src/ui/viewer/components/TerminalPreview.tsx`. Use for Ask transcripts,
   pipeline logs, and recall result detail panes. *Surface*: web UI.
2. **Warm-dark design token set with per-category accents.**
   Base `#1a1916`/cards `#252320`; category accents (blue observations, gold
   summaries, purple prompts) with 12–15 % alpha badge tints; monospace stack.
   Evidence: `src/ui/viewer-template.html`. Adopt as our Mantine theme base and
   map to our memory categories (claims/decisions/evidence) — this is the
   "cmem.ai-style dark glow" language, softened into a coherent token system.
   *Surface*: web UI.
3. **SSE/WebSocket live pipeline feed with processing indicators.**
   Stream memory items as they are captured → extracted → stored; show
   `isProcessing` + queue-depth state so the pipeline is visibly alive.
   Evidence: `src/services/worker/SSEBroadcaster.ts`,
   `src/ui/viewer/hooks/useSSE.ts`. This is the foundation for animated
   pipeline/graph visualization when agents save/process info (animate item
   arrival along capture→extract→store→graph edges). *Surface*: web UI + Rust
   core (event stream already implied by our worker; needs a broadcast channel).
4. **Animated ASCII banner for the Rust plugin/runner CLI.**
   Video→ASCII frame technique: offline-rendered frames, deflate+base64 bundle,
   truecolor/256/NO_COLOR fallbacks, TTY/CI detection, terminal-size
   downsample, wordmark reveal + tagline typewriter + brightness pulse,
   fail-open decoding. Evidence: `src/npx-cli/banner.ts`, `banner-frames.ts`.
   Rust equivalents exist (e.g., image-to-ASCII at build time); the *interaction
   design* is what to copy. *Surface*: plugin CLI.
5. **Ambient "memory TV" mode.**
   Fullscreen near-black display cycling one memory at a time with slow
   fade/scale transitions — a wall/second-monitor view of Brain activity.
   Evidence: `src/ui/tv.html`. Cheap to build on the live feed; strong demo
   value for the animated-UI direction. *Surface*: web UI.
6. **Confidence-scored recall presentation (our own addition).**
   claude-mem has no confidence scores — we can differentiate: render recall
   results with confidence bars/badges, provenance chips, and recency, in the
   terminal-card style. Pairs with recommendation (a)1's cost column.
   *Surface*: web UI + Rust core (expose score fields in protocol).
7. **Micro-status animations.** Spinning favicon / pulsing status dot while the
   worker processes (`src/ui/viewer/hooks/useSpinningFavicon.ts`); skeleton
   loaders for feed cards. *Surface*: web UI.

## What NOT to copy

- **cmem.ai cloud vendor lock-in**: per-user sync hub, Pro subscription,
  browser magic-link sign-in, better-auth account system, CMEM token references
  (`docs/public/cloud-sync.mdx`, `README.md`). We are local-first with a shared
  private server; no SaaS identity layer.
- **Bun/Node runtime dependency chain**: `bun-runner.js` Node→Bun bridge, uv for
  Python Chroma, esbuild bundling of the worker (`package.json`,
  `docs/architecture-overview.md`). Conflicts with a single Rust binary + our
  Docker stack.
- **ChromaDB via stdio MCP subprocess** as the vector store
  (`src/services/sync/ChromaMcpManager.ts`). We use Postgres (vector) + Neo4j
  in-process; no extra Python process.
- **Single-file SQLite as primary store** and the 30+ version hand-migration
  chain (`src/services/sqlite/SessionStore.ts`). Our Postgres schema is designed
  forward, not patched in place.
- **Per-user port arithmetic** `37700 + (uid % 100)`
  (`src/shared/SettingsDefaultsManager.ts`) — a multi-user hack we don't need
  with our server model.
- **Claude Agent SDK as the observer engine** (`@anthropic-ai/claude-agent-sdk`
  in `package.json`, `src/services/worker/agents/`). Anthropic-specific; our
  learning worker follows our provider policy.
- **Plugin-marketplace cache versioning machinery**: nvm PATH surgery and
  semver-sorted cache-directory scanning baked into every hook command string
  (`plugin/hooks/hooks.json`). Our Rust plugin binary + managed runner makes
  this unnecessary complexity.
- **Telemetry (posthog), Discord release notifications, token/CMEM community
  material** (`package.json`, `README.md` "What About CMEM?").
- **Their mode JSON packs as a user-facing feature**: interesting internally,
  but our per-Brain admission policy and provider policy already own that
  configuration surface; don't ship a parallel mode system.
