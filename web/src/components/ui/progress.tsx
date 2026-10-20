import { cn } from "../../lib/utils";

/** 4px progress bar that eases to each new reading; never animates ahead. */
export function Progress({
  value,
  label,
  className,
  indeterminate = false,
}: {
  value?: number;
  label?: string;
  className?: string;
  indeterminate?: boolean;
}) {
  const clamped = Math.max(0, Math.min(100, value ?? 0));
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={indeterminate ? undefined : 0}
      aria-valuemax={indeterminate ? undefined : 100}
      aria-valuenow={indeterminate ? undefined : Math.round(clamped)}
      className={cn("h-1 overflow-hidden rounded-full bg-secondary", className)}
    >
      <span
        className={cn(
          "block h-full rounded-full bg-brand",
          indeterminate
            ? "w-2/5 animate-indeterminate"
            : "transition-[width] duration-slow ease-out",
        )}
        style={indeterminate ? undefined : { width: `${clamped}%` }}
      />
    </div>
  );
}
