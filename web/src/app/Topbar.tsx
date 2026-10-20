import { Fragment } from "react";
import { Link } from "@tanstack/react-router";
import type { UseQueryResult } from "@tanstack/react-query";
import { ChevronRight, PanelLeft, RefreshCw, Sparkles } from "lucide-react";
import type { Brain, Session, Status } from "../api";
import { cn } from "../lib/utils";
import { StatusDot } from "../components/StatusDot";
import { Button, IconButton } from "../components/ui/button";
import { Separator } from "../components/ui/separator";
import { OperationsDialog } from "../OperationsDialog";

/** Sticky blurred topbar: sidebar toggle, breadcrumb, service status, Ask. */
export function Topbar({
  user,
  path,
  brain,
  brainId,
  section,
  selectedLabel,
  health,
  onToggleSidebar,
  railed,
}: {
  user: Session;
  path: string;
  brain?: Brain;
  brainId?: string;
  section: string;
  selectedLabel: string;
  health: UseQueryResult<Status>;
  onToggleSidebar: () => void;
  railed: boolean;
}) {
  const crumbs: { label: string; to?: string; params?: Record<string, string> }[] =
    brainId
      ? [
          { label: "Brains", to: "/" },
          {
            label: brain?.name ?? "Brain",
            to: "/brains/$brainId/dashboard",
            params: { brainId },
          },
        ]
      : [{ label: path === "/connectors" ? "Installation" : "Workspace", to: "/" }];
  const ready = !health.isError && !!health.data?.ready;
  const statusText = health.isPending
    ? "Checking services"
    : health.isError
      ? "Health unavailable"
      : health.data?.ready
        ? "Core stores reachable"
        : "Services need attention";
  return (
    <header className="sticky top-0 z-20 flex h-[52px] shrink-0 items-center gap-2 border-b border-border bg-card/82 pl-2 pr-3 backdrop-blur-chrome">
      <IconButton
        label={railed ? "Expand sidebar" : "Collapse sidebar"}
        aria-pressed={railed}
        onClick={onToggleSidebar}
      >
        <PanelLeft aria-hidden="true" />
      </IconButton>
      <Separator orientation="vertical" className="mx-1" />
      <nav aria-label="Breadcrumb" className="flex min-w-0 items-center gap-1.5 text-body">
        {crumbs.map((crumb) => (
          <Fragment key={crumb.label}>
            <Link
              to={crumb.to ?? "/"}
              params={crumb.params}
              search={{}}
              className="max-w-[28ch] truncate text-muted-foreground no-underline transition-colors duration-fast hover:text-foreground"
            >
              {crumb.label}
            </Link>
            <ChevronRight className="size-3.5 shrink-0 text-faint-foreground" aria-hidden="true" />
          </Fragment>
        ))}
        <strong className="truncate font-medium text-foreground" aria-current="page">
          {selectedLabel}
        </strong>
      </nav>
      <div className="ml-auto flex items-center gap-1">
        <span
          className="flex items-center gap-2 pr-1 text-caption text-muted-foreground"
        >
          <StatusDot tone={ready ? "accent" : health.isPending ? "idle" : "warning"} size={7} />
          {statusText}
        </span>
        <IconButton label="Refresh service status" size="icon-sm" onClick={() => void health.refetch()}>
          <RefreshCw className={cn(health.isFetching && "animate-spin")} aria-hidden="true" />
        </IconButton>
        {user.user.installation_owner && <OperationsDialog />}
        {brainId && section !== "ask" && (
          <>
            <Separator orientation="vertical" className="mx-1" />
            <Button asChild variant="brand" size="sm">
              <Link to="/brains/$brainId/ask" params={{ brainId }} search={{}}>
                <Sparkles aria-hidden="true" />
                Ask
              </Link>
            </Button>
          </>
        )}
      </div>
    </header>
  );
}
