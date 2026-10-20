import { lazy, Suspense, useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { client, result, type Session } from "../api";
import { ErrorState as Failure } from "../components/AsyncState";
import { InspectorHost } from "../components/Inspector";
import { PageTransition } from "../components/PageTransition";
import { usePollInterval } from "../usePollInterval";
import { WorkspaceContext } from "./context";
import { brainNavigation, globalNavigation } from "./navigation";
import { replaceSession } from "./session";
import { Sidebar } from "./Sidebar";
import { Topbar } from "./Topbar";
import { useSidebar } from "./sidebarState";

// The palette (cmdk) loads on first use, not with the shell.
const CommandPalette = lazy(() =>
  import("./CommandPalette").then((module) => ({ default: module.CommandPalette })),
);

/**
 * The authenticated shell (ADR 0025): the sidebar sits directly on the canvas
 * and content lives in one inset main panel with a sticky topbar. The panel
 * scrolls inside itself; the record inspector docks at its right edge.
 */
export function AppShell({ user }: { user: Session }) {
  const pollNormal = usePollInterval("normal");
  const pollSlow = usePollInterval("slow");
  const cache = useQueryClient();
  const navigate = useNavigate();
  const sidebar = useSidebar();
  const [paletteOpen, setPaletteOpenState] = useState(false);
  const [paletteLoaded, setPaletteLoaded] = useState(false);
  const setPaletteOpen = (next: boolean | ((open: boolean) => boolean)) => {
    setPaletteLoaded(true);
    setPaletteOpenState(next);
  };
  const path = useRouterState({ select: (state) => state.location.pathname });
  const brainId = path.startsWith("/brains/") ? path.split("/")[2] : undefined;
  const section = path.split("/")[3] || "dashboard";
  const [lastBrain, setLastBrain] = useState<{
    actor: string;
    id: string;
  } | null>(null);
  const brains = useQuery({
    queryKey: ["brains"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/brains", { signal })),
    refetchInterval: pollNormal,
  });
  const health = useQuery({
    queryKey: ["status"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/status", { signal })),
    refetchInterval: pollSlow,
  });
  const logout = useMutation({
    mutationFn: async () => {
      const response = await client.POST("/api/auth/logout");
      if (response.response.status !== 401) result(response);
    },
    onSuccess: async () => {
      await replaceSession(cache, null);
      void navigate({ to: "/" });
    },
  });
  const visibleBrains = brains.isError ? [] : (brains.data ?? []);
  const current = visibleBrains.find((b) => b.id === brainId);
  useEffect(() => {
    if (brainId && current && !current.archived && !brains.error)
      setLastBrain({ actor: user.user.id, id: brainId });
    else if (
      brains.error ||
      (lastBrain &&
        (lastBrain.actor !== user.user.id ||
          !visibleBrains.some((b) => b.id === lastBrain.id && !b.archived)))
    )
      setLastBrain(null);
  }, [
    brainId,
    current?.id,
    current?.archived,
    brains.error,
    user.user.id,
    lastBrain?.actor,
    lastBrain?.id,
    visibleBrains,
  ]);
  const retainedBrain =
    path === "/connectors" && lastBrain?.actor === user.user.id && !brains.error
      ? visibleBrains.find((b) => b.id === lastBrain.id && !b.archived)
      : undefined;
  const navigationBrain = brainId ? current : retainedBrain;
  const navigationBrainId = brainId ?? retainedBrain?.id;
  // The palette may hand a question to Ask in the current or last Brain.
  const askBrain =
    current ??
    (lastBrain?.actor === user.user.id
      ? visibleBrains.find((b) => b.id === lastBrain.id && !b.archived)
      : undefined);

  const selectedLabel = brainId
    ? section === "tv"
      ? "Ambient display"
      : (brainNavigation.find((n) => n.section === section)?.label ?? "Ask")
    : (globalNavigation.find((n) => n.to === path)?.label ?? "Workspace");
  useEffect(() => {
    document.title = `${current ? `${current.name} · ` : ""}${selectedLabel} · Recollect`;
  }, [current?.name, selectedLabel]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <WorkspaceContext.Provider value={user}>
      <a
        className="fixed left-4 top-[-80px] z-[100] rounded-md bg-primary px-4 py-2.5 text-body font-medium text-primary-foreground shadow-lg focus:top-3"
        href="#main-content"
      >
        Skip to content
      </a>
      <div className="flex h-full min-h-0 bg-background text-foreground">
        <Sidebar
          user={user}
          rail={sidebar.rail}
          path={path}
          brainId={brainId}
          section={brainId ? section : undefined}
          navigationBrain={navigationBrain}
          navigationBrainId={navigationBrainId}
          brains={visibleBrains}
          brainsPending={brains.isPending}
          onSearch={() => setPaletteOpen(true)}
          onSignOut={() => logout.mutate()}
          signingOut={logout.isPending}
        />
        <main
          id="main-content"
          tabIndex={-1}
          className="my-2 mr-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-border bg-card shadow-sm outline-none"
        >
          <Topbar
            user={user}
            path={path}
            brain={current}
            brainId={brainId}
            section={section}
            selectedLabel={selectedLabel}
            health={health}
            onToggleSidebar={sidebar.toggle}
            railed={sidebar.rail}
          />
          <InspectorHost>
            <div
              id="main-scroll"
              className="relative flex min-h-0 flex-1 flex-col overflow-auto scrollbar-thin"
            >
              {logout.error && (
                <div className="px-6 pt-6">
                  <Failure error={logout.error} />
                </div>
              )}
              <PageTransition />
            </div>
          </InspectorHost>
        </main>
      </div>
      {paletteLoaded && (
        <Suspense fallback={null}>
          <CommandPalette
            open={paletteOpen}
            onOpenChange={setPaletteOpen}
            user={user}
            brains={visibleBrains}
            brain={current}
            askBrain={askBrain}
            onToggleSidebar={sidebar.toggle}
            onRefreshStatus={() => void health.refetch()}
          />
        </Suspense>
      )}
    </WorkspaceContext.Provider>
  );
}
