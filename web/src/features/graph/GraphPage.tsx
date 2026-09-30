import { GraphPanel } from "../../GraphPanel";
import { useBrain } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
// Feature-local chrome styles; imported last so they win the cascade over the
// shared feature view defaults for this view only.
import "./graph.css";
export function GraphPage() {
  const brain = useBrain();
  return (
    <>
      <PageHeader title="Graph" />
      <GraphPanel key={brain.id} brain={brain} />
    </>
  );
}
