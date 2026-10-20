import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Dialog as DialogPrimitive } from "radix-ui";
import {
  ArrowRight,
  Boxes,
  CornerDownLeft,
  Database,
  Monitor,
  Moon,
  NotebookText,
  PanelLeft,
  RefreshCw,
  Sparkles,
  Sun,
  type LucideIcon,
} from "lucide-react";
import { client, result, type Brain, type Session } from "../api";
import { useDebouncedValue } from "../lib/hooks";
import { setThemePreference } from "../design/theme-mode";
import { label as humanize } from "../claimsShared";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "../components/ui/command";
import { overlayClass } from "../components/ui/dialog";
import { Kbd, Kbds, modKey } from "../components/ui/kbd";
import { Spinner } from "../components/ui/spinner";
import { brainNavigation, globalNavigation } from "./navigation";
import { useReturnFocus } from "../lib/focus";
import { handOffToAsk } from "./askHandoff";

// ⌘K palette (desktop contract, 2026-10-19). Searches memory and sources with
// the existing authorized literal list search, jumps to routes this account
// may open, offers non-destructive actions and hands the typed query to Ask
// (first row, or ⌘↵) without putting it in a URL. It never calls a model and
// never runs a destructive action. At most eight rows per group.
const MAX = 8;
const views = [
  { section: "memory", label: "Memory", icon: NotebookText },
  { section: "sources", label: "Sources", icon: Database },
  { section: "repositories", label: "Repositories", icon: Boxes },
] as const;

export function CommandPalette({
  open,
  onOpenChange,
  user,
  brains,
  brain,
  askBrain,
  onToggleSidebar,
  onRefreshStatus,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  user: Session;
  brains: Brain[];
  brain?: Brain;
  askBrain?: Brain;
  onToggleSidebar: () => void;
  onRefreshStatus: () => void;
}) {
  const navigate = useNavigate();
  const focus = useReturnFocus();
  const [text, setText] = useState("");
  const [query] = useDebouncedValue(text.trim(), 200);
  useEffect(() => {
    if (!open) setText("");
  }, [open]);
  const target = brain ?? askBrain;
  const searching = open && !!target && query.length >= 2;
  // Superseded requests are cancelled: each query string is its own key and
  // the abandoned one has no observer left once the key changes.
  const memory = useQuery({
    queryKey: ["palette-claims", target?.id, query],
    enabled: searching,
    gcTime: 0,
    retry: false,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/claims", {
          signal,
          params: { path: { brain: target!.id }, query: { q: query } },
        }),
      ),
  });
  const sources = useQuery({
    queryKey: ["palette-sources", target?.id, query],
    enabled: searching,
    gcTime: 0,
    retry: false,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          signal,
          params: { path: { brain: target!.id }, query: { q: query } },
        }),
      ),
  });
  const close = () => onOpenChange(false);
  const go = (run: () => void) => {
    close();
    run();
  };
  const ask = () => {
    if (!target || !text.trim()) return;
    handOffToAsk(target.id, text);
    go(() =>
      void navigate({
        to: "/brains/$brainId/ask",
        params: { brainId: target.id },
        search: {},
      }),
    );
  };
  const owner = user.user.installation_owner;
  const destinations: { key: string; label: string; icon: LucideIcon; hint?: string; run: () => void }[] = [];
  if (target) {
    for (const item of brainNavigation) {
      if (item.section === "memory" || item.section === "sources" || item.section === "repositories")
        continue;
      destinations.push({
        key: `brain-${item.section}`,
        label: item.label,
        icon: item.icon,
        hint: target.name,
        run: () =>
          void navigate({
            to: `/brains/$brainId/${item.section}`,
            params: { brainId: target.id },
            search: {},
          }),
      });
    }
    for (const view of views)
      destinations.push({
        key: `brain-${view.section}`,
        label: `Explore · ${view.label}`,
        icon: view.icon,
        hint: target.name,
        run: () =>
          void navigate({
            to: `/brains/$brainId/${view.section}`,
            params: { brainId: target.id },
            search: {},
          }),
      });
  }
  for (const item of globalNavigation) {
    if ((item.to === "/team" || item.to === "/connectors") && !owner) continue;
    destinations.push({
      key: `global-${item.to}`,
      label: item.label,
      icon: item.icon,
      hint: "Workspace",
      run: () => void navigate({ to: item.to, search: {} }),
    });
  }
  const lower = text.trim().toLowerCase();
  const match = (value: string) => !lower || value.toLowerCase().includes(lower);
  const shownDestinations = destinations.filter((d) => match(`${d.label} ${d.hint ?? ""}`)).slice(0, MAX);
  const shownBrains = brains
    .filter((b) => b.id !== brain?.id && match(b.name))
    .slice(0, MAX);
  const actions: { key: string; label: string; icon: LucideIcon; run: () => void }[] = [
    { key: "theme-light", label: "Use Light theme", icon: Sun, run: () => setThemePreference("light") },
    { key: "theme-dark", label: "Use Ink theme", icon: Moon, run: () => setThemePreference("dark") },
    { key: "theme-system", label: "Match system theme", icon: Monitor, run: () => setThemePreference("system") },
    { key: "sidebar", label: "Toggle sidebar", icon: PanelLeft, run: onToggleSidebar },
    { key: "status", label: "Refresh service status", icon: RefreshCw, run: onRefreshStatus },
  ];
  const shownActions = actions.filter((a) => match(a.label)).slice(0, MAX);
  const claimRows = searching ? (memory.data?.items ?? []).slice(0, MAX) : [];
  const sourceRows = searching ? (sources.data?.sources ?? []).slice(0, MAX) : [];
  const loading = searching && (memory.isFetching || sources.isFetching);
  const askRow = !!target && !!text.trim();

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className={overlayClass} />
        <DialogPrimitive.Content
          aria-label="Search and commands"
          {...focus}
          className="fixed left-1/2 top-[14vh] z-50 w-[var(--palette-width)] max-w-[calc(100vw-32px)] -translate-x-1/2 overflow-hidden rounded-xl border border-border bg-popover shadow-lg outline-none data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98] data-[state=open]:slide-in-from-bottom-2 data-[state=open]:duration-[220ms] data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:duration-150"
        >
          <DialogPrimitive.Title className="sr-only">Search and commands</DialogPrimitive.Title>
          <DialogPrimitive.Description className="sr-only">
            Search memory and sources, jump to a page, run an action, or ask the Brain.
          </DialogPrimitive.Description>
          <Command shouldFilter={false} loop label="Search and commands">
            <CommandInput
              value={text}
              onValueChange={setText}
              placeholder={target ? `Search ${target.name} or ask a question…` : "Search pages and actions…"}
              onKeyDown={(event) => {
                if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                  event.preventDefault();
                  ask();
                }
              }}
              trailing={loading ? <Spinner size={14} label="Searching" /> : <Kbd>Esc</Kbd>}
            />
            <CommandList>
              <CommandEmpty>Nothing matches “{text.trim()}”.</CommandEmpty>
              {askRow && (
                <CommandGroup heading="Ask">
                  <CommandItem value="ask" onSelect={ask} className="[&>svg:first-child]:!text-brand-text">
                    <Sparkles aria-hidden="true" />
                    <span className="min-w-0 truncate">
                      Ask <span className="font-medium">“{text.trim()}”</span>
                    </span>
                    <small className="truncate">in {target!.name}</small>
                    <Kbds keys={[modKey, "↵"]} className="ml-auto" />
                  </CommandItem>
                </CommandGroup>
              )}
              {claimRows.length > 0 && (
                <CommandGroup heading="Memory">
                  {claimRows.map((view) => (
                    <CommandItem
                      key={view.revision.claim_id}
                      value={`claim-${view.revision.claim_id}`}
                      onSelect={() =>
                        go(() =>
                          void navigate({
                            to: "/brains/$brainId/memory",
                            params: { brainId: target!.id },
                            search: { claim: view.revision.claim_id },
                          }),
                        )
                      }
                    >
                      <NotebookText aria-hidden="true" />
                      <span className="min-w-0 truncate">
                        {view.revision.content.subject} · {humanize(view.revision.content.predicate)}{" "}
                        <span className="font-medium">{view.revision.content.value}</span>
                      </span>
                      <ArrowRight aria-hidden="true" className="ml-auto" />
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
              {sourceRows.length > 0 && (
                <CommandGroup heading="Sources">
                  {sourceRows.map((source) => (
                    <CommandItem
                      key={source.id}
                      value={`source-${source.id}`}
                      onSelect={() =>
                        go(() =>
                          void navigate({
                            to: "/brains/$brainId/sources",
                            params: { brainId: target!.id },
                            search: { source: source.id },
                          }),
                        )
                      }
                    >
                      <Database aria-hidden="true" />
                      <span className="min-w-0 truncate">{source.version.title}</span>
                      <small className="ml-auto shrink-0">{source.version.contributor}</small>
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
              {shownDestinations.length > 0 && (
                <CommandGroup heading="Go to">
                  {shownDestinations.map((item) => (
                    <CommandItem key={item.key} value={item.key} onSelect={() => go(item.run)}>
                      <item.icon aria-hidden="true" />
                      <span className="min-w-0 truncate">{item.label}</span>
                      {item.hint && <small className="ml-auto truncate">{item.hint}</small>}
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
              {shownBrains.length > 0 && (
                <CommandGroup heading="Brains">
                  {shownBrains.map((b) => (
                    <CommandItem
                      key={b.id}
                      value={`brain-${b.id}`}
                      onSelect={() =>
                        go(() =>
                          void navigate({
                            to: "/brains/$brainId/dashboard",
                            params: { brainId: b.id },
                            search: {},
                          }),
                        )
                      }
                    >
                      <Boxes aria-hidden="true" />
                      <span className="min-w-0 truncate">{b.name}</span>
                      <small className="ml-auto">{b.archived ? "Archived" : `${b.role} access`}</small>
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
              {shownActions.length > 0 && (
                <CommandGroup heading="Actions">
                  {shownActions.map((item) => (
                    <CommandItem key={item.key} value={item.key} onSelect={() => go(item.run)}>
                      <item.icon aria-hidden="true" />
                      <span>{item.label}</span>
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
            </CommandList>
            <div className="flex items-center gap-4 border-t border-border bg-muted px-4 py-2.5 text-caption text-muted-foreground">
              <span className="inline-flex items-center gap-1.5">
                <Kbd>↑</Kbd>
                <Kbd>↓</Kbd> to move
              </span>
              <span className="inline-flex items-center gap-1.5">
                <Kbd>
                  <CornerDownLeft className="size-3" aria-hidden="true" />
                </Kbd>{" "}
                to open
              </span>
              {target && (
                <span className="inline-flex items-center gap-1.5">
                  <Kbds keys={[modKey, "↵"]} /> to ask
                </span>
              )}
              <span className="ml-auto">
                {searching ? "Memory and sources match exact text" : "Type to search"}
              </span>
            </div>
          </Command>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
