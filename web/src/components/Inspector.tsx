import {
  createContext,
  useContext,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { AnimatePresence, motion } from "motion/react";
import { X } from "lucide-react";
import { cn } from "../lib/utils";
import { paneIn } from "../design/motion";
import { useWideLayout } from "../lib/hooks";
import { Sheet, SheetBody, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "./ui/sheet";

// The one shared record inspector (ADR 0025). From 1440px it docks at 400px
// inside the main panel (a non-modal dialog beside the content, never over a
// canvas); below that it opens as a sheet with a scrim. Only one inspector is
// open at a time: the most recently opened one owns the dock. Closing returns
// focus to the element that opened it.

type Host = {
  node: HTMLElement | null;
  claim: (id: string) => void;
  release: (id: string) => void;
  owner: string | null;
};
const InspectorHostContext = createContext<Host | null>(null);

/** The dock column. AppShell renders it once at the right of the main panel. */
export function InspectorHost({ children }: { children: ReactNode }) {
  const [node, setNode] = useState<HTMLElement | null>(null);
  const [owners, setOwners] = useState<string[]>([]);
  const owner = owners.at(-1) ?? null;
  const host: Host = {
    node,
    owner,
    claim: (id) => setOwners((list) => [...list.filter((entry) => entry !== id), id]),
    release: (id) => setOwners((list) => list.filter((entry) => entry !== id)),
  };
  return (
    <InspectorHostContext.Provider value={host}>
      <div className="flex min-h-0 min-w-0 flex-1">
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">{children}</div>
        <div
          ref={setNode}
          data-inspector-dock=""
          className={cn(
            "relative hidden min-h-0 shrink-0 overflow-hidden border-l border-border bg-card",
            owner && "flex w-[var(--inspector-width)]",
          )}
        />
      </div>
    </InspectorHostContext.Provider>
  );
}

function useOpenerFocus(opened: boolean, returnFocus: boolean) {
  const opener = useRef<HTMLElement | null>(null);
  const was = useRef(false);
  if (opened && !was.current) opener.current = document.activeElement as HTMLElement | null;
  was.current = opened;
  useEffect(() => {
    if (opened || !returnFocus) return;
    const target = opener.current;
    const timer = setTimeout(() => {
      if (target?.isConnected) target.focus({ preventScroll: true });
      opener.current = null;
    }, 30);
    return () => clearTimeout(timer);
  }, [opened, returnFocus]);
  useEffect(
    () => () => {
      const target = opener.current;
      if (returnFocus && target?.isConnected) target.focus({ preventScroll: true });
    },
    [returnFocus],
  );
}

export type DetailInspectorProps = {
  opened: boolean;
  onClose: () => void;
  title?: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  className?: string;
  /** Header actions placed before the close button. */
  actions?: ReactNode;
  closeOnEscape?: boolean;
  returnFocus?: boolean;
  /** Accepted for call-site compatibility; docking decides placement. */
  position?: string;
  size?: number | string;
  withinPortal?: boolean;
  trapFocus?: boolean;
  "aria-label"?: string;
  /** Accessible name of the close button (default "Close dialog"). */
  closeLabel?: string;
};

export function DetailInspector({
  opened,
  onClose,
  title,
  description,
  children,
  className,
  actions,
  closeOnEscape = true,
  returnFocus = true,
  "aria-label": ariaLabel,
  closeLabel = "Close dialog",
}: DetailInspectorProps) {
  const host = useContext(InspectorHostContext);
  const wide = useWideLayout();
  const id = useId();
  // Opened from inside a modal (a sheet or dialog), a docked panel would sit
  // behind that modal's scrim and be hidden from assistive technology; it
  // opens as a sheet on top instead.
  const fromModal = useRef(false);
  const wasOpened = useRef(false);
  if (opened && !wasOpened.current)
    fromModal.current = !!(document.activeElement as HTMLElement | null)?.closest(
      '[data-slot="sheet-content"], [data-slot="dialog-content"], [role="alertdialog"]',
    );
  // The entrance plays when the inspector opens, not when a resize moves an
  // already open inspector between the sheet and the dock.
  const openedAt = useRef(0);
  if (opened && !wasOpened.current) openedAt.current = Date.now();
  wasOpened.current = opened;
  const docked = wide && !!host?.node && !fromModal.current;
  const entering = Date.now() - openedAt.current < 400;
  useOpenerFocus(opened, returnFocus);
  useLayoutEffect(() => {
    if (!host || !docked || !opened) return;
    host.claim(id);
    return () => host.release(id);
    // host identity changes every render; claim/release are stable in effect.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [docked, opened, id, host?.node]);
  if (!docked)
    return (
      <Sheet open={opened} onOpenChange={(open) => !open && onClose()}>
        <SheetContent
          width={400}
          closeLabel={closeLabel}
          aria-label={ariaLabel}
          className={className}
          onEscapeKeyDown={(event) => !closeOnEscape && event.preventDefault()}
        >
          <SheetHeader>
            <SheetTitle>{title ?? ariaLabel ?? "Details"}</SheetTitle>
            {description ? (
              <SheetDescription>{description}</SheetDescription>
            ) : (
              <SheetDescription className="sr-only">{title ?? ariaLabel ?? "Details"}</SheetDescription>
            )}
          </SheetHeader>
          {actions && <div className="flex gap-1 border-b border-border px-4 py-2">{actions}</div>}
          <SheetBody>{children}</SheetBody>
        </SheetContent>
      </Sheet>
    );
  const active = opened && host.owner === id;
  return createPortal(
    <AnimatePresence>
      {active && (
        <DockedPanel
          key={id}
          title={title}
          description={description}
          actions={actions}
          ariaLabel={ariaLabel}
          className={className}
          onClose={onClose}
          closeOnEscape={closeOnEscape}
          closeLabel={closeLabel}
          entering={entering}
        >
          {children}
        </DockedPanel>
      )}
    </AnimatePresence>,
    host.node!,
  );
}

function DockedPanel({
  title,
  description,
  actions,
  ariaLabel,
  className,
  onClose,
  closeOnEscape,
  closeLabel,
  entering,
  children,
}: {
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  ariaLabel?: string;
  className?: string;
  onClose: () => void;
  closeOnEscape: boolean;
  closeLabel: string;
  entering: boolean;
  children?: ReactNode;
}) {
  const titleId = useId();
  const panel = useRef<HTMLElement>(null);
  useEffect(() => {
    const node = panel.current;
    if (node && !node.contains(document.activeElement))
      node.focus({ preventScroll: true });
  }, []);
  return (
    <motion.section
      ref={panel}
      role="dialog"
      aria-modal="false"
      aria-labelledby={title ? titleId : undefined}
      aria-label={title ? undefined : (ariaLabel ?? "Details")}
      tabIndex={-1}
      variants={paneIn}
      initial={entering ? "initial" : false}
      animate="animate"
      exit="exit"
      className={cn("absolute inset-0 flex min-w-0 flex-col bg-card outline-none", className)}
      onKeyDown={(event) => {
        if (event.key === "Escape" && closeOnEscape && !event.defaultPrevented) {
          event.stopPropagation();
          onClose();
        }
      }}
    >
      <header className="flex h-[52px] shrink-0 items-center gap-2 border-b border-border pl-4 pr-2">
        <div className="min-w-0 flex-1">
          <h2 id={titleId} className="m-0 truncate text-title font-semibold text-foreground">
            {title ?? ariaLabel ?? "Details"}
          </h2>
        </div>
        {actions}
        <button
          type="button"
          aria-label={closeLabel}
          title="Close"
          onClick={onClose}
          className="grid size-8 place-items-center rounded-md text-muted-foreground transition-colors duration-fast hover:bg-accent hover:text-foreground"
        >
          <X className="size-4" aria-hidden="true" />
        </button>
      </header>
      {description && (
        <p className="m-0 border-b border-border px-4 py-2 text-label text-muted-foreground">
          {description}
        </p>
      )}
      <div className="min-h-0 flex-1 overflow-auto p-4 scrollbar-thin">{children}</div>
    </motion.section>
  );
}

/** A titled block inside an inspector, separated by a hairline. */
export function InspectorSection({
  label,
  children,
  className,
}: {
  label?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={cn("-mx-4 grid gap-2.5 border-b border-border px-4 py-4 first:pt-0 last:border-b-0", className)}>
      {label && <h3 className="section-label m-0">{label}</h3>}
      {children}
    </section>
  );
}

/** Two-column property list: muted terms, foreground values. */
export function PropList({
  items,
  className,
}: {
  items: { term: ReactNode; value: ReactNode }[];
  className?: string;
}) {
  return (
    <dl className={cn("m-0 grid grid-cols-[112px_minmax(0,1fr)] gap-x-3 gap-y-2 text-label leading-5", className)}>
      {items.map((item, index) => (
        <div key={index} className="contents">
          <dt className="text-muted-foreground">{item.term}</dt>
          <dd className="m-0 flex min-w-0 items-center gap-1.5 text-foreground [overflow-wrap:anywhere]">{item.value}</dd>
        </div>
      ))}
    </dl>
  );
}

/** Serif quotation for remembered text. */
export function Quote({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <blockquote className={cn("m-0 border-l-2 border-border-strong pl-3 font-serif text-[15px] leading-6 text-foreground", className)}>
      {children}
    </blockquote>
  );
}
