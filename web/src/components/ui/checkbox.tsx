import * as React from "react";
import { Check, Minus } from "lucide-react";
import { cn } from "../../lib/utils";
import { useFieldControl } from "./field";

// Native inputs keep getByLabel().check(), form semantics and focus order; the
// visible box and track are drawn over them. Checked uses brand. The label is
// linked with htmlFor and the description with aria-describedby, so the
// description never becomes part of the control's accessible name.
type ToggleProps = Omit<React.ComponentProps<"input">, "type" | "size"> & {
  label?: React.ReactNode;
  description?: React.ReactNode;
  indeterminate?: boolean;
};

function useToggle(rest: Omit<ToggleProps, "label" | "description" | "indeterminate" | "className">, description: React.ReactNode) {
  const props = useFieldControl(rest);
  const generated = React.useId();
  const id = props.id ?? `toggle-${generated}`;
  const descriptionId = description ? `${id}-description` : undefined;
  const describedBy =
    [props["aria-describedby"], descriptionId].filter(Boolean).join(" ") || undefined;
  return { props: { ...props, id, "aria-describedby": describedBy }, id, descriptionId };
}

function ToggleRow({
  id,
  label,
  description,
  descriptionId,
  control,
  className,
  disabled,
}: {
  id: string;
  label?: React.ReactNode;
  description?: React.ReactNode;
  descriptionId?: string;
  control: React.ReactNode;
  className?: string;
  disabled?: boolean;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-start gap-2.5 text-body text-foreground",
        disabled && "opacity-50",
        className,
      )}
    >
      {control}
      {(label || description) && (
        <span className="min-w-0">
          {label && (
            <label
              htmlFor={id}
              className={cn("block cursor-pointer select-none", disabled && "cursor-not-allowed")}
            >
              {label}
            </label>
          )}
          {description && (
            <span id={descriptionId} className="block text-caption text-muted-foreground">
              {description}
            </span>
          )}
        </span>
      )}
    </span>
  );
}

const inputCover =
  "peer absolute inset-0 m-0 cursor-pointer appearance-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:cursor-not-allowed";

export function Checkbox({
  label,
  description,
  indeterminate = false,
  className,
  ...rest
}: ToggleProps) {
  const { props, id, descriptionId } = useToggle(rest, description);
  const ref = React.useRef<HTMLInputElement>(null);
  React.useEffect(() => {
    if (ref.current) ref.current.indeterminate = indeterminate;
  }, [indeterminate]);
  return (
    <ToggleRow
      id={id}
      label={label}
      description={description}
      descriptionId={descriptionId}
      className={className}
      disabled={props.disabled}
      control={
        <span className="relative mt-0.5 grid size-4 shrink-0 place-items-center">
          <input
            ref={ref}
            type="checkbox"
            className={cn(
              inputCover,
              "rounded-xs border border-input bg-card shadow-xs transition-colors duration-fast ease-standard checked:border-brand checked:bg-brand indeterminate:border-brand indeterminate:bg-brand",
            )}
            {...props}
          />
          {indeterminate ? (
            <Minus
              aria-hidden="true"
              className="pointer-events-none relative size-3 stroke-[3] text-brand-foreground"
            />
          ) : (
            <Check
              aria-hidden="true"
              className="pointer-events-none relative size-3 scale-40 stroke-[3] text-brand-foreground opacity-0 transition-[transform,opacity] duration-fast ease-bounce peer-checked:scale-100 peer-checked:opacity-100"
            />
          )}
        </span>
      }
    />
  );
}

export function Switch({
  label,
  description,
  className,
  ...rest
}: Omit<ToggleProps, "indeterminate">) {
  const { props, id, descriptionId } = useToggle(rest, description);
  return (
    <ToggleRow
      id={id}
      label={label}
      description={description}
      descriptionId={descriptionId}
      className={className}
      disabled={props.disabled}
      control={
        <span className="relative mt-px inline-flex h-[18px] w-8 shrink-0">
          <input
            type="checkbox"
            role="switch"
            className={cn(
              inputCover,
              "rounded-full bg-input transition-colors duration-fast ease-standard checked:bg-brand",
            )}
            {...props}
          />
          <span
            aria-hidden="true"
            className="pointer-events-none absolute left-0.5 top-0.5 size-3.5 rounded-full bg-thumb shadow-xs transition-transform duration-fast ease-bounce peer-checked:translate-x-3.5"
          />
        </span>
      }
    />
  );
}

export function Radio({
  label,
  description,
  className,
  ...rest
}: Omit<ToggleProps, "indeterminate">) {
  const { props, id, descriptionId } = useToggle(rest, description);
  return (
    <ToggleRow
      id={id}
      label={label}
      description={description}
      descriptionId={descriptionId}
      className={className}
      disabled={props.disabled}
      control={
        <span className="relative mt-0.5 grid size-4 shrink-0 place-items-center">
          <input
            type="radio"
            className={cn(
              inputCover,
              "rounded-full border border-input bg-card shadow-xs transition-colors duration-fast checked:border-brand",
            )}
            {...props}
          />
          <span
            aria-hidden="true"
            className="pointer-events-none relative size-2 scale-0 rounded-full bg-brand transition-transform duration-fast ease-bounce peer-checked:scale-100"
          />
        </span>
      }
    />
  );
}
