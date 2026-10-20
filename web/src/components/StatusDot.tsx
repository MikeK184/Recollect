import { cn } from "../lib/utils";

// Status dot for live and processing indicators. Colour carries tone only; any
// state meaning stays in an adjacent icon or label (non-colour rule). `live`
// pings once per ambient cycle and only while work is actually happening.
const tones = {
  accent: "text-brand",
  idle: "text-faint-foreground",
  warning: "text-warning",
  danger: "text-destructive",
  muted: "text-muted-foreground",
} as const;

export function StatusDot({
  tone = "accent",
  live = false,
  size = 8,
  label,
  className,
}: {
  tone?: keyof typeof tones;
  live?: boolean;
  size?: number;
  label?: string;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "relative inline-block shrink-0 rounded-full bg-current",
        tones[tone],
        live &&
          "after:absolute after:inset-0 after:animate-ping after:rounded-[inherit] after:bg-current after:content-['']",
        className,
      )}
      style={{ width: size, height: size }}
      role={label ? "status" : undefined}
      aria-hidden={label ? undefined : true}
    >
      {label ? <span className="sr-only">{label}</span> : null}
    </span>
  );
}
