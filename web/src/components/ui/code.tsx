import * as React from "react";
import { cn } from "../../lib/utils";

/** Inline code. Use CodeBlock (components/CodeBlock) for multi-line code. */
export function Code({
  className,
  block = false,
  ...props
}: React.ComponentProps<"code"> & { block?: boolean }) {
  if (block)
    return (
      <pre className="m-0 overflow-auto rounded-md border border-border bg-muted px-3 py-2 scrollbar-thin">
        <code
          className={cn("mono whitespace-pre-wrap break-words text-label text-foreground", className)}
          {...props}
        />
      </pre>
    );
  return (
    <code
      className={cn(
        "mono rounded-xs border border-border bg-secondary px-[5px] py-px text-caption text-foreground",
        className,
      )}
      {...props}
    />
  );
}
