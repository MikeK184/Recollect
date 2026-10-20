import * as React from "react";
import { CircleAlert } from "lucide-react";
import { cn } from "../../lib/utils";

// Field wiring: <Field label description error required> wraps one control and
// gives it an id, aria-describedby and aria-invalid through FieldContext, so
// getByLabel() and screen readers see the same name as the visible label.
type FieldState = {
  id: string;
  describedBy?: string;
  invalid: boolean;
  required: boolean;
  disabled?: boolean;
};
const FieldContext = React.createContext<FieldState | null>(null);
export const useField = () => React.useContext(FieldContext);

export function Label({
  className,
  required,
  children,
  ...props
}: React.ComponentProps<"label"> & { required?: boolean }) {
  return (
    <label
      data-slot="label"
      className={cn(
        "text-label font-medium text-foreground [&_.optional]:font-normal [&_.optional]:text-muted-foreground",
        className,
      )}
      {...props}
    >
      {children}
      {required && (
        <span aria-hidden="true" className="ml-0.5 text-destructive-text">
          *
        </span>
      )}
    </label>
  );
}

export function Field({
  label,
  description,
  error,
  required = false,
  disabled,
  id,
  className,
  children,
  labelProps,
}: {
  label?: React.ReactNode;
  description?: React.ReactNode;
  error?: React.ReactNode;
  required?: boolean;
  disabled?: boolean;
  id?: string;
  className?: string;
  children: React.ReactNode;
  labelProps?: React.ComponentProps<"label">;
}) {
  const generated = React.useId();
  const controlId = id ?? `field-${generated}`;
  const descriptionId = description ? `${controlId}-description` : undefined;
  const errorId = error ? `${controlId}-error` : undefined;
  const describedBy = [descriptionId, errorId].filter(Boolean).join(" ") || undefined;
  return (
    <FieldContext.Provider
      value={{ id: controlId, describedBy, invalid: !!error, required, disabled }}
    >
      <div data-slot="field" className={cn("grid w-full gap-1.5", className)}>
        {label && (
          <Label htmlFor={controlId} required={required} {...labelProps}>
            {label}
          </Label>
        )}
        {description && (
          <p
            id={descriptionId}
            className="-mt-0.5 m-0 text-caption text-muted-foreground"
          >
            {description}
          </p>
        )}
        {children}
        {error && (
          <p
            id={errorId}
            className="m-0 flex items-center gap-1.5 text-caption text-destructive-text"
          >
            <CircleAlert className="size-3.5 shrink-0" aria-hidden="true" />
            {error}
          </p>
        )}
      </div>
    </FieldContext.Provider>
  );
}

/** Props a control takes from its Field, overridden by explicit props. */
export function useFieldControl<T extends {
  id?: string;
  "aria-describedby"?: string;
  "aria-invalid"?: React.AriaAttributes["aria-invalid"];
  required?: boolean;
  disabled?: boolean;
}>(props: T) {
  const field = useField();
  if (!field) return props;
  return {
    ...props,
    id: props.id ?? field.id,
    "aria-describedby": props["aria-describedby"] ?? field.describedBy,
    "aria-invalid": props["aria-invalid"] ?? (field.invalid || undefined),
    required: props.required ?? (field.required || undefined),
    disabled: props.disabled ?? field.disabled,
  };
}

export const controlClass =
  "w-full min-w-0 rounded-md border border-input bg-card text-body text-foreground shadow-xs transition-[border-color,box-shadow] duration-fast ease-standard placeholder:text-muted-foreground hover:border-muted-foreground focus-visible:border-ring focus-visible:shadow-focus focus-visible:outline-none aria-invalid:border-destructive disabled:cursor-not-allowed disabled:opacity-50 read-only:hover:border-input";
