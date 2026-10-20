import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { ErrorState } from "../../components/AsyncState";
import {
  Button,
  Checkbox,
  Drawer,
  MultiSelect,
  Select,
  TextField,
} from "../../components/ui";
import { useEvidenceList, useWorkspaceCatalogue } from "../../catalogueQueries";

type Request = components["schemas"]["RecallRequest"];
const label = (s: string) => s.replaceAll("_", " ");

/** "Refine evidence": the retrieval limits that apply to the answer. */
export function AskFilters({
  brain,
  opened,
  close,
  request,
  change,
  reset,
}: {
  brain: Brain;
  opened: boolean;
  close: () => void;
  request: Request;
  change: (value: Request) => void;
  reset: () => void;
}) {
  const catalogue = useWorkspaceCatalogue(brain.id, undefined, { enabled: opened });
  const groups = useEvidenceList(brain.id, undefined, { enabled: opened });
  const [manifestOffset, setManifestOffset] = useState(0);
  const manifests = useQuery({
    queryKey: [
      "ask-manifests",
      brain.id,
      request.selection?.environment_id,
      manifestOffset,
    ],
    enabled: opened,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/revision-manifests", {
          params: {
            path: { brain: brain.id },
            query: {
              environment_id: request.selection?.environment_id ?? undefined,
              offset: manifestOffset,
            },
          },
          signal,
        }),
      ),
  });
  const manifestOptions = !manifests.isError
    ? (manifests.data?.items.map((m) => ({
        value: m.id,
        label: `${m.name} · ${label(m.kind)}`,
      })) ?? [])
    : [];
  if (
    request.manifest_revision_id &&
    !manifestOptions.some((m) => m.value === request.manifest_revision_id)
  )
    manifestOptions.unshift({
      value: request.manifest_revision_id,
      label: `Selected manifest · ${request.manifest_revision_id.slice(0, 8)}`,
    });
  const patch = (value: Partial<Request>) => change({ ...request, ...value });
  return (
    <Drawer
      opened={opened}
      onClose={close}
      title="Refine evidence"
      description="These limits apply to retrieval and the answer. Changing them clears the current conversation."
      size={480}
      bodyClassName="grid content-start gap-4"
      footer={
        <>
          <Button variant="secondary" onClick={reset}>
            Reset filters
          </Button>
          <Button onClick={close}>Done</Button>
        </>
      }
    >
      <ErrorState error={catalogue.error ?? groups.error ?? manifests.error} />
      <MultiSelect
        label="Repositories"
        placeholder="All in this Brain"
        searchable
        clearable
        data={
          catalogue.data?.repositories.map((r) => ({
            value: r.id,
            label: r.canonical_origin,
          })) ?? []
        }
        value={request.selection?.repository_ids ?? []}
        onChange={(repository_ids) =>
          patch({ selection: { ...request.selection, repository_ids } })
        }
      />
      <MultiSelect
        label="Areas"
        placeholder="All in this Brain"
        searchable
        clearable
        data={
          catalogue.data?.areas.map((a) => ({
            value: a.id,
            label: a.name,
          })) ?? []
        }
        value={request.selection?.area_ids ?? []}
        onChange={(area_ids) =>
          patch({ selection: { ...request.selection, area_ids } })
        }
      />
      <Select
        label="Environment"
        placeholder="All in this Brain"
        clearable
        data={
          catalogue.data?.environments.map((a) => ({
            value: a.id,
            label: a.name,
          })) ?? []
        }
        value={request.selection?.environment_id ?? null}
        onChange={(environment_id) => {
          setManifestOffset(0);
          patch({
            selection: { ...request.selection, environment_id },
            manifest_revision_id: null,
          });
        }}
      />
      <Select
        label="Collection"
        placeholder="All collections"
        clearable
        data={
          groups.data?.groups
            .filter((g) => g.kind === "collection")
            .map((g) => ({ value: g.id, label: g.name })) ?? []
        }
        value={request.collection_id ?? null}
        onChange={(collection_id) => patch({ collection_id })}
      />
      <Select
        label="Revision manifest"
        placeholder="Select exact revisions when needed"
        clearable
        data={manifestOptions}
        value={request.manifest_revision_id ?? null}
        onChange={(manifest_revision_id) =>
          patch({
            manifest_revision_id,
            selection: {
              ...request.selection,
              environment_id:
                manifests.data?.items.find((m) => m.id === manifest_revision_id)
                  ?.environment_id ?? request.selection?.environment_id,
            },
          })
        }
      />
      {(manifests.data?.total ?? 0) > 20 && (
        <div className="flex gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={!manifestOffset}
            onClick={() => setManifestOffset(Math.max(0, manifestOffset - 20))}
          >
            Previous manifests
          </Button>
          <Button
            variant="ghost"
            size="sm"
            disabled={manifestOffset + 20 >= (manifests.data?.total ?? 0)}
            onClick={() => setManifestOffset(manifestOffset + 20)}
          >
            More manifests
          </Button>
        </div>
      )}
      <Select
        label="Evidence eligibility"
        value={request.mode}
        allowDeselect={false}
        data={[
          {
            value: "investigation",
            label: "Include qualified and disputed evidence",
          },
          { value: "strict_accepted", label: "Accepted and current only" },
          {
            value: "strict_operational",
            label: "Operationally verified only",
          },
          { value: "history", label: "Qualified history" },
        ]}
        onChange={(v) => patch({ mode: v ?? "investigation" })}
      />
      <div className="grid grid-cols-2 gap-3">
        <TextField
          label="Knowledge cutoff (UTC)"
          type="datetime-local"
          value={request.knowledge_at?.slice(0, 16) ?? ""}
          onChange={(e) =>
            patch({
              knowledge_at: e.currentTarget.value
                ? new Date(`${e.currentTarget.value}Z`).toISOString()
                : null,
            })
          }
        />
        <TextField
          label="Fact time (UTC)"
          type="datetime-local"
          value={request.fact_at?.slice(0, 16) ?? ""}
          onChange={(e) =>
            patch({
              fact_at: e.currentTarget.value
                ? new Date(`${e.currentTarget.value}Z`).toISOString()
                : null,
            })
          }
        />
      </div>
      <Select
        label="Retrieval strategy"
        value={request.strategy ?? "auto"}
        allowDeselect={false}
        data={[
          { value: "auto", label: "Automatic" },
          { value: "manual", label: "Choose methods" },
        ]}
        onChange={(value) =>
          patch({ strategy: value === "manual" ? "manual" : "auto" })
        }
      />
      {request.strategy === "manual" && (
        <Checkbox
          label="Also search by meaning"
          description="Uses the separately permitted embedding model and its budget."
          checked={request.channels?.includes("semantic") ?? false}
          onChange={(e) =>
            patch({
              channels: e.currentTarget.checked
                ? ["exact", "lexical", "semantic"]
                : ["exact", "lexical"],
            })
          }
        />
      )}
      <p className="m-0 text-caption text-muted-foreground">
        Automatic retrieval uses permitted methods within this scope. Each
        submission may use the approved embedding model; no request is sent
        while editing.
      </p>
      {request.strategy === "manual" && (
        <TextField
          label="Exact text search expression (optional)"
          description="Leave empty to find the subjects named in your question."
          value={request.query ?? ""}
          maxLength={512}
          onChange={(e) => patch({ query: e.currentTarget.value })}
        />
      )}
    </Drawer>
  );
}
