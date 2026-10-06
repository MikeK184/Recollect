import type { CSSProperties, ReactNode } from "react";
import { ansiForeground, palette } from "../design/tokens";

// Bounded safe ANSI subset for untrusted terminal text: SGR reset (0), bold
// (1), dim (2) and standard/bright foreground colors (30–37, 90–97). Every
// other escape — cursor moves, OSC sequences, hyperlinks, unknown SGR
// parameters — is dropped. Input is always rendered as text nodes only.

type SgrState = { bold: boolean; dim: boolean; fg: string | null };
const INITIAL_SGR: SgrState = { bold: false, dim: false, fg: null };

function applySgr(state: SgrState, params: number[]): SgrState {
  let next = state;
  for (const param of params.length ? params : [0]) {
    if (param === 0) next = { ...INITIAL_SGR };
    else if (param === 1) next = { ...next, bold: true };
    else if (param === 2) next = { ...next, dim: true };
    else if (ansiForeground[param] !== undefined)
      next = { ...next, fg: ansiForeground[param] };
    // Unknown parameters are ignored.
  }
  return next;
}

function sgrStyle(state: SgrState): CSSProperties | undefined {
  if (!state.bold && !state.dim && state.fg === null) return undefined;
  return {
    ...(state.fg !== null ? { color: state.fg } : {}),
    ...(state.bold ? { fontWeight: 700 } : {}),
    ...(state.dim ? { opacity: 0.65 } : {}),
  };
}

/** Render untrusted text with the bounded safe ANSI subset applied. */
export function renderAnsi(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  let state = INITIAL_SGR;
  let plain = "";
  const flush = () => {
    if (plain) {
      nodes.push(
        state.bold || state.dim || state.fg !== null ? (
          <span key={nodes.length} style={sgrStyle(state)}>
            {plain}
          </span>
        ) : (
          plain
        ),
      );
      plain = "";
    }
  };
  let i = 0;
  while (i < text.length) {
    const char = text[i];
    if (char !== "\u001b") {
      plain += char;
      i += 1;
      continue;
    }
    // An escape sequence: decide what it is, then drop or apply it.
    const next = text[i + 1];
    if (next === "[") {
      let j = i + 2;
      while (j < text.length && /[0-9;]/.test(text[j])) j += 1;
      const final = text[j];
      if (final === "m") {
        flush();
        const params = text
          .slice(i + 2, j)
          .split(";")
          .filter((part) => part.length > 0 && /^\d+$/.test(part))
          .map(Number);
        state = applySgr(state, params);
      }
      i = j + 1; // drop the whole CSI sequence
    } else if (next === "]") {
      // OSC (title, hyperlink…): consume to BEL or ST and drop.
      let j = i + 2;
      while (j < text.length && text[j] !== "\u0007" && !(text[j] === "\u001b" && text[j + 1] === "\\"))
        j += 1;
      i = text[j] === "\u0007" ? j + 1 : j + 2;
    } else if (next !== undefined && next >= " " && next <= "/") {
      // Other two-byte escapes with intermediates: consume to the final byte.
      let j = i + 2;
      while (j < text.length && text[j] >= " " && text[j] <= "_") j += 1;
      i = j + 1;
    } else {
      i += 1; // lone escape: drop it
    }
  }
  flush();
  return nodes;
}

export type TerminalStatusTone = "ok" | "error" | "pending" | "muted";

/**
 * macOS-chrome terminal card for agent and tool output. The body renders
 * untrusted text through the bounded safe ANSI subset; it never widens what
 * the caller passes in.
 */
export function TerminalCard({
  title,
  status,
  text,
  emptyText = "No output captured",
}: {
  title?: string;
  status?: { label: string; tone: TerminalStatusTone };
  text: string;
  emptyText?: string;
}) {
  return (
    <div className="terminal-card" aria-label={title ?? "Terminal output"}>
      <div className="terminal-card-bar">
        <span className="terminal-dots" aria-hidden>
          <span style={{ background: palette.trafficRed }} />
          <span style={{ background: palette.trafficYellow }} />
          <span style={{ background: palette.trafficGreen }} />
        </span>
        {title && <span className="terminal-card-title">{title}</span>}
        {status && (
          <span className={`terminal-chip terminal-chip-${status.tone}`}>
            {status.label}
          </span>
        )}
      </div>
      <pre className="terminal-card-body">
        {text ? renderAnsi(text) : (
          <span className="terminal-card-empty">{emptyText}</span>
        )}
      </pre>
    </div>
  );
}
