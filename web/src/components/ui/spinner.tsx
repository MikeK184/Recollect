import { LoaderCircle } from "lucide-react";
import { cn } from "../../lib/utils";

/** Inline loader for buttons, rows and short waits. Never use in answer areas. */
export function Spinner({
  className,
  label,
  size = 16,
}: {
  className?: string;
  label?: string;
  size?: number;
}) {
  return (
    <span
      role={label ? "status" : undefined}
      className={cn("inline-flex items-center gap-2 text-muted-foreground", className)}
    >
      <LoaderCircle
        size={size}
        className="animate-spin text-current"
        aria-hidden="true"
      />
      {label && <span className="sr-only">{label}</span>}
    </span>
  );
}
