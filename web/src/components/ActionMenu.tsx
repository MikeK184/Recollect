import type { ReactNode } from "react";

/** Native disclosure keeps uncommon actions keyboard reachable without changing their handlers. */
export function ActionMenu({
  children,
  label,
}: {
  children: ReactNode;
  label: string;
}) {
  return (
    <details className="feature-details">
      <summary>{label}</summary>
      {children}
    </details>
  );
}
