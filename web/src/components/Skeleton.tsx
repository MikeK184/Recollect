import type { CSSProperties } from "react";

// Skeleton loader block: a shimmering raised surface standing in for content
// while a read is in flight.
export function Skeleton({
  width = "100%",
  height = 14,
  style,
}: {
  width?: number | string;
  height?: number;
  style?: CSSProperties;
}) {
  return (
    <span
      aria-hidden="true"
      className="rc-skeleton"
      style={{ width, height, ...style }}
    />
  );
}

/** A block of shimmering lines shaped like list rows for loading regions. */
export function SkeletonRows({
  rows = 5,
  label = "Loading…",
}: {
  rows?: number;
  label?: string;
}) {
  return (
    <div className="rc-skeleton-rows" role="status">
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} style={{ display: "grid", gap: 8 }}>
          <Skeleton width={`${55 + ((i * 17) % 30)}%`} height={14} />
          <Skeleton width={`${28 + ((i * 23) % 26)}%`} height={10} />
        </div>
      ))}
      <span className="sr-only">{label}</span>
    </div>
  );
}
