import * as React from "react";
import { motion } from "motion/react";
import { cn } from "../../lib/utils";
import { glide } from "../../design/motion";

// Segmented control on native radios (the replaced component exposed radios),
// with a Motion layoutId thumb that glides between segments.
export type SegmentOption<T extends string> =
  | T
  | { value: T; label: React.ReactNode; disabled?: boolean };

export function SegmentedControl<T extends string>({
  data,
  value,
  onChange,
  label,
  name,
  size = "default",
  fullWidth = false,
  disabled = false,
  className,
}: {
  data: readonly SegmentOption<T>[];
  value: T;
  onChange: (value: T) => void;
  /** Accessible name of the group. */
  label?: string;
  name?: string;
  size?: "sm" | "default";
  fullWidth?: boolean;
  disabled?: boolean;
  className?: string;
}) {
  const generated = React.useId();
  const group = name ?? `segment-${generated}`;
  const options = data.map((item) =>
    typeof item === "string" ? { value: item, label: item, disabled: false } : item,
  );
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className={cn(
        "relative inline-flex rounded-md bg-secondary p-0.5",
        size === "sm" ? "h-7" : "h-8",
        fullWidth && "flex w-full",
        disabled && "opacity-50",
        className,
      )}
    >
      {options.map((option) => {
        const checked = option.value === value;
        return (
          <label
            key={option.value}
            className={cn(
              "relative z-[1] inline-flex cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap rounded-sm px-3 text-label font-medium text-muted-foreground transition-colors duration-fast ease-standard has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-ring hover:text-foreground [&_svg]:size-3.5",
              size === "sm" && "px-2.5 text-caption",
              fullWidth && "flex-1",
              checked && "text-foreground",
              (option.disabled || disabled) && "cursor-not-allowed hover:text-muted-foreground",
            )}
          >
            <input
              type="radio"
              className="absolute inset-0 z-[2] m-0 cursor-[inherit] appearance-none rounded-sm opacity-0"
              name={group}
              value={option.value}
              checked={checked}
              disabled={option.disabled || disabled}
              onChange={() => onChange(option.value)}
            />
            {checked && (
              <motion.span
                layoutId={`${group}-thumb`}
                transition={glide}
                aria-hidden="true"
                className="absolute inset-0 -z-[1] rounded-sm bg-elevated shadow-xs"
              />
            )}
            {option.label}
          </label>
        );
      })}
    </div>
  );
}
