import * as React from "react";
import { Field } from "./field";
import { Input, PasswordInput, Textarea } from "./input";
import { NumberInput } from "./number-input";

// Labelled controls with the previous TextInput/Textarea/NumberInput shape:
// `label`, `description`, `error` and `required` go to the Field, everything
// else to the control. Use <Field> + a bare control for custom layouts.
type Labelled = {
  label?: React.ReactNode;
  description?: React.ReactNode;
  error?: React.ReactNode;
  required?: boolean;
  /** Class for the field wrapper; pass inputClassName for the control. */
  className?: string;
  inputClassName?: string;
};

export function TextField({
  label,
  description,
  error,
  required,
  className,
  inputClassName,
  id,
  ...props
}: Labelled & Omit<React.ComponentProps<typeof Input>, "className">) {
  return (
    <Field label={label} description={description} error={error} required={required} id={id} className={className}>
      <Input className={inputClassName} {...props} />
    </Field>
  );
}

export function PasswordField({
  label,
  description,
  error,
  required,
  className,
  inputClassName,
  id,
  ...props
}: Labelled & Omit<React.ComponentProps<typeof PasswordInput>, "className">) {
  return (
    <Field label={label} description={description} error={error} required={required} id={id} className={className}>
      <PasswordInput className={inputClassName} {...props} />
    </Field>
  );
}

export function TextareaField({
  label,
  description,
  error,
  required,
  className,
  inputClassName,
  id,
  ...props
}: Labelled & Omit<React.ComponentProps<typeof Textarea>, "className">) {
  return (
    <Field label={label} description={description} error={error} required={required} id={id} className={className}>
      <Textarea className={inputClassName} {...props} />
    </Field>
  );
}

export function NumberField({
  label,
  description,
  error,
  required,
  className,
  inputClassName,
  id,
  ...props
}: Labelled & Omit<React.ComponentProps<typeof NumberInput>, "className">) {
  return (
    <Field label={label} description={description} error={error} required={required} id={id} className={className}>
      <NumberInput className={inputClassName} {...props} />
    </Field>
  );
}
