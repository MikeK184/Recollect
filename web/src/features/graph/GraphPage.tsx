import { useEffect } from "react";
import { GraphPanel } from "../../GraphPanel";
import { useBrain } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
import { useKnowledgeSelection } from "../knowledge/selection";
import { useBrainSearch } from "../../app/useBrainSearch";
import "./graph.css";

export function GraphPage() {
  const brain = useBrain();
  const { select } = useKnowledgeSelection();
  const [search] = useBrainSearch();
  useEffect(() => {
    select(
      search.claim
        ? { kind: "memory", id: search.claim }
        : search.source
          ? { kind: "source", id: search.source, version: search.version }
          : search.center?.startsWith("claim:")
            ? { kind: "graph-node", key: search.center }
            : null,
    );
  }, [search.claim, search.source, search.version, search.center, select]);
  useEffect(() => () => select(null), [select]);
  return (
    <>
      <PageHeader title="Graph" />
      <GraphPanel key={brain.id} brain={brain} />
    </>
  );
}
