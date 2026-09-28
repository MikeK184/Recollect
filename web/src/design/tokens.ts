// Atlas primitives are deliberately centralized; features consume semantic roles.
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
} as const;

export const fonts = {
  display: 'Newsreader, Georgia, "Times New Roman", serif',
  ui: 'Manrope, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  mono: '"DM Mono", ui-monospace, SFMono-Regular, monospace',
};

export const iconSize = { small: 16, navigation: 18, action: 20 } as const;
export const iconStroke = 1.6;
