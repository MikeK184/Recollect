import type { CSSProperties } from "react";
import { cn } from "../lib/utils";

// Shaped skeletons with a slow sweep, standing in for content while a read is
// in flight. Static (no sweep) under reduced motion.
export function Skeleton({
  width = "100%",
  height = 12,
  className,
  style,
}: {
  width?: number | string;
  height?: number;
  className?: string;
  style?: CSSProperties;
}) {
  return (
    <span
      aria-hidden="true"
      className={cn("block rounded-sm skeleton-sweep", className)}
      style={{ width, height, ...style }}
    />
  );
}

/** Row-shaped skeletons for list and table regions. */
export function SkeletonRows({
  rows = 5,
  label = "Loading…",
}: {
  rows?: number;
  label?: string;
}) {
  return (
    <div className="grid w-full" role="status">
      {Array.from({ length: rows }, (_, i) => (
        <div
          key={i}
          className="flex h-11 items-center gap-3 border-b border-border last:border-b-0"
        >
          <Skeleton width={24} height={24} className="rounded-full" />
          <div className="grid flex-1 gap-1.5">
            <Skeleton width={`${55 + ((i * 17) % 30)}%`} height={12} />
            <Skeleton width={`${28 + ((i * 23) % 26)}%`} height={9} />
          </div>
        </div>
      ))}
      <span className="sr-only">{label}</span>
    </div>
  );
}
