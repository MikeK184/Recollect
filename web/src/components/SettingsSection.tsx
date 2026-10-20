import type { ReactNode } from "react";

/** A settings group: title and description beside or above its controls. */
export function SettingsSection({
  title,
  description,
  children,
  actions,
}: {
  title: string;
  description?: string;
  children: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <section className="grid animate-enter gap-4 border-b border-border py-6 first:pt-0 last:border-b-0">
      <div className="flex items-end justify-between gap-4">
        <div className="min-w-0">
          <h2 className="m-0 text-title font-semibold text-foreground">{title}</h2>
          {description && (
            <p className="m-0 mt-0.5 max-w-[68ch] text-body text-muted-foreground">
              {description}
            </p>
          )}
        </div>
        {actions && <div className="flex shrink-0 gap-2">{actions}</div>}
      </div>
      <div className="grid gap-4">{children}</div>
    </section>
  );
}
