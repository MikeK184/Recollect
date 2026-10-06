import {
  MessageSquare,
  LayoutDashboard,
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
  Compass,
} from "lucide-react";

// Ordered sidebar tiers. The order here is the rendered order and is contractual:
// Four daily destinations, with management reached through one disclosure.
export const brainTiers = ["Brain", "Manage"] as const;
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
  {
    section: "dashboard",
    label: "Dashboard",
    tier: "Brain",
    icon: LayoutDashboard,
    hidden: false,
  },
  {
    section: "ask",
    label: "Ask",
    tier: "Brain",
    icon: MessageSquare,
    hidden: false,
  },
  {
    section: "memory",
    label: "Memory",
    tier: "Brain",
    icon: NotebookText,
    hidden: true,
  },
  {
    section: "sources",
    label: "Sources",
    tier: "Brain",
    icon: Database,
    hidden: true,
  },
  {
    section: "graph",
    label: "Graph",
    tier: "Brain",
    icon: Network,
    hidden: false,
  },
  {
    section: "repositories",
    label: "Repositories",
    tier: "Brain",
    icon: FolderGit2,
    hidden: true,
  },
  {
    section: "explore",
    label: "Explore",
    tier: "Brain",
    icon: Compass,
    hidden: false,
  },
  {
    section: "agents",
    label: "Agents",
    tier: "Manage",
    icon: Bot,
    hidden: false,
  },
  {
    section: "connections",
    label: "Connections",
    tier: "Manage",
    icon: Unplug,
    hidden: false,
  },
  {
    section: "settings",
    label: "Settings",
    tier: "Manage",
    icon: Settings2,
    hidden: false,
  },
  {
    section: "activity",
    label: "Activity",
    tier: "Manage",
    icon: History,
    hidden: true,
  },
] as const;
export type BrainSection = (typeof brainNavigation)[number]["section"];
// Devices is intentionally absent from global navigation: it remains a
// direct-URL surface for pairing-approval deep links, while per-Brain agent
// management lives on each Brain's Agents surface. The global Agents page is
// the account-level roster with cross-Brain usage.
export const globalNavigation = [
  { to: "/", label: "Brains", icon: Boxes },
  { to: "/agents", label: "My agents", icon: Bot, hidden: false },
  { to: "/connectors", label: "Connectors", icon: Unplug },
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
