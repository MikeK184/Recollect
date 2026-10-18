import type { Dispatch, SetStateAction } from "react";
import {
  Button,
  Drawer,
  Group,
  MultiSelect,
  Select,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import type { components } from "../../api-schema";
import {
  evidenceModes,
  initial,
  label,
  localTime,
  type Scope,
} from "./graphScope";
import { Failure, type GraphGroups } from "./GraphPanelParts";

type OffsetSetter = Dispatch<SetStateAction<number>>;

export function GraphFiltersDrawer({
  opened,
  onClose,
  scope,
  change,
  repositories,
  availableGroups,
  snapshots,
  snapshotOffset,
  setSnapshotOffset,
  manifests,
  manifestOffset,
  setManifestOffset,
  error,
  needsSelection,
  loading,
  onApply,
}: {
  opened: boolean;
  onClose: () => void;
  scope: Scope;
  change: (next: Scope) => void;
  repositories: { id: string; canonical_origin: string }[] | undefined;
  availableGroups: GraphGroups;
  snapshots: components["schemas"]["SnapshotPage"] | undefined;
  snapshotOffset: number;
  setSnapshotOffset: OffsetSetter;
  manifests: components["schemas"]["ManifestPage"] | undefined;
  manifestOffset: number;
  setManifestOffset: OffsetSetter;
  error: Error | null;
  needsSelection: boolean;
  loading: boolean;
  onApply: () => void;
}) {
  return (
    <Drawer
      className="feature-drawer"
      opened={opened}
      onClose={onClose}
      position="right"
      title="Graph filters"
      size="lg"
    >
      <Stack>
        <SimpleGrid cols={{ base: 1, sm: 2 }}>
          <Select
            label="Graph kind"
            value={scope.kind}
            data={[
              {
                value: "knowledge",
                label: "Knowledge and supporting evidence",
              },
              { value: "repository", label: "Repository structure" },
              { value: "combined", label: "Combined repositories" },
            ]}
            onChange={(kind) =>
              change({ ...initial(), kind: kind ?? "knowledge" })
            }
          />
          <Select
            label="Graph evidence mode"
            value={scope.mode}
            data={evidenceModes}
            onChange={(mode) =>
              change({ ...scope, mode: mode ?? "investigation" })
            }
          />
          <MultiSelect
            label="Graph repositories"
            searchable
            value={scope.selection.repository_ids}
            maxValues={scope.kind === "repository" ? 1 : 100}
            data={(repositories ?? []).map((r) => ({
              value: r.id,
              label: r.canonical_origin,
            }))}
            onChange={(ids) => {
              setSnapshotOffset(0);
              change({
                ...scope,
                snapshot_id: null,
                selection: { ...scope.selection, repository_ids: ids },
              });
            }}
          />
          <TextInput
            label="Graph fact time"
            description="Optional applicability time in your local timezone."
            type="datetime-local"
            value={localTime(scope.fact_at)}
            onChange={(event) =>
              change({
                ...scope,
                fact_at: event.currentTarget.value
                  ? new Date(event.currentTarget.value).toISOString()
                  : null,
              })
            }
          />
          {scope.kind === "repository" && (
            <Stack gap="xs">
              <Select
                label="Graph snapshot"
                searchable
                clearable
                value={scope.snapshot_id}
                data={(snapshots?.items ?? []).map((s) => ({
                  value: s.id,
                  label: `${s.revision.slice(0, 12)} · ${new Date(s.created_at).toLocaleString()} · ${s.processing}`,
                }))}
                onChange={(id) => change({ ...scope, snapshot_id: id })}
              />
              {snapshots && snapshots.total > 20 && (
                <Group>
                  <Button
                    size="xs"
                    disabled={!snapshotOffset}
                    onClick={() => setSnapshotOffset((n) => n - 20)}
                  >
                    Previous snapshots
                  </Button>
                  <Button
                    size="xs"
                    disabled={snapshotOffset + 20 >= snapshots.total}
                    onClick={() => setSnapshotOffset((n) => n + 20)}
                  >
                    More snapshots
                  </Button>
                </Group>
              )}
            </Stack>
          )}
          <Select
            label="Graph environment"
            clearable
            value={scope.selection.environment_id}
            data={availableGroups
              .filter((g) => g.kind === "environment")
              .map((g) => ({ value: g.id, label: g.name }))}
            onChange={(id) => {
              setManifestOffset(0);
              change({
                ...scope,
                manifest_revision_id: null,
                selection: { ...scope.selection, environment_id: id },
              });
            }}
          />
          {scope.selection.environment_id && (
            <Stack gap="xs">
              <Select
                label="Graph manifest"
                clearable
                value={scope.manifest_revision_id}
                data={(manifests?.items ?? []).map((m) => ({
                  value: m.id,
                  label: `${m.name} · ${label(m.kind)} · ${m.id.slice(0, 8)}`,
                }))}
                onChange={(id) =>
                  change({ ...scope, manifest_revision_id: id })
                }
              />
              {manifests && manifests.total > 20 && (
                <Group>
                  <Button
                    size="xs"
                    disabled={!manifestOffset}
                    onClick={() => setManifestOffset((n) => n - 20)}
                  >
                    Previous manifests
                  </Button>
                  <Button
                    size="xs"
                    disabled={manifestOffset + 20 >= manifests.total}
                    onClick={() => setManifestOffset((n) => n + 20)}
                  >
                    More manifests
                  </Button>
                </Group>
              )}
            </Stack>
          )}
          <MultiSelect
            label="Graph areas"
            searchable
            value={scope.selection.area_ids}
            data={availableGroups
              .filter((g) => g.kind === "area")
              .map((g) => ({ value: g.id, label: g.name }))}
            onChange={(ids) =>
              change({
                ...scope,
                selection: { ...scope.selection, area_ids: ids },
              })
            }
          />
          {scope.kind === "knowledge" && (
            <Select
              label="Graph collection"
              clearable
              value={scope.collection_id}
              data={availableGroups
                .filter((g) => g.kind === "collection")
                .map((g) => ({ value: g.id, label: g.name }))}
              onChange={(id) => change({ ...scope, collection_id: id })}
            />
          )}
          <MultiSelect
            label="Graph relations"
            searchable
            value={scope.relations}
            data={(scope.kind === "knowledge"
              ? ["supported_by", "contributed_to"]
              : [
                  "declares",
                  "imports",
                  "calls",
                  "implements",
                  "depends_on",
                  "instantiates",
                  "injects",
                  "has_method",
                  "handled_by",
                  "implemented_by",
                  "names",
                  ...(scope.kind === "combined" ? ["terraform_module"] : []),
                ]
            ).map((value) => ({ value, label: label(value) }))}
            onChange={(relations) => change({ ...scope, relations })}
          />
        </SimpleGrid>
        <Failure error={error} />

        <Text size="sm" c="dimmed">
          Selected revisions stay exact. A path follows only the selected
          evidence and relation types; committed structure does not prove
          deployed behavior.
        </Text>
        <Button onClick={onApply} loading={loading} disabled={needsSelection}>
          Apply graph filters
        </Button>
      </Stack>
    </Drawer>
  );
}
