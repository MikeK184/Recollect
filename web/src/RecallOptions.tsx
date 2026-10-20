import { ChevronDown } from "lucide-react";
import type { components } from "./api-schema";
import type { useEvidenceList } from "./catalogueQueries";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
  Alert,
  Button,
  Checkbox,
  MultiSelect,
  NumberField,
  Select,
  TextField,
} from "./components/ui";
import { label, type Filters } from "./recallFilters";

type Catalogue = components["schemas"]["WorkspaceCatalogue"];
type Manifests = components["schemas"]["ManifestPage"];
type Groups = NonNullable<ReturnType<typeof useEvidenceList>["data"]>;
type Change = (patch: Partial<Filters>) => void;

const channelOptions = [
  { value: "exact", label: "Exact identities and literals" },
  { value: "lexical", label: "Text search" },
  { value: "graph", label: "Graph relationships" },
  { value: "semantic", label: "Semantic similarity" },
] as const;

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
    <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
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
        <div className="flex items-center gap-2 md:col-span-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={!manifestOffset}
            onClick={() => setManifestOffset(Math.max(0, manifestOffset - 20))}
          >
            Previous manifests
          </Button>
          <span className="mono text-caption text-muted-foreground">
            {manifestOffset + 1}–{Math.min(manifestOffset + 20, manifests!.total)}{" "}
            of {manifests!.total}
          </span>
          <Button
            variant="ghost"
            size="sm"
            disabled={manifestOffset + 20 >= manifests!.total}
            onClick={() => setManifestOffset(manifestOffset + 20)}
          >
            Next manifests
          </Button>
        </div>
      )}
      <TextField
        type="datetime-local"
        label="Recall knowledge time (UTC)"
        step={1}
        value={filters.knowledge}
        onChange={(e) => change({ knowledge: e.currentTarget.value })}
      />
      <TextField
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
      <TextField
        label="Exact record UUID"
        disabled={!filters.exactKind}
        value={filters.exactId}
        inputClassName="mono"
        onChange={(e) => change({ exactId: e.currentTarget.value })}
      />
    </div>
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
    <div className="grid gap-3 rounded-md border border-border p-3">
      <p className="m-0 text-body text-foreground-secondary">
        Graph search expands up to three eligible matches from the other
        selected channels. Recorded relationships do not establish truth or
        deployment behavior. Current knowledge only.
      </p>
      {(filters.mode === "history" ||
        !!filters.knowledge ||
        filters.channels.length === 1) && (
        <Alert variant="warning" title="Graph search needs current query anchors">
          Select another search channel and current knowledge, or disable graph
          search for history.
        </Alert>
      )}
      <div className="grid grid-cols-3 gap-3">
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
        <NumberField
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
      </div>
      <TextField
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
          <p className="m-0 text-label text-warning-text">
            Select one repository above for its structural graph.
          </p>
        )}
      {filters.graphKind === "combined" &&
        (!filters.environment || !filters.manifest) && (
          <p className="m-0 text-label text-warning-text">
            Select an environment and exact manifest above for combined
            relationships.
          </p>
        )}
    </div>
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
    <div className="grid gap-3">
      {filters.strategy === "manual" ? (
        <fieldset className="m-0 grid gap-2 border-0 p-0">
          <legend className="mb-2 p-0 text-label font-medium text-foreground">
            Search channels
          </legend>
          <div className="flex flex-wrap gap-x-5 gap-y-2">
            {channelOptions.map((option) => (
              <Checkbox
                key={option.value}
                label={option.label}
                value={option.value}
                checked={filters.channels.includes(option.value)}
                onChange={(e) => {
                  const next = e.currentTarget.checked
                    ? [...filters.channels, option.value]
                    : filters.channels.filter((c) => c !== option.value);
                  change({
                    channels: channelOptions
                      .map((c) => c.value as string)
                      .filter((c) => next.includes(c)),
                    strategy: "manual",
                  });
                }}
              />
            ))}
          </div>
        </fieldset>
      ) : (
        <p className="m-0 text-body text-muted-foreground">
          Automatic retrieval chooses eligible methods for this query. Scope,
          time and evidence eligibility stay fixed.
        </p>
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
        <div className="grid gap-2 rounded-md border border-border p-3">
          <p className="m-0 text-body text-foreground-secondary">
            Each Recall action may send this query to the approved embedding
            model. Failed queries are not automatically resent. Similarity does
            not establish truth or acceptance.
          </p>
          <NumberField
            label="Minimum semantic similarity"
            min={0}
            max={1}
            step={0.05}
            value={filters.similarity}
            className="max-w-[240px]"
            onChange={(v) =>
              change({
                similarity: typeof v === "number" ? v : 0,
              })
            }
          />
          <p className="m-0 text-caption text-muted-foreground">
            Zero applies no positive similarity threshold. A higher value can
            exclude useful evidence; it is not a confidence score.
          </p>
        </div>
      )}
    </div>
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
      className="group/refine rounded-lg border border-border bg-card"
      open={open}
      onToggle={(e) => setOpen(e.currentTarget.open)}
    >
      <summary className="flex cursor-pointer list-none items-center gap-2 rounded-lg px-4 py-3 text-body font-medium text-foreground hover:bg-accent focus-visible:outline-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
        Refine evidence search
        <ChevronDown
          className="ml-auto size-4 text-muted-foreground transition-transform duration-fast group-open/refine:rotate-180"
          aria-hidden="true"
        />
      </summary>
      <div className="grid gap-4 border-t border-border px-4 pb-2 pt-4">
        <Select
          label="Retrieval strategy"
          value={filters.strategy}
          allowDeselect={false}
          className="max-w-[320px]"
          data={[
            { value: "auto", label: "Automatic" },
            { value: "manual", label: "Choose methods" },
          ]}
          onChange={(value) =>
            change({ strategy: value === "manual" ? "manual" : "auto" })
          }
        />
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
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
        </div>
        <Accordion type="single" collapsible>
          <AccordionItem value="filters" className="border-t border-border">
            <AccordionTrigger>Scope, time and exact lookup</AccordionTrigger>
            <AccordionContent className="grid gap-4">
              <RecallScopeFields
                filters={filters}
                change={change}
                catalogue={catalogue}
                manifests={manifests}
                manifestOffset={manifestOffset}
                setManifestOffset={setManifestOffset}
              />
              <RecallChannelFields filters={filters} change={change} />
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      </div>
    </details>
  );
}
