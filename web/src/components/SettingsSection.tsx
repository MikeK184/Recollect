import type { ReactNode } from "react";
import { Text, Title } from "@mantine/core";

export function SettingsSection({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <section className="settings-section">
      <div className="settings-section-heading">
        <Title order={2}>{title}</Title>
        {description && (
          <Text c="dimmed" size="sm">
            {description}
          </Text>
        )}
      </div>
      <div className="settings-section-body">{children}</div>
    </section>
  );
}
