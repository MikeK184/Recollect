import { useEffect, useState } from "react";
import { Button, Drawer } from "@mantine/core";
import { History } from "lucide-react";
import { ClaimsPanel } from "../../ClaimsPanel";
import { HandoversPanel } from "../../HandoversPanel";
import { useBrain } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { PageHeader } from "../../components/PageHeader";
import { useKnowledgeSelection } from "../knowledge/selection";
import "../knowledge/explore.css";
const kinds = ["claim", "decision", "procedure", "handover"];
export function MemoryPage({ embedded = false }: { embedded?: boolean }) {
  const brain = useBrain();
  const [search, patch] = useBrainSearch();
  const { select } = useKnowledgeSelection();
  const [handovers, setHandovers] = useState(false);
  // Older type-tab links resolve to the optional filter. Keep unrelated tabs
  // (for example repository environments) out of the memory query.
  useEffect(() => {
    if (search.tab && kinds.includes(search.tab))
      patch({ kind: search.kind ?? search.tab, tab: null });
  }, [search.tab, search.kind]);
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
      {!embedded && <PageHeader title="Memory" />}
      <ClaimsPanel key={brain.id} brain={brain} />
      <div className="explore-secondary-actions">
        <Button
          variant="subtle"
          size="xs"
          leftSection={<History size={14} />}
          onClick={() => setHandovers(true)}
        >
          Handover tools &amp; history
        </Button>
      </div>
      <Drawer
        opened={handovers}
        onClose={() => setHandovers(false)}
        title="Handover tools & history"
        closeButtonProps={{ "aria-label": "Close handover history" }}
        position="right"
        size="xl"
      >
        {handovers && <HandoversPanel key={brain.id} brain={brain} />}
      </Drawer>
    </>
  );
}
