import type { ReactNode } from "react";
import { Text, Title } from "@mantine/core";

export function PageHeader({
  title,
  description,
  eyebrow,
  actions,
}: {
  title: string;
  description?: string;
  eyebrow?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="page-heading">
      <div>
        {eyebrow && <span className="eyebrow">{eyebrow}</span>}
        <Title order={1}>{title}</Title>
        {description && (
          <Text className="page-description" c="dimmed">
            {description}
          </Text>
        )}
      </div>
      {actions && <div className="page-actions">{actions}</div>}
    </div>
  );
}
