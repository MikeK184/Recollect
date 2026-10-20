import * as React from "react";
import { Accordion as AccordionPrimitive, Collapsible as CollapsiblePrimitive } from "radix-ui";
import { ChevronDown } from "lucide-react";
import { cn } from "../../lib/utils";

export const Accordion = AccordionPrimitive.Root;
export function AccordionItem({
  className,
  ...props
}: React.ComponentProps<typeof AccordionPrimitive.Item>) {
  return (
    <AccordionPrimitive.Item
      className={cn("border-b border-border last:border-b-0", className)}
      {...props}
    />
  );
}
export function AccordionTrigger({
  className,
  children,
  ...props
}: React.ComponentProps<typeof AccordionPrimitive.Trigger>) {
  return (
    <AccordionPrimitive.Header className="m-0 flex">
      <AccordionPrimitive.Trigger
        className={cn(
          "group/acc flex flex-1 items-center gap-2 py-3 text-left text-body font-medium text-foreground outline-none transition-colors hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring",
          className,
        )}
        {...props}
      >
        <span className="min-w-0 flex-1">{children}</span>
        <ChevronDown
          aria-hidden="true"
          className="size-4 shrink-0 text-muted-foreground transition-transform duration-fast ease-standard group-data-[state=open]/acc:rotate-180"
        />
      </AccordionPrimitive.Trigger>
    </AccordionPrimitive.Header>
  );
}
export function AccordionContent({
  className,
  children,
  ...props
}: React.ComponentProps<typeof AccordionPrimitive.Content>) {
  return (
    <AccordionPrimitive.Content
      className="overflow-hidden data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down"
      {...props}
    >
      <div className={cn("pb-4 text-body text-foreground-secondary", className)}>{children}</div>
    </AccordionPrimitive.Content>
  );
}

export const Collapsible = CollapsiblePrimitive.Root;
export const CollapsibleTrigger = CollapsiblePrimitive.Trigger;
export function CollapsibleContent({
  className,
  ...props
}: React.ComponentProps<typeof CollapsiblePrimitive.Content>) {
  return (
    <CollapsiblePrimitive.Content
      className={cn(
        "overflow-hidden data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down",
        className,
      )}
      {...props}
    />
  );
}
