import { Toaster as Sonner, toast } from "sonner";
import { useThemeMode } from "../../design/theme-mode";

// Bottom-right stacked toasts: rounded-lg, shadow-lg, popover surface.
export function Toaster() {
  const { resolved } = useThemeMode();
  return (
    <Sonner
      theme={resolved}
      position="bottom-right"
      gap={8}
      toastOptions={{
        classNames: {
          toast:
            "!rounded-lg !border !border-border !bg-popover !text-popover-foreground !shadow-lg !font-sans",
          title: "!text-body !font-medium",
          description: "!text-label !text-muted-foreground",
          actionButton: "!bg-primary !text-primary-foreground !rounded-md",
          cancelButton: "!bg-secondary !text-foreground !rounded-md",
          success: "[&_[data-icon]]:!text-brand",
          error: "[&_[data-icon]]:!text-destructive-text",
        },
      }}
    />
  );
}
export { toast };
