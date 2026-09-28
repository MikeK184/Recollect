import { Button, Modal, Stack } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "../api";
import { EvidenceDialog } from "../ClaimsPanel";
import { ErrorState, LoadingState } from "./AsyncState";

/** Resolve an exact opaque evidence ID through the authorized canonical read.
 * This also handles deep links whose source is outside the current list page. */
export function CanonicalEvidenceInspector({
  brain,
  kind,
  id,
  close,
}: {
  brain: Brain;
  kind:
    | "source_version"
    | "manifest_revision"
    | "repository_file"
    | "repository_fact";
  id: string;
  close: () => void;
}) {
  const query = useQuery({
    queryKey: ["claim-evidence-detail", brain.id, kind, id],
    gcTime: 0,
    retry: false,
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/claim-evidence/{kind}/{evidence}",
          { params: { path: { brain: brain.id, kind, evidence: id } }, signal },
        ),
      ),
  });
  if (query.isError || !query.data)
    return (
      <Modal opened onClose={close} title="Supporting evidence">
        <Stack>
          {query.isPending ? (
            <LoadingState label="Opening exact evidence…" />
          ) : (
            <ErrorState
              error={query.error}
              retry={() => void query.refetch()}
            />
          )}
          <Button variant="default" onClick={close}>
            Return to page
          </Button>
        </Stack>
      </Modal>
    );
  return (
    <EvidenceDialog
      key={`${kind}-${id}`}
      brain={brain}
      evidence={query.data.evidence}
      onClose={close}
      returnLabel="Return to page"
    />
  );
}
