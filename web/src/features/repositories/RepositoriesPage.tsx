import { WorkspacePanel } from "../../WorkspacePanel";
import { PublicationPanel, SnapshotDialog } from "../../PublicationPanel";
import { useBrain } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { CanonicalEvidenceInspector } from "../../components/CanonicalEvidenceInspector";
const tabs = [
  { value: "repositories", label: "Repositories" },
  { value: "environments", label: "Environments" },
] as const;
export function RepositoriesPage() {
  const brain = useBrain();
  const [search, patch] = useBrainSearch();
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "repositories",
  );
  return (
    <>
      <PageHeader
        title="Repositories"
        description="Explore published code and the exact revisions your knowledge refers to."
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        {tab === "repositories" ? (
          <WorkspacePanel brain={brain} section="repositories" />
        ) : (
          <PublicationPanel brain={brain} section="environments" />
        )}
        {search.snapshot && (
          <SnapshotDialog
            key={search.snapshot}
            brain={brain}
            id={search.snapshot}
            onClose={() => patch({ snapshot: null })}
          />
        )}
        {search.revision && !search.snapshot && (
          <CanonicalEvidenceInspector
            brain={brain}
            kind="manifest_revision"
            id={search.revision}
            close={() => patch({ revision: null })}
          />
        )}
      </FeatureTabs>
    </>
  );
}
