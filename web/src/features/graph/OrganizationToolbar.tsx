import { Alert, Badge, Button, Group, Loader, Text } from "@mantine/core";
import { ArrowLeft, RefreshCw, SlidersHorizontal } from "lucide-react";
import { iconSize } from "../../design/tokens";
import type { Center, Scope } from "./organizationModel";

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
    <Group justify="space-between" className="organization-toolbar">
      <Group>
        <Badge variant="light">Automatic organization</Badge>
        {center && (
          <Button
            size="xs"
            variant="subtle"
            leftSection={<ArrowLeft size={iconSize.small} />}
            onClick={onAllTopics}
          >
            All topics
          </Button>
        )}
      </Group>
      <Group>
        <Button variant="subtle" disabled={!hasView} onClick={onAliases}>
          Review aliases
        </Button>
        <Button
          variant="default"
          leftSection={<SlidersHorizontal size={iconSize.small} />}
          onClick={onScope}
        >
          Scope
        </Button>
        <Button
          variant="subtle"
          leftSection={<RefreshCw size={iconSize.small} />}
          onClick={onRefresh}
        >
          Refresh organization
        </Button>
      </Group>
    </Group>
  );
}

export function ScopeBadges({ scope }: { scope: Scope }) {
  return (
    <Group gap="xs" aria-label="Active organization scope">
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
    </Group>
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
  return (
    <>
      {failure && (
        <Alert color="red" title="Organization unavailable">
          {failure.message}
          <Button size="xs" variant="subtle" onClick={onRefresh}>
            Read current organization
          </Button>
        </Alert>
      )}
      {expires && (
        <Alert color="yellow">
          This organization reached its retention deadline. Its names, counts
          and graph have been cleared.
          <Button size="xs" variant="subtle" onClick={onRefresh}>
            Read current organization
          </Button>
        </Alert>
      )}
      {saved && !failure && (
        <Alert variant="light" color="teal" role="status">
          Topic change saved.
        </Alert>
      )}
      {!hasView && !failure && !expires && (
        <Group role="status" py="xl">
          <Loader size="sm" />
          <Text>Reading current topics and source evidence…</Text>
        </Group>
      )}
    </>
  );
}
