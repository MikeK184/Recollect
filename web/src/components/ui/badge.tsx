import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { Check, CircleAlert, CircleDashed, Clock3 } from "lucide-react";
import { cn } from "../../lib/utils";
import type { Kind } from "../../design/tokens";

export const badgeVariants = cva(
  "inline-flex h-[22px] max-w-full shrink-0 items-center gap-1 whitespace-nowrap rounded-full px-2 text-caption font-medium [&>svg]:-ml-0.5 [&>svg]:size-[13px]",
  {
    variants: {
      variant: {
        default: "bg-secondary text-foreground-secondary",
        brand: "bg-brand-soft text-brand-text",
        warning: "bg-warning-soft text-warning-text",
        destructive: "bg-destructive-soft text-destructive-text",
        info: "bg-info-soft text-info",
        outline: "border border-border bg-transparent text-foreground-secondary",
        solid: "bg-primary text-primary-foreground",
      },
      dot: {
        true: "before:size-1.5 before:rounded-full before:bg-current before:content-['']",
        false: "",
      },
    },
    defaultVariants: { variant: "default", dot: false },
  },
);

export function Badge({
  className,
  variant,
  dot,
  ...props
}: React.ComponentProps<"span"> & VariantProps<typeof badgeVariants>) {
  return (
    <span
      data-slot="badge"
      className={cn(badgeVariants({ variant, dot }), className)}
      {...props}
    />
  );
}

export type StatusState = "positive" | "attention" | "negative" | "neutral";
const statusVariant = {
  positive: "brand",
  attention: "warning",
  negative: "destructive",
  neutral: "default",
} as const;
const statusIcon = {
  positive: Check,
  attention: Clock3,
  negative: CircleAlert,
  neutral: CircleDashed,
} as const;

/** Status always pairs its colour with an icon and a word. */
export function StatusBadge({
  children,
  state = "neutral",
  className,
}: {
  children: React.ReactNode;
  state?: StatusState;
  className?: string;
}) {
  const Icon = statusIcon[state];
  return (
    <Badge variant={statusVariant[state]} className={className}>
      <Icon aria-hidden="true" />
      {children}
    </Badge>
  );
}

const kindClass: Record<Kind, string> = {
  claim: "bg-kind-claim-soft text-kind-claim",
  decision: "bg-kind-decision-soft text-kind-decision",
  procedure: "bg-kind-procedure-soft text-kind-procedure",
  handover: "bg-kind-handover-soft text-kind-handover",
  source: "bg-kind-source-soft text-kind-source",
  entity: "bg-kind-entity-soft text-kind-entity",
  tool: "bg-kind-tool-soft text-kind-tool",
};

/** Category chip for memory kinds and record categories. */
export function KindChip({
  kind,
  children,
  className,
  ...props
}: React.ComponentProps<"span"> & { kind: Kind }) {
  return (
    <span
      data-slot="kind-chip"
      className={cn(
        "inline-flex h-[22px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-sm px-2 text-caption font-medium before:size-1.5 before:rounded-[2px] before:bg-current before:content-['']",
        kindClass[kind],
        className,
      )}
      {...props}
    >
      {children}
    </span>
  );
}

/** Small mono counter, e.g. beside a nav item or tab. */
export function Count({ className, ...props }: React.ComponentProps<"span">) {
  return (
    <span
      data-slot="count"
      className={cn(
        "mono inline-grid h-5 min-w-5 place-items-center rounded-full bg-secondary px-1.5 text-[11px] font-medium leading-4 text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}
