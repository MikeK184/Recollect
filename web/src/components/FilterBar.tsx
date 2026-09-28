import type { ReactNode } from "react";

export function FilterBar({
  children,
  label = "Search and filter actions",
}: {
  children: ReactNode;
  label?: string;
}) {
  return (
    <div className="feature-toolbar" role="group" aria-label={label}>
      {children}
    </div>
  );
}
