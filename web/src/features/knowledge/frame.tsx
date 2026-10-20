import {
  createContext,
  useContext,
  useEffect,
  type MouseEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { motion } from "motion/react";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useBrain } from "../../app/context";
import type { BrainSearch } from "../../app/useBrainSearch";
import { Count } from "../../components/ui";
import { glide } from "../../design/motion";
import { cn } from "../../lib/utils";

// The Explore frame shared by Memory, Sources and Repositories: one compact
// header (title, description, at most two page actions) and one view switcher
// above the hosted browser. The Knowledge surface renders it; hosted pages
// render only their body and reach the header through these helpers.

export type ExploreView = "memory" | "sources" | "repositories";

export const exploreViews: readonly { view: ExploreView; label: string }[] = [
  { view: "memory", label: "Memory" },
  { view: "sources", label: "Sources" },
  { view: "repositories", label: "Repositories" },
];

export type ExploreFrameValue = {
  /** Header action slot; null until the header has mounted. */
  actions: HTMLElement | null;
  /** Report the active view's record count for its switcher tab. */
  setCount: (view: ExploreView, count: number | null) => void;
};

export const ExploreFrameContext = createContext<ExploreFrameValue | null>(
  null,
);

/**
 * Place page actions (at most two, primary first) in the Explore header.
 * Outside the Knowledge surface the actions render in place.
 */
export function ExploreHeaderActions({ children }: { children: ReactNode }) {
  const frame = useContext(ExploreFrameContext);
  if (!frame) return <>{children}</>;
  if (!frame.actions) return null;
  return createPortal(children, frame.actions);
}

/** Show the hosted view's record count on its switcher tab while mounted. */
export function useExploreCount(
  view: ExploreView,
  count: number | null | undefined,
) {
  const setCount = useContext(ExploreFrameContext)?.setCount;
  useEffect(() => {
    setCount?.(view, count ?? null);
  }, [setCount, view, count]);
  useEffect(() => () => setCount?.(view, null), [setCount, view]);
}

// Switching browsing scope inside Explore clears every view-specific
// identifier, so a selection is never substituted into a view that cannot
// resolve it. Other shared criteria stay as they were.
const exploreCleared: Partial<Record<keyof BrainSearch, undefined>> = {
  claim: undefined,
  source: undefined,
  version: undefined,
  repository: undefined,
  snapshot: undefined,
  revision: undefined,
  detail: undefined,
  center: undefined,
  tab: undefined,
  kind: undefined,
  mode: undefined,
  environment: undefined,
  knowledge: undefined,
  fact: undefined,
};

/**
 * The one Knowledge view switcher: real links with aria-current, so every
 * switch is a history entry that Back and forward walk. Inside `/explore` the
 * views are the `view` search value; the legacy `/memory`, `/sources` and
 * `/repositories` routes switch between their own paths.
 */
export function KnowledgeSwitcher({
  view,
  explore,
  counts,
}: {
  view: string;
  explore: boolean;
  counts: Partial<Record<ExploreView, number>>;
}) {
  const brain = useBrain();
  const router = useRouter();
  const navigate = useNavigate();
  return (
    <div
      role="group"
      aria-label="Knowledge views"
      className="knowledge-switcher relative flex shrink-0 items-center gap-1 overflow-x-auto border-b border-border scrollbar-none"
    >
      {exploreViews.map((item) => {
        const active = item.view === view;
        const target = (explore
          ? {
              to: "/brains/$brainId/explore" as const,
              params: { brainId: brain.id },
              search: (previous: BrainSearch) => ({
                ...previous,
                ...exploreCleared,
                view: item.view,
              }),
            }
          : {
              to: `/brains/$brainId/${item.view}` as const,
              params: { brainId: brain.id },
              search: {},
            }) as unknown as Parameters<typeof navigate>[0];
        const href = router.buildLocation(
          target as unknown as Parameters<typeof router.buildLocation>[0],
        ).href;
        const count = active ? counts[item.view] : undefined;
        return (
          <a
            key={item.view}
            href={href}
            aria-current={active ? "page" : undefined}
            onClick={(event: MouseEvent<HTMLAnchorElement>) => {
              if (
                event.defaultPrevented ||
                event.button !== 0 ||
                event.metaKey ||
                event.ctrlKey ||
                event.shiftKey ||
                event.altKey
              )
                return;
              event.preventDefault();
              if (active) return;
              void navigate(target);
            }}
            className={cn(
              "relative inline-flex h-10 shrink-0 items-center gap-1.5 whitespace-nowrap px-2.5 text-body font-medium text-muted-foreground no-underline outline-none transition-colors duration-fast ease-standard hover:text-foreground hover:no-underline focus-visible:rounded-sm focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring",
              active && "text-foreground",
            )}
          >
            {item.label}
            {count !== undefined && <Count aria-hidden="true">{count}</Count>}
            {active && (
              <motion.span
                layoutId="knowledge-view-indicator"
                transition={glide}
                aria-hidden="true"
                className="absolute inset-x-0 -bottom-px h-0.5 rounded-xs bg-foreground"
              />
            )}
          </a>
        );
      })}
    </div>
  );
}
