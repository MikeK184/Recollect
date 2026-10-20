import * as React from "react";
import { cn } from "../../lib/utils";

export function Kbd({ className, ...props }: React.ComponentProps<"kbd">) {
  return (
    <kbd
      data-slot="kbd"
      className={cn(
        "inline-flex h-5 min-w-5 items-center justify-center rounded-xs border border-b-2 border-border bg-card px-[5px] font-sans text-[11px] font-medium leading-4 text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}

/** A key combination such as ⌘K, rendered as adjacent keys. */
export function Kbds({ keys, className }: { keys: string[]; className?: string }) {
  return (
    <span className={cn("inline-flex gap-0.5", className)} aria-hidden="true">
      {keys.map((key) => (
        <Kbd key={key}>{key}</Kbd>
      ))}
    </span>
  );
}

export const isMac =
  typeof navigator !== "undefined" &&
  /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
/** The platform's command key label: ⌘ on Apple platforms, Ctrl elsewhere. */
export const modKey = isMac ? "⌘" : "Ctrl";
