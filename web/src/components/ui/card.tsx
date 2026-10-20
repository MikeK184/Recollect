import * as React from "react";
import { cn } from "../../lib/utils";

// Cards have no rest shadow; interactive cards lift 1px with a small shadow.
export function Card({
  className,
  interactive = false,
  ...props
}: React.ComponentProps<"div"> & { interactive?: boolean }) {
  return (
    <div
      data-slot="card"
      className={cn(
        "rounded-lg border border-border bg-card p-4 text-card-foreground",
        interactive &&
          "cursor-pointer transition-[border-color,box-shadow,transform] duration-fast ease-standard hover:-translate-y-px hover:border-border-strong hover:shadow-sm",
        className,
      )}
      {...props}
    />
  );
}
export function CardHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-header"
      className={cn("flex items-center gap-3", className)}
      {...props}
    />
  );
}
export function CardTitle({
  className,
  as: Comp = "h3",
  ...props
}: React.ComponentProps<"h3"> & { as?: "h2" | "h3" | "h4" | "div" }) {
  return (
    <Comp
      data-slot="card-title"
      className={cn("m-0 text-title font-semibold text-foreground", className)}
      {...props}
    />
  );
}
export function CardDescription({
  className,
  ...props
}: React.ComponentProps<"p">) {
  return (
    <p
      data-slot="card-description"
      className={cn("m-0 mt-1 text-body text-muted-foreground", className)}
      {...props}
    />
  );
}
/** Mono meta strip separated from the body by a hairline. */
export function CardMeta({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-meta"
      className={cn(
        "mono mt-3.5 flex items-center gap-3 border-t border-border pt-3 text-caption text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}
/** Header bar for flush cards whose body is a list or table. */
export function CardBar({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-bar"
      className={cn(
        "flex min-h-12 items-center gap-2.5 border-b border-border px-4 py-2 [&_h2]:m-0 [&_h2]:text-body [&_h2]:font-semibold [&_h3]:m-0 [&_h3]:text-body [&_h3]:font-semibold",
        className,
      )}
      {...props}
    />
  );
}

/** Square icon tile: brand-soft for AI/learning, muted otherwise. */
export function Tile({
  className,
  tone = "muted",
  size = "md",
  ...props
}: React.ComponentProps<"span"> & {
  tone?: "brand" | "muted" | "warning" | "destructive" | "info";
  size?: "sm" | "md" | "lg";
}) {
  return (
    <span
      data-slot="tile"
      className={cn(
        "grid shrink-0 place-items-center rounded-md",
        size === "sm" && "size-7 [&_svg]:size-3.5",
        size === "md" && "size-9 [&_svg]:size-4",
        size === "lg" && "size-11 rounded-lg [&_svg]:size-5",
        tone === "brand" && "bg-brand-soft text-brand-text",
        tone === "muted" && "border border-border bg-muted text-muted-foreground",
        tone === "warning" && "bg-warning-soft text-warning-text",
        tone === "destructive" && "bg-destructive-soft text-destructive-text",
        tone === "info" && "bg-info-soft text-info",
        className,
      )}
      {...props}
    />
  );
}

/** Large tabular statistic, 28/34 semibold. */
export function Stat({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="stat"
      className={cn(
        "text-page font-semibold tabular-nums tracking-[-0.025em]",
        className,
      )}
      {...props}
    />
  );
}
