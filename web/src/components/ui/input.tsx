import * as React from "react";
import { Eye, EyeOff, Search } from "lucide-react";
import { cn } from "../../lib/utils";
import { controlClass, useFieldControl } from "./field";

type InputProps = Omit<React.ComponentProps<"input">, "size"> & {
  size?: "sm" | "default" | "lg";
  /** Icon or element at the start of the control. */
  leading?: React.ReactNode;
  /** Icon, Kbd or button at the end of the control. */
  trailing?: React.ReactNode;
  /** Class for the wrapper that holds leading/trailing adornments. */
  wrapperClassName?: string;
};

export function Input({
  className,
  size = "default",
  leading,
  trailing,
  wrapperClassName,
  type = "text",
  ...rest
}: InputProps) {
  const props = useFieldControl(rest);
  const input = (
    <input
      data-slot="input"
      type={type}
      className={cn(
        controlClass,
        "px-2.5",
        size === "sm" && "h-7 text-label",
        size === "default" && "h-8",
        size === "lg" && "h-10",
        leading && "pl-8",
        trailing && "pr-9",
        className,
      )}
      {...props}
    />
  );
  if (!leading && !trailing) return input;
  return (
    <div className={cn("relative flex w-full items-center", wrapperClassName)}>
      {leading && (
        <span className="pointer-events-none absolute left-2.5 flex text-muted-foreground [&_svg]:size-4">
          {leading}
        </span>
      )}
      {input}
      {trailing && (
        <span className="absolute right-1.5 flex items-center text-muted-foreground [&_svg]:size-4">
          {trailing}
        </span>
      )}
    </div>
  );
}

export function SearchInput(props: React.ComponentProps<typeof Input>) {
  return (
    // type="text" keeps the textbox role the previous inputs exposed.
    <Input
      type="text"
      inputMode="search"
      enterKeyHint="search"
      leading={<Search aria-hidden="true" />}
      {...props}
    />
  );
}

export function PasswordInput(props: React.ComponentProps<typeof Input>) {
  const [visible, setVisible] = React.useState(false);
  return (
    <Input
      {...props}
      type={visible ? "text" : "password"}
      trailing={
        <button
          type="button"
          tabIndex={-1}
          className="grid size-6 place-items-center rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground"
          aria-label={visible ? "Hide password" : "Show password"}
          onClick={() => setVisible((value) => !value)}
        >
          {visible ? <EyeOff aria-hidden="true" /> : <Eye aria-hidden="true" />}
        </button>
      }
    />
  );
}

export function Textarea({
  className,
  autosize = false,
  minRows = 3,
  maxRows,
  ...rest
}: React.ComponentProps<"textarea"> & {
  autosize?: boolean;
  minRows?: number;
  maxRows?: number;
}) {
  const props = useFieldControl(rest);
  const ref = React.useRef<HTMLTextAreaElement | null>(null);
  React.useLayoutEffect(() => {
    const node = ref.current;
    if (!autosize || !node) return;
    node.style.height = "auto";
    const line = parseFloat(getComputedStyle(node).lineHeight) || 20;
    const max = maxRows ? maxRows * line + 18 : Infinity;
    node.style.height = `${Math.min(node.scrollHeight + 2, max)}px`;
  }, [autosize, maxRows, props.value]);
  return (
    <textarea
      data-slot="textarea"
      ref={(node) => {
        ref.current = node;
      }}
      rows={minRows}
      className={cn(
        controlClass,
        "block min-h-[84px] resize-y px-2.5 py-2 leading-5 scrollbar-thin",
        autosize && "resize-none",
        className,
      )}
      {...props}
    />
  );
}
