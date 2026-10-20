import type { ReactNode } from "react";

/** Applied restrictions stay visible as a quiet chip row. */
export function ScopeSummary({
  children,
  label = "Applied scope",
}: {
  children: ReactNode;
  label?: string;
}) {
  return (
    <div
      className="flex flex-wrap items-center gap-1.5 text-label text-muted-foreground"
      role="group"
      aria-label={label}
    >
      {children}
    </div>
  );
}
