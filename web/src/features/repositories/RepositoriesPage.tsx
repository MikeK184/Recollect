import { useEffect, useState } from "react";
import { Tabs as TabsPrimitive } from "radix-ui";
import { RepositoryBrowser } from "./RepositoryBrowser";
import { SegmentedTabsList } from "./SegmentedTabs";
import { PublicationPanel, SnapshotDialog } from "../../PublicationPanel";
import { useBrain } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { useFeatureTab } from "../../components/FeatureTabs";
import { CanonicalEvidenceInspector } from "../../components/CanonicalEvidenceInspector";
import { useKnowledgeSelection } from "../knowledge/selection";
const tabs = [
  { value: "repositories", label: "Repositories" },
  { value: "environments", label: "Environments" },
] as const;
/**
 * The Repositories body. The Knowledge surface renders the page frame, header
 * and view switcher around it, both on the Repositories route and inside
 * Explore (`embedded`, kept for call-site compatibility).
 */
export function RepositoriesPage(_props: { embedded?: boolean }) {
  const brain = useBrain();
  const [search, patch] = useBrainSearch();
  const { select } = useKnowledgeSelection();
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "repositories",
  );
  // The active panel renders its search, count and actions into the switch
  // row, so the segmented tabs stay mounted (and focused) across switches.
  const [toolbar, setToolbar] = useState<HTMLElement | null>(null);
  // Leaving this view clears its selection so an identifier is never carried
  // into a view that cannot resolve it.
  useEffect(() => () => select(null), [select]);
  useEffect(() => {
    if (tab !== "repositories") select(null);
  }, [tab, select]);
  return (
    <TabsPrimitive.Root
      value={tab}
      onValueChange={setTab}
      className="flex flex-col gap-4"
    >
      <div className="flex items-center gap-2">
        <SegmentedTabsList tabs={tabs} value={tab} label="Repository view" />
        <div
          ref={setToolbar}
          className="flex min-w-0 flex-1 items-center gap-2"
        />
      </div>
      <TabsPrimitive.Content
        value="repositories"
        className="outline-none data-[state=active]:animate-enter"
      >
        <RepositoryBrowser key={brain.id} brain={brain} toolbar={toolbar} />
      </TabsPrimitive.Content>
      <TabsPrimitive.Content
        value="environments"
        className="outline-none data-[state=active]:animate-enter"
      >
        <PublicationPanel
          brain={brain}
          section="environments"
          toolbar={toolbar}
        />
      </TabsPrimitive.Content>
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
    </TabsPrimitive.Root>
  );
}
