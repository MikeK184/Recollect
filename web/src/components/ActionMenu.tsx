import type { ReactNode } from "react";
import { ChevronDown } from "lucide-react";

/** Native disclosure keeps uncommon actions keyboard reachable without changing their handlers. */
export function ActionMenu({
  children,
  label,
}: {
  children: ReactNode;
  label: string;
}) {
  return (
    <details className="group/details rounded-lg border border-border bg-card">
      <summary className="flex h-10 cursor-pointer list-none items-center gap-2 px-3 text-label font-medium text-foreground-secondary hover:text-foreground [&::-webkit-details-marker]:hidden">
        {label}
        <ChevronDown
          aria-hidden="true"
          className="ml-auto size-4 text-muted-foreground transition-transform duration-fast group-open/details:rotate-180"
        />
      </summary>
      <div className="grid gap-3 border-t border-border p-3">{children}</div>
    </details>
  );
}
