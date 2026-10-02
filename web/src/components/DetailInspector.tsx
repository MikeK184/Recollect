import { Drawer, type DrawerProps } from "@mantine/core";
import { useEffect, useRef } from "react";

/** Shared overlay defaults retain Mantine's focus trap, Escape and focus return. */
export function DetailInspector({
  className,
  position = "right",
  size = "min(42rem, 90vw)",
  opened,
  returnFocus = true,
  ...props
}: DrawerProps) {
  // Capture the opener before the child focus trap mounts. Keep it across
  // nested dialogs, whose temporary trap changes must not replace the opener.
  const lastActive = useRef<HTMLElement | null>(null);
  const wasOpened = useRef(false);
  if (opened && !wasOpened.current) {
    lastActive.current = document.activeElement as HTMLElement | null;
  }
  wasOpened.current = opened;
  useEffect(() => {
    if (opened || !returnFocus) return;
    const timer = setTimeout(() => {
      if (lastActive.current?.isConnected)
        lastActive.current.focus({ preventScroll: true });
      lastActive.current = null;
    }, 220);
    return () => clearTimeout(timer);
  }, [opened, returnFocus]);
  useEffect(
    () => () => {
      const target = lastActive.current;
      if (returnFocus && target?.isConnected) {
        target.focus({ preventScroll: true });
      }
    },
    [returnFocus],
  );
  return (
    <Drawer
      {...props}
      position={position}
      size={size}
      opened={opened}
      returnFocus={false}
      className={["feature-drawer", className].filter(Boolean).join(" ")}
    />
  );
}
