import type { ReactNode } from "react";
import { Alert, Button, Loader, Text, Title } from "@mantine/core";
import { CircleAlert, Inbox, type LucideIcon } from "lucide-react";

export function EmptyState({
  icon: Icon = Inbox,
  title,
  description,
  action,
}: {
  icon?: LucideIcon;
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty-state">
      <span className="empty-icon">
        <Icon size={28} />
      </span>
      <Title order={2}>{title}</Title>
      <Text c="dimmed" maw={440} ta="center">
        {description}
      </Text>
      {action}
    </div>
  );
}
export function ErrorState({
  error,
  retry,
}: {
  error: Error | null;
  retry?: () => void;
}) {
  if (!error) return null;
  return (
    <Alert
      color="red"
      title="Something needs attention"
      icon={<CircleAlert size={18} />}
    >
      <Text size="sm">{error.message}</Text>
      {retry && (
        <Button variant="subtle" color="red" size="xs" mt="sm" onClick={retry}>
          Try again
        </Button>
      )}
    </Alert>
  );
}
export function LoadingState({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="loading-state" role="status">
      <Loader size="sm" />
      <Text c="dimmed">{label}</Text>
    </div>
  );
}
