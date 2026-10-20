import * as React from "react";
import { Tooltip as TooltipPrimitive } from "radix-ui";
import { cn } from "../../lib/utils";

// Ink tooltips with a 400ms delay. Wrap the app once in TooltipProvider.
export function TooltipProvider(
  props: React.ComponentProps<typeof TooltipPrimitive.Provider>,
) {
  return <TooltipPrimitive.Provider delayDuration={400} skipDelayDuration={200} {...props} />;
}

export function Tooltip({
  label,
  children,
  side = "top",
  disabled = false,
  className,
}: {
  label: React.ReactNode;
  children: React.ReactElement;
  side?: "top" | "bottom" | "left" | "right";
  disabled?: boolean;
  className?: string;
}) {
  if (disabled || !label) return children;
  return (
    <TooltipPrimitive.Root>
      <TooltipPrimitive.Trigger asChild>{children}</TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          side={side}
          sideOffset={6}
          className={cn(
            "z-[60] inline-flex max-w-72 items-center gap-2 rounded-sm bg-primary px-2 py-[5px] text-caption font-medium text-primary-foreground shadow-md",
            "data-[state=delayed-open]:animate-in data-[state=delayed-open]:fade-in-0 data-[state=delayed-open]:zoom-in-96 data-[state=instant-open]:animate-in data-[state=instant-open]:fade-in-0",
            "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:duration-100",
            className,
          )}
        >
          {label}
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
}
