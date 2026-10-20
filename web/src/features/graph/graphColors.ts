import { useMemo } from "react";
import { resolveColor } from "../../design/tokens";
import { useThemeMode } from "../../design/theme-mode";

// Graph renderers (Cytoscape, three.js) cannot read CSS variables. They take
// concrete colours resolved from the Light and Ink tokens and re-read them
// whenever the applied theme changes; the legend uses the same token roles
// through utilities, so both always agree.

export type KindGroup =
  | "memory"
  | "sources"
  | "repositories"
  | "people"
  | "agents"
  | "other";

const groupKinds: Record<Exclude<KindGroup, "other">, string[]> = {
  memory: ["claim"],
  sources: ["source_version", "capture"],
  repositories: [
    "repository",
    "repository_fact",
    "repository_file",
    "snapshot",
    "manifest_revision",
  ],
  people: ["contributor"],
  agents: ["device"],
};

export function kindGroup(kind: string): KindGroup {
  for (const [group, kinds] of Object.entries(groupKinds))
    if (kinds.includes(kind)) return group as KindGroup;
  return "other";
}

export const groupLabel: Record<KindGroup, string> = {
  memory: "Memory",
  sources: "Sources & inputs",
  repositories: "Repository evidence",
  people: "People",
  agents: "Agents",
  other: "Other evidence",
};

const groupToken: Record<KindGroup, string> = {
  memory: "kind-claim",
  sources: "kind-source",
  repositories: "kind-decision",
  people: "kind-handover",
  agents: "kind-tool",
  other: "kind-entity",
};

/** Legend swatches; the same token roles the renderers resolve. */
export const groupSwatch: Record<KindGroup, string> = {
  memory: "bg-kind-claim",
  sources: "bg-kind-source",
  repositories: "bg-kind-decision",
  people: "bg-kind-handover",
  agents: "bg-kind-tool",
  other: "bg-kind-entity",
};

/** Swatch shapes echo the 2D node shapes, so kind never relies on colour alone. */
export const groupShape: Record<KindGroup, string> = {
  memory: "rounded-[2px]",
  sources: "rounded-full",
  repositories: "rotate-45 rounded-[1px]",
  people: "rounded-t-full",
  agents: "rotate-45 rounded-[1px] scale-75",
  other: "rounded-full",
};

export const groupOrder: KindGroup[] = [
  "memory",
  "sources",
  "repositories",
  "people",
  "agents",
  "other",
];

export type GraphColors = {
  background: string;
  card: string;
  popover: string;
  foreground: string;
  muted: string;
  border: string;
  edge: string;
  brand: string;
  cross: string;
  kinds: Record<KindGroup, string>;
};

/** Concrete #rrggbb for a token; both renderers parse hex reliably. */
function hex(token: string) {
  const value = resolveColor(token);
  const parts = value.match(/[\d.]+/g)?.map(Number);
  if (!value.startsWith("rgb") || !parts || parts.length < 3) return value;
  return `#${parts
    .slice(0, 3)
    .map((part) =>
      Math.round(Math.max(0, Math.min(255, part)))
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`;
}

export function readGraphColors(): GraphColors {
  const kinds = Object.fromEntries(
    groupOrder.map((group) => [group, hex(groupToken[group])]),
  ) as Record<KindGroup, string>;
  return {
    background: hex("background"),
    card: hex("card"),
    popover: hex("popover"),
    foreground: hex("foreground"),
    muted: hex("muted-foreground"),
    border: hex("border"),
    edge: hex("border-strong"),
    brand: hex("brand"),
    cross: hex("kind-procedure"),
    kinds,
  };
}

/** Token colours for the current theme; recomputed when Light/Ink changes. */
export function useGraphColors() {
  const { resolved } = useThemeMode();
  // `resolved` is the dependency: the computed tokens change with it.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return useMemo(readGraphColors, [resolved]);
}
