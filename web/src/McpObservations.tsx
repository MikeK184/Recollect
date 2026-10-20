import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { CapturedSource } from "./CapturePanel";
import { Alert, Badge, Button, Spinner } from "./components/ui";
import { useFocusReturn } from "./lib/hooks";
import { useContentDeadline } from "./useContentDeadline";
import { usePollInterval } from "./usePollInterval";
type Observation = components["schemas"]["McpObservation"];
const label = (value: string) => value.replaceAll("_", " ");

function ObservationRow({
  observation: o,
  inspect,
}: {
  observation: Observation;
  inspect: () => void;
}) {
  const expired = useContentDeadline(o.expires_at);
  const state = expired && o.state !== "removed" ? "expired" : o.state;
  return (
    <div className="grid gap-1 border-t border-border pt-2.5 first:border-t-0 first:pt-0">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-label text-foreground">
          {label(o.stage)} · {label(o.outcome)}
        </span>
        <Badge variant={state === "published" ? "brand" : "default"}>
          {state}
        </Badge>
      </div>
      <p className="m-0 text-caption text-muted-foreground">
        Observed{" "}
        <span className="mono">{new Date(o.captured_at).toLocaleString()}</span>{" "}
        · <span className="mono">{o.attempts}</span> publication attempts
      </p>
      {!expired && o.error_code && (
        <p className="m-0 text-label text-foreground-secondary">
          {label(o.error_code)}
        </p>
      )}
      {!expired && !!o.coverage.length && (
        <p className="m-0 text-caption text-muted-foreground">
          Coverage: {o.coverage.map(label).join(" · ")}
        </p>
      )}
      {!expired &&
        o.state === "published" &&
        o.source_id &&
        o.source_version_id && (
          <Button
            variant="secondary"
            size="sm"
            className="justify-self-start"
            onClick={inspect}
          >
            Inspect captured evidence
          </Button>
        )}
    </div>
  );
}
export function McpObservations({
  brain,
  call,
  disposition,
  onInspectChange,
}: {
  brain: Brain;
  call: string;
  disposition: string;
  onInspectChange: (opened: boolean) => void;
}) {
  const pollFast = usePollInterval("fast");
  const [offset, setOffset] = useState(0);
  const [viewing, setViewing] = useState<Observation | null>(null);
  const query = useQuery({
    queryKey: ["mcp-observations", brain.id, call, offset],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/calls/{id}/observations", {
          params: { path: { brain: brain.id, id: call }, query: { offset } },
          signal,
        }),
      ),
    refetchInterval: pollFast,
    retry: false,
    gcTime: 0,
  });
  useFocusReturn({ opened: viewing !== null && !query.error });
  useEffect(() => {
    onInspectChange(viewing !== null && !query.error);
    return () => onInspectChange(false);
  }, [viewing, query.error, onInspectChange]);
  const messages: Record<string, string> = {
    eligible:
      "Capture was enabled for this call. Publication runs automatically after an outcome is recorded.",
    disabled: "Managed capture was disabled when this call was admitted.",
    knowledge_write_required:
      "This caller could run the tool but did not have knowledge-write permission.",
    receipt_lookup: "Receipt evidence is linked to the original call.",
    removed:
      "This call's captured evidence was erased. Late results cannot restore it.",
  };
  return (
    <div className="grid gap-2.5" aria-label="Managed observations">
      <h3 className="section-label m-0">Captured observations</h3>
      <p className="m-0 text-label text-muted-foreground">
        {messages[disposition] ?? label(disposition)}
      </p>
      {query.isPending && <Spinner label="Loading observations" />}
      {query.error && (
        <Alert
          variant="destructive"
          actions={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void query.refetch()}
            >
              Refresh observations
            </Button>
          }
        >
          {query.error.message}
        </Alert>
      )}
      {!query.error &&
        query.data?.items.map((o) => (
          <ObservationRow
            key={o.id}
            observation={o}
            inspect={() => setViewing(o)}
          />
        ))}
      {!query.error && query.data?.total === 0 && (
        <p className="m-0 text-label text-foreground-secondary">
          No published observation yet.
        </p>
      )}
      {query.data && query.data.total > 20 && (
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={!offset}
            onClick={() => setOffset(offset - 20)}
          >
            Previous observations
          </Button>
          <span className="mono text-caption text-muted-foreground">
            {offset + 1}–{Math.min(offset + 20, query.data.total)} of{" "}
            {query.data.total}
          </span>
          <Button
            variant="ghost"
            size="sm"
            disabled={offset + 20 >= query.data.total}
            onClick={() => setOffset(offset + 20)}
          >
            Next observations
          </Button>
        </div>
      )}
      {viewing && !query.error && (
        <CapturedSource
          brain={brain}
          source={viewing.source_id!}
          version={viewing.source_version_id!}
          close={() => setViewing(null)}
        />
      )}
    </div>
  );
}
