import { useEffect, useState } from "react";
import {
  Accordion,
  Alert,
  Button,
  Group,
  Loader,
  Modal,
  NumberInput,
  Select,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { GraphExplorer } from "./GraphExplorer";
import { RecallScope, recallEvidence } from "./RecallResults";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";

type Item = components["schemas"]["RecallItem"];
type Answer = components["schemas"]["RecallResponse"];
type Request = components["schemas"]["RecallRequest"];
type Scope = components["schemas"]["GraphSelection"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type PathRequest = components["schemas"]["GraphPathRequest"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];

export function RecallGraphDialog({
  brain,
  item,
  answer,
  request,
  catalogue,
  onClose,
  onInvalidated,
  onClaim,
  onEvidence,
}: {
  brain: Brain;
  item: Item;
  answer: Answer;
  request: Request;
  catalogue?: Catalogue;
  onClose: () => void;
  onInvalidated: () => void;
  onClaim: (id: string) => void;
  onEvidence: (e: Evidence) => void;
}) {
  const [scope] = useState<Scope>(
    () =>
      answer.graph?.view.scope ?? {
        kind: item.kind === "repository_fact" ? "repository" : "knowledge",
        snapshot_id:
          item.kind === "repository_fact"
            ? (item.provenance.find((p) => p.snapshot_id)?.snapshot_id ?? null)
            : null,
        selection: answer.selection,
        operation_id: request.operation_id ?? null,
        collection_id: request.collection_id ?? null,
        manifest_revision_id: answer.manifest_revision_id ?? null,
        fact_at: answer.fact_at ?? null,
        mode: answer.context.mode,
        relations: [],
      },
  );
  const center = `${item.kind}:${item.revision_id}`;
  const [start, setStart] = useState(center);
  const [end, setEnd] = useState("");
  const [direction, setDirection] = useState(
    request.graph?.direction ?? "both",
  );
  const [hops, setHops] = useState(6);
  const [pathControls, setPathControls] = useState<string | null>(null);
  const [pathRequest, setPathRequest] = useState<{
    nonce: string;
    body: PathRequest;
  } | null>(null);
  const view = useQuery({
    queryKey: [
      "investigation-graph",
      brain.id,
      answer.knowledge_at,
      item.revision_id,
      scope,
    ],
    retry: false,
    gcTime: 0,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/view", {
          params: { path: { brain: brain.id } },
          body: { scope, offset: 0 },
          signal,
        }),
      ),
  });
  const path = useQuery({
    queryKey: ["graph-path", brain.id, "investigation", pathRequest],
    enabled: !!pathRequest,
    retry: false,
    gcTime: 0,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/graph/path", {
          params: { path: { brain: brain.id } },
          body: pathRequest!.body,
          signal,
        }),
      ),
  });
  const changed =
    (!!view.data && view.data.memory_epoch !== answer.memory_epoch) ||
    (!!path.data && path.data.view.memory_epoch !== answer.memory_epoch);
  useEffect(() => {
    if (changed) onInvalidated();
  }, [changed, onInvalidated]);
  return (
    <Modal
      opened
      onClose={onClose}
      title="Relationships for recalled evidence"
      size="90%"
      closeButtonProps={{ "aria-label": "Close recalled graph" }}
    >
      <Stack>
        <Text fw={600} className="claim-wrap">
          {item.label}
        </Text>
        <Text size="sm">
          Current graph read using the investigation selection. The evidence
          query was recorded at {new Date(answer.knowledge_at).toLocaleString()}
          ; this does not reconstruct a historical graph.
        </Text>
        <RecallScope selection={scope.selection} catalogue={catalogue} />
        <Text size="xs" className="claim-wrap">
          Exact manifest: {scope.manifest_revision_id ?? "None selected"} ·
          Snapshot: {scope.snapshot_id ?? "Knowledge / manifest selection"} ·
          Mode: {scope.mode}
        </Text>
        {view.isPending && <Loader aria-label="Loading recalled graph" />}
        {view.error && (
          <Alert color="red" title="Graph inspection unavailable">
            {view.error.message} Return to the investigation to check scope or
            graph readiness.
          </Alert>
        )}
        {view.data && !view.isError && !changed && (
          <>
            <Accordion value={pathControls} onChange={setPathControls}>
              <Accordion.Item value="path">
                <Accordion.Control>Shortest path controls</Accordion.Control>
                <Accordion.Panel>
                  <Group align="end">
                    <TextInput
                      label="Investigation path start"
                      value={start}
                      onChange={(e) => {
                        setStart(e.currentTarget.value);
                        setPathRequest(null);
                      }}
                      style={{ flex: 1 }}
                    />
                    <TextInput
                      label="Investigation path end"
                      value={end}
                      onChange={(e) => {
                        setEnd(e.currentTarget.value);
                        setPathRequest(null);
                      }}
                      style={{ flex: 1 }}
                    />
                    <Select
                      label="Investigation path direction"
                      value={direction}
                      data={["both", "incoming", "outgoing"]}
                      onChange={(v) => {
                        setDirection(v ?? "both");
                        setPathRequest(null);
                      }}
                    />
                    <NumberInput
                      label="Investigation path hops"
                      value={hops}
                      min={1}
                      max={8}
                      onChange={(v) => {
                        setHops(Number(v) || 1);
                        setPathRequest(null);
                      }}
                      w={120}
                    />
                    <Button
                      disabled={!start || !end || start === end}
                      loading={path.isFetching}
                      onClick={() =>
                        setPathRequest({
                          nonce: crypto.randomUUID(),
                          body: {
                            scope: view.data!.scope,
                            start,
                            end,
                            direction,
                            max_hops: hops,
                          },
                        })
                      }
                    >
                      Find investigation path
                    </Button>
                  </Group>
                </Accordion.Panel>
              </Accordion.Item>
            </Accordion>
            {path.error && (
              <Alert color="red" title="Path unavailable">
                {path.error.message}
              </Alert>
            )}
            {path.data && !path.isError && (
              <Text size="sm">
                Path: {path.data.status.replaceAll("_", " ")}
              </Text>
            )}
            <GraphExplorer
              brain={brain}
              view={view.data}
              path={!path.isError ? (path.data ?? null) : null}
              compactControls
              expectedMemoryEpoch={answer.memory_epoch}
              initialCenter={center}
              initialDirection={request.graph?.direction ?? "both"}
              initialHops={request.graph?.max_hops ?? 2}
              onExpired={onInvalidated}
              onStart={(key) => {
                setPathControls("path");
                setStart(key);
                setPathRequest(null);
              }}
              onEnd={(key) => {
                setPathControls("path");
                setEnd(key);
                setPathRequest(null);
              }}
              inspect={(node) => {
                if (node.evidence.kind === "claim") onClaim(node.evidence.id);
                else if (node.evidence.provenance.length)
                  onEvidence(recallEvidence(node.evidence, 0));
              }}
            />
          </>
        )}
        <Button variant="subtle" onClick={onClose}>
          Back to investigation
        </Button>
      </Stack>
    </Modal>
  );
}
