import type { ReactNode } from "react";
import { ChevronRight } from "lucide-react";
import { cn } from "../../lib/utils";

// Small presentational pieces shared by the Connections and Agents surfaces.

/**
 * Native disclosure (details/summary keeps its keyboard and test semantics)
 * with a chevron that turns when open and a body that enters softly.
 */
export function Disclosure({
  summary,
  children,
  className,
  bodyClassName,
  defaultOpen,
  ...props
}: {
  summary: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
  defaultOpen?: boolean;
  "data-testid"?: string;
}) {
  return (
    <details className={cn("group/disclosure min-w-0", className)} open={defaultOpen} {...props}>
      <summary className="inline-flex cursor-pointer select-none list-none items-center gap-1.5 rounded-sm text-label font-medium text-foreground-secondary transition-colors duration-fast hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
        <ChevronRight
          aria-hidden="true"
          className="size-3.5 shrink-0 text-muted-foreground transition-transform duration-fast ease-standard group-open/disclosure:rotate-90"
        />
        {summary}
      </summary>
      <div className={cn("mt-2.5 grid animate-enter gap-2.5", bodyClassName)}>
        {children}
      </div>
    </details>
  );
}

/** Labelled fact rows: muted term left, value right, hairline separated. */
export function FactRows({
  items,
  className,
  columns = 1,
}: {
  items: { term: ReactNode; value: ReactNode; mono?: boolean }[];
  className?: string;
  columns?: 1 | 2;
}) {
  return (
    <dl
      className={cn(
        "m-0 grid gap-x-6",
        columns === 2 && "grid-cols-2",
        className,
      )}
    >
      {items.map((item, index) => (
        <div
          key={index}
          className="flex min-w-0 items-baseline justify-between gap-3 border-t border-border py-2"
        >
          <dt className="shrink-0 text-caption text-muted-foreground">
            {item.term}
          </dt>
          <dd
            className={cn(
              "m-0 min-w-0 text-right text-label text-foreground [overflow-wrap:anywhere]",
              item.mono && "mono text-caption",
            )}
          >
            {item.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}

/** Small uppercase section label with optional trailing content. */
export function SectionLabel({
  children,
  actions,
  className,
  as: Heading = "h3",
}: {
  children: ReactNode;
  actions?: ReactNode;
  className?: string;
  as?: "h2" | "h3" | "h4";
}) {
  return (
    <div className={cn("flex min-h-7 items-center justify-between gap-2", className)}>
      <Heading className="section-label m-0">{children}</Heading>
      {actions}
    </div>
  );
}
