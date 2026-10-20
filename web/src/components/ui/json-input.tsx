import * as React from "react";
import { cn } from "../../lib/utils";
import { Field } from "./field";
import { Textarea } from "./input";

/** JSON editor: monospace textarea that validates and formats on blur. */
export function JsonInput({
  value,
  onChange,
  label,
  description,
  error,
  validationError = "Invalid JSON",
  formatOnBlur = false,
  required,
  className,
  minRows = 6,
  maxRows,
  autosize = true,
  ...props
}: Omit<React.ComponentProps<"textarea">, "value" | "onChange"> & {
  value: string;
  onChange: (value: string) => void;
  label?: React.ReactNode;
  description?: React.ReactNode;
  error?: React.ReactNode;
  validationError?: React.ReactNode;
  formatOnBlur?: boolean;
  minRows?: number;
  maxRows?: number;
  autosize?: boolean;
}) {
  const [invalid, setInvalid] = React.useState(false);
  return (
    <Field
      label={label}
      description={description}
      error={error ?? (invalid ? validationError : undefined)}
      required={required}
      className={className}
    >
      <Textarea
        spellCheck={false}
        autosize={autosize}
        minRows={minRows}
        maxRows={maxRows}
        className={cn("mono text-label")}
        value={value}
        onChange={(event) => {
          onChange(event.currentTarget.value);
          if (invalid) setInvalid(false);
        }}
        onBlur={(event) => {
          const text = event.currentTarget.value;
          if (!text.trim()) return setInvalid(false);
          try {
            const parsed = JSON.parse(text);
            setInvalid(false);
            if (formatOnBlur) onChange(JSON.stringify(parsed, null, 2));
          } catch {
            setInvalid(true);
          }
          props.onBlur?.(event);
        }}
        {...props}
      />
    </Field>
  );
}
