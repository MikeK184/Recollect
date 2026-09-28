import { EvidencePanel } from "../../EvidencePanel";
import { useBrain } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
export function SourcesPage() {
  const brain = useBrain();
  return (
    <>
      <PageHeader
        title="Sources"
        description="The original evidence behind what your Brain knows."
      />
      <EvidencePanel key={brain.id} brain={brain} />
    </>
  );
}
