import type { ReactNode } from "react";

export function ScopeSummary({
  children,
  label = "Applied scope",
}: {
  children: ReactNode;
  label?: string;
}) {
  return (
    <div className="feature-scope" role="group" aria-label={label}>
      {children}
    </div>
  );
}
