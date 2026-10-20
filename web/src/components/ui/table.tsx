import * as React from "react";
import { cn } from "../../lib/utils";

// Sticky blurred header, 44px rows, mono numerals (`num` cells), soft jade
// selection mixed in oklab (see the design system's table notes).
export function TableWrap({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="table-wrap"
      className={cn(
        "overflow-auto rounded-lg border border-border bg-card scrollbar-thin",
        className,
      )}
      {...props}
    />
  );
}
export function Table({ className, ...props }: React.ComponentProps<"table">) {
  return (
    <table
      data-slot="table"
      className={cn("w-full border-separate border-spacing-0 text-body", className)}
      {...props}
    />
  );
}
export function TableHead({ className, ...props }: React.ComponentProps<"thead">) {
  return <thead className={cn(className)} {...props} />;
}
export function TableBody({ className, ...props }: React.ComponentProps<"tbody">) {
  return <tbody className={cn(className)} {...props} />;
}
export function TableRow({
  className,
  selected,
  ...props
}: React.ComponentProps<"tr"> & { selected?: boolean }) {
  return (
    <tr
      data-selected={selected || undefined}
      aria-selected={selected}
      className={cn(
        "group/row transition-colors duration-fast ease-standard [tbody_&]:hover:bg-accent [&:last-child>td]:border-b-0",
        selected &&
          "bg-[color-mix(in_oklab,var(--brand-soft)_55%,var(--card))] [tbody_&]:hover:bg-[color-mix(in_oklab,var(--brand-soft)_55%,var(--card))]",
        className,
      )}
      {...props}
    />
  );
}
export function TableHeader({ className, ...props }: React.ComponentProps<"th">) {
  return (
    <th
      className={cn(
        "sticky top-0 z-[1] h-9 border-b border-border bg-card/85 px-3 text-left text-caption font-medium text-muted-foreground backdrop-blur-chrome",
        className,
      )}
      {...props}
    />
  );
}
export function TableCell({
  className,
  primary,
  num,
  ...props
}: React.ComponentProps<"td"> & { primary?: boolean; num?: boolean }) {
  return (
    <td
      className={cn(
        "h-11 border-b border-border px-3 text-foreground-secondary",
        primary && "font-medium text-foreground",
        num && "mono text-right text-caption text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}
/** Row actions that appear on hover or selection. */
export function RowActions({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "flex items-center justify-end gap-1 opacity-0 transition-opacity duration-fast group-hover/row:opacity-100 group-focus-within/row:opacity-100 group-data-[selected]/row:opacity-100",
        className,
      )}
      {...props}
    />
  );
}
