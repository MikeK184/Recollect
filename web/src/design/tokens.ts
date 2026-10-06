// Atlas primitives are deliberately centralized; features consume semantic roles.
// Original light cream, dark ink and sage palette; motion and category accents
// remain shared across the desktop, pipeline, terminal cards and ambient view.
export const palette = {
  ink: "#101b2a",
  inkSoft: "#2f3d4f",
  paper: "#f7f4ec",
  paperDeep: "#eee9de",
  paperLight: "#fffdf8",
  line: "#d7d0c2",
  lineDark: "#a9a192",
  sage: "#8fb7a2",
  sageDark: "#3d6b59",
  amber: "#e8b861",
  amberSoft: "#f4e2bd",
  rose: "#d98272",
  blue: "#7f9fc6",
  violet: "#a58fbd",
  cyan: "#7ab5b2",
  field: "#857c6b",
  selection: "#e8f0e9",
  danger: "#9b3f31",
  dangerSoft: "#f8e5df",
  muted: "#59636a",
  terminal: "#fffdf8",
  terminalBar: "#eee9de",
  trafficRed: "#ff5f57",
  trafficYellow: "#febc2e",
  trafficGreen: "#28c840",
} as const;

export const tokens = {
  canvas: palette.paper,
  surface: palette.paperLight,
  sidebar: palette.paperDeep,
  ink: palette.ink,
  secondary: palette.inkSoft,
  muted: palette.muted,
  border: palette.line,
  field: palette.field,
  accent: palette.sageDark,
  selection: palette.selection,
  danger: palette.danger,
  dangerSoft: palette.dangerSoft,
  attention: palette.amberSoft,
  focus: palette.sageDark,
  amber: "#805817",
  terminal: palette.terminal,
  terminalBar: palette.terminalBar,
} as const;

// Per-category accents for memory kinds and related record categories. Each
// accent carries a low-alpha tint (~12–16%) for badges, glows and selection.
export const categories = {
  claim: { color: "#365f8c", tint: "rgba(127, 159, 198, 0.14)" },
  decision: { color: "#805817", tint: "rgba(232, 184, 97, 0.14)" },
  procedure: { color: "#715389", tint: "rgba(165, 143, 189, 0.14)" },
  handover: { color: "#286963", tint: "rgba(122, 181, 178, 0.14)" },
  source: { color: palette.sageDark, tint: "rgba(143, 183, 162, 0.14)" },
  graphNode: { color: "#326747", tint: "rgba(143, 183, 162, 0.14)" },
  toolCall: { color: "#884568", tint: "rgba(189, 143, 165, 0.14)" },
} as const;

// Terminal ANSI colors retain their meaning with readable light-theme shades.
// White/bright-white map to ink so captured output stays visible on paper.
export const ansiForeground: Record<number, string> = {
  30: palette.ink,
  31: palette.danger,
  32: categories.graphNode.color,
  33: categories.decision.color,
  34: categories.claim.color,
  35: categories.procedure.color,
  36: categories.handover.color,
  37: palette.inkSoft,
  90: palette.muted,
  91: palette.danger,
  92: categories.graphNode.color,
  93: categories.decision.color,
  94: categories.claim.color,
  95: categories.procedure.color,
  96: categories.handover.color,
  97: palette.ink,
};

// Motion system: durations, easings and the stagger step for entrances.
export const motion = {
  fast: "150ms",
  base: "250ms",
  slow: "450ms",
  ambient: "1200ms",
  standard: "cubic-bezier(0.2, 0, 0, 1)",
  emphasized: "cubic-bezier(0.16, 1, 0.3, 1)",
  staggerStep: "40ms",
} as const;

export const fonts = {
  display: 'Newsreader, Georgia, "Times New Roman", serif',
  ui: 'Manrope, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  mono: '"DM Mono", ui-monospace, SFMono-Regular, monospace',
};

export const iconSize = { small: 16, navigation: 18, action: 20 } as const;
export const iconStroke = 1.6;
