import { GraphPanel } from "../../GraphPanel";
import { useBrain } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
export function GraphPage() {
  const brain = useBrain();
  return (
    <>
      <PageHeader
        title="Graph"
        description="Explore the connections between knowledge and its evidence."
      />
      <GraphPanel key={brain.id} brain={brain} />
    </>
  );
}
