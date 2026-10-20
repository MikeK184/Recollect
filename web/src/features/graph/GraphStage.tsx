import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { cn } from "../../lib/utils";
import { DetailInspector } from "../../components/Inspector";
import { IconButton } from "../../components/ui";

// The Graph canvas is full-bleed (ADR 0025): every control floats on it in
// one of four corner zones, or in the zones of the page header card. Each
// component contributes its chrome with <ChromeSlot>; the stage stacks the
// pieces so they never overlap one another, and renderers read the free band
// between the top and bottom chrome rows before fitting the camera.

export type ChromeZone =
  | "top-left"
  | "top-right"
  | "bottom-left"
  | "bottom-right"
  | "header-meta"
  | "header-tools"
  | "header-extra";

type StageValue = {
  zones: Partial<Record<ChromeZone, HTMLElement | null>>;
  register: (zone: ChromeZone, node: HTMLElement | null) => void;
  /** True when the stage owns corner zones over a canvas. */
  canvas: boolean;
  /** Header zones exist (or will exist) in this stage. */
  header: boolean;
  /** Stage pixels covered by the top and bottom chrome rows. */
  insets: () => { top: number; bottom: number };
};

const GraphStageContext = createContext<StageValue | null>(null);
/** True on the Graph page: record inspectors dock beside the canvas. */
export const GraphDockContext = createContext(false);

export const useGraphStage = () => useContext(GraphStageContext);

/** Floating chrome surface: a popover card that keeps text readable over the canvas. */
export const chromeCard =
  "pointer-events-auto rounded-lg border border-border bg-popover/95 text-popover-foreground shadow-md backdrop-blur-chrome";

const noInsets = () => ({ top: 0, bottom: 0 });

function useZones() {
  const [zones, setZones] = useState<
    Partial<Record<ChromeZone, HTMLElement | null>>
  >({});
  const register = useCallback(
    (zone: ChromeZone, node: HTMLElement | null) =>
      setZones((old) => (old[zone] === node ? old : { ...old, [zone]: node })),
    [],
  );
  return { zones, register };
}

/** Stable ref callbacks per zone (a new callback each render would re-register). */
export function useZoneRefs(register: StageValue["register"] | undefined) {
  return useMemo(() => {
    const ref = (zone: ChromeZone) => (node: HTMLElement | null) =>
      register?.(zone, node);
    return {
      "top-left": ref("top-left"),
      "top-right": ref("top-right"),
      "bottom-left": ref("bottom-left"),
      "bottom-right": ref("bottom-right"),
      "header-meta": ref("header-meta"),
      "header-tools": ref("header-tools"),
      "header-extra": ref("header-extra"),
    };
  }, [register]);
}

/**
 * Supplies header zones without a canvas (the Topics overview, whose content
 * flows as a document). Renderers inside it create their own stage.
 */
export function GraphZoneProvider({ children }: { children: ReactNode }) {
  const { zones, register } = useZones();
  const value = useMemo(
    () => ({ zones, register, canvas: false, header: true, insets: noInsets }),
    [zones, register],
  );
  return (
    <GraphStageContext.Provider value={value}>
      {children}
    </GraphStageContext.Provider>
  );
}

/** A full-bleed canvas stage with its four chrome corners. */
export function GraphStage({
  children,
  className,
  header = false,
}: {
  children: ReactNode;
  className?: string;
  /** The page header card registers header zones inside this stage. */
  header?: boolean;
}) {
  const { zones, register } = useZones();
  const refs = useZoneRefs(register);
  const stage = useRef<HTMLDivElement>(null),
    top = useRef<HTMLDivElement>(null),
    bottom = useRef<HTMLDivElement>(null);
  const insets = useCallback(() => {
    const box = stage.current?.getBoundingClientRect();
    if (!box) return { top: 0, bottom: 0 };
    const extent = (row: HTMLDivElement | null, edge: "top" | "bottom") => {
      if (!row) return 0;
      const items = [...row.querySelectorAll<HTMLElement>("[data-chrome-item]")];
      const rects = items
        .map((item) => item.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      if (!rects.length) return 0;
      return edge === "top"
        ? Math.max(...rects.map((rect) => rect.bottom)) - box.top
        : box.bottom - Math.min(...rects.map((rect) => rect.top));
    };
    return {
      top: Math.max(0, extent(top.current, "top")),
      bottom: Math.max(0, extent(bottom.current, "bottom")),
    };
  }, []);
  const value = useMemo(
    () => ({ zones, register, canvas: true, header, insets }),
    [zones, register, header, insets],
  );
  return (
    <GraphStageContext.Provider value={value}>
      <div
        ref={stage}
        data-graph-stage=""
        className={cn(
          "relative isolate flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden dot-grid",
          className,
        )}
      >
        {children}
        <div className="pointer-events-none absolute inset-0 z-10 flex flex-col justify-between gap-2 p-3">
          <div ref={top} className="flex flex-wrap items-start justify-between gap-2">
            <div
              ref={refs["top-left"]}
              className="flex min-w-0 max-w-full flex-col items-start gap-2"
            />
            <div
              ref={refs["top-right"]}
              className="ml-auto flex min-w-0 max-w-full flex-col items-end gap-2"
            />
          </div>
          <div ref={bottom} className="flex flex-wrap items-end justify-between gap-2">
            <div
              ref={refs["bottom-left"]}
              className="flex min-w-0 max-w-full flex-col items-start gap-2"
            />
            <div
              ref={refs["bottom-right"]}
              className="ml-auto flex min-w-0 max-w-full flex-col items-end gap-2"
            />
          </div>
        </div>
      </div>
    </GraphStageContext.Provider>
  );
}

/** Reuse the surrounding canvas stage, or create one for an embedded graph. */
export function EnsureGraphStage({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  const stage = useGraphStage();
  if (stage?.canvas)
    return (
      <div className="relative flex min-h-0 min-w-0 flex-1 flex-col">
        {children}
      </div>
    );
  return <GraphStage className={className}>{children}</GraphStage>;
}

const headerFallback: Partial<Record<ChromeZone, ChromeZone>> = {
  "header-meta": "top-left",
  "header-tools": "top-left",
  "header-extra": "top-left",
};

/**
 * Place chrome in a stage zone. `order` sorts pieces from different
 * components inside one zone. Header zones fall back to the top-left corner
 * in stages without a page header (embedded graphs, the Ask dialog).
 */
export function ChromeSlot({
  zone,
  order = 0,
  className,
  children,
}: {
  zone: ChromeZone;
  order?: number;
  className?: string;
  children: ReactNode;
}) {
  const stage = useGraphStage();
  if (!stage) return <>{children}</>;
  const target =
    zone in headerFallback && !stage.header
      ? stage.zones[headerFallback[zone]!]
      : stage.zones[zone];
  if (!target) return null;
  return createPortal(
    <div
      data-chrome-item=""
      className={cn("pointer-events-auto min-w-0 max-w-full", className)}
      style={{ order }}
    >
      {children}
    </div>,
    target,
  );
}

/**
 * The record inspector for a graph selection. On the Graph page it docks via
 * DetailInspector (400px from 1440px, a sheet below) so it never covers the
 * canvas; elsewhere (the Ask dialog) it sits in a column beside the canvas.
 */
export function GraphInspector({
  opened,
  onClose,
  title,
  label,
  closeLabel = "Close graph summary",
  children,
}: {
  opened: boolean;
  onClose: () => void;
  /** The dialog title (the record kind); the record name leads the body. */
  title: ReactNode;
  /** Accessible name of the complementary landmark holding the record. */
  label: string;
  closeLabel?: string;
  children: ReactNode;
}) {
  const docked = useContext(GraphDockContext);
  const body = (
    <aside
      role="complementary"
      aria-label={label}
      className="grid min-w-0 gap-4 [overflow-wrap:anywhere]"
    >
      {children}
    </aside>
  );
  if (docked)
    return (
      <DetailInspector opened={opened} onClose={onClose} title={title}>
        {opened ? body : null}
      </DetailInspector>
    );
  if (!opened) return null;
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex h-[52px] shrink-0 items-center gap-2 border-b border-border pl-4 pr-2">
        <h2 className="m-0 min-w-0 flex-1 truncate text-title font-semibold">
          {title}
        </h2>
        <IconButton label={closeLabel} size="icon-sm" onClick={onClose}>
          <X aria-hidden="true" />
        </IconButton>
      </div>
      <div className="min-h-0 flex-1 overflow-auto p-4 scrollbar-thin">
        {body}
      </div>
    </div>
  );
}
