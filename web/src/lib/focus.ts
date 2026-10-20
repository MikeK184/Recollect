import { useEffect, useRef } from "react";

/**
 * Radix returns focus to a dialog's Trigger, and to nothing when a dialog is
 * opened from code. These handlers remember the element that had focus when
 * the dialog opened and give focus back to it on close. If that element was
 * re-rendered away meanwhile (an inspector whose content follows URL state),
 * focus goes to its nearest focusable container instead — the docked
 * inspector panel or the main panel — so keyboard users keep their place.
 */
export function useReturnFocus<
  E extends { preventDefault(): void; defaultPrevented: boolean },
>(
  onOpenAutoFocus?: (event: Event) => void,
  onCloseAutoFocus?: (event: E) => void,
) {
  const opener = useRef<HTMLElement | null>(null);
  const container = useRef<HTMLElement | null>(null);
  const target = () =>
    opener.current?.isConnected
      ? opener.current
      : container.current?.isConnected
        ? container.current
        : null;
  // A dialog that unmounts while open (a nested editor rendered only while
  // needed) gets its focus back from Radix a task later, so a key pressed
  // meanwhile reaches <body>. Return focus as soon as the commit settles; the
  // later Radix event then finds nothing to do. Focus that moved is kept.
  useEffect(
    () => () => {
      queueMicrotask(() => {
        const element = target();
        opener.current = null;
        container.current = null;
        const active = document.activeElement;
        if (active && active !== document.body) return;
        element?.focus({ preventScroll: true });
      });
    },
    [],
  );
  return {
    onOpenAutoFocus: (event: Event) => {
      const active = document.activeElement;
      opener.current =
        active instanceof HTMLElement && active !== document.body
          ? active
          : null;
      container.current =
        opener.current?.closest<HTMLElement>(
          '[role="dialog"][tabindex], #main-content',
        ) ?? null;
      onOpenAutoFocus?.(event);
    },
    onCloseAutoFocus: (event: E) => {
      onCloseAutoFocus?.(event);
      if (event.defaultPrevented) return;
      const element = target();
      opener.current = null;
      container.current = null;
      if (element) {
        event.preventDefault();
        element.focus({ preventScroll: true });
      }
    },
  };
}

/**
 * True when a keyboard event started inside a modal dialog or sheet. React
 * events bubble through portals to the component that rendered the modal, so
 * an ancestor's Escape handler must leave such events to the modal.
 */
export function fromModal(target: EventTarget | null) {
  return (
    target instanceof Element &&
    !!target.closest(
      '[data-slot="dialog-content"], [data-slot="sheet-content"], [role="alertdialog"]',
    )
  );
}

/** True while any modal dialog or sheet is open (read at keypress time). */
export function modalOpen() {
  return !!document.querySelector(
    '[data-slot="dialog-content"][data-state="open"], [data-slot="sheet-content"][data-state="open"], [role="alertdialog"][data-state="open"]',
  );
}
