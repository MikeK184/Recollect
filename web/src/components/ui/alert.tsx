import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import {
  CircleAlert,
  CircleCheck,
  Info,
  Sparkles,
  TriangleAlert,
  type LucideIcon,
} from "lucide-react";
import { cn } from "../../lib/utils";

// Callout: soft fill, no border, rounded-lg. role="alert" by default like the
// component it replaces; pass role="status" for non-urgent notices.
const alertVariants = cva(
  "flex w-full gap-2.5 rounded-lg px-3.5 py-3 text-foreground [&>svg]:mt-0.5 [&>svg]:size-4 [&>svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "bg-muted [&>svg]:text-muted-foreground",
        brand: "bg-brand-soft [&>svg]:text-brand-text",
        warning: "bg-warning-soft [&>svg]:text-warning-text",
        destructive: "bg-destructive-soft [&>svg]:text-destructive-text",
        info: "bg-info-soft [&>svg]:text-info",
      },
    },
    defaultVariants: { variant: "default" },
  },
);
const defaultIcon: Record<string, LucideIcon> = {
  default: Info,
  brand: Sparkles,
  warning: TriangleAlert,
  destructive: CircleAlert,
  info: Info,
  success: CircleCheck,
};

export function Alert({
  className,
  variant = "default",
  title,
  icon,
  children,
  actions,
  role = "alert",
  ...props
}: Omit<React.ComponentProps<"div">, "title"> &
  VariantProps<typeof alertVariants> & {
    title?: React.ReactNode;
    /** An icon element, or false for none. Defaults by variant. */
    icon?: React.ReactNode | false;
    actions?: React.ReactNode;
  }) {
  const Icon = defaultIcon[variant ?? "default"];
  return (
    <div
      data-slot="alert"
      role={role}
      className={cn(alertVariants({ variant }), className)}
      {...props}
    >
      {icon === false ? null : (icon ?? <Icon aria-hidden="true" />)}
      <div className="min-w-0 flex-1">
        {title && (
          <p
            className={cn(
              "m-0 text-body font-medium",
              variant === "destructive" && "text-destructive-text",
            )}
          >
            {title}
          </p>
        )}
        {children && (
          <div
            className={cn(
              "text-body text-foreground-secondary [&_p]:m-0",
              title && "mt-0.5",
            )}
          >
            {children}
          </div>
        )}
        {actions && <div className="mt-2.5 flex flex-wrap gap-2">{actions}</div>}
      </div>
    </div>
  );
}
