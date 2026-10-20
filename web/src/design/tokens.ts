// Recollect design tokens for script consumers (ADR 0025). The CSS custom
// properties in tokens.css are the source of truth for both themes. Canvas,
// WebGL and graph renderers cannot read var(); they call resolveColor() and
// re-read on useThemeMode().resolved changes. Styles use the utilities.

/** Read the current computed value of one token, e.g. readToken("brand"). */
export function readToken(name: string): string {
  if (typeof document === "undefined") return "";
  return getComputedStyle(document.documentElement)
    .getPropertyValue(`--${name}`)
    .trim();
}

/**
 * Resolve a token to a concrete sRGB colour string for renderers that do not
 * parse oklch() (Cytoscape, three.js). Falls back to the computed token text.
 */
export function resolveColor(name: string): string {
  const value = readToken(name);
  if (typeof document === "undefined" || !value) return value;
  const probe = document.createElement("canvas").getContext("2d");
  if (!probe) return value;
  probe.fillStyle = value;
  probe.fillRect(0, 0, 1, 1);
  const [r, g, b, a] = probe.getImageData(0, 0, 1, 1).data;
  return a === 255 ? `rgb(${r} ${g} ${b})` : `rgb(${r} ${g} ${b} / ${a / 255})`;
}

const token = (name: string) => `var(--${name})`;

export const kinds = [
  "claim",
  "decision",
  "procedure",
  "handover",
  "source",
  "entity",
  "tool",
] as const;
export type Kind = (typeof kinds)[number];

// Terminal ANSI foregrounds on the fixed dark terminal surface (both themes).
export const ansiForeground: Record<number, string> = {
  30: token("terminal-muted"),
  31: token("ansi-red"),
  32: token("ansi-green"),
  33: token("ansi-yellow"),
  34: token("ansi-blue"),
  35: token("ansi-magenta"),
  36: token("ansi-cyan"),
  37: token("terminal-foreground"),
  90: token("terminal-muted"),
  91: token("ansi-red"),
  92: token("ansi-green"),
  93: token("ansi-yellow"),
  94: token("ansi-blue"),
  95: token("ansi-magenta"),
  96: token("ansi-cyan"),
  97: token("terminal-foreground"),
};

// Concrete family stacks for canvas text, which cannot read CSS variables.
export const fonts = {
  display: 'Newsreader, ui-serif, Georgia, serif',
  ui: '"Mona Sans", ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif',
  mono: '"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, monospace',
};
