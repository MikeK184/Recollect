import { ClaimsPanel } from "../../ClaimsPanel";
import { HandoversPanel } from "../../HandoversPanel";
import { useBrain } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
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
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    tabs.some((t) => t.value === search.kind) ? search.kind! : "all",
  );
  return (
    <>
      <PageHeader
        title="Memory"
        description="What your Brain has learned, with evidence you can inspect and correct."
      />
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
