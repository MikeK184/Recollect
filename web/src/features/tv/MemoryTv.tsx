import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { motion, useReducedMotion } from "motion/react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { FileText } from "lucide-react";
import { client, result } from "../../api";
import { useBrain } from "../../app/context";
import { memoryKinds } from "../../MemoryForms";
import type { components } from "../../api-schema";
import { useContentDeadline } from "../../useContentDeadline";
import { usePollInterval } from "../../usePollInterval";
import { Brand } from "../../components/Brand";
import { Badge, Button, KindChip, Kbd } from "../../components/ui";
import { cn } from "../../lib/utils";
import type { Kind } from "../../design/tokens";

type View = components["schemas"]["ClaimView"];

// Cycle cadence and transition length. The enter transition uses the shared
// ambient duration (motion-ambient, 1.6s); the outgoing item stays mounted for
// exactly that long. A Brain with fewer than two memories is still.
const INTERVAL_MS = 8000;
const TRANSITION_MS = 1600;
const ambient = [0.2, 0, 0, 1] as const; // ease-standard

const label = (s: string) => s.replaceAll("_", " ");
const time = (s?: string | null) => (s ? new Date(s).toLocaleString() : "");
const memoryKind = (kind: string): Kind =>
  (["claim", "decision", "procedure", "handover"] as const).includes(
    kind as "claim",
  )
    ? (kind as Kind)
    : "claim";

// Category glow behind the memory: the kind's soft tint, crossfading slowly.
const glow: Record<string, string> = {
  claim: "from-kind-claim-soft",
  decision: "from-kind-decision-soft",
  procedure: "from-kind-procedure-soft",
  handover: "from-kind-handover-soft",
};

function bodyOf(view: View): string {
  const content = view.revision.content;
  return (
    [content.value, content.rationale]
      .map((s) => (s ?? "").trim())
      .find(Boolean) ?? ""
  );
}

function ItemContent({ view }: { view: View }) {
  const content = view.revision.content;
  const body = bodyOf(view);
  const source = view.evidence[0];
  const flags = [
    view.revision.lifecycle === "withdrawn" && "Withdrawn",
    !!view.eligibility.conflicting_claim_ids?.length && "Unresolved conflict",
    !!view.eligibility.rule_ids?.length && "Blocked by review rule",
  ].filter(Boolean);
  return (
    <>
      <KindChip kind={memoryKind(content.kind)}>
        {memoryKinds.find((k) => k.value === content.kind)?.label ??
          label(content.kind)}
      </KindChip>
      <div
        className={cn(
          "tv-qualification mono mt-3.5 text-caption text-muted-foreground",
          flags.length > 0 &&
            "rounded-md bg-warning-soft px-3 py-2 text-warning-text",
        )}
        data-attention={flags.length > 0}
      >
        Review: {label(view.revision.review)} · Freshness:{" "}
        {label(view.eligibility.effective_freshness)} · Operational:{" "}
        {label(content.operational)}
        {flags.length > 0 && (
          <strong className="mt-0.5 block font-sans text-label font-semibold">
            {flags.join(" · ")}
          </strong>
        )}
      </div>
      <h1 className="tv-title m-0 mt-5 text-balance font-serif text-[clamp(48px,4.2vw,88px)] font-medium leading-[1.06] tracking-[-0.025em] text-foreground [overflow-wrap:anywhere]">
        {content.subject || content.predicate}
      </h1>
      {body && (
        <p className="m-0 mt-6 line-clamp-5 max-w-[72ch] text-[20px] leading-8 text-foreground-secondary">
          {body}
        </p>
      )}
      <div className="mt-7 flex flex-wrap items-center gap-3">
        <span className="mono text-caption text-muted-foreground">
          {time(view.revision.recorded_at)} · {label(view.revision.origin)}
          {view.revision.actor_name ? ` · ${view.revision.actor_name}` : ""}
        </span>
        {source && (
          <Badge variant="outline" className="max-w-[44ch]">
            <FileText aria-hidden="true" />
            <span className="truncate">{source.label}</span>
          </Badge>
        )}
      </div>
    </>
  );
}

export function MemoryTv() {
  const pollNormal = usePollInterval("normal");
  const brain = useBrain();
  const navigate = useNavigate();
  const router = useRouter();
  const reduced = useReducedMotion();
  const surface = useRef<HTMLDivElement>(null);
  const [hidden, setHidden] = useState(() => document.hidden);
  // The same bounded authorized memory list read the Memory surface uses,
  // unfiltered: the recent window, newest first. No mock items.
  const claims = useQuery({
    queryKey: ["tv-claims", brain.id],
    gcTime: 0,
    staleTime: 0,
    retry: false,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/claims", {
          signal,
          params: { path: { brain: brain.id }, query: { offset: 0 } },
        }),
      ),
    refetchInterval: pollNormal,
  });

  const expired = useContentDeadline(claims.data?.expires_at);
  const available = !claims.isError && !expired;
  const items = available ? (claims.data?.items ?? []) : [];
  const latestItems = useRef(items);
  latestItems.current = items;
  const [selected, setSelected] = useState<string | null>(null);
  const [previous, setPrevious] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const current = items.find((view) => view.revision.id === selected);
  const position = current ? items.indexOf(current) + 1 : 0;
  const prev =
    previous !== selected
      ? items.find((view) => view.revision.id === previous)
      : undefined;

  useEffect(() => {
    if (!current) {
      setSelected(items[0]?.revision.id ?? null);
      setPrevious(null);
    }
  }, [current, claims.data, available]);
  useEffect(() => {
    if (expired && !hidden) void claims.refetch();
  }, [expired, hidden]);

  useEffect(() => {
    const app = document.getElementById("root");
    const inert = app?.inert ?? false;
    const focused = document.activeElement as HTMLElement | null;
    if (app) app.inert = true;
    surface.current?.focus();
    return () => {
      if (app) app.inert = inert;
      if (focused?.isConnected) focused.focus();
    };
  }, []);

  // Pause the cycle while the tab is hidden to avoid battery drain on a wall
  // display; resume where it left off when visible again.
  useEffect(() => {
    const onChange = () => setHidden(document.hidden);
    document.addEventListener("visibilitychange", onChange);
    return () => document.removeEventListener("visibilitychange", onChange);
  }, []);

  const advance = useCallback(() => {
    const latest = latestItems.current;
    if (latest.length < 2) return;
    const index = latest.findIndex((view) => view.revision.id === selected);
    setPrevious(selected);
    setSelected(latest[(index + 1) % latest.length].revision.id);
    setTick((t) => t + 1);
  }, [selected]);

  useEffect(() => {
    if (hidden || items.length < 2) return;
    const id = window.setTimeout(advance, INTERVAL_MS);
    return () => window.clearTimeout(id);
  }, [hidden, selected, items.length, advance]);

  // The outgoing item stays mounted only for the length of the transition.
  useEffect(() => {
    if (previous === null) return;
    const id = window.setTimeout(() => setPrevious(null), TRANSITION_MS);
    return () => window.clearTimeout(id);
  }, [previous, tick]);

  const exit = useCallback(() => {
    // A direct URL entry has no in-app history to return to.
    if (router.history.canGoBack()) router.history.go(-1);
    else
      void navigate({
        to: "/brains/$brainId/activity",
        params: { brainId: brain.id },
      });
  }, [router, navigate, brain.id]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        exit();
      }
      if (event.key === "Tab") {
        const controls = surface.current?.querySelectorAll<HTMLButtonElement>(
          "button:not(:disabled)",
        );
        if (!controls?.length) return;
        const first = controls[0],
          last = controls[controls.length - 1];
        if (
          event.shiftKey &&
          (document.activeElement === first ||
            document.activeElement === surface.current)
        ) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [exit]);

  const kind = current?.revision.content.kind;
  const cycling = !!current && items.length > 1 && !hidden;
  const root = (
    <div
      ref={surface}
      className="tv-root fixed inset-0 z-[950] flex flex-col overflow-hidden bg-background px-10 pb-8 pt-7 text-foreground outline-none"
      role="dialog"
      aria-modal="true"
      aria-label={`Ambient memory display for ${brain.name}`}
      tabIndex={-1}
    >
      {Object.entries(glow).map(([name, tint]) => (
        <div
          key={name}
          aria-hidden="true"
          className={cn(
            "pointer-events-none absolute inset-0 bg-radial-[ellipse_55%_45%_at_42%_48%] to-transparent to-72% transition-opacity duration-[1600ms] ease-standard",
            tint,
            kind === name ? "opacity-100" : "opacity-0",
          )}
        />
      ))}
      <header className="relative flex items-center gap-3">
        <Brand />
        <span aria-hidden="true" className="h-5 w-px bg-border" />
        <span className="min-w-0 truncate text-body text-muted-foreground">
          {brain.name}
        </span>
        {available && claims.data && (
          <span className="mono ml-auto shrink-0 text-caption text-muted-foreground first-letter:uppercase">
            last updated {time(claims.data.knowledge_at)}
          </span>
        )}
      </header>
      <main className="relative grid min-h-0 flex-1 place-items-center py-10">
        {claims.isPending && (
          <div className="grid justify-items-center gap-4 text-center">
            <Brand size="lg" compact />
            <p role="status" className="m-0 text-body">
              <span className="shimmer-text">
                Recalling this Brain’s memories…
              </span>
            </p>
          </div>
        )}
        {(claims.isError || expired) && (
          <div className="grid max-w-[52ch] justify-items-center gap-4 text-center">
            <p className="m-0 text-title text-foreground-secondary">
              {expired
                ? "This display reached its content deadline. Refreshing memories…"
                : "Memories are unavailable. Displayed content has been cleared."}
            </p>
            <Button variant="secondary" size="sm" onClick={() => void claims.refetch()}>
              Try again
            </Button>
          </div>
        )}
        {!claims.isPending && available && items.length === 0 && (
          <div className="grid max-w-[560px] justify-items-center gap-3 text-center">
            <h1 className="m-0 font-serif text-[40px] font-medium leading-[46px] tracking-[-0.02em] text-foreground-secondary">
              Nothing remembered yet
            </h1>
            <p className="m-0 max-w-[46ch] text-body text-muted-foreground">
              Memories will appear here as this Brain learns from its sources
              and activity.
            </p>
          </div>
        )}
        {current && (
          <div className="grid w-full max-w-[880px] max-h-full">
            {prev && !reduced && (
              <motion.article
                key={`exit-${prev.revision.id}-${tick}`}
                className="tv-item tv-item-exit pointer-events-none flex min-w-0 flex-col items-start [grid-area:1/1]"
                aria-hidden="true"
                initial={{ opacity: 1, scale: 1 }}
                animate={{ opacity: 0, scale: 1.015 }}
                transition={{ duration: TRANSITION_MS / 2000, ease: [0.4, 0, 1, 1] }}
              >
                <ItemContent view={prev} />
              </motion.article>
            )}
            <motion.article
              key={current.revision.id}
              className="tv-item tv-item-enter flex min-w-0 flex-col items-start [grid-area:1/1]"
              initial={reduced ? false : { opacity: 0, scale: 0.985 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{
                duration: TRANSITION_MS / 1000,
                ease: ambient,
                delay: prev ? 0.2 : 0,
              }}
            >
              <ItemContent view={current} />
            </motion.article>
          </div>
        )}
      </main>
      <footer className="relative grid gap-3.5">
        {/* Time until the next memory; absent (not empty) when nothing cycles. */}
        <div
          aria-hidden="true"
          className={cn(
            "h-1 overflow-hidden rounded-full",
            cycling && !reduced && "bg-secondary",
          )}
        >
          {cycling && !reduced && (
            <motion.span
              key={`progress-${current!.revision.id}-${tick}`}
              className="block h-full origin-left rounded-full bg-brand"
              initial={{ scaleX: 0 }}
              animate={{ scaleX: 1 }}
              transition={{ duration: INTERVAL_MS / 1000, ease: "linear" }}
            />
          )}
        </div>
        <div className="flex items-center">
          <Button
            variant="ghost"
            size="sm"
            className="-ml-2.5"
            aria-label="Exit display · Esc"
            onClick={exit}
          >
            Exit display
            <Kbd>Esc</Kbd>
          </Button>
          {current && items.length > 1 && (
            <span className="mono ml-auto text-caption text-muted-foreground">
              {position} of {items.length}
            </span>
          )}
        </div>
      </footer>
    </div>
  );

  // Portal to <body>: the app chrome and its page-transition transform must
  // never become this overlay's containing block.
  return createPortal(root, document.body);
}
