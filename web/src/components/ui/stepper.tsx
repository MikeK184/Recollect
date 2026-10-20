import * as React from "react";
import { Check } from "lucide-react";
import { cn } from "../../lib/utils";

/** Horizontal step indicator; the active step is announced as current. */
export function Stepper({
  steps,
  active,
  onStepClick,
  className,
}: {
  steps: { label: React.ReactNode; description?: React.ReactNode }[];
  active: number;
  onStepClick?: (index: number) => void;
  className?: string;
}) {
  return (
    <ol className={cn("m-0 flex list-none items-start gap-2 p-0", className)}>
      {steps.map((step, index) => {
        const done = index < active;
        const current = index === active;
        return (
          <li key={index} className="flex min-w-0 flex-1 items-start gap-2">
            <button
              type="button"
              disabled={!onStepClick}
              aria-current={current ? "step" : undefined}
              onClick={() => onStepClick?.(index)}
              className="flex min-w-0 items-start gap-2.5 text-left disabled:cursor-default"
            >
              <span
                className={cn(
                  "mono grid size-6 shrink-0 place-items-center rounded-full border text-caption font-medium transition-colors duration-fast",
                  done && "border-brand bg-brand text-brand-foreground",
                  current && "border-foreground text-foreground",
                  !done && !current && "border-border text-muted-foreground",
                )}
              >
                {done ? <Check className="size-3.5 stroke-[3]" aria-hidden="true" /> : index + 1}
              </span>
              <span className="min-w-0">
                <span className={cn("block text-label font-medium", current || done ? "text-foreground" : "text-muted-foreground")}>
                  {step.label}
                </span>
                {step.description && (
                  <span className="block text-caption text-muted-foreground">{step.description}</span>
                )}
              </span>
            </button>
            {index < steps.length - 1 && (
              <span aria-hidden="true" className={cn("mt-3 h-px min-w-4 flex-1", done ? "bg-brand/50" : "bg-border")} />
            )}
          </li>
        );
      })}
    </ol>
  );
}
