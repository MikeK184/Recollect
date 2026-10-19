import { iconSize } from "../../design/tokens";
import {
  ChartNoAxesCombined,
  Info,
  List,
  MoreHorizontal,
  Route,
  SlidersHorizontal,
} from "lucide-react";
import { Network } from "lucide-react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Menu,
  SegmentedControl,
  Stack,
  Text,
} from "@mantine/core";
import type { Brain } from "../../api";
import { ClaimDialog, EvidenceDialog } from "../../ClaimsPanel";
import { EmptyState } from "../../components/AsyncState";
import type { useEvidenceList } from "../../catalogueQueries";
import {
  evidenceModes,
  label,
  type GraphDetail,
  type GraphDrawer,
  type GraphNodeView,
  type Scope,
} from "./graphScope";

export type GraphGroups = NonNullable<
  ReturnType<typeof useEvidenceList>["data"]
>["groups"];

export function Failure({ error }: { error: Error | null }) {
  return error ? <Alert color="red">{error.message}</Alert> : null;
}

export function GraphEntityCard({
  n,
  choose,
  repositories,
  onInspect,
  onStart,
  onEnd,
}: {
  n: GraphNodeView;
  choose: boolean;
  repositories: { id: string; canonical_origin: string }[] | undefined;
  onInspect: (n: GraphNodeView) => void;
  onStart: (key: string) => void;
  onEnd: (key: string) => void;
}) {
  return (
    <Card
      withBorder
      p="sm"
      data-testid={choose ? "graph-entity" : "graph-path-entity"}
    >
      <Stack gap="xs">
        <Group justify="space-between">
          <Text fw={600} style={{ overflowWrap: "anywhere", minWidth: 0 }}>
            {n.evidence.label}
          </Text>
          <Badge>{label(n.evidence.kind)}</Badge>
        </Group>
        <Text size="xs" style={{ overflowWrap: "anywhere" }}>
          {n.key}
        </Text>
        {n.evidence.provenance[0]?.repository_id && (
          <Text size="sm" style={{ overflowWrap: "anywhere" }}>
            {repositories?.find(
              (r) => r.id === n.evidence.provenance[0]?.repository_id,
            )?.canonical_origin ?? n.evidence.provenance[0].repository_id}
            {n.evidence.provenance[0].path
              ? ` · ${n.evidence.provenance[0].path}`
              : ""}
          </Text>
        )}
        {n.evidence.provenance[0]?.revision && (
          <Text size="xs" style={{ overflowWrap: "anywhere" }}>
            Commit: {n.evidence.provenance[0].revision}
          </Text>
        )}
        {n.evidence.claim && (
          <Text size="xs">
            {label(n.evidence.claim.review)} ·{" "}
            {label(n.evidence.claim.freshness)} ·{" "}
            {label(n.evidence.claim.operational)}
          </Text>
        )}
        <Text size="xs" c="dimmed">
          {n.evidence.qualifications.map(label).join(" · ")}
        </Text>
        <Group>
          <Button size="xs" variant="light" onClick={() => onInspect(n)}>
            Inspect evidence
          </Button>
          {choose && (
            <>
              <Button size="xs" variant="subtle" onClick={() => onStart(n.key)}>
                Use as start
              </Button>
              <Button size="xs" variant="subtle" onClick={() => onEnd(n.key)}>
                Use as end
              </Button>
            </>
          )}
        </Group>
      </Stack>
    </Card>
  );
}

// Canvas chrome: kind selection, filters, path, insights, status and entity
// pages live in one compact row above the canvas region.
export function GraphToolbar({
  scope,
  readable,
  onKindChange,
  onOpen,
}: {
  scope: Scope;
  readable: boolean;
  onKindChange: (kind: string) => void;
  onOpen: (drawer: GraphDrawer) => void;
}) {
  return (
    <div className="feature-toolbar graph-chrome" data-graph-chrome="">
      <SegmentedControl
        aria-label="Graph kind"
        value={scope.kind}
        data={[
          { value: "knowledge", label: "Knowledge" },
          { value: "repository", label: "Repository" },
          { value: "combined", label: "Combined" },
        ]}
        onChange={onKindChange}
      />
      <Button
        variant="default"
        leftSection={<SlidersHorizontal size={iconSize.small} />}
        onClick={() => onOpen("filters")}
      >
        Filters
      </Button>
      <Menu position="bottom-end" withinPortal>
        <Menu.Target>
          <Button
            variant="subtle"
            ml="auto"
            leftSection={<MoreHorizontal size={iconSize.small} />}
          >
            Graph tools
          </Button>
        </Menu.Target>
        <Menu.Dropdown>
          <Menu.Item
            leftSection={<Route size={iconSize.small} />}
            onClick={() => onOpen("path")}
            disabled={!readable}
          >
            Find path
          </Menu.Item>
          <Menu.Item
            leftSection={<ChartNoAxesCombined size={iconSize.small} />}
            onClick={() => onOpen("insights")}
          >
            Insights
          </Menu.Item>
          <Menu.Item
            leftSection={<Info size={iconSize.small} />}
            onClick={() => onOpen("status")}
          >
            Graph status
          </Menu.Item>
          <Menu.Item
            leftSection={<List size={iconSize.small} />}
            onClick={() => onOpen("entities")}
          >
            Entities
          </Menu.Item>
        </Menu.Dropdown>
      </Menu>
    </div>
  );
}

export function GraphScopeSummary({
  scope,
  availableGroups,
}: {
  scope: Scope;
  availableGroups: GraphGroups;
}) {
  return (
    <div className="feature-scope graph-chrome" data-graph-chrome="">
      <Badge variant="light" color="gray">
        {scope.mode === "investigation"
          ? "Include uncertainties"
          : (evidenceModes.find((mode) => mode.value === scope.mode)?.label ??
            label(scope.mode))}
      </Badge>
      <Text size="xs" c="dimmed">
        {(scope.selection.repository_ids ?? []).length
          ? `${(scope.selection.repository_ids ?? []).length} selected repositories`
          : "All repositories in this Brain"}
      </Text>
      {scope.selection.environment_id && (
        <Badge variant="light">
          {availableGroups.find(
            (group) => group.id === scope.selection.environment_id,
          )?.name ?? "Selected environment"}
        </Badge>
      )}
      {scope.snapshot_id && (
        <Badge variant="outline">
          Snapshot {scope.snapshot_id.slice(0, 8)}
        </Badge>
      )}
      {scope.manifest_revision_id && (
        <Badge variant="outline">
          Manifest {scope.manifest_revision_id.slice(0, 8)}
        </Badge>
      )}
      {scope.fact_at && (
        <Badge variant="light">
          Fact time {new Date(scope.fact_at).toLocaleString()}
        </Badge>
      )}
      {!!(scope.selection.area_ids ?? []).length && (
        <Badge variant="light">
          {(scope.selection.area_ids ?? []).length} areas
        </Badge>
      )}
      {scope.collection_id && (
        <Badge variant="light">
          {availableGroups.find((group) => group.id === scope.collection_id)
            ?.name ?? "Selected collection"}
        </Badge>
      )}
      {!!scope.relations.length && (
        <Badge variant="light">{scope.relations.map(label).join(", ")}</Badge>
      )}
    </div>
  );
}

export function GraphEmptyState({
  needsSelection,
  unavailable,
  onChooseInputs,
  onLoad,
}: {
  needsSelection: boolean;
  unavailable: boolean;
  onChooseInputs: () => void;
  onLoad: () => void;
}) {
  return (
    <EmptyState
      icon={Network}
      title={
        needsSelection
          ? "Choose an exact graph view"
          : unavailable
            ? "This graph view is unavailable"
            : "Explore the evidence"
      }
      description={
        needsSelection
          ? "Choose a repository snapshot or exact environment manifest in Filters. Recollect will keep that revision selection explicit."
          : "Load a bounded view of eligible evidence. If a graph is too large, narrow its repositories, areas or relationships."
      }
      action={
        <Button onClick={() => (needsSelection ? onChooseInputs() : onLoad())}>
          {needsSelection ? "Choose inputs" : "Load graph view"}
        </Button>
      }
    />
  );
}

export function GraphDetailDialogs({
  brain,
  detail,
  onSaved,
  onClose,
}: {
  brain: Brain;
  detail: GraphDetail | null;
  onSaved: () => void;
  onClose: () => void;
}) {
  return (
    <>
      {detail?.claim && (
        <ClaimDialog
          brain={brain}
          id={detail.claim}
          onSaved={onSaved}
          onClose={onClose}
        />
      )}
      {detail?.evidence && (
        <EvidenceDialog
          brain={brain}
          evidence={detail.evidence}
          returnLabel="Back to graph"
          onClose={onClose}
        />
      )}
    </>
  );
}
