import * as React from "react";
import {
  AlertDialog as AlertPrimitive,
  Dialog as DialogPrimitive,
} from "radix-ui";
import { X } from "lucide-react";
import { cn } from "../../lib/utils";
import { useReturnFocus } from "../../lib/focus";
import { Button } from "./button";

// Dialogs rise 8px from 0.98 over a blurred scrim; rounded-xl with a muted
// footer strip. Radix traps focus, closes on Escape and returns focus to the
// element that opened the dialog.
export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogClose = DialogPrimitive.Close;
export const DialogPortal = DialogPrimitive.Portal;

export const overlayClass =
  "fixed inset-0 z-50 bg-overlay backdrop-blur-overlay data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:duration-150";

export function DialogOverlay({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Overlay>) {
  return (
    <DialogPrimitive.Overlay
      className={cn(overlayClass, className)}
      {...props}
    />
  );
}

const widths = {
  sm: "w-[400px]",
  md: "w-[480px]",
  lg: "w-[640px]",
  xl: "w-[800px]",
  full: "w-[min(1200px,calc(100vw-64px))]",
} as const;

export function DialogContent({
  className,
  children,
  size = "md",
  showClose = true,
  closeLabel = "Close dialog",
  onOpenAutoFocus,
  onCloseAutoFocus,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Content> & {
  size?: keyof typeof widths;
  showClose?: boolean;
  closeLabel?: string;
}) {
  const focus = useReturnFocus(onOpenAutoFocus, onCloseAutoFocus);
  return (
    <DialogPortal>
      <DialogOverlay />
      <DialogPrimitive.Content
        data-slot="dialog-content"
        className={cn(
          "fixed left-1/2 top-1/2 z-50 flex max-h-[calc(100vh-64px)] max-w-[calc(100vw-32px)] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-xl border border-border bg-popover text-popover-foreground shadow-lg outline-none",
          "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98] data-[state=open]:slide-in-from-bottom-2 data-[state=open]:duration-[220ms] data-[state=open]:ease-out",
          "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-[0.98] data-[state=closed]:duration-150",
          widths[size],
          className,
        )}
        {...props}
        {...focus}
      >
        {children}
        {showClose && (
          <DialogPrimitive.Close asChild>
            <button
              type="button"
              aria-label={closeLabel}
              className="absolute right-3 top-3 grid size-8 place-items-center rounded-md text-muted-foreground transition-colors duration-fast hover:bg-accent hover:text-foreground"
            >
              <X className="size-4" aria-hidden="true" />
            </button>
          </DialogPrimitive.Close>
        )}
      </DialogPrimitive.Content>
    </DialogPortal>
  );
}

export function DialogHeader({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-header"
      className={cn("flex flex-col gap-1 px-5 pr-12 pt-5", className)}
      {...props}
    />
  );
}
export function DialogBody({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-body"
      className={cn(
        "min-h-0 flex-1 overflow-auto px-5 pb-5 pt-4 scrollbar-thin",
        className,
      )}
      {...props}
    />
  );
}
export function DialogFooter({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-footer"
      className={cn(
        "flex shrink-0 items-center justify-end gap-2 border-t border-border bg-muted px-5 py-3",
        className,
      )}
      {...props}
    />
  );
}
export function DialogTitle({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Title>) {
  return (
    <DialogPrimitive.Title
      className={cn("m-0 text-title font-semibold text-foreground", className)}
      {...props}
    />
  );
}
export function DialogDescription({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Description>) {
  return (
    <DialogPrimitive.Description
      className={cn("m-0 text-body text-muted-foreground", className)}
      {...props}
    />
  );
}

/**
 * Convenience dialog with the previous Modal's shape: controlled by `opened`
 * and `onClose`, titled, optional description and footer.
 */
export function Modal({
  opened,
  onClose,
  title,
  description,
  footer,
  size = "md",
  children,
  className,
  bodyClassName,
  closeOnClickOutside = true,
  closeOnEscape = true,
  withCloseButton = true,
  "aria-label": ariaLabel,
}: {
  opened: boolean;
  onClose: () => void;
  title?: React.ReactNode;
  description?: React.ReactNode;
  footer?: React.ReactNode;
  size?: keyof typeof widths;
  children?: React.ReactNode;
  className?: string;
  bodyClassName?: string;
  closeOnClickOutside?: boolean;
  closeOnEscape?: boolean;
  withCloseButton?: boolean;
  "aria-label"?: string;
}) {
  return (
    <Dialog open={opened} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        size={size}
        className={className}
        showClose={withCloseButton}
        aria-label={ariaLabel}
        aria-describedby={description ? undefined : undefined}
        onPointerDownOutside={(event) =>
          !closeOnClickOutside && event.preventDefault()
        }
        onEscapeKeyDown={(event) => {
          if (closeOnEscape) return;
          // Declined (a nested dialog is open): mark it so the dialog that
          // holds focus closes itself instead (see onKeyDown).
          event.preventDefault();
          (
            event as KeyboardEvent & { recollectDeclined?: boolean }
          ).recollectDeclined = true;
        }}
        onKeyDown={(event) => {
          // Escape always closes the innermost open dialog: if the topmost
          // dismissable layer declined it, the dialog containing focus does.
          if (event.key !== "Escape" || !closeOnEscape) return;
          const native = event.nativeEvent as KeyboardEvent & {
            recollectDeclined?: boolean;
          };
          if (native.defaultPrevented && !native.recollectDeclined) return;
          if (
            (event.target as Element).closest?.(
              "[data-radix-popper-content-wrapper]",
            )
          )
            return;
          event.stopPropagation();
          onClose();
        }}
      >
        {title || description ? (
          <DialogHeader>
            {title ? <DialogTitle>{title}</DialogTitle> : null}
            {description ? (
              <DialogDescription>{description}</DialogDescription>
            ) : null}
          </DialogHeader>
        ) : (
          <DialogPrimitive.Title className="sr-only">
            {ariaLabel ?? "Dialog"}
          </DialogPrimitive.Title>
        )}
        {!description && <DialogPrimitive.Description className="sr-only" />}
        <DialogBody className={bodyClassName}>{children}</DialogBody>
        {footer && <DialogFooter>{footer}</DialogFooter>}
      </DialogContent>
    </Dialog>
  );
}

/** Confirmation for irreversible actions; never opened from the palette. */
export function ConfirmDialog({
  opened,
  onClose,
  onConfirm,
  title,
  description,
  confirmLabel,
  cancelLabel = "Cancel",
  destructive = true,
  loading = false,
  children,
}: {
  opened: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: React.ReactNode;
  description?: React.ReactNode;
  confirmLabel: string;
  cancelLabel?: string;
  destructive?: boolean;
  loading?: boolean;
  children?: React.ReactNode;
}) {
  const focus = useReturnFocus();
  return (
    <AlertPrimitive.Root
      open={opened}
      onOpenChange={(open) => !open && onClose()}
    >
      <AlertPrimitive.Portal>
        <AlertPrimitive.Overlay className={overlayClass} />
        <AlertPrimitive.Content
          {...focus}
          className={cn(
            "fixed left-1/2 top-1/2 z-50 flex w-[440px] max-w-[calc(100vw-32px)] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-xl border border-border bg-popover text-popover-foreground shadow-lg outline-none",
            "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98] data-[state=open]:slide-in-from-bottom-2 data-[state=open]:duration-[220ms]",
          )}
        >
          <div className="flex flex-col gap-1 px-5 pt-5">
            <AlertPrimitive.Title className="m-0 text-title font-semibold">
              {title}
            </AlertPrimitive.Title>
            <AlertPrimitive.Description
              className={cn(
                "m-0 text-body text-muted-foreground",
                !description && "sr-only",
              )}
            >
              {description ?? title}
            </AlertPrimitive.Description>
          </div>
          <div className="px-5 pb-5 pt-3">{children}</div>
          <div className="flex justify-end gap-2 border-t border-border bg-muted px-5 py-3">
            <AlertPrimitive.Cancel asChild>
              <Button variant="secondary">{cancelLabel}</Button>
            </AlertPrimitive.Cancel>
            <Button
              variant={destructive ? "destructive" : "default"}
              loading={loading}
              onClick={onConfirm}
            >
              {confirmLabel}
            </Button>
          </div>
        </AlertPrimitive.Content>
      </AlertPrimitive.Portal>
    </AlertPrimitive.Root>
  );
}
