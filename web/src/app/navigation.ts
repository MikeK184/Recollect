import {
  MessageSquare,
  NotebookText,
  Database,
  Network,
  FolderGit2,
  Bot,
  Unplug,
  History,
  Settings2,
  Boxes,
  Users,
} from "lucide-react";

// Ordered sidebar tiers. The order here is the rendered order and is contractual:
// Ask, Knowledge, Wiring, Assurance. Tier labels are visible group headings.
export const brainTiers = ["Ask", "Knowledge", "Wiring", "Assurance"] as const;
export type BrainTier = (typeof brainTiers)[number];

// The Knowledge surface hosts exactly these sections; each remains a real route.
export const knowledgeSections = [
  "memory",
  "sources",
  "graph",
  "repositories",
] as const;
export type KnowledgeSection = (typeof knowledgeSections)[number];

export const brainNavigation = [
  { section: "ask", label: "Ask", tier: "Ask", icon: MessageSquare },
  {
    section: "memory",
    label: "Memory",
    tier: "Knowledge",
    icon: NotebookText,
  },
  { section: "sources", label: "Sources", tier: "Knowledge", icon: Database },
  { section: "graph", label: "Graph", tier: "Knowledge", icon: Network },
  {
    section: "repositories",
    label: "Repositories",
    tier: "Knowledge",
    icon: FolderGit2,
  },
  { section: "agents", label: "Agents", tier: "Wiring", icon: Bot },
  {
    section: "connections",
    label: "Connections",
    tier: "Wiring",
    icon: Unplug,
  },
  { section: "settings", label: "Settings", tier: "Wiring", icon: Settings2 },
  { section: "activity", label: "Activity", tier: "Assurance", icon: History },
] as const;
export type BrainSection = (typeof brainNavigation)[number]["section"];
// Devices is intentionally absent from global navigation: it remains a
// direct-URL surface for pairing-approval deep links, while per-Brain agent
// management lives on each Brain's Agents surface. The global Agents page is
// the account-level roster with cross-Brain usage.
export const globalNavigation = [
  { to: "/", label: "Brains", icon: Boxes },
  { to: "/agents", label: "Agents", icon: Bot },
  { to: "/team", label: "Team", icon: Users },
] as const;

// The Knowledge surface's documented default. An unrecognized view value falls
// back here rather than rendering an empty or guessed view.
export const defaultKnowledgeView: KnowledgeSection = "memory";

/** Validate a raw view value against the four contracted Knowledge routes. */
export function resolveKnowledgeView(
  value: string | undefined,
): KnowledgeSection {
  return knowledgeSections.includes(value as KnowledgeSection)
    ? (value as KnowledgeSection)
    : defaultKnowledgeView;
}
