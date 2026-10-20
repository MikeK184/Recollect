import * as React from "react";
import { Command as CommandPrimitive } from "cmdk";
import { Search } from "lucide-react";
import { cn } from "../../lib/utils";

// cmdk building blocks styled for the ⌘K palette (640px, rounded-xl, 40px
// rows, muted footer). Also usable inside a Popover for searchable lists.
export function Command({
  className,
  ...props
}: React.ComponentProps<typeof CommandPrimitive>) {
  return (
    <CommandPrimitive
      className={cn("flex h-full w-full flex-col overflow-hidden bg-popover text-popover-foreground", className)}
      {...props}
    />
  );
}
export function CommandInput({
  className,
  trailing,
  ...props
}: React.ComponentProps<typeof CommandPrimitive.Input> & { trailing?: React.ReactNode }) {
  return (
    <div className="flex h-[52px] shrink-0 items-center gap-2.5 border-b border-border px-4">
      <Search className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
      <CommandPrimitive.Input
        className={cn(
          "h-full min-w-0 flex-1 border-0 bg-transparent text-[16px] leading-6 text-foreground outline-none placeholder:text-muted-foreground",
          className,
        )}
        {...props}
      />
      {trailing}
    </div>
  );
}
export function CommandList({
  className,
  ...props
}: React.ComponentProps<typeof CommandPrimitive.List>) {
  return (
    <CommandPrimitive.List
      className={cn("max-h-[360px] overflow-y-auto overflow-x-hidden p-1.5 scrollbar-thin", className)}
      {...props}
    />
  );
}
export function CommandEmpty(props: React.ComponentProps<typeof CommandPrimitive.Empty>) {
  return <CommandPrimitive.Empty className="px-3 py-6 text-center text-body text-muted-foreground" {...props} />;
}
export function CommandGroup({
  className,
  ...props
}: React.ComponentProps<typeof CommandPrimitive.Group>) {
  return (
    <CommandPrimitive.Group
      className={cn(
        "[&_[cmdk-group-heading]]:px-2.5 [&_[cmdk-group-heading]]:pb-1 [&_[cmdk-group-heading]]:pt-2 [&_[cmdk-group-heading]]:text-caption [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}
export function CommandItem({
  className,
  ...props
}: React.ComponentProps<typeof CommandPrimitive.Item>) {
  return (
    <CommandPrimitive.Item
      className={cn(
        "flex h-10 cursor-pointer select-none items-center gap-2.5 rounded-md px-2.5 text-body text-foreground outline-none data-[selected=true]:bg-accent data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:text-muted-foreground [&_small]:text-label [&_small]:text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}
export function CommandSeparator(props: React.ComponentProps<typeof CommandPrimitive.Separator>) {
  return <CommandPrimitive.Separator className="-mx-1.5 my-1 h-px bg-border" {...props} />;
}
