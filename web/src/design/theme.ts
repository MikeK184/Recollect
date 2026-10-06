import {
  createTheme,
  defaultVariantColorsResolver,
  Button,
  Input,
  InputWrapper,
  Card,
  Modal,
  Drawer,
  Tabs,
  Title,
  Table,
  Badge,
  Menu,
  Code,
  type CSSVariablesResolver,
  type VariantColorsResolver,
} from "@mantine/core";
import { categories, fonts, motion, palette, tokens } from "./tokens";

const sage = [
  palette.selection,
  palette.selection,
  palette.sage,
  palette.sage,
  palette.sage,
  palette.sageDark,
  palette.sageDark,
  palette.sageDark,
  palette.inkSoft,
  palette.ink,
] as const;
// Category accent pairs keep Mantine's named colors honest on the light paper
// surfaces: soft tints for light variants, readable accents for text.
const categoryRole = (name: string) => {
  switch (name) {
    case "blue":
      return { soft: categories.claim.tint, foreground: categories.claim.color };
    case "violet":
      return {
        soft: categories.procedure.tint,
        foreground: categories.procedure.color,
      };
    case "cyan":
      return {
        soft: categories.handover.tint,
        foreground: categories.handover.color,
      };
    case "indigo":
      return { soft: "rgba(133, 137, 190, 0.14)", foreground: "#55599a" };
    default:
      return undefined;
  }
};
const variants: VariantColorsResolver = (input) => {
  const normal = defaultVariantColorsResolver(input);
  const name = input.color ?? input.theme.primaryColor;
  const role =
    name === "red"
      ? { soft: tokens.dangerSoft, foreground: tokens.danger }
      : ["yellow", "orange"].includes(name)
        ? { soft: tokens.attention, foreground: tokens.ink }
        : name === "gray"
          ? { soft: tokens.sidebar, foreground: tokens.secondary }
          : ["brand", "teal", "green"].includes(name)
            ? { soft: tokens.selection, foreground: tokens.accent }
            : categoryRole(name);
  if (!role) return normal;
  if (input.variant === "light")
    return {
      ...normal,
      background: role.soft,
      hover: role.soft,
      color: role.foreground,
    };
  if (["outline", "subtle", "transparent"].includes(input.variant))
    return {
      ...normal,
      color: role.foreground,
      hover: role.soft,
      ...(input.variant === "outline"
        ? { border: `1px solid ${role.foreground}` }
        : {}),
    };
  if (input.variant === "filled")
    return {
      ...normal,
      background:
        name === "yellow" || name === "orange" ? role.soft : role.foreground,
      hover:
        name === "yellow" || name === "orange" ? role.soft : tokens.secondary,
      color:
        name === "yellow" || name === "orange" ? tokens.ink : tokens.surface,
    };
  return normal;
};
export const theme = createTheme({
  primaryColor: "brand",
  primaryShade: 6,
  colors: {
    brand: sage,
    teal: sage,
    green: sage,
    red: [
      palette.dangerSoft,
      palette.dangerSoft,
      palette.rose,
      palette.rose,
      palette.rose,
      palette.danger,
      palette.danger,
      palette.danger,
      palette.inkSoft,
      palette.ink,
    ],
    gray: [
      palette.paper,
      palette.paperDeep,
      palette.line,
      palette.line,
      palette.lineDark,
      palette.field,
      palette.muted,
      palette.inkSoft,
      palette.inkSoft,
      palette.ink,
    ],
    yellow: [
      palette.amberSoft,
      palette.amberSoft,
      palette.amber,
      palette.amber,
      palette.amber,
      tokens.amber,
      tokens.amber,
      tokens.amber,
      palette.inkSoft,
      palette.ink,
    ],
    blue: [
      categories.claim.tint,
      categories.claim.tint,
      palette.blue,
      palette.blue,
      palette.blue,
      categories.claim.color,
      categories.claim.color,
      categories.claim.color,
      palette.inkSoft,
      palette.ink,
    ],
    violet: [
      categories.procedure.tint,
      categories.procedure.tint,
      palette.violet,
      palette.violet,
      palette.violet,
      categories.procedure.color,
      categories.procedure.color,
      categories.procedure.color,
      palette.inkSoft,
      palette.ink,
    ],
    cyan: [
      categories.handover.tint,
      categories.handover.tint,
      palette.cyan,
      palette.cyan,
      palette.cyan,
      categories.handover.color,
      categories.handover.color,
      categories.handover.color,
      palette.inkSoft,
      palette.ink,
    ],
    indigo: [
      "rgba(133, 137, 190, 0.14)",
      "rgba(133, 137, 190, 0.14)",
      "#55599a",
      "#55599a",
      "#55599a",
      "#55599a",
      "#55599a",
      "#55599a",
      palette.inkSoft,
      palette.ink,
    ],
  },
  variantColorResolver: variants,
  fontFamily: fonts.ui,
  fontFamilyMonospace: fonts.mono,
  defaultRadius: "sm",
  fontSizes: {
    xs: "0.75rem",
    sm: "0.875rem",
    md: "1rem",
    lg: "1.125rem",
    xl: "1.25rem",
  },
  lineHeights: { xs: "1.5", sm: "1.45", md: "1.55", lg: "1.5", xl: "1.4" },
  spacing: {
    xs: "0.5rem",
    sm: "0.75rem",
    md: "1rem",
    lg: "1.5rem",
    xl: "2rem",
  },
  radius: {
    xs: "0.25rem",
    sm: "0.25rem",
    md: "0.375rem",
    lg: "0.5rem",
    xl: "0.5rem",
  },
  headings: {
    fontFamily: fonts.display,
    fontWeight: "500",
    sizes: {
      h1: { fontSize: "2.5rem", lineHeight: "1.1" },
      h2: { fontSize: "1.5rem", lineHeight: "1.2" },
      h3: { fontSize: "1.125rem", lineHeight: "1.4" },
      h4: { fontSize: "1rem", lineHeight: "1.4" },
      h5: { fontSize: "0.875rem", lineHeight: "1.45" },
    },
  },
  components: {
    Button: Button.extend({
      defaultProps: { size: "sm", radius: "sm" },
      classNames: { root: "rc-button" },
      vars: (theme, props) => ({
        root: {
          ...(props.size === "xs"
            ? { "--button-height": "2rem" }
            : props.size === "sm"
              ? { "--button-height": "2.5rem" }
              : {}),
          ...((props.variant ?? "filled") === "filled" &&
          ["brand", "teal"].includes(props.color ?? theme.primaryColor)
            ? {
                "--button-bg": tokens.ink,
                "--button-hover": tokens.secondary,
                "--button-color": tokens.surface,
              }
            : {}),
        },
      }),
    }),
    Input: Input.extend({
      classNames: { input: "rc-input" },
      vars: (_theme, props) => ({
        wrapper:
          props.size === "xs"
            ? { "--input-height": "2rem" }
            : (props.size ?? "sm") === "sm"
              ? { "--input-height": "2.5rem" }
              : {},
      }),
    }),
    InputWrapper: InputWrapper.extend({
      classNames: {
        label: "rc-input-label",
        description: "rc-input-description",
      },
    }),
    Card: Card.extend({
      defaultProps: { radius: "md", shadow: undefined },
      classNames: { root: "rc-card" },
    }),
    Modal: Modal.extend({
      defaultProps: {
        radius: "lg",
        closeButtonProps: { "aria-label": "Close dialog" },
        overlayProps: { backgroundOpacity: 0.2, blur: 1 },
      },
      classNames: {
        content: "rc-overlay",
        header: "rc-overlay-header",
        title: "rc-overlay-title",
      },
    }),
    Drawer: Drawer.extend({
      defaultProps: {
        position: "right",
        size: 440,
        closeButtonProps: { "aria-label": "Close dialog" },
        overlayProps: { backgroundOpacity: 0.2 },
      },
      classNames: {
        content: "rc-overlay",
        header: "rc-overlay-header",
        title: "rc-overlay-title",
      },
    }),
    Tabs: Tabs.extend({
      defaultProps: { keepMounted: false },
      classNames: { list: "rc-tabs", tab: "rc-tab" },
    }),
    Title: Title.extend({
      styles: (_theme, props) =>
        props.order && props.order >= 3
          ? { root: { fontFamily: fonts.ui, fontWeight: 600 } }
          : {},
    }),
    Table: Table.extend({
      classNames: { table: "rc-table", th: "rc-th", td: "rc-td" },
    }),
    Badge: Badge.extend({
      defaultProps: { variant: "light", radius: "sm", tt: "none", fw: 500 },
      classNames: { root: "rc-badge" },
    }),
    Menu: Menu.extend({ classNames: { dropdown: "rc-overlay" } }),
    Code: Code.extend({ classNames: { root: "rc-code" } }),
  },
});

export const cssVariablesResolver: CSSVariablesResolver = () => ({
  variables: {
    ...Object.fromEntries(
      Object.entries(tokens).map(([key, value]) => [`--rc-${key}`, value]),
    ),
    "--rc-font-display": fonts.display,
    "--rc-font-ui": fonts.ui,
    "--rc-font-mono": fonts.mono,
    "--rc-motion-fast": motion.fast,
    "--rc-motion-base": motion.base,
    "--rc-motion-slow": motion.slow,
    "--rc-motion-ambient": motion.ambient,
    "--rc-ease-standard": motion.standard,
    "--rc-ease-emphasized": motion.emphasized,
    "--rc-stagger-step": motion.staggerStep,
    ...Object.fromEntries(
      Object.entries(categories).flatMap(([name, value]) => [
        [`--rc-cat-${name}`, value.color],
        [`--rc-cat-${name}-tint`, value.tint],
      ]),
    ),
  },
  light: {
    "--mantine-color-body": tokens.canvas,
    "--mantine-color-text": tokens.ink,
    "--mantine-color-dimmed": tokens.muted,
    "--mantine-color-default": tokens.surface,
    "--mantine-color-default-border": tokens.border,
    "--mantine-color-default-hover": tokens.sidebar,
    "--mantine-color-default-color": tokens.ink,
    "--mantine-color-placeholder": tokens.muted,
    "--mantine-color-anchor": tokens.accent,
    "--mantine-color-error": tokens.danger,
  },
  dark: {},
});
