import { Drawer, type DrawerProps } from "@mantine/core";

/** Shared overlay defaults retain Mantine's focus trap, Escape and focus return. */
export function DetailInspector({
  className,
  position = "right",
  size = "min(42rem, 90vw)",
  ...props
}: DrawerProps) {
  return (
    <Drawer
      {...props}
      position={position}
      size={size}
      className={["feature-drawer", className].filter(Boolean).join(" ")}
    />
  );
}
