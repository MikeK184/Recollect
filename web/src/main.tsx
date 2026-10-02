import React from "react";
import ReactDOM from "react-dom/client";
import { MantineProvider } from "@mantine/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createRootRoute,
  createRoute,
  createRouter,
  defaultParseSearch,
  defaultStringifySearch,
  lazyRouteComponent,
  redirect,
  RouterProvider,
} from "@tanstack/react-router";
import "@mantine/core/styles.css";
import "./styles.css";
import "./design/typography.css";
import { App } from "./App";
import { BrainLayout } from "./app/BrainLayout";
import { KnowledgeSurface } from "./features/knowledge/KnowledgeSurface";
import { validateBrainSearch } from "./app/useBrainSearch";
import { theme, cssVariablesResolver } from "./design/theme";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: false, staleTime: 10_000, refetchOnWindowFocus: true },
  },
});
const root = createRootRoute({ component: App });
const home = createRoute({
  getParentRoute: () => root,
  path: "/",
  component: lazyRouteComponent(
    () => import("./features/brains/BrainsPage"),
    "BrainsPage",
  ),
});
const brain = createRoute({
  getParentRoute: () => root,
  path: "/brains/$brainId",
  component: BrainLayout,
  validateSearch: validateBrainSearch,
});
const brainIndex = createRoute({
  getParentRoute: () => brain,
  path: "/",
  beforeLoad: ({ params }) => {
    throw redirect({ to: "/brains/$brainId/ask", params, search: {} });
  },
});
const ask = createRoute({
  getParentRoute: () => brain,
  path: "ask",
  component: lazyRouteComponent(
    () => import("./features/ask/AskPage"),
    "AskPage",
  ),
});
// One Knowledge surface hosts the four knowledge views as a pathless layout, so
// every contracted URL keeps its own path while sharing one shell and one
// inspector region. The views stay lazily bundled per route.
const knowledge = createRoute({
  getParentRoute: () => brain,
  id: "knowledge",
  component: KnowledgeSurface,
});
const memory = createRoute({
  getParentRoute: () => knowledge,
  path: "memory",
  component: lazyRouteComponent(
    () => import("./features/memory/MemoryPage"),
    "MemoryPage",
  ),
});
const sources = createRoute({
  getParentRoute: () => knowledge,
  path: "sources",
  component: lazyRouteComponent(
    () => import("./features/sources/SourcesPage"),
    "SourcesPage",
  ),
});
const graph = createRoute({
  getParentRoute: () => knowledge,
  path: "graph",
  component: lazyRouteComponent(
    () => import("./features/graph/GraphPage"),
    "GraphPage",
  ),
});
const repositories = createRoute({
  getParentRoute: () => knowledge,
  path: "repositories",
  component: lazyRouteComponent(
    () => import("./features/repositories/RepositoriesPage"),
    "RepositoriesPage",
  ),
});
const agents = createRoute({
  getParentRoute: () => brain,
  path: "agents",
  component: lazyRouteComponent(
    () => import("./features/agents/AgentsPage"),
    "AgentsPage",
  ),
});
const connections = createRoute({
  getParentRoute: () => brain,
  path: "connections",
  component: lazyRouteComponent(
    () => import("./features/connections/ConnectionsPage"),
    "ConnectionsPage",
  ),
});
const activity = createRoute({
  getParentRoute: () => brain,
  path: "activity",
  component: lazyRouteComponent(
    () => import("./features/activity/ActivityPage"),
    "ActivityPage",
  ),
});
const settings = createRoute({
  getParentRoute: () => brain,
  path: "settings",
  component: lazyRouteComponent(
    () => import("./features/settings/SettingsPage"),
    "SettingsPage",
  ),
});
const unknownSection = createRoute({
  getParentRoute: () => brain,
  path: "$",
  beforeLoad: ({ params }) => {
    throw redirect({
      to: "/brains/$brainId/ask",
      params: { brainId: params.brainId },
      search: {},
    });
  },
});
const team = createRoute({
  getParentRoute: () => root,
  path: "/team",
  component: lazyRouteComponent(
    () => import("./features/workspace/GlobalPages"),
    "TeamPage",
  ),
});
const globalAgents = createRoute({
  getParentRoute: () => root,
  path: "/agents",
  component: lazyRouteComponent(
    () => import("./features/workspace/GlobalPages"),
    "AgentsGlobalPage",
  ),
});
const devices = createRoute({
  getParentRoute: () => root,
  path: "/devices",
  component: lazyRouteComponent(() => import("./DevicesPanel"), "DevicesPanel"),
  validateSearch: (search: Record<string, unknown>): { code?: string } => ({
    code:
      typeof search.code === "string" && /^[a-f0-9]{8}$/i.test(search.code)
        ? search.code
        : undefined,
  }),
});
const router = createRouter({
  routeTree: root.addChildren([
    home,
    brain.addChildren([
      brainIndex,
      ask,
      knowledge.addChildren([memory, sources, graph, repositories]),
      agents,
      connections,
      activity,
      settings,
      unknownSection,
    ]),
    team,
    globalAgents,
    devices,
  ]),
  parseSearch: (raw) => {
    const parsed = defaultParseSearch(raw);
    const codes = new URLSearchParams(raw).getAll("code");
    // A public pairing code is an opaque eight-character identifier, including
    // all-digit strings and hexadecimal strings that resemble JSON exponents.
    if (codes.length === 1 && /^[a-f0-9]{8}$/i.test(codes[0]))
      return { ...parsed, code: codes[0] };
    return parsed;
  },
  stringifySearch: (search) => {
    const params = new URLSearchParams(
      defaultStringifySearch({ ...search, code: undefined }),
    );
    if (typeof search.code === "string") params.set("code", search.code);
    const value = params.toString();
    return value ? `?${value}` : "";
  },
});
declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <MantineProvider
      theme={theme}
      cssVariablesResolver={cssVariablesResolver}
      forceColorScheme="light"
    >
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </MantineProvider>
  </React.StrictMode>,
);
