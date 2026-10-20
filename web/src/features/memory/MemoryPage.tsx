import { useEffect, useState } from "react";
import { ClaimsPanel } from "../../ClaimsPanel";
import { HandoversPanel } from "../../HandoversPanel";
import { useBrain } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "../../components/ui";
import { useKnowledgeSelection } from "../knowledge/selection";
const kinds = ["claim", "decision", "procedure", "handover"];

/**
 * The Memory browser. The Knowledge surface supplies the Explore header and
 * view switcher, so this page renders only its toolbar, table and secondary
 * handover tools. `embedded` is kept for the Explore route's call site.
 */
export function MemoryPage(_props: { embedded?: boolean }) {
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
      <ClaimsPanel
        key={brain.id}
        brain={brain}
        onOpenHandovers={() => setHandovers(true)}
      />
      {/* Handover tools are secondary tooling, not record inspection: they
          open in a side sheet rather than the docked inspector. */}
      <Sheet open={handovers} onOpenChange={setHandovers}>
        <SheetContent width={640} closeLabel="Close handover history">
          <SheetHeader>
            <SheetTitle>Handover tools &amp; history</SheetTitle>
            <SheetDescription>
              Generated handovers and their contributing memory.
            </SheetDescription>
          </SheetHeader>
          <SheetBody>
            {handovers && (
              <HandoversPanel
                key={brain.id}
                brain={brain}
                onInspect={(claim) => {
                  // The record opens in the page's docked inspector, which a
                  // modal sheet would cover, so the tools close first.
                  setHandovers(false);
                  patch({ claim, revision: null, detail: "record" });
                }}
              />
            )}
          </SheetBody>
        </SheetContent>
      </Sheet>
    </>
  );
}
