import React from "react";
import ReactDOM from "react-dom/client";
import { MantineProvider, createTheme } from "@mantine/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createRootRoute,
  createRoute,
  createRouter,
  defaultParseSearch,
  defaultStringifySearch,
  RouterProvider,
} from "@tanstack/react-router";
import "@mantine/core/styles.css";
import "./styles.css";
import { App } from "./App";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: false, staleTime: 10_000, refetchOnWindowFocus: true },
  },
});
const root = createRootRoute({ component: App });
const home = createRoute({ getParentRoute: () => root, path: "/" });
const brain = createRoute({
  getParentRoute: () => root,
  path: "/brains/$brainId",
});
const team = createRoute({ getParentRoute: () => root, path: "/team" });
const devices = createRoute({
  getParentRoute: () => root,
  path: "/devices",
  validateSearch: (search: Record<string, unknown>): { code?: string } => ({
    code:
      typeof search.code === "string" && /^[a-f0-9]{8}$/i.test(search.code)
        ? search.code
        : undefined,
  }),
});
const router = createRouter({
  routeTree: root.addChildren([home, brain, team, devices]),
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
const theme = createTheme({
  primaryColor: "teal",
  defaultRadius: "md",
  fontFamily:
    'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  headings: { fontFamily: "inherit", fontWeight: "600" },
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <MantineProvider theme={theme}>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </MantineProvider>
  </React.StrictMode>,
);
