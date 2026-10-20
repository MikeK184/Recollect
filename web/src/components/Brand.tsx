import { cn } from "../lib/utils";

/** The original SVG mark on its paper tile, with the Newsreader wordmark. */
export function BrandMark({ size = "md", className }: { size?: "sm" | "md" | "lg"; className?: string }) {
  return (
    <span
      className={cn(
        "grid shrink-0 place-items-center bg-brand-paper shadow-[inset_0_0_0_1px_var(--border),var(--shadow-xs)]",
        size === "sm" && "size-6 rounded-sm",
        size === "md" && "size-7 rounded-md",
        size === "lg" && "size-12 rounded-lg",
        className,
      )}
    >
      <img
        src="/brand/recollect-symbol.svg"
        alt=""
        className={cn(size === "sm" ? "size-4" : size === "md" ? "size-5" : "size-[34px]")}
      />
    </span>
  );
}

export function Brand({ compact = false, size = "md" }: { compact?: boolean; size?: "md" | "lg" }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-2 font-serif font-semibold tracking-[-0.02em] text-foreground",
        size === "md" ? "text-[20px] leading-6" : "gap-3 text-[32px] leading-9",
      )}
      role="img"
      aria-label="Recollect"
    >
      <BrandMark size={size === "lg" ? "lg" : "md"} />
      {!compact && (
        <span aria-hidden="true">
          Recollect<span className="text-brand-text">.</span>
        </span>
      )}
    </span>
  );
}
