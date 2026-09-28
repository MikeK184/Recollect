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
  Laptop,
} from "lucide-react";

export const brainNavigation = [
  { section: "ask", label: "Ask", group: "Knowledge", icon: MessageSquare },
  {
    section: "memory",
    label: "Memory",
    group: "Knowledge",
    icon: NotebookText,
  },
  { section: "sources", label: "Sources", group: "Knowledge", icon: Database },
  { section: "graph", label: "Graph", group: "Knowledge", icon: Network },
  {
    section: "repositories",
    label: "Repositories",
    group: "Workspace",
    icon: FolderGit2,
  },
  { section: "agents", label: "Agents", group: "Workspace", icon: Bot },
  {
    section: "connections",
    label: "Connections",
    group: "Manage",
    icon: Unplug,
  },
  { section: "activity", label: "Activity", group: "Manage", icon: History },
  { section: "settings", label: "Settings", group: "Manage", icon: Settings2 },
] as const;
export type BrainSection = (typeof brainNavigation)[number]["section"];
export const globalNavigation = [
  { to: "/", label: "Brains", icon: Boxes },
  { to: "/team", label: "Team", icon: Users },
  { to: "/devices", label: "Devices", icon: Laptop },
] as const;
