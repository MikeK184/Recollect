import { useEffect } from "react";
import { ClaimsPanel } from "../../ClaimsPanel";
import { HandoversPanel } from "../../HandoversPanel";
import { useBrain } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { useKnowledgeSelection } from "../knowledge/selection";
const tabs = [
  { value: "all", label: "All memory" },
  { value: "claim", label: "Claims" },
  { value: "decision", label: "Decisions" },
  { value: "procedure", label: "Procedures" },
  { value: "handover", label: "Handovers" },
] as const;
export function MemoryPage() {
  const brain = useBrain();
  const [search] = useBrainSearch();
  const { select } = useKnowledgeSelection();
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    tabs.some((t) => t.value === search.kind) ? search.kind! : "all",
  );
  // Selection lives in validated URL state, including browser Back/Forward.
  useEffect(() => {
    select(
      search.claim
        ? { kind: "memory", id: search.claim }
        : search.center?.startsWith("claim:")
          ? { kind: "graph-node", key: search.center }
          : null,
    );
  }, [search.claim, search.center, select]);
  // Leaving this view clears its selection so an identifier is never carried
  // into a view that cannot resolve it.
  useEffect(() => () => select(null), [select]);
  return (
    <>
      <PageHeader title="Memory" />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        <ClaimsPanel
          key={`${brain.id}-${tab}`}
          brain={brain}
          kind={tab === "all" ? null : tab}
        />
        {tab === "handover" && <HandoversPanel brain={brain} />}
      </FeatureTabs>
    </>
  );
}
