import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import {
  ArrowUpRight,
  Check,
  History,
  ShieldAlert,
  Tv,
} from "lucide-react";
import { client, result, type Audit } from "../../api";
import { useBrain, useWorkspace } from "../../app/context";
import { CapturePanel } from "../../CapturePanel";
import { JobsPanel } from "../../JobsPanel";
import { ModelsPanel } from "../../ModelsPanel";
import { McpPanel } from "../../McpPanel";
import { RetentionPanel } from "../../RetentionPanel";
import { PageHeader } from "../../components/PageHeader";
import { Page } from "../../components/Page";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { DetailInspector, InspectorSection, PropList } from "../../components/Inspector";
import { useBrainSearch } from "../../app/useBrainSearch";
import { AssuranceActivity } from "./Assurance";
import { PipelineView } from "./PipelineView";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
import { Alert, Button, Code, Tile } from "../../components/ui";
import { cn } from "../../lib/utils";
import { useFirstLoadStagger } from "./firstLoad";
import { usePollInterval } from "../../usePollInterval";
const tabs = [
  { value: "capture", label: "Captured activity" },
  { value: "pipeline", label: "Pipeline" },
  { value: "attention", label: "Needs attention" },
  { value: "processing", label: "Processing" },
  { value: "tools", label: "Tool calls" },
  { value: "models", label: "Model usage" },
  { value: "removal", label: "Data removal" },
  { value: "timeline", label: "Administrative timeline" },
] as const;
export function ActivityPage() {
  const pollNormal = usePollInterval("normal");
  const brain = useBrain();
  const session = useWorkspace();
  const navigate = useNavigate();
  const [search, patch] = useBrainSearch();
  const roster = useQuery({
    queryKey: ["brain-agents", brain.id, true],
    enabled: !!search.device && (!search.tab || search.tab === "capture"),
    retry: false,
    gcTime: 0,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/agents", {
          signal,
          params: {
            path: { brain: brain.id },
            query: { include_hidden: true },
          },
        }),
      ),
  });
  const agent = roster.isError
    ? undefined
    : roster.data?.groups
        .flatMap((g) => g.agents)
        .find((a) => a.device_id === search.device);
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "capture",
  );
  return (
    <Page>
      <PageHeader
        title="Activity"
        description="What reached this Brain and what Recollect did with it."
        actions={
          <Button
            variant="secondary"
            onClick={() =>
              void navigate({
                to: "/brains/$brainId/tv",
                params: { brainId: brain.id },
              })
            }
          >
            <Tv aria-hidden="true" />
            Ambient display
          </Button>
        }
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        <div className="grid gap-4">
          {tab === "capture" && search.device && (
            <Alert
              variant="info"
              role="status"
              className="items-center"
            >
              <div className="flex items-center justify-between gap-3">
                <span>
                  Agent capture ·{" "}
                  <strong className="font-medium text-foreground">
                    {agent?.name ?? (
                      <>
                        Selected agent{" "}
                        <span className="mono">{search.device.slice(0, 8)}</span>
                      </>
                    )}
                  </strong>{" "}
                  {roster.isError ? "· identity unavailable" : ""}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => patch({ device: undefined })}
                >
                  Show all agents
                </Button>
              </div>
            </Alert>
          )}
          {tab === "capture" && (
            <CapturePanel
              key={`${brain.id}:${search.device ?? "all"}:${search.source ?? ""}:${search.version ?? ""}`}
              brain={brain}
              deviceId={search.device}
              selectedSource={search.source}
              selectedVersion={search.version}
            />
          )}
          {tab === "pipeline" && <PipelineView />}
          {tab === "attention" && <AssuranceActivity />}
          {tab === "timeline" && <Timeline />}
          {tab === "processing" && (
            <JobsPanel
              id={brain.id}
              admin={brain.role === "admin"}
              selectedJob={search.job}
            />
          )}
          {tab === "tools" && (
            <McpPanel
              key={search.call}
              brain={brain}
              actor={session.user.id}
              section="activity"
              initialCall={search.call}
            />
          )}
          {tab === "models" && <ModelsPanel brain={brain} section="activity" />}
          {tab === "removal" && (
            <RetentionPanel
              brain={brain}
              section="activity"
              selectedRequest={search.erasure}
            />
          )}
        </div>
      </FeatureTabs>
    </Page>
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
const actionLabel = (event: Audit) =>
  labels[event.action] ??
  event.action.replaceAll(".", " ").replaceAll("_", " ");
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
const dayLabel = (iso: string) =>
  new Date(iso).toLocaleDateString(undefined, {
    weekday: "short",
    day: "numeric",
    month: "short",
    year: "numeric",
  });
const clock = (iso: string) =>
  new Date(iso).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
/** Recent audit events grouped by local date, newest first, mono times. */
function Timeline() {
  const pollNormal = usePollInterval("normal");
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
    refetchInterval: pollNormal,
    gcTime: 0,
  });
  const stagger = useFirstLoadStagger(query.data?.map((event) => event.id));
  if (brain.role !== "admin")
    return (
      <Alert role="status" title="Administrative timeline" icon={<ShieldAlert aria-hidden="true" />}>
        Brain administrators can view the audit timeline. Your permitted
        processing, tool calls, and model usage are available in the activity
        selector.
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
  const days = new Map<string, Audit[]>();
  for (const event of query.data) {
    const day = dayLabel(event.created_at);
    days.set(day, [...(days.get(day) ?? []), event]);
  }
  const current = query.data.find((event) => event.id === selected);
  let index = 0;
  return (
    <div className="grid gap-3">
      <p className="m-0 text-caption text-muted-foreground">
        Recent authorized audit events. Independent events may overlap in time.
      </p>
      <div className="overflow-clip rounded-lg border border-border bg-card">
        {[...days].map(([day, events]) => (
          <section key={day} aria-label={day}>
            <h3 className="sticky top-0 z-[1] m-0 border-b border-border bg-muted/85 px-4 py-1.5 text-caption font-medium text-muted-foreground backdrop-blur-chrome">
              {day}
            </h3>
            {events.map((event) => {
              const target = destination(event);
              const entrance = stagger(event.id, index++);
              return (
                <div
                  className={cn(
                    "grid grid-cols-[72px_auto_minmax(0,1fr)_auto] items-center gap-3 border-b border-border px-4 py-2 last:border-b-0",
                    selected === event.id && "bg-brand-soft/50",
                    entrance.className,
                  )}
                  style={entrance.style}
                  key={event.id}
                >
                  <time
                    className="mono text-caption text-muted-foreground"
                    dateTime={event.created_at}
                    title={new Date(event.created_at).toLocaleString()}
                  >
                    {clock(event.created_at)}
                  </time>
                  <Check
                    className="size-4 text-muted-foreground"
                    aria-hidden="true"
                  />
                  <div className="grid min-w-0 gap-0.5">
                    <span className="truncate text-body font-medium text-foreground first-letter:uppercase">
                      {actionLabel(event)}
                    </span>
                    <span className="text-caption text-muted-foreground first-letter:uppercase">
                      {event.disposition.replaceAll("_", " ")} ·{" "}
                      {event.actor_id === brain.owner_id
                        ? "Brain owner"
                        : "Member"}
                    </span>
                  </div>
                  {target ? (
                    <Button variant="ghost" size="sm" asChild>
                      <Link
                        to={`/brains/$brainId/${target.section}`}
                        params={{ brainId: brain.id }}
                        search={target.search}
                      >
                        Open record
                        <ArrowUpRight aria-hidden="true" />
                      </Link>
                    </Button>
                  ) : (
                    <Button
                      variant="ghost"
                      size="sm"
                      aria-expanded={selected === event.id}
                      onClick={() => setSelected(event.id)}
                    >
                      Details
                    </Button>
                  )}
                </div>
              );
            })}
          </section>
        ))}
      </div>
      <DetailInspector
        opened={!!selected}
        onClose={() => setSelected(null)}
        title="Activity record"
      >
        {current ? (
          <>
            <InspectorSection>
              <div className="flex items-center gap-3">
                <Tile size="sm">
                  <Check aria-hidden="true" />
                </Tile>
                <div className="grid min-w-0 gap-0.5">
                  <p className="m-0 text-body font-semibold text-foreground first-letter:uppercase">
                    {actionLabel(current)}
                  </p>
                  <p className="m-0 text-caption text-muted-foreground">
                    <span className="mono">
                      {new Date(current.created_at).toLocaleString()}
                    </span>{" "}
                    · {current.disposition.replaceAll("_", " ")}
                  </p>
                </div>
              </div>
              <p className="m-0 text-label text-foreground-secondary">
                This is a recorded action. Its target may identify a command or
                review receipt rather than a current memory.
              </p>
            </InspectorSection>
            <InspectorSection label="Identifiers">
              <PropList
                items={[
                  { term: "Event", value: <Code>{current.id}</Code> },
                  { term: "Recorded by", value: <Code>{current.actor_id}</Code> },
                  { term: "Target", value: <Code>{current.target_id}</Code> },
                ]}
              />
            </InspectorSection>
          </>
        ) : (
          selected && (
            <Alert role="status">
              This event is no longer in the recent activity window.
            </Alert>
          )
        )}
      </DetailInspector>
    </div>
  );
}
