import { useCallback, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";

/**
 * Render a panel's search, count and actions into the switch row the page
 * provides (so the switch stays mounted across panels), or inline when the
 * panel is used on its own. `null` means the row is not mounted yet.
 */
export function Toolbar({
  target,
  children,
}: {
  target: HTMLElement | null | undefined;
  children: ReactNode;
}) {
  if (target === undefined)
    return <div className="flex flex-wrap items-center gap-2">{children}</div>;
  return target ? createPortal(children, target) : null;
}

/**
 * Remember the control that opened a dialog and focus it again after the
 * dialog closes. Dialog chains that swap one dialog for another (a list that
 * opens a detail in its place) lose Radix's own focus return, so the opener
 * is restored explicitly.
 */
export function useOpener() {
  const opener = useRef<HTMLElement | null>(null);
  const remember = useCallback(() => {
    opener.current = document.activeElement as HTMLElement | null;
  }, []);
  const restore = useCallback(() => {
    const target = opener.current;
    setTimeout(() => {
      if (target?.isConnected) target.focus({ preventScroll: true });
    }, 0);
  }, []);
  return { remember, restore };
}
