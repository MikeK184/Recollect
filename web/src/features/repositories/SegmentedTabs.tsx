import { useId } from "react";
import { Tabs as TabsPrimitive } from "radix-ui";
import { motion } from "motion/react";
import { glide } from "../../design/motion";

/**
 * A segmented control with tab semantics: it switches which panel shows and
 * keeps role="tab", aria-selected, arrow-key movement and the URL `?tab=`
 * state, while the selected segment's thumb glides with a layoutId.
 * Render inside a Radix Tabs root.
 */
export function SegmentedTabsList({
  tabs,
  value,
  label,
}: {
  tabs: readonly { value: string; label: string }[];
  value: string;
  label: string;
}) {
  const group = useId();
  return (
    <TabsPrimitive.List
      aria-label={label}
      className="relative inline-flex h-8 shrink-0 items-stretch rounded-md bg-secondary p-0.5"
    >
      {tabs.map((tab) => (
        <TabsPrimitive.Trigger
          key={tab.value}
          value={tab.value}
          className="relative z-[1] inline-flex items-center whitespace-nowrap rounded-sm px-3 text-label font-medium text-muted-foreground outline-none transition-colors duration-fast ease-standard hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring data-[state=active]:text-foreground"
        >
          {tab.value === value && (
            <motion.span
              layoutId={`${group}-thumb`}
              transition={glide}
              aria-hidden="true"
              className="absolute inset-0 -z-[1] rounded-sm bg-elevated shadow-xs"
            />
          )}
          {tab.label}
        </TabsPrimitive.Trigger>
      ))}
    </TabsPrimitive.List>
  );
}
