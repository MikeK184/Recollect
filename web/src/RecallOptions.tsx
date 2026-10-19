import {
  Accordion,
  Alert,
  Checkbox,
  Group,
  MultiSelect,
  NumberInput,
  Select,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Button,
} from "@mantine/core";
import type { components } from "./api-schema";
import type { useEvidenceList } from "./catalogueQueries";
import { label, type Filters } from "./recallFilters";

type Catalogue = components["schemas"]["WorkspaceCatalogue"];
type Manifests = components["schemas"]["ManifestPage"];
type Groups = NonNullable<ReturnType<typeof useEvidenceList>["data"]>;
type Change = (patch: Partial<Filters>) => void;

function RecallScopeFields({
  filters,
  change,
  catalogue,
  manifests,
  manifestOffset,
  setManifestOffset,
}: {
  filters: Filters;
  change: Change;
  catalogue?: Catalogue;
  manifests?: Manifests;
  manifestOffset: number;
  setManifestOffset: (offset: number) => void;
}) {
  const manifestOptions =
    manifests?.items.map((m) => ({
      value: m.id,
      label: `${m.name} · ${label(m.kind)}`,
    })) ?? [];
  if (
    filters.manifest &&
    !manifestOptions.some((m) => m.value === filters.manifest)
  ) {
    manifestOptions.push({
      value: filters.manifest,
      label: "Selected exact manifest revision",
    });
  }
  return (
    <SimpleGrid cols={{ base: 1, sm: 2 }}>
      <MultiSelect
        label="Recall repositories"
        placeholder="All repositories"
        searchable
        clearable
        value={filters.repositories}
        data={
          catalogue?.repositories.map((r) => ({
            value: r.id,
            label: r.canonical_origin,
          })) ?? []
        }
        onChange={(v) => change({ repositories: v })}
        maxValues={100}
      />
      <MultiSelect
        label="Recall areas"
        placeholder="All areas"
        searchable
        clearable
        value={filters.areas}
        data={
          catalogue?.areas.map((a) => ({
            value: a.id,
            label: a.name,
          })) ?? []
        }
        onChange={(v) => change({ areas: v })}
        maxValues={100}
      />
      <Select
        label="Recall environment"
        clearable
        placeholder="All environments"
        value={filters.environment}
        data={
          catalogue?.environments.map((a) => ({
            value: a.id,
            label: a.name,
          })) ?? []
        }
        onChange={(v) => change({ environment: v, manifest: null })}
      />
      <Select
        label="Recall revision manifest"
        clearable
        placeholder="Select an exact manifest"
        value={filters.manifest}
        data={manifestOptions}
        onChange={(v) =>
          change({
            manifest: v,
            environment:
              manifests?.items.find((m) => m.id === v)?.environment_id ??
              filters.environment,
          })
        }
      />
      {(manifests?.total ?? 0) > 20 && (
        <Group>
          <Button
            variant="subtle"
            size="xs"
            disabled={!manifestOffset}
            onClick={() => setManifestOffset(Math.max(0, manifestOffset - 20))}
          >
            Previous manifests
          </Button>
          <Text size="xs">
            {manifestOffset + 1}–{Math.min(manifestOffset + 20, manifests!.total)}{" "}
            of {manifests!.total}
          </Text>
          <Button
            variant="subtle"
            size="xs"
            disabled={manifestOffset + 20 >= manifests!.total}
            onClick={() => setManifestOffset(manifestOffset + 20)}
          >
            Next manifests
          </Button>
        </Group>
      )}
      <TextInput
        type="datetime-local"
        label="Recall knowledge time (UTC)"
        step={1}
        value={filters.knowledge}
        onChange={(e) => change({ knowledge: e.currentTarget.value })}
      />
      <TextInput
        type="datetime-local"
        label="Recall fact time (UTC)"
        step={1}
        value={filters.fact}
        onChange={(e) => change({ fact: e.currentTarget.value })}
      />
      <Select
        label="Exact record kind"
        clearable
        placeholder="Optional exact lookup"
        value={filters.exactKind || null}
        data={[
          "claim",
          "source_version",
          "repository_fact",
          "manifest_revision",
        ].map((value) => ({ value, label: label(value) }))}
        onChange={(v) =>
          change({
            exactKind: v ?? "",
            channels: v
              ? Array.from(new Set([...filters.channels, "exact"]))
              : filters.channels,
          })
        }
      />
      <TextInput
        label="Exact record UUID"
        disabled={!filters.exactKind}
        value={filters.exactId}
        onChange={(e) => change({ exactId: e.currentTarget.value })}
      />
    </SimpleGrid>
  );
}

function RecallGraphFields({
  filters,
  change,
}: {
  filters: Filters;
  change: Change;
}) {
  return (
    <Stack gap="xs">
      <Text size="sm">
        Graph search expands up to three eligible matches from the other
        selected channels. Recorded relationships do not establish truth or
        deployment behavior. Current knowledge only.
      </Text>
      {(filters.mode === "history" ||
        !!filters.knowledge ||
        filters.channels.length === 1) && (
        <Alert color="yellow" title="Graph search needs current query anchors">
          Select another search channel and current knowledge, or disable graph
          search for history.
        </Alert>
      )}
      <SimpleGrid cols={3}>
        <Select
          label="Recall graph"
          value={filters.graphKind}
          data={["knowledge", "repository", "combined"].map((value) => ({
            value,
            label: label(value),
          }))}
          onChange={(value) => change({ graphKind: value ?? "knowledge" })}
        />
        <Select
          label="Recall graph direction"
          value={filters.graphDirection}
          data={["both", "outgoing", "incoming"].map((value) => ({
            value,
            label: label(value),
          }))}
          onChange={(value) => change({ graphDirection: value ?? "both" })}
        />
        <NumberInput
          label="Recall graph hops"
          min={1}
          max={3}
          allowDecimal={false}
          value={filters.graphHops}
          onChange={(value) =>
            change({
              graphHops: typeof value === "number" ? value : 2,
            })
          }
        />
      </SimpleGrid>
      <TextInput
        label="Recall relationships"
        description="Optional comma-separated relationship names; empty includes all eligible relationships."
        value={filters.graphRelations}
        onChange={(e) =>
          change({
            graphRelations: e.currentTarget.value,
          })
        }
      />
      {filters.graphKind === "repository" &&
        filters.repositories.length !== 1 && (
          <Text c="orange">
            Select one repository above for its structural graph.
          </Text>
        )}
      {filters.graphKind === "combined" &&
        (!filters.environment || !filters.manifest) && (
          <Text c="orange">
            Select an environment and exact manifest above for combined
            relationships.
          </Text>
        )}
    </Stack>
  );
}

function RecallChannelFields({
  filters,
  change,
}: {
  filters: Filters;
  change: Change;
}) {
  return (
    <>
      {filters.strategy === "manual" ? (
        <Checkbox.Group
          label="Search channels"
          value={filters.channels}
          onChange={(v) => change({ channels: v, strategy: "manual" })}
        >
          <Group mt="xs">
            <Checkbox value="exact" label="Exact identities and literals" />
            <Checkbox value="lexical" label="Text search" />
            <Checkbox value="graph" label="Graph relationships" />
            <Checkbox value="semantic" label="Semantic similarity" />
          </Group>
        </Checkbox.Group>
      ) : (
        <Text size="sm" c="dimmed">
          Automatic retrieval chooses eligible methods for this query. Scope,
          time and evidence eligibility stay fixed.
        </Text>
      )}
      <Checkbox
        label="Prefer source coverage before adding depth"
        checked={filters.sourceDiversity}
        onChange={(e) => change({ sourceDiversity: e.currentTarget.checked })}
      />
      {filters.strategy === "manual" && filters.channels.includes("graph") && (
        <RecallGraphFields filters={filters} change={change} />
      )}
      {filters.strategy === "manual" && filters.channels.includes("semantic") && (
        <>
          <Text size="sm">
            Each Recall action may send this query to the approved embedding
            model. Failed queries are not automatically resent. Similarity does
            not establish truth or acceptance.
          </Text>
          <NumberInput
            label="Minimum semantic similarity"
            min={0}
            max={1}
            step={0.05}
            decimalScale={2}
            value={filters.similarity}
            onChange={(v) =>
              change({
                similarity: typeof v === "number" ? v : 0,
              })
            }
          />
          <Text size="xs" c="dimmed">
            Zero applies no positive similarity threshold. A higher value can
            exclude useful evidence; it is not a confidence score.
          </Text>
        </>
      )}
    </>
  );
}

export function RecallOptions({
  filters,
  change,
  open,
  setOpen,
  catalogue,
  groups,
  manifests,
  manifestOffset,
  setManifestOffset,
}: {
  filters: Filters;
  change: Change;
  open: boolean;
  setOpen: (open: boolean) => void;
  catalogue?: Catalogue;
  groups?: Groups;
  manifests?: Manifests;
  manifestOffset: number;
  setManifestOffset: (offset: number) => void;
}) {
  return (
    <details
      className="feature-advanced evidence-search-options"
      open={open}
      onToggle={(e) => setOpen(e.currentTarget.open)}
    >
      <summary>Refine evidence search</summary>
      <Select
        label="Retrieval strategy"
        value={filters.strategy}
        allowDeselect={false}
        data={[
          { value: "auto", label: "Automatic" },
          { value: "manual", label: "Choose methods" },
        ]}
        onChange={(value) =>
          change({ strategy: value === "manual" ? "manual" : "auto" })
        }
        mb="md"
      />
      <SimpleGrid cols={{ base: 1, sm: 2 }}>
        <Select
          label="Recall mode"
          value={filters.mode}
          allowDeselect={false}
          data={[
            { value: "investigation", label: "Investigation" },
            { value: "strict_accepted", label: "Accepted and current" },
            {
              value: "strict_operational",
              label: "Accepted and operationally verified",
            },
            { value: "history", label: "Qualified history" },
          ]}
          onChange={(v) => change({ mode: v ?? "investigation" })}
        />
        <Select
          label="Recall collection"
          clearable
          placeholder="All collections"
          value={filters.collection}
          data={
            groups?.groups
              .filter((g) => g.kind === "collection")
              .map((g) => ({ value: g.id, label: g.name })) ?? []
          }
          onChange={(v) => change({ collection: v })}
        />
      </SimpleGrid>
      <Accordion>
        <Accordion.Item value="filters">
          <Accordion.Control>Scope, time and exact lookup</Accordion.Control>
          <Accordion.Panel>
            <Stack>
              <RecallScopeFields
                filters={filters}
                change={change}
                catalogue={catalogue}
                manifests={manifests}
                manifestOffset={manifestOffset}
                setManifestOffset={setManifestOffset}
              />
              <RecallChannelFields filters={filters} change={change} />
            </Stack>
          </Accordion.Panel>
        </Accordion.Item>
      </Accordion>
    </details>
  );
}
