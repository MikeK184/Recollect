import { useEffect } from "react";
import { SegmentedControl } from "@mantine/core";
import { GraphPanel } from "../../GraphPanel";
import { GraphProvenance } from "../../GraphProvenance";
import { useBrain } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
import { useKnowledgeSelection } from "../knowledge/selection";
import { useBrainSearch } from "../../app/useBrainSearch";
import "./graph.css";

export function GraphPage() {
  const brain = useBrain();
  const { select } = useKnowledgeSelection();
  const [search, patch] = useBrainSearch();
  const perspective = search.perspective ?? "knowledge";
  useEffect(() => {
    select(
      perspective === "provenance"
        ? null
        : search.claim
          ? { kind: "memory", id: search.claim }
          : search.source
            ? { kind: "source", id: search.source, version: search.version }
            : search.center?.startsWith("claim:")
              ? { kind: "graph-node", key: search.center }
              : null,
    );
  }, [
    perspective,
    search.claim,
    search.source,
    search.version,
    search.center,
    select,
  ]);
  useEffect(() => () => select(null), [select]);
  return (
    <>
      <PageHeader
        title="Graph"
        actions={
          <SegmentedControl
            aria-label="Graph perspective"
            value={perspective}
            data={[
              { value: "knowledge", label: "Evidence" },
              { value: "provenance", label: "Contributions" },
            ]}
            onChange={(next) => {
              patch({
                perspective: next === "provenance" ? "provenance" : null,
                ...(next === "knowledge" && search.snapshot
                  ? {
                      kind: "repository",
                      repositories: search.repository
                        ? [search.repository]
                        : null,
                      areas: null,
                      area: null,
                      environment: null,
                      manifest: null,
                      collection: null,
                      knowledge: null,
                      fact: null,
                      relations: null,
                      center: null,
                      claim: null,
                      source: null,
                      version: null,
                      revision: null,
                      direction: null,
                      hops: null,
                      mode: "investigation",
                    }
                  : {}),
              });
              select(null);
            }}
          />
        }
      />
      {perspective === "knowledge" ? (
        <GraphPanel key={brain.id} brain={brain} />
      ) : (
        <GraphProvenance key={brain.id} brain={brain} />
      )}
    </>
  );
}
