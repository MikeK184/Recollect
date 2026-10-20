import type { ReactNode } from "react";
import { CircleAlert, Inbox, type LucideIcon } from "lucide-react";
import { Alert } from "./ui/alert";
import { Button } from "./ui/button";
import { Tile } from "./ui/card";
import { SkeletonRows } from "./Skeleton";

export function EmptyState({
  icon: Icon = Inbox,
  title,
  description,
  action,
  className,
}: {
  icon?: LucideIcon;
  title: string;
  description: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`grid animate-enter justify-items-center gap-2 px-6 py-12 text-center ${className ?? ""}`}
    >
      <Tile size="lg" className="mb-2">
        <Icon aria-hidden="true" />
      </Tile>
      <h2 className="m-0 text-title font-semibold text-foreground">{title}</h2>
      <p className="m-0 max-w-[360px] text-body text-muted-foreground">
        {description}
      </p>
      {action && <div className="mt-3 flex flex-wrap justify-center gap-2">{action}</div>}
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
      variant="destructive"
      title="Something needs attention"
      icon={<CircleAlert aria-hidden="true" />}
      className="animate-enter"
      actions={
        retry && (
          <Button variant="destructive-ghost" size="sm" onClick={retry}>
            Try again
          </Button>
        )
      }
    >
      {error.message}
    </Alert>
  );
}

export function LoadingState({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="grid animate-enter gap-3" role="status">
      <SkeletonRows label={label} />
      <p className="m-0 text-label text-muted-foreground">{label}</p>
    </div>
  );
}
