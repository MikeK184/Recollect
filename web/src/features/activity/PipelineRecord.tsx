import { Link } from "@tanstack/react-router";
import { ArrowRight, FileInput, NotebookTabs, Sparkles, Terminal, UserRound } from "lucide-react";
import type { Brain } from "../../api";
import { InspectorSection, PropList } from "../../components/Inspector";
import { Alert, Button } from "../../components/ui";
import {
  learningFailure,
  learningState,
  origin,
  processingState,
  stateLabel,
  type PipelineItem,
} from "./pipelineModel";

const outcomeKeys = [
  "accepted",
  "proposed",
  "reused",
  "revised",
  "retired",
  "blocked",
  "conflicting",
] as const;
const when = (value: string) => new Date(value).toLocaleString();

/** Body of the "Processing record" inspector: only recorded facts. */
export function PipelineRecord({
  brain,
  item,
  at,
}: {
  brain: Brain;
  item: PipelineItem;
  at: string;
}) {
  const learning = learningState(item, at);
  const run = item.learning;
  return (
    <div className="grid">
      <InspectorSection>
        <p className="m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
          {item.title}
        </p>
        <div className="flex flex-wrap items-center gap-2 rounded-md bg-brand-soft px-3 py-2.5 text-label text-brand-text">
          <UserRound className="size-4" aria-hidden="true" />
          <span>{item.contributor}</span>
          <ArrowRight className="size-3.5" aria-hidden="true" />
          <Terminal className="size-4" aria-hidden="true" />
          <span>{origin(item)}</span>
        </div>
        <PropList
          items={[
            {
              term: "Received",
              value: <span className="mono">{when(item.received_at)}</span>,
            },
            { term: "Kind", value: item.kind.replaceAll("_", " ") },
            ...(item.tool_name ? [{ term: "Tool", value: <span className="mono">{item.tool_name}</span> }] : []),
            ...(item.agent_id
              ? [{ term: "Reported agent", value: <span className="mono">{item.agent_id}</span> }]
              : []),
          ]}
        />
        {!!item.coverage.length && (
          <Alert variant="warning" role="status" title="Capture coverage">
            {item.coverage.map((c) => c.replaceAll("_", " ")).join(" · ")}
          </Alert>
        )}
        {item.source_id && item.source_version_id ? (
          <Button asChild variant="secondary" className="w-fit">
            <Link
              to="/brains/$brainId/sources"
              params={{ brainId: brain.id }}
              search={{
                source: item.source_id,
                version: item.source_version_id,
              }}
            >
              <FileInput aria-hidden="true" />
              Read exact source version
            </Link>
          </Button>
        ) : (
          <Alert role="status">
            This capture has no retained source. No learning path is inferred.
          </Alert>
        )}
      </InspectorSection>
      <InspectorSection label="Source processing">
        <p className="m-0 text-body text-foreground">
          {stateLabel(processingState(item, at))}
        </p>
        {item.processing_job?.error_code && (
          <p className="mono m-0 text-label text-destructive-text">
            {item.processing_job.error_code}
          </p>
        )}
      </InspectorSection>
      <InspectorSection label="Recollect learning">
        <p className="m-0 text-body text-foreground">
          {learning === "failed"
            ? learningFailure(run?.job.error_code).label
            : stateLabel(learning)}
        </p>
        <p className="m-0 text-caption text-muted-foreground">
          Background processing is separate from the contributing agent.
        </p>
        {run && (
          <>
            <p className="mono m-0 text-caption text-muted-foreground">
              Recorded {when(run.created_at)}
              {run.finished_at ? ` · Finished ${when(run.finished_at)}` : ""}
            </p>
            {run.job.error_code && (
              <Alert variant="destructive" title={learningFailure(run.job.error_code).label}>
                {learningFailure(run.job.error_code).detail}
                <p className="mono mt-1.5 text-caption">
                  Reason: {run.job.error_code}
                </p>
              </Alert>
            )}
            {run.state === "succeeded" && (
              <>
                <div className="grid grid-cols-4 gap-2">
                  {outcomeKeys.map((key) => (
                    <div key={key} className="rounded-md bg-muted px-2.5 py-2">
                      <strong className="mono block text-section font-semibold text-foreground">
                        {run[key]}
                      </strong>
                      <small className="text-caption text-muted-foreground">{key}</small>
                    </div>
                  ))}
                </div>
                <p className="m-0 text-caption text-muted-foreground">
                  Recorded dispositions can overlap. These are this run’s
                  results, not the current memory inventory.
                </p>
                {!run.claim_ids.length && (
                  <p className="m-0 text-body text-foreground">
                    Completed without memory changes.
                  </p>
                )}
              </>
            )}
            <Link
              to="/brains/$brainId/activity"
              params={{ brainId: brain.id }}
              search={{ tab: "processing", job: run.job.id }}
              className="inline-flex w-fit items-center gap-1.5 text-label font-medium text-brand-text hover:underline"
            >
              <Sparkles className="size-3.5" aria-hidden="true" />
              Open learning job
            </Link>
            {run.claim_ids.slice(0, 5).map((id) => (
              <Link
                key={id}
                to="/brains/$brainId/memory"
                params={{ brainId: brain.id }}
                search={{ claim: id }}
                className="inline-flex w-fit items-center gap-1.5 text-label text-brand-text hover:underline"
              >
                <NotebookTabs className="size-3.5" aria-hidden="true" />
                Open current memory · <span className="mono">{id.slice(0, 8)}</span>
              </Link>
            ))}
          </>
        )}
      </InspectorSection>
    </div>
  );
}
