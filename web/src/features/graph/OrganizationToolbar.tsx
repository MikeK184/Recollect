import { ArrowLeft, RefreshCw, SlidersHorizontal } from "lucide-react";
import { Alert, Badge, Button, Spinner } from "../../components/ui";
import { ChromeSlot } from "./GraphStage";
import type { Center, Scope } from "./organizationModel";

// Topics tools live in the Graph header card: the organization badge beside
// the title, the actions beside the perspective switch.
export function OrganizationToolbar({
  center,
  hasView,
  onAllTopics,
  onAliases,
  onScope,
  onRefresh,
}: {
  center: Center | null;
  hasView: boolean;
  onAllTopics: () => void;
  onAliases: () => void;
  onScope: () => void;
  onRefresh: () => void;
}) {
  return (
    <>
      <ChromeSlot zone="header-meta" order={0}>
        <Badge variant="brand">Automatic organization</Badge>
      </ChromeSlot>
      <ChromeSlot
        zone="header-tools"
        order={0}
        className="organization-toolbar flex flex-wrap items-center gap-1.5"
      >
        {center && (
          <Button size="sm" variant="ghost" onClick={onAllTopics}>
            <ArrowLeft aria-hidden="true" />
            All topics
          </Button>
        )}
        <Button
          size="sm"
          variant="secondary"
          onClick={onScope}
        >
          <SlidersHorizontal aria-hidden="true" />
          Scope
        </Button>
        <Button size="sm" variant="ghost" disabled={!hasView} onClick={onAliases}>
          Review aliases
        </Button>
        <Button size="sm" variant="ghost" onClick={onRefresh}>
          <RefreshCw aria-hidden="true" />
          Refresh organization
        </Button>
      </ChromeSlot>
    </>
  );
}

export function ScopeBadges({ scope }: { scope: Scope }) {
  return (
    <ChromeSlot zone="header-extra" order={0}>
      <div
        className="flex flex-wrap items-center gap-1.5"
        aria-label="Active organization scope"
      >
        <Badge variant="outline">
          {(scope.selection.repository_ids?.length ?? 0)
            ? `${scope.selection.repository_ids?.length ?? 0} repositories`
            : "All repositories"}
        </Badge>
        {(scope.selection.area_ids?.length ?? 0) > 0 && (
          <Badge variant="outline">
            {scope.selection.area_ids?.length ?? 0} areas
          </Badge>
        )}
        {scope.selection.environment_id && (
          <Badge variant="outline">Selected environment</Badge>
        )}
        {scope.collection_id && (
          <Badge variant="outline">Selected collection</Badge>
        )}
        {scope.manifest_revision_id && (
          <Badge variant="outline">Exact manifest</Badge>
        )}
        {scope.fact_at && (
          <Badge variant="outline">
            Fact time {new Date(scope.fact_at).toLocaleString()}
          </Badge>
        )}
      </div>
    </ChromeSlot>
  );
}

export function OrganizationStatus({
  failure,
  expires,
  saved,
  hasView,
  onRefresh,
}: {
  failure: Error | null | undefined;
  expires: boolean;
  saved: boolean;
  hasView: boolean;
  onRefresh: () => void;
}) {
  const reread = (
    <Button size="sm" variant="secondary" onClick={onRefresh}>
      Read current organization
    </Button>
  );
  return (
    <>
      {failure && (
        <Alert
          variant="destructive"
          title="Organization unavailable"
          actions={reread}
        >
          {failure.message}
        </Alert>
      )}
      {expires && (
        <Alert variant="warning" role="status" actions={reread}>
          This organization reached its retention deadline. Its names, counts
          and graph have been cleared.
        </Alert>
      )}
      {saved && !failure && (
        <Alert variant="brand" role="status">
          Topic change saved.
        </Alert>
      )}
      {!hasView && !failure && !expires && (
        <p
          role="status"
          className="m-0 flex items-center gap-2 py-8 text-body text-foreground-secondary"
        >
          <Spinner />
          Reading current topics and source evidence…
        </p>
      )}
    </>
  );
}
