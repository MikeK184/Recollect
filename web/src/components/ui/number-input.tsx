import * as React from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import { cn } from "../../lib/utils";
import { controlClass, useFieldControl } from "./field";

// A text input with numeric parsing (inputMode keeps the textbox role the
// previous component exposed). onChange receives a number, or "" when empty.
export function NumberInput({
  value,
  onChange,
  min,
  max,
  step = 1,
  allowDecimal = true,
  hideControls = false,
  className,
  onBlur,
  onKeyDown,
  suffix,
  ...rest
}: Omit<React.ComponentProps<"input">, "value" | "onChange" | "min" | "max" | "step"> & {
  value: number | string | null | undefined;
  onChange: (value: number | "") => void;
  min?: number;
  max?: number;
  step?: number;
  allowDecimal?: boolean;
  hideControls?: boolean;
  suffix?: React.ReactNode;
}) {
  const props = useFieldControl(rest);
  const [draft, setDraft] = React.useState(
    value === null || value === undefined ? "" : String(value),
  );
  React.useEffect(() => {
    const next = value === null || value === undefined ? "" : String(value);
    setDraft((current) => (Number(current) === Number(next) && current !== "" ? current : next));
  }, [value]);
  const clamp = (n: number) =>
    Math.min(max ?? Infinity, Math.max(min ?? -Infinity, n));
  const commit = (text: string) => {
    if (text.trim() === "") return onChange("");
    const parsed = Number(text);
    if (Number.isFinite(parsed)) onChange(parsed);
  };
  const nudge = (direction: 1 | -1) => {
    const current = Number(draft) || 0;
    const next = clamp(Number((current + direction * step).toFixed(6)));
    setDraft(String(next));
    onChange(next);
  };
  const pattern = allowDecimal ? /^-?\d*(?:\.\d*)?$/ : /^-?\d*$/;
  return (
    <div className="relative flex w-full items-center">
      <input
        type="text"
        inputMode={allowDecimal ? "decimal" : "numeric"}
        data-slot="number-input"
        aria-valuemin={min}
        aria-valuemax={max}
        className={cn(
          controlClass,
          "mono h-8 px-2.5",
          !hideControls && "pr-8",
          suffix && (hideControls ? "pr-12" : "pr-[72px]"),
          className,
        )}
        value={draft}
        onChange={(event) => {
          const text = event.currentTarget.value;
          if (!pattern.test(text)) return;
          setDraft(text);
          commit(text);
        }}
        onKeyDown={(event) => {
          if (event.key === "ArrowUp") {
            event.preventDefault();
            nudge(1);
          } else if (event.key === "ArrowDown") {
            event.preventDefault();
            nudge(-1);
          }
          onKeyDown?.(event);
        }}
        onBlur={(event) => {
          if (draft !== "" && Number.isFinite(Number(draft))) {
            const clamped = clamp(Number(draft));
            if (clamped !== Number(draft)) {
              setDraft(String(clamped));
              onChange(clamped);
            }
          }
          onBlur?.(event);
        }}
        {...props}
      />
      {suffix && (
        <span
          className={cn(
            "pointer-events-none absolute text-caption text-muted-foreground",
            hideControls ? "right-2.5" : "right-8",
          )}
        >
          {suffix}
        </span>
      )}
      {!hideControls && (
        <span className="absolute right-1 flex flex-col">
          <button
            type="button"
            tabIndex={-1}
            aria-hidden="true"
            disabled={props.disabled}
            className="grid h-3.5 w-6 place-items-center rounded-xs text-muted-foreground hover:bg-accent hover:text-foreground [&_svg]:size-3"
            onClick={() => nudge(1)}
          >
            <ChevronUp />
          </button>
          <button
            type="button"
            tabIndex={-1}
            aria-hidden="true"
            disabled={props.disabled}
            className="grid h-3.5 w-6 place-items-center rounded-xs text-muted-foreground hover:bg-accent hover:text-foreground [&_svg]:size-3"
            onClick={() => nudge(-1)}
          >
            <ChevronDown />
          </button>
        </span>
      )}
    </div>
  );
}
