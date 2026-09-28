import { useState } from "react";
import {
  Alert,
  Button,
  Card,
  Code,
  Drawer,
  Group,
  Stack,
  Text,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Check, History } from "lucide-react";
import { client, result, type Audit } from "../../api";
import { useBrain, useWorkspace } from "../../app/context";
import { JobsPanel } from "../../JobsPanel";
import { ModelsPanel } from "../../ModelsPanel";
import { McpPanel } from "../../McpPanel";
import { RetentionPanel } from "../../RetentionPanel";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
const tabs = [
  { value: "timeline", label: "Timeline" },
  { value: "processing", label: "Processing" },
  { value: "tools", label: "Tool calls" },
  { value: "models", label: "Model usage" },
  { value: "removal", label: "Data removal" },
] as const;
export function ActivityPage() {
  const brain = useBrain();
  const session = useWorkspace();
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "timeline",
  );
  return (
    <>
      <PageHeader
        title="Activity"
        description="See what happened, understand exceptions, and follow the evidence."
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        {tab === "timeline" && <Timeline />}
        {tab === "processing" && (
          <JobsPanel id={brain.id} admin={brain.role === "admin"} />
        )}
        {tab === "tools" && (
          <McpPanel brain={brain} actor={session.user.id} section="activity" />
        )}
        {tab === "models" && <ModelsPanel brain={brain} section="activity" />}
        {tab === "removal" && (
          <RetentionPanel brain={brain} section="activity" />
        )}
      </FeatureTabs>
    </>
  );
}
const labels: Record<string, string> = {
  "brain.create": "Brain created",
  "brain.update": "Brain updated",
  "projection.refresh": "Brain views updated",
  "source.import": "Source imported",
  "source.process": "Source processing completed",
  "source.update": "Source version added",
  "claim.propose": "Memory recorded",
  "claim.review": "Memory review recorded",
  "repository.publish": "Repository snapshot published",
  "manifest.revise": "Environment revisions recorded",
  "task.create": "Working context started",
  "task.close": "Working context closed",
  "job.cancel": "Background job cancelled",
  "job.retry": "Background job retried",
  "memory.erase": "Memory erasure requested",
  "retention.update": "Retention policy updated",
};
function destination(event: Audit) {
  if (
    [
      "source.import",
      "source.update",
      "source.capture",
      "source.excerpt",
      "source.reprocess",
    ].includes(event.action)
  )
    return {
      section: "sources" as const,
      search: { version: event.target_id },
    };
  if (event.action === "source.organize")
    return { section: "sources" as const, search: { source: event.target_id } };
  if (event.action === "claim.propose")
    return {
      section: "memory" as const,
      search: { claim: event.target_id, knowledge: event.created_at },
    };
  if (["repository.publish", "repository.reprocess"].includes(event.action))
    return {
      section: "repositories" as const,
      search: { snapshot: event.target_id },
    };
  if (event.action === "manifest.revise")
    return {
      section: "repositories" as const,
      search: { tab: "environments", revision: event.target_id },
    };
  // Several audit targets are command/review receipts rather than resource IDs.
  // Do not manufacture a resource link from those unrelated UUIDs.
  return null;
}
function Timeline() {
  const brain = useBrain();
  const [selected, setSelected] = useState<string | null>(null);
  const query = useQuery({
    queryKey: ["audit", brain.id],
    enabled: brain.role === "admin",
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{id}/audit", {
          params: { path: { id: brain.id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    gcTime: 0,
  });
  if (brain.role !== "admin")
    return (
      <Alert color="gray" title="Administrative timeline">
        Brain administrators can view the audit timeline. Your permitted
        processing, tool calls, and model usage are available in their tabs.
      </Alert>
    );
  if (query.isError)
    return (
      <ErrorState error={query.error} retry={() => void query.refetch()} />
    );
  if (query.isPending) return <LoadingState label="Loading recent activity…" />;
  if (!query.data?.length)
    return (
      <EmptyState
        icon={History}
        title="No recorded activity yet"
        description="Changes and processing outcomes will appear here as this Brain is used."
      />
    );
  return (
    <Stack>
      <Text size="xs" c="dimmed">
        Recent authorized audit events. Independent events may overlap in time.
      </Text>
      <Card withBorder p="lg">
        {query.data.map((event) => {
          const target = destination(event);
          return (
            <div className="audit-row" key={event.id}>
              <span className="audit-icon">
                <Check size={15} />
              </span>
              <div className="job-info">
                <Text fw={600} size="sm">
                  {labels[event.action] ??
                    event.action.replaceAll(".", " ").replaceAll("_", " ")}
                </Text>
                <Text size="xs" c="dimmed" mt={4}>
                  {event.disposition.replaceAll("_", " ")} ·{" "}
                  {event.actor_id === brain.owner_id ? "Brain owner" : "Member"}
                </Text>
              </div>
              <Group gap="lg">
                <Text size="xs" c="dimmed">
                  {new Date(event.created_at).toLocaleString()}
                </Text>
                {target ? (
                  <Link
                    to={`/brains/$brainId/${target.section}`}
                    params={{ brainId: brain.id }}
                    search={target.search}
                  >
                    Open record
                  </Link>
                ) : (
                  <Button
                    variant="subtle"
                    size="xs"
                    onClick={() => setSelected(event.id)}
                  >
                    Details
                  </Button>
                )}
              </Group>
            </div>
          );
        })}
      </Card>
      <Drawer
        opened={!!selected}
        onClose={() => setSelected(null)}
        title="Activity record"
      >
        {query.data
          .filter((event) => event.id === selected)
          .map((event) => (
            <Stack key={event.id}>
              <Text fw={600}>
                {labels[event.action] ??
                  event.action.replaceAll(".", " ").replaceAll("_", " ")}
              </Text>
              <Text size="sm">
                {new Date(event.created_at).toLocaleString()} ·{" "}
                {event.disposition.replaceAll("_", " ")}
              </Text>
              <Text size="sm" c="dimmed">
                This is a recorded action. Its target may identify a command or
                review receipt rather than a current memory.
              </Text>
              <Text size="xs">
                Event <Code>{event.id}</Code>
              </Text>
              <Text size="xs">
                Recorded by <Code>{event.actor_id}</Code>
              </Text>
              <Text size="xs">
                Target <Code>{event.target_id}</Code>
              </Text>
            </Stack>
          ))}
        {selected && !query.data.some((event) => event.id === selected) && (
          <Alert color="gray">
            This event is no longer in the recent activity window.
          </Alert>
        )}
      </Drawer>
    </Stack>
  );
}
