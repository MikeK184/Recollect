import type { ReactNode } from "react";
import { cn } from "../lib/utils";

/**
 * Page body inside the main panel. Readable pages cap at 1200px, table-heavy
 * pages at 1600px; `full` is for full-bleed canvases (Graph, pipeline) that
 * fill the panel with no padding.
 */
export function Page({
  width = "readable",
  className,
  children,
}: {
  width?: "readable" | "wide" | "full";
  className?: string;
  children: ReactNode;
}) {
  return (
    <div
      className={cn(
        "flex w-full flex-col",
        width === "full"
          ? "min-h-0 flex-1"
          : "mx-auto gap-6 px-6 pb-10 pt-6",
        // The caps apply to content inside the 24px gutters, as on the boards.
        width === "readable" && "max-w-[1248px]",
        width === "wide" && "max-w-[1648px]",
        className,
      )}
    >
      {children}
    </div>
  );
}
