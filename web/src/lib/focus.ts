import { useRef } from "react";

/**
 * Radix returns focus to a dialog's Trigger, and to nothing when a dialog is
 * opened from code. These handlers remember the element that had focus when
 * the dialog opened and give focus back to it on close.
 */
export function useReturnFocus<E extends { preventDefault(): void; defaultPrevented: boolean }>(
  onOpenAutoFocus?: (event: Event) => void,
  onCloseAutoFocus?: (event: E) => void,
) {
  const opener = useRef<HTMLElement | null>(null);
  return {
    onOpenAutoFocus: (event: Event) => {
      const active = document.activeElement;
      opener.current = active instanceof HTMLElement && active !== document.body ? active : null;
      onOpenAutoFocus?.(event);
    },
    onCloseAutoFocus: (event: E) => {
      onCloseAutoFocus?.(event);
      if (event.defaultPrevented) return;
      const target = opener.current;
      opener.current = null;
      if (target?.isConnected) {
        event.preventDefault();
        target.focus({ preventScroll: true });
      }
    },
  };
}
