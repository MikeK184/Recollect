import type { Dispatch, SetStateAction } from "react";
import {
  Alert,
  Button,
  Drawer,
  MultiSelect,
  Select,
} from "../../components/ui";
import { useWorkspaceCatalogue, useEvidenceList } from "../../catalogueQueries";
import type { Scope } from "./organizationModel";

export function OrganizationScopeDrawer({
  brainId,
  opened,
  scope,
  draft,
  setDraft,
  onClose,
  onApply,
}: {
  brainId: string;
  opened: boolean;
  scope: Scope;
  draft: Scope;
  setDraft: Dispatch<SetStateAction<Scope>>;
  onClose: () => void;
  onApply: () => void;
}) {
  const catalogue = useWorkspaceCatalogue(brainId, undefined, {
    enabled: opened,
  });
  const manual = useEvidenceList(brainId, undefined, { enabled: opened });
  const manualGroups = !manual.isError ? (manual.data?.groups ?? []) : [];
  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title="Organization scope"
      description="Choose which source evidence may contribute names, memberships and counts."
      size={480}
      bodyClassName="grid content-start gap-4"
      footer={
        <>
          <Button
            variant="ghost"
            onClick={() =>
              setDraft({
                ...scope,
                collection_id: null,
                manifest_revision_id: null,
                fact_at: null,
                selection: {
                  repository_ids: [],
                  area_ids: [],
                  environment_id: null,
                },
              })
            }
          >
            Clear scope
          </Button>
          <Button onClick={onApply}>Apply organization scope</Button>
        </>
      }
    >
        {(catalogue.error || manual.error) && (
          <Alert variant="destructive">
            Scope choices could not be refreshed. Existing semantic results do
            not supply scope labels.
          </Alert>
        )}
        <MultiSelect
          label="Organization repositories"
          searchable
          value={draft.selection.repository_ids ?? []}
          data={(!catalogue.isError
            ? (catalogue.data?.repositories ?? [])
            : []
          ).map((repository) => ({
            value: repository.id,
            label: repository.canonical_origin,
          }))}
          onChange={(repository_ids) =>
            setDraft((old) => ({
              ...old,
              selection: { ...old.selection, repository_ids },
            }))
          }
        />
        <MultiSelect
          label="Organization areas"
          searchable
          value={draft.selection.area_ids ?? []}
          data={manualGroups
            .filter((group) => group.kind === "area")
            .map((group) => ({ value: group.id, label: group.name }))}
          onChange={(area_ids) =>
            setDraft((old) => ({
              ...old,
              selection: { ...old.selection, area_ids },
            }))
          }
        />
        <Select
          label="Organization environment"
          clearable
          value={draft.selection.environment_id}
          data={manualGroups
            .filter((group) => group.kind === "environment")
            .map((group) => ({ value: group.id, label: group.name }))}
          onChange={(environment_id) =>
            setDraft((old) => ({
              ...old,
              selection: { ...old.selection, environment_id },
            }))
          }
        />
        <Select
          label="Organization collection"
          clearable
          value={draft.collection_id}
          data={manualGroups
            .filter((group) => group.kind === "collection")
            .map((group) => ({ value: group.id, label: group.name }))}
          onChange={(collection_id) =>
            setDraft((old) => ({ ...old, collection_id }))
          }
        />
    </Drawer>
  );
}
