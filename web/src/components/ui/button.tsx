import * as React from "react";
import { Slot } from "radix-ui";
import { cva, type VariantProps } from "class-variance-authority";
import { LoaderCircle } from "lucide-react";
import { cn } from "../../lib/utils";

// Primary actions are ink (`primary`), never jade; `brand` is reserved for AI
// actions such as Ask. Sizes: sm 28px, default 32px, lg 40px, icon squares.
export const buttonVariants = cva(
  "inline-flex shrink-0 select-none items-center justify-center gap-1.5 whitespace-nowrap rounded-md border border-transparent font-medium transition-[background-color,border-color,color,box-shadow,transform] duration-fast ease-standard active:scale-[0.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary-hover",
        secondary:
          "border-border bg-card text-foreground shadow-xs hover:border-border-strong hover:bg-accent",
        ghost:
          "bg-transparent text-foreground-secondary hover:bg-accent hover:text-foreground",
        brand: "bg-brand text-brand-foreground hover:bg-brand-hover",
        destructive:
          "bg-destructive text-destructive-foreground hover:bg-destructive-text",
        "destructive-ghost":
          "bg-transparent text-destructive-text hover:bg-destructive-soft",
        "destructive-outline":
          "border-border bg-card text-destructive-text shadow-xs hover:border-destructive hover:bg-destructive-soft",
        link: "h-auto border-0 bg-transparent p-0 text-brand-text underline-offset-[3px] hover:underline active:scale-100",
      },
      size: {
        sm: "h-7 px-2.5 text-caption",
        default: "h-8 px-3 text-label",
        lg: "h-10 px-4 text-body",
        icon: "size-8 p-0 text-muted-foreground",
        "icon-sm": "size-7 p-0 text-muted-foreground [&_svg:not([class*='size-'])]:size-3.5",
      },
    },
    compoundVariants: [
      { variant: "link", size: "sm", className: "h-auto px-0" },
      { variant: "link", size: "default", className: "h-auto px-0" },
    ],
    defaultVariants: { variant: "default", size: "default" },
  },
);

export type ButtonProps = React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    /** Render the single child (a Link or anchor) with button styling. */
    asChild?: boolean;
    /** Shows a spinner, sets aria-busy and disables the button. */
    loading?: boolean;
  };

export function Button({
  className,
  variant,
  size,
  asChild = false,
  loading = false,
  disabled,
  children,
  type,
  ...props
}: ButtonProps) {
  const Comp = asChild ? Slot.Root : "button";
  return (
    <Comp
      data-slot="button"
      data-loading={loading || undefined}
      aria-busy={loading || undefined}
      className={cn(buttonVariants({ variant, size }), className)}
      disabled={asChild ? undefined : disabled || loading}
      type={asChild ? undefined : (type ?? "button")}
      {...props}
    >
      {asChild ? (
        children
      ) : (
        <>
          {loading && (
            <LoaderCircle className="animate-spin" aria-hidden="true" />
          )}
          {children}
        </>
      )}
    </Comp>
  );
}

/** A square, label-only button: `label` becomes its accessible name and title. */
export function IconButton({
  label,
  size = "icon",
  variant = "ghost",
  className,
  ...props
}: Omit<ButtonProps, "size"> & {
  label: string;
  size?: "icon" | "icon-sm";
}) {
  return (
    <Button
      aria-label={label}
      title={props.title ?? label}
      size={size}
      variant={variant}
      className={cn(
        variant === "ghost" &&
          "text-muted-foreground hover:text-foreground aria-expanded:bg-accent aria-expanded:text-foreground aria-pressed:bg-accent aria-pressed:text-foreground",
        className,
      )}
      {...props}
    />
  );
}
