import type { ReactNode } from "react";

/**
 * One policy setting: label and optional description in the left column, its
 * value or inline control in a fixed right column. Fine separators between
 * rows and a subtle focus-within highlight mark the pair being edited.
 * `privacy-policy-row` is kept for the Playwright row-stability check.
 */
export function PolicyRow({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="privacy-policy-row grid min-h-11 grid-cols-[minmax(0,1fr)_minmax(0,300px)] items-center gap-6 border-b border-border/70 px-3 py-2 transition-colors duration-fast last:border-b-0 focus-within:bg-brand-soft/40">
      <div className="grid min-w-0 gap-0.5">
        <span className="text-label font-medium text-foreground">{title}</span>
        {description && (
          <span className="text-caption text-muted-foreground">
            {description}
          </span>
        )}
      </div>
      <div className="flex min-w-0 justify-end text-label text-foreground-secondary">
        {children}
      </div>
    </div>
  );
}
