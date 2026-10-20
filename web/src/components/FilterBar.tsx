import type { ReactNode } from "react";

/** One horizontal toolbar row for search, filters and list actions. */
export function FilterBar({
  children,
  label = "Search and filter actions",
}: {
  children: ReactNode;
  label?: string;
}) {
  return (
    <div
      className="flex flex-wrap items-center gap-2 [&>[data-grow]]:min-w-60 [&>[data-grow]]:flex-1"
      role="group"
      aria-label={label}
    >
      {children}
    </div>
  );
}
