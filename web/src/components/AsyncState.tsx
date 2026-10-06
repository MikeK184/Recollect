import type { ReactNode } from "react";
import { Alert, Button, Text, Title } from "@mantine/core";
import { CircleAlert, Inbox, type LucideIcon } from "lucide-react";
import { SkeletonRows } from "./Skeleton";

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
    <div className="empty-state rc-enter">
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
    <div className="rc-enter">
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
    </div>
  );
}
export function LoadingState({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="loading-state rc-enter" role="status">
      <SkeletonRows label={label} />
      <Text c="dimmed">{label}</Text>
    </div>
  );
}
