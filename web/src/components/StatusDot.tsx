import type { CSSProperties } from "react";

// Pulsing status dot for processing/live indicators. Color carries tone only;
// any state meaning must stay in an adjacent icon or label (non-color rule).
const tones = {
  accent: "var(--rc-accent)",
  idle: "var(--rc-field)",
  danger: "var(--rc-danger)",
  muted: "var(--rc-muted)",
} as const;

export function StatusDot({
  tone = "accent",
  live = false,
  size = 8,
  label,
  style,
}: {
  tone?: keyof typeof tones;
  live?: boolean;
  size?: number;
  label?: string;
  style?: CSSProperties;
}) {
  return (
    <span
      className={`rc-pulse-dot${live ? " live" : ""}`}
      style={{ width: size, height: size, color: tones[tone], ...style }}
      role={label ? "status" : undefined}
      aria-hidden={label ? undefined : true}
    >
      {label ? <span className="sr-only">{label}</span> : null}
    </span>
  );
}
