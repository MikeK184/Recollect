import * as React from "react";
import { Tabs as TabsPrimitive } from "radix-ui";
import { motion } from "motion/react";
import { cn } from "../../lib/utils";
import { glide } from "../../design/motion";

// Underline tabs with a sliding layoutId indicator (selection moves, it does
// not fade). Radix keeps role="tab", aria-selected and arrow-key navigation.
const TabsValue = React.createContext<{ value?: string; group: string }>({ group: "tabs" });

export function Tabs({
  value,
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Root>) {
  const group = React.useId();
  return (
    <TabsValue.Provider value={{ value, group }}>
      <TabsPrimitive.Root
        data-slot="tabs"
        value={value}
        className={cn("flex flex-col gap-5", className)}
        {...props}
      />
    </TabsValue.Provider>
  );
}

export function TabsList({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.List>) {
  return (
    <TabsPrimitive.List
      data-slot="tabs-list"
      className={cn(
        "relative flex shrink-0 items-center gap-1 overflow-x-auto border-b border-border scrollbar-none",
        className,
      )}
      {...props}
    />
  );
}

export function TabsTrigger({
  className,
  children,
  value,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Trigger>) {
  const context = React.useContext(TabsValue);
  return (
    <TabsPrimitive.Trigger
      data-slot="tabs-trigger"
      value={value}
      className={cn(
        "relative inline-flex h-10 shrink-0 items-center gap-1.5 whitespace-nowrap px-2.5 text-body font-medium text-muted-foreground outline-none transition-colors duration-fast ease-standard hover:text-foreground focus-visible:rounded-sm focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-50 data-[state=active]:text-foreground [&_svg]:size-4",
        className,
      )}
      {...props}
    >
      {children}
      {context.value === value && (
        <motion.span
          layoutId={`${context.group}-indicator`}
          transition={glide}
          aria-hidden="true"
          className="absolute inset-x-0 -bottom-px h-0.5 rounded-[2px] bg-foreground"
        />
      )}
    </TabsPrimitive.Trigger>
  );
}

export function TabsContent({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Content>) {
  return (
    <TabsPrimitive.Content
      data-slot="tabs-content"
      className={cn("outline-none data-[state=active]:animate-enter", className)}
      {...props}
    />
  );
}
