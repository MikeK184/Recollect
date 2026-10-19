import type { Dispatch, SetStateAction } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Drawer,
  Group,
  Loader,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import type { UseQueryResult } from "@tanstack/react-query";
import type { Brain } from "../../api";
import type { components } from "../../api-schema";
import type { SceneGenerations } from "../../graph-generation-status";
import { label, type Scope } from "./graphScope";
import { Failure } from "./GraphPanelParts";
import type { useGraphRebuild } from "./useGraphScopeData";

export function GraphStatusDrawer({
  opened,
  onClose,
  brain,
  scene,
  scope,
  status,
  rebuild,
  offset,
  setOffset,
}: {
  opened: boolean;
  onClose: () => void;
  brain: Brain;
  scene: SceneGenerations | null;
  scope: Scope;
  status: UseQueryResult<components["schemas"]["GraphStatus"], Error>;
  rebuild: ReturnType<typeof useGraphRebuild>;
  offset: number;
  setOffset: Dispatch<SetStateAction<number>>;
}) {
  return (
    <Drawer
      className="feature-drawer"
      opened={opened}
      onClose={onClose}
      position="right"
      title="Graph status and maintenance"
      size="lg"
    >
      <Stack>
        {scene && (
          <>
            <Group>
              <Badge>{label(scene.state)}</Badge>
              <Text size="sm">
                {scene.total_nodes} eligible entities · {scene.total_edges}{" "}
                eligible relationships
              </Text>
            </Group>
            <Text size="xs" style={{ overflowWrap: "anywhere" }}>
              Generation {scene.generation.id} · input epoch{" "}
              {scene.generation.input_epoch} ·{" "}
              {scene.scope.kind === "combined"
                ? "origin epoch"
                : "memory epoch"}{" "}
              {scene.scope.kind === "combined"
                ? scene.generation.input_epoch
                : scene.memory_epoch}
            </Text>
            {scene.scope.kind === "combined" && (
              <Stack gap="xs">
                <Text fw={600}>Exact repository inputs</Text>
                {scene.inputs.map((input) => (
                  <Text
                    key={input.id}
                    size="xs"
                    style={{ overflowWrap: "anywhere" }}
                  >
                    Snapshot {input.snapshot_id} · generation {input.id}
                  </Text>
                ))}
                {!!scene.link_issues_total && (
                  <Alert color="yellow" title="Unresolved repository links">
                    <Text size="sm">
                      Showing {scene.link_issues.length} of{" "}
                      {scene.link_issues_total} eligible source records.
                    </Text>
                    {scene.link_issues.map((issue) => (
                      <Text
                        key={issue.source}
                        size="xs"
                        style={{ overflowWrap: "anywhere" }}
                      >
                        {label(issue.code)} · {issue.source}
                      </Text>
                    ))}
                  </Alert>
                )}
              </Stack>
            )}
            {scene.coverage.partial && (
              <Alert color="yellow">
                {scene.coverage.reasons.map(label).join(" · ")}
              </Alert>
            )}
            {!scene.total_nodes && (
              <Text>No entities are eligible in this view.</Text>
            )}
          </>
        )}
        {brain.role !== "reader" && (
          <Button
            variant="default"
            loading={rebuild.isPending}
            disabled={
              brain.archived ||
              (scope.kind === "repository" && !scope.snapshot_id) ||
              (scope.kind === "combined" && !scope.manifest_revision_id)
            }
            onClick={() => rebuild.mutate()}
          >
            Rebuild graph
          </Button>
        )}
        <Failure error={rebuild.error} />
        {rebuild.isSuccess && (
          <Alert color="teal">
            Graph rebuild queued. Progress appears below.
          </Alert>
        )}
        <Title order={4}>Projection processing</Title>
        <Failure error={status.error} />
        {status.isPending && <Loader size="sm" />}
        {status.data?.generations.length === 0 && (
          <Text size="sm">
            No graph generation yet. The worker discovers materialized
            repositories and claim evidence automatically.
          </Text>
        )}
        {status.data?.generations.map((g) => (
          <Card withBorder p="sm" key={g.id}>
            <Stack gap="xs">
              <Group justify="space-between">
                <Text fw={600}>{label(g.kind)} graph</Text>
                <Badge>{label(g.state)}</Badge>
              </Group>
              <Text size="xs" style={{ overflowWrap: "anywhere" }}>
                Generation {g.id}
                {g.snapshot_id ? ` · snapshot ${g.snapshot_id}` : ""}
                {g.kind === "combined"
                  ? ` · ${g.input_snapshot_ids?.length ?? 0} exact repository inputs · ${g.adapter}`
                  : ""}
              </Text>
              <Text size="sm">
                {g.node_count} entities · {g.edge_count} relationships ·{" "}
                {g.unresolved} unresolved · {g.ambiguous} ambiguous ·{" "}
                {g.unsupported} unsupported
              </Text>
              <Text size="xs">
                {new Date(g.created_at).toLocaleString()} ·{" "}
                {g.kind === "knowledge" &&
                g.input_epoch !== status.data?.memory_epoch
                  ? "Knowledge input changed; reads use current eligibility."
                  : g.kind === "combined" &&
                      g.input_epoch !== status.data?.link_epoch
                    ? "Origin bindings changed; current links are being rebuilt."
                    : "Exact input retained."}
              </Text>
              {g.error_code && <Alert color="red">{label(g.error_code)}</Alert>}
              {status.data?.jobs.find((j) => j.id === g.job_id) && (
                <Text size="xs">
                  Job:{" "}
                  {label(
                    status.data.jobs.find((j) => j.id === g.job_id)!.state,
                  )}{" "}
                  · {status.data.jobs.find((j) => j.id === g.job_id)!.progress}%
                </Text>
              )}
            </Stack>
          </Card>
        ))}
        {status.data && status.data.total > 20 && (
          <Group>
            <Button disabled={!offset} onClick={() => setOffset((n) => n - 20)}>
              Previous generations
            </Button>
            <Button
              disabled={offset + 20 >= status.data.total}
              onClick={() => setOffset((n) => n + 20)}
            >
              More generations
            </Button>
          </Group>
        )}
      </Stack>
    </Drawer>
  );
}
