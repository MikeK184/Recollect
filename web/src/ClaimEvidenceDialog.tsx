import {
  Alert,
  Badge,
  Button,
  Loader,
  Modal,
  Stack,
  Text,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { Failure, label, time, type Evidence } from "./claimsShared";
import { EraseAction } from "./RetentionPanel";
import { useContentDeadline } from "./useContentDeadline";
import { usePollInterval } from "./usePollInterval";

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
    <Modal
      opened
      onClose={onClose}
      size="xl"
      title="Supporting evidence"
      closeButtonProps={{ "aria-label": "Close evidence" }}
    >
      <Stack>
        <Text fw={600}>{data?.evidence.label ?? "Evidence"}</Text>
        <Failure error={query.error} />
        {query.isPending && <Loader />}
        {expired && (
          <Alert color="yellow">
            This evidence reached its retention deadline. Its content has been
            cleared.
          </Alert>
        )}
        {(query.isError || expired) && (
          <Button variant="light" onClick={() => void query.refetch()}>
            Refresh evidence
          </Button>
        )}
        {data && (
          <>
            <Badge w="fit-content">{label(data.evidence.availability)}</Badge>
            <Text size="sm">
              {label(data.evidence.kind)} · recorded{" "}
              {time(data.evidence.created_at)}
            </Text>
            {data.evidence.revision && (
              <Text size="xs" className="claim-wrap">
                Exact revision: {data.evidence.revision}
              </Text>
            )}
            {data.evidence.reference && (
              <Text size="sm" className="claim-wrap">
                Reference: {data.evidence.reference}
              </Text>
            )}
            {data.evidence.observed_at && (
              <Text size="sm">Observed: {time(data.evidence.observed_at)}</Text>
            )}
            {data.text !== null && data.text !== undefined ? (
              <pre className="source-content" data-testid="claim-evidence-text">
                {data.text}
              </pre>
            ) : evidence.kind === "source_version" ? (
              <Alert color="yellow">
                Source text is unavailable here. A reference does not confirm
                remote availability.
              </Alert>
            ) : null}
            <details className="feature-details">
              <summary>Technical record</summary>
              <pre className="source-content">
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
        <Button variant="subtle" onClick={onClose}>
          {returnLabel}
        </Button>
      </Stack>
    </Modal>
  );
}
