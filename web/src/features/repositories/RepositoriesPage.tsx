import { useEffect } from "react";
import { RepositoryBrowser } from "./RepositoryBrowser";
import { PublicationPanel, SnapshotDialog } from "../../PublicationPanel";
import { useBrain } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { CanonicalEvidenceInspector } from "../../components/CanonicalEvidenceInspector";
import { useKnowledgeSelection } from "../knowledge/selection";
const tabs = [
  { value: "repositories", label: "Repositories" },
  { value: "environments", label: "Environments" },
] as const;
export function RepositoriesPage({ embedded = false }: { embedded?: boolean }) {
  const brain = useBrain();
  const [search, patch] = useBrainSearch();
  const { select } = useKnowledgeSelection();
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "repositories",
  );
  // Leaving this view clears its selection so an identifier is never carried
  // into a view that cannot resolve it.
  useEffect(() => () => select(null), [select]);
  useEffect(() => {
    if (tab !== "repositories") select(null);
  }, [tab, select]);
  return (
    <>
      {!embedded && <PageHeader title="Repositories" />}
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        {tab === "repositories" ? (
          <RepositoryBrowser key={brain.id} brain={brain} />
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
