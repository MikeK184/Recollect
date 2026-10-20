import { EvidencePanel } from "../../EvidencePanel";
import { useBrain } from "../../app/context";

/** The Sources body; the Knowledge surface renders the page frame, header
 * and view switcher around it. */
export function SourcesPage() {
  const brain = useBrain();
  return <EvidencePanel key={brain.id} brain={brain} />;
}
