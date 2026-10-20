import * as React from "react";
import { Dialog as DialogPrimitive } from "radix-ui";
import { X } from "lucide-react";
import { cn } from "../../lib/utils";
import { overlayClass } from "./dialog";
import { useReturnFocus } from "../../lib/focus";

// Side sheet for drawers that are not record inspection (Graph filters,
// insights, status and paths; Agents Manage access), and for the record
// inspector below 1440px. Slides 16px in from its edge with `settle`.
export const Sheet = DialogPrimitive.Root;
export const SheetTrigger = DialogPrimitive.Trigger;
export const SheetClose = DialogPrimitive.Close;

export function SheetContent({
  className,
  children,
  side = "right",
  width = 440,
  closeLabel = "Close dialog",
  showClose = true,
  onOpenAutoFocus,
  onCloseAutoFocus,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Content> & {
  side?: "right" | "left";
  width?: number | string;
  closeLabel?: string;
  showClose?: boolean;
}) {
  // Sheets open with focus on the sheet itself (announced by its title), not
  // on its first control, which may be an action such as "Rebuild graph".
  const focus = useReturnFocus((event) => {
    onOpenAutoFocus?.(event);
    if (event.defaultPrevented) return;
    event.preventDefault();
    (event.currentTarget as HTMLElement | null)?.focus({ preventScroll: true });
  }, onCloseAutoFocus);
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay className={overlayClass} />
      <DialogPrimitive.Content
        data-slot="sheet-content"
        className={cn(
          "fixed inset-y-2 z-50 flex max-w-[calc(100vw-16px)] flex-col overflow-hidden rounded-xl border border-border bg-popover text-popover-foreground shadow-lg outline-none",
          side === "right" ? "right-2" : "left-2",
          "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:duration-[360ms] data-[state=open]:ease-out",
          side === "right"
            ? "data-[state=open]:slide-in-from-right-4 data-[state=closed]:slide-out-to-right-4"
            : "data-[state=open]:slide-in-from-left-4 data-[state=closed]:slide-out-to-left-4",
          "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:duration-150",
          className,
        )}
        style={{ width }}
        tabIndex={-1}
        {...props}
        {...focus}
      >
        {children}
        {showClose && (
          <DialogPrimitive.Close asChild>
            <button
              type="button"
              aria-label={closeLabel}
              className="absolute right-2 top-2.5 grid size-8 place-items-center rounded-md text-muted-foreground transition-colors duration-fast hover:bg-accent hover:text-foreground"
            >
              <X className="size-4" aria-hidden="true" />
            </button>
          </DialogPrimitive.Close>
        )}
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}
export function SheetHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sheet-header"
      className={cn(
        "flex min-h-[52px] shrink-0 flex-col justify-center gap-0.5 border-b border-border py-2.5 pl-4 pr-12",
        className,
      )}
      {...props}
    />
  );
}
export function SheetTitle({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Title>) {
  return (
    <DialogPrimitive.Title
      className={cn("m-0 text-title font-semibold", className)}
      {...props}
    />
  );
}
export function SheetDescription({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Description>) {
  return (
    <DialogPrimitive.Description
      className={cn("m-0 text-label text-muted-foreground", className)}
      {...props}
    />
  );
}
export function SheetBody({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sheet-body"
      className={cn("min-h-0 flex-1 overflow-auto p-4 scrollbar-thin", className)}
      {...props}
    />
  );
}
export function SheetFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "flex shrink-0 items-center justify-end gap-2 border-t border-border bg-muted px-4 py-3",
        className,
      )}
      {...props}
    />
  );
}

/** Convenience sheet with the previous Drawer's shape (opened/onClose/title). */
export function Drawer({
  opened,
  onClose,
  title,
  description,
  footer,
  size = 440,
  position = "right",
  children,
  className,
  bodyClassName,
  "aria-label": ariaLabel,
}: {
  opened: boolean;
  onClose: () => void;
  title?: React.ReactNode;
  description?: React.ReactNode;
  footer?: React.ReactNode;
  size?: number | string;
  position?: "right" | "left";
  children?: React.ReactNode;
  className?: string;
  bodyClassName?: string;
  "aria-label"?: string;
}) {
  return (
    <Sheet open={opened} onOpenChange={(open) => !open && onClose()}>
      <SheetContent
        side={position}
        width={size}
        className={className}
        aria-label={ariaLabel}
      >
        <SheetHeader>
          {title ? (
            <SheetTitle>{title}</SheetTitle>
          ) : (
            <DialogPrimitive.Title className="sr-only">{ariaLabel ?? "Panel"}</DialogPrimitive.Title>
          )}
          {description ? (
            <SheetDescription>{description}</SheetDescription>
          ) : (
            <DialogPrimitive.Description className="sr-only" />
          )}
        </SheetHeader>
        <SheetBody className={bodyClassName}>{children}</SheetBody>
        {footer && <SheetFooter>{footer}</SheetFooter>}
      </SheetContent>
    </Sheet>
  );
}
