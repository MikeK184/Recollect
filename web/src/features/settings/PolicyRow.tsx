import type { ReactNode } from "react";
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
    <div className="privacy-policy-row">
      <div>
        <strong>{title}</strong>
        {description && <small>{description}</small>}
      </div>
      <div className="privacy-policy-value">{children}</div>
    </div>
  );
}
