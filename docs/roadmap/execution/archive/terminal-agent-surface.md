# Terminal agent surface: terminal cards in the web UI and animated plugin CLI presentation

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Give agent/tool output a proper terminal look — a macOS-chrome terminal card component in the web UI for tool-call and session output, and an animated banner plus live status presentation in the Rust plugin CLI (`recollect-mcp-runner` / `recollect-agent`) so capture and MCP bridging feel alive on the host.
- Non-goals: No interactive TUI application; no new hook payloads or wire changes; no full ANSI interpreter (bounded safe subset only); no changes to what the runner does.
- Delivery shape: One web component + its applications, one Rust presentation module shared by the agent binaries, focused tests, live verification.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md) (amended 2026-10-03: terminal-style surfaces)
- [MCP plugin direct auth contract](../../../contracts/mcp-plugin-direct-auth.md) (runner behavior unchanged; presentation only)
- [claude-mem study, 2026-10-03](../../../research/claude-mem-study-2026-10-03.md) (TerminalPreview traffic-light chrome + ANSI rendering pattern; banner.ts offline frame/truecolor/NO_COLOR fallback interaction design)
- [Owning epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Web — a `TerminalCard` component in `web/src/components/` with macOS traffic-light chrome, monospace body (DM Mono), bounded safe ANSI subset rendering (colors, bold, dim; no cursor escapes; sanitized input), optional header title and status chip, scrollable body with max height, applied to the Activity Tool calls detail (tool I/O in terminal style) and the session/console output views where agent text is displayed. Rust — a small presentation module in `crates/agent` used by the CLI binaries: an animated startup banner (wordmark reveal + tagline, a few frames as static frame data — no video decoding) and live status lines for capture/MCP operations (spinner while working, colored ✓/✗/● outcomes, elapsed time); TTY detection with plain single-line output for non-TTY, `NO_COLOR`/`CLICOLOR=0` or CI. No new dependencies beyond what the workspace already has (check existing deps first; prefer hand-rolled frame data over adding an image crate).
- Out of scope: Interactive commands in the runner; changing hook exit codes, payloads or timing; web terminal for arbitrary user input; theme settings UI.
- Blockers: None — depends on `motion-design-foundation` tokens for the web card styling.

## Surface and Interface Changes

- Interfaces: None — presentation over existing reads and CLI behavior.
- Storage: None.
- Ownership: The agent crate owns CLI presentation; the web shell owns the terminal card.

## Data and Authority

- Inputs: Existing tool-call payloads, session output text, runner operation state.
- Authority: Unchanged; rendering grants nothing.
- Blind spots: Tool I/O can contain secrets or large blobs — the terminal card must respect the same truncation/redaction the current detail views apply; never widen what is displayed.

## States and Edge Cases

- Loading: Runner shows a spinner line while an operation runs; web card shows a skeleton body.
- Empty: No tool I/O → "No output captured" inside the card frame; runner prints nothing extra.
- Error: Failed operations render a red ✗ line with the existing error text (no new details invented); web card status chip turns red.
- Blocked: No new blocked state.
- No-access: Standard authorization applies to which payloads are fetched at all.
- Duplicate or replay: Re-rendering the same payload is idempotent; no animation on unchanged content.
- Stale data: Unchanged feed cadences.
- Reconciliation divergence: None — no writes.

## Integrations and Runtime Inputs

- Providers: None.
- Environment: `NO_COLOR`, `CLICOLOR`, TTY detection for the CLI; none for web.
- Secrets: None displayed beyond what current views already show; no new secret surface.
- Failure handling: ANSI parsing is total (unknown escapes render as literal text); banner decode failure falls back to a static wordmark line.

## Tests and Acceptance

- Automated: Web typecheck and design checks; Rust unit tests for the presentation module (TTY/NO_COLOR/plain fallback, spinner frame cycling, banner fallback) and `cargo clippy -p recollect-agent`.
- Manual: Owner login at the live stack — Activity → Tool calls detail renders tool I/O in a terminal card with readable monospace text and no raw escape garbage; run the runner locally against the live stack and observe the animated banner + spinner/✓ status lines on a TTY, and plain output piped to a file.
- Acceptance: Browser verification passes on real tool-call data; CLI verified on a TTY and non-TTY; typecheck/design checks and clippy clean; `./scripts/validate.sh` passes; live stack rebuilt and healthy.

## Closeout

- Planned: TerminalCard web component + applications, Rust presentation module (banner + status), tests, browser + TTY proof.
- Shipped:
  - Web `TerminalCard` (`web/src/components/TerminalCard.tsx` + `.terminal-card*` rules in `web/src/styles.css`, tokens `terminal`/`terminalBar`/traffic-light brand colors in `web/src/design/tokens.ts`): macOS traffic-light chrome, monospace body (DM Mono via `--rc-font-mono`), deep surface (`#0d0c0a`) below the page canvas, 420px scrollable body, optional title and status chip (ok/error/pending/muted). Bounded safe ANSI subset renderer in the component: SGR reset/bold/dim + standard and bright foregrounds (30–37, 90–97); all other escapes (cursor moves, OSC, hyperlinks, unknown SGR) are dropped; input renders as text nodes only.
  - Applied to the Activity Tool calls detail (`web/src/McpCallDialog.tsx`): sanitized tool result renders in a `TerminalCard` titled `mcp.call ·` plus the call's tool name, with a status chip from the call state; the existing "Full response" details and all truncation/redaction behavior are unchanged — no more data is displayed than before.
  - Rust presentation module (`crates/agent/src/presentation.rs`, exported from `lib.rs`): animated 5-frame ASCII wordmark reveal + tagline on a TTY (80ms frames, `\r`-clear), plain `recollect-agent 0.1.0` line on non-TTY/`NO_COLOR`/`CLICOLOR=0`/CI; live status lines (`status::start`/`finish`) with braille spinner on a background thread (80ms), colored ✓/✗ + detail + elapsed ("1.2s"), plain label-only fallback lines (label with ellipsis while working, then `done` or `failed:` plus the detail), and a quiet no-op mode for machine-owned streams. Status lines write stderr so stdout stays a clean contract (JSON reports, MCP stdio protocol).
  - Wired into `recollect-mcp-runner.rs` (banner + runner-execution status), `recollect-mcp-bridge.rs` (quiet banner + serve-loop status), `recollect-plugin.rs` (quiet banner) and `main.rs` (quiet banner after the hot `capture hook` path + capture-run status). No control flow restructured; existing log/output behavior intact.
  - Evidence: browser proof on the live stack — Activity → Tool calls on SWEG Brain (`5c054930-d266-4c18-a42b-942729f942aa`), fresh `resolve-library-id` call `b7769ee9-a820-4977-ba51-e58aa6e301ba` (Succeeded) renders tool I/O in the terminal card with traffic lights, monospace text, Succeeded chip, no raw escape garbage: screenshot `terminal-call-detail.png`. CLI proof: runner piped to a file prints plain `recollect-agent 0.1.0` with zero ANSI bytes and the original usage error; under a pseudo-TTY (`script`) the banner plays all 5 frames, bold wordmark and tagline; `recollect-agent whoami` piped prints 0 stdout bytes (quiet banner preserves JSON contracts).
- Not shipped: Session/console output views are not terminal-card-styled in this slice (the authorized application surface is the Activity Tool calls detail); full 256-color/truecolor ANSI support remains out of scope per the bounded-subset decision.
- New blockers: None.
- Docs updated: This pack closeout only; no contract, ADR or mapping changes were required (presentation over existing reads and CLI behavior).
- Validation: `web` `npm run typecheck` (design checks + tsc) passes; `cargo clippy -p recollect-agent` clean (0 warnings); `cargo test -p recollect-agent --lib presentation` 8/8 pass (banner fallback selection, plain banner line, bounded ASCII frames, spinner frame cycling, plain/ANSI status lines, elapsed formatting, plain round trip); `./scripts/validate.sh` passes; live stack rebuilt healthy via `./scripts/stack.sh up --build`; browser + CLI TTY/plain proofs above.
- Version: N/A: no release policy or version bump in this scope.
- Commit: Uncommitted; no commit, push or release performed.
