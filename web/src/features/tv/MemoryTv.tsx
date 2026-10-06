import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Badge, Button } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { client, result } from "../../api";
import { useBrain } from "../../app/context";
import { memoryKinds } from "../../MemoryForms";
import type { components } from "../../api-schema";
import { useContentDeadline } from "../../useContentDeadline";
import "./tv.css";

type View = components["schemas"]["ClaimView"];

// Cycle cadence and transition length. --tv-interval in tv.css must mirror
// INTERVAL_MS; the ambient duration comes from the shared motion tokens.
const INTERVAL_MS = 8000;
const TRANSITION_MS = 1300;

const label = (s: string) => s.replaceAll("_", " ");
const time = (s?: string | null) => (s ? new Date(s).toLocaleString() : "");
// Mantine color names resolve to the per-category accents in design/theme.ts.
const kindBadgeColor: Record<string, string> = {
  claim: "blue",
  decision: "yellow",
  procedure: "violet",
  handover: "cyan",
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
      <Badge color={kindBadgeColor[content.kind] ?? "gray"}>
        {memoryKinds.find((k) => k.value === content.kind)?.label ??
          label(content.kind)}
      </Badge>
      <div className="tv-qualification" data-attention={flags.length > 0}>
        Review: {label(view.revision.review)} · Freshness:{" "}
        {label(view.eligibility.effective_freshness)} · Operational:{" "}
        {label(content.operational)}
        {flags.length > 0 && <strong>{flags.join(" · ")}</strong>}
      </div>
      <h1 className="tv-title">{content.subject || content.predicate}</h1>
      {body && <p className="tv-body">{body}</p>}
      <div className="tv-meta">
        <span>{time(view.revision.recorded_at)}</span>
        <span aria-hidden="true">·</span>
        <span>
          {label(view.revision.origin)}
          {view.revision.actor_name ? ` · ${view.revision.actor_name}` : ""}
        </span>
        {source && <span className="tv-source-chip">{source.label}</span>}
      </div>
    </>
  );
}

export function MemoryTv() {
  const brain = useBrain();
  const navigate = useNavigate();
  const router = useRouter();
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
    refetchInterval: hidden ? false : 4000,
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

  const root = (
    <div
      ref={surface}
      className="tv-root"
      role="dialog"
      aria-modal="true"
      aria-label={`Ambient memory display for ${brain.name}`}
      tabIndex={-1}
    >
      <div className="tv-glow" data-kind={current?.revision.content.kind} />
      {claims.isPending && (
        <div className="tv-center">
          <span className="tv-wordmark tv-pulse">Recollect.</span>
          <p className="tv-empty-copy">Recalling this Brain’s memories…</p>
        </div>
      )}
      {(claims.isError || expired) && (
        <div className="tv-center">
          <p className="tv-error-line">
            {expired
              ? "This display reached its content deadline. Refreshing memories…"
              : "Memories are unavailable. Displayed content has been cleared."}
          </p>
          <Button
            variant="subtle"
            size="xs"
            onClick={() => void claims.refetch()}
          >
            Try again
          </Button>
        </div>
      )}
      {!claims.isPending && available && items.length === 0 && (
        <div className="tv-center">
          <h1 className="tv-empty-title">Nothing remembered yet</h1>
          <p className="tv-empty-copy">
            Memories will appear here as this Brain learns from its sources and
            activity.
          </p>
        </div>
      )}
      {current && (
        <div className="tv-stage">
          {prev && (
            <article
              key={`exit-${prev.revision.id}-${tick}`}
              className="tv-item tv-item-exit"
              aria-hidden="true"
            >
              <ItemContent view={prev} />
            </article>
          )}
          <article key={current.revision.id} className="tv-item tv-item-enter">
            <ItemContent view={current} />
          </article>
        </div>
      )}
      <header className="tv-corner tv-top-left">
        <img src="/brand/recollect-symbol.svg" width={22} height={22} alt="" />
        <span>Recollect</span>
        <span aria-hidden="true">·</span>
        <span className="tv-brain-name">{brain.name}</span>
      </header>
      {available && claims.data && (
        <span className="tv-corner tv-bottom-right">
          last updated {time(claims.data.knowledge_at)}
        </span>
      )}
      <Button
        className="tv-corner tv-bottom-left"
        variant="subtle"
        size="xs"
        onClick={exit}
      >
        Exit display · Esc
      </Button>
      {current && items.length > 1 && !hidden && (
        <div
          key={`progress-${current.revision.id}-${tick}`}
          className="tv-progress"
          aria-hidden="true"
        >
          <span />
        </div>
      )}
    </div>
  );

  // Portal to <body>: the app chrome and its page-transition transform must
  // never become this overlay's containing block.
  return createPortal(root, document.body);
}
