import * as React from "react";
import { AnimatePresence, motion } from "motion/react";
import { Check, Copy } from "lucide-react";
import { snap } from "../../design/motion";
import { Button, type ButtonProps } from "./button";

/** Copy text and report a short-lived `copied` state (2s by default). */
export function useClipboard(timeout = 2000) {
  const [copied, setCopied] = React.useState(false);
  const [error, setError] = React.useState<Error | null>(null);
  const timer = React.useRef<ReturnType<typeof setTimeout>>(undefined);
  React.useEffect(() => () => clearTimeout(timer.current), []);
  const copy = React.useCallback(
    async (value: string) => {
      try {
        await navigator.clipboard.writeText(value);
        setError(null);
        setCopied(true);
        clearTimeout(timer.current);
        timer.current = setTimeout(() => setCopied(false), timeout);
      } catch (cause) {
        setError(cause instanceof Error ? cause : new Error("Copy failed"));
      }
    },
    [timeout],
  );
  return { copy, copied, error };
}

/** Render-prop copy helper with the previous CopyButton's shape. */
export function CopyButton({
  value,
  timeout,
  children,
}: {
  value: string;
  timeout?: number;
  children: (state: { copied: boolean; copy: () => void }) => React.ReactNode;
}) {
  const clipboard = useClipboard(timeout);
  return <>{children({ copied: clipboard.copied, copy: () => void clipboard.copy(value) })}</>;
}

/** A ready-made copy button whose icon ticks with a small snap. */
export function CopyAction({
  value,
  label = "Copy",
  copiedLabel = "Copied",
  iconOnly = false,
  ...props
}: Omit<ButtonProps, "value" | "children"> & {
  value: string;
  label?: string;
  copiedLabel?: string;
  iconOnly?: boolean;
}) {
  const { copy, copied } = useClipboard();
  return (
    <Button
      variant="secondary"
      size={iconOnly ? "icon" : props.size}
      aria-label={iconOnly ? label : undefined}
      title={iconOnly ? label : undefined}
      onClick={() => void copy(value)}
      {...props}
    >
      <AnimatePresence mode="wait" initial={false}>
        <motion.span
          key={copied ? "done" : "copy"}
          initial={{ scale: 0.6, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          exit={{ scale: 0.6, opacity: 0 }}
          transition={snap}
          className="flex"
        >
          {copied ? <Check className="text-brand-text" aria-hidden="true" /> : <Copy aria-hidden="true" />}
        </motion.span>
      </AnimatePresence>
      {!iconOnly && <span>{copied ? copiedLabel : label}</span>}
      <span className="sr-only" role="status">
        {copied ? copiedLabel : ""}
      </span>
    </Button>
  );
}

/** Render-prop file picker with the previous FileButton's shape. */
export function FileButton({
  onChange,
  accept,
  multiple = false,
  disabled,
  children,
  resetRef,
}: {
  onChange: (file: File | null) => void;
  accept?: string;
  multiple?: boolean;
  disabled?: boolean;
  resetRef?: React.Ref<() => void>;
  children: (props: { onClick: () => void; disabled?: boolean }) => React.ReactNode;
}) {
  const input = React.useRef<HTMLInputElement>(null);
  React.useImperativeHandle(resetRef, () => () => {
    if (input.current) input.current.value = "";
  });
  return (
    <>
      {children({ onClick: () => input.current?.click(), disabled })}
      <input
        ref={input}
        type="file"
        hidden
        accept={accept}
        multiple={multiple}
        disabled={disabled}
        onChange={(event) => onChange(event.currentTarget.files?.[0] ?? null)}
      />
    </>
  );
}
