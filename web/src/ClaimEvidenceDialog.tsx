import { ChevronDown, RefreshCw } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import {
  Alert,
  Badge,
  Button,
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./components/ui";
import { PropList } from "./components/Inspector";
import { Failure, label, time, type Evidence } from "./claimsShared";
import { EraseAction } from "./RetentionPanel";
import { useContentDeadline } from "./useContentDeadline";
import { usePollInterval } from "./usePollInterval";

/**
 * The exact supporting evidence for a claim, read through the canonical
 * evidence endpoint. It opens over whatever inspector or dialog asked for it
 * (lineage, claim history, review, editor, Ask, Graph), so it is a modal
 * reading surface rather than a second docked record.
 */
export function EvidenceDialog({
  brain,
  evidence,
  onClose,
  returnLabel = "Back to claim",
  eraseSource = false,
}: {
  brain: Brain;
  evidence: Evidence;
  onClose: () => void;
  returnLabel?: string;
  eraseSource?: boolean;
}) {
  const pollNormal = usePollInterval("normal");
  const query = useQuery({
    queryKey: ["claim-evidence-detail", brain.id, evidence.kind, evidence.id],
    gcTime: 0,
    retry: false,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/claim-evidence/{kind}/{evidence}",
          {
            signal,
            params: {
              path: {
                brain: brain.id,
                kind: evidence.kind,
                evidence: evidence.id,
              },
            },
          },
        ),
      ),
  });
  const expired = useContentDeadline(query.data?.expires_at);
  const data = !query.isError && !expired ? query.data : undefined;
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="xl" closeLabel="Close evidence">
        <DialogHeader>
          <DialogTitle>Supporting evidence</DialogTitle>
          <DialogDescription className="[overflow-wrap:anywhere]">
            {data?.evidence.label ?? evidence.label ?? "Evidence"}
          </DialogDescription>
        </DialogHeader>
        <DialogBody className="grid content-start gap-4">
          <Failure error={query.error} />
          {query.isPending && (
            <p
              role="status"
              className="m-0 flex items-center gap-2 text-label text-muted-foreground"
            >
              <span
                aria-hidden="true"
                className="size-3.5 animate-spin rounded-full border-2 border-border-strong border-t-transparent"
              />
              Loading evidence…
            </p>
          )}
          {expired && (
            <Alert variant="warning" role="status">
              This evidence reached its retention deadline. Its content has been
              cleared.
            </Alert>
          )}
          {(query.isError || expired) && (
            <Button
              variant="secondary"
              size="sm"
              className="w-fit"
              onClick={() => void query.refetch()}
            >
              <RefreshCw aria-hidden="true" />
              Refresh evidence
            </Button>
          )}
          {data && (
            <>
              <div className="flex flex-wrap items-center gap-1.5">
                <Badge variant="outline">{label(data.evidence.kind)}</Badge>
                <Badge>{label(data.evidence.availability)}</Badge>
              </div>
              <PropList
                items={[
                  {
                    term: "Recorded",
                    value: (
                      <span className="mono text-caption">
                        {time(data.evidence.created_at)}
                      </span>
                    ),
                  },
                  ...(data.evidence.revision
                    ? [
                        {
                          term: "Exact revision",
                          value: (
                            <span className="mono text-caption">
                              {data.evidence.revision}
                            </span>
                          ),
                        },
                      ]
                    : []),
                  ...(data.evidence.reference
                    ? [{ term: "Reference", value: data.evidence.reference }]
                    : []),
                  ...(data.evidence.observed_at
                    ? [
                        {
                          term: "Observed",
                          value: (
                            <span className="mono text-caption">
                              {time(data.evidence.observed_at)}
                            </span>
                          ),
                        },
                      ]
                    : []),
                ]}
              />
              {data.text !== null && data.text !== undefined ? (
                <pre
                  className="mono m-0 max-h-[420px] overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted px-3 py-2.5 text-label text-foreground [overflow-wrap:anywhere] scrollbar-thin"
                  data-testid="claim-evidence-text"
                >
                  {data.text}
                </pre>
              ) : evidence.kind === "source_version" ? (
                <Alert variant="warning" role="status">
                  Source text is unavailable here. A reference does not confirm
                  remote availability.
                </Alert>
              ) : null}
              <details className="group/record rounded-lg border border-border bg-card">
                <summary className="flex h-10 cursor-pointer list-none items-center gap-2 px-3 text-label font-medium text-foreground-secondary hover:text-foreground [&::-webkit-details-marker]:hidden">
                  Technical record
                  <ChevronDown
                    aria-hidden="true"
                    className="ml-auto size-4 text-muted-foreground transition-transform duration-fast group-open/record:rotate-180"
                  />
                </summary>
                <pre className="mono m-0 max-h-[320px] overflow-auto whitespace-pre-wrap border-t border-border px-3 py-2.5 text-caption text-foreground-secondary [overflow-wrap:anywhere] scrollbar-thin">
                  {JSON.stringify(data.data, null, 2)}
                </pre>
              </details>
            </>
          )}
          {eraseSource && evidence.source_id && (
            <EraseAction
              brain={brain.id}
              permitted={brain.role === "admin"}
              target={{ kind: "source", id: evidence.source_id }}
              name="Source evidence"
            />
          )}
        </DialogBody>
        <DialogFooter>
          <Button variant="secondary" onClick={onClose}>
            {returnLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
