import { useEffect } from "react";
import { SegmentedControl } from "@mantine/core";
import { GraphPanel } from "../../GraphPanel";
import { GraphProvenance } from "../../GraphProvenance";
import { useBrain } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
import { useKnowledgeSelection } from "../knowledge/selection";
import { useBrainSearch } from "../../app/useBrainSearch";
import { OrganizationPanel } from "./OrganizationPanel";
import { organizationScope } from "./organization-scope";
import "./graph.css";

export function GraphPage() {
  const brain = useBrain();
  const { select } = useKnowledgeSelection();
  const [search, patch] = useBrainSearch();
  const evidenceLink = !!(
    search.claim ||
    search.source ||
    search.center ||
    search.snapshot ||
    search.manifest ||
    search.kind === "repository" ||
    search.kind === "combined"
  );
  const perspective =
    search.perspective === "provenance"
      ? "provenance"
      : search.tab === "evidence" ||
          (search.tab !== "organization" && evidenceLink)
        ? "knowledge"
        : "organization";
  const scope = organizationScope(search);
  useEffect(() => {
    select(
      perspective !== "knowledge"
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
              { value: "organization", label: "Topics" },
              { value: "knowledge", label: "Evidence" },
              { value: "provenance", label: "Contributions" },
            ]}
            onChange={(next) => {
              patch({
                perspective: next === "provenance" ? "provenance" : null,
                tab:
                  next === "knowledge"
                    ? "evidence"
                    : next === "organization"
                      ? "organization"
                      : null,
                ...(next === "organization"
                  ? {
                      center: null,
                      claim: null,
                      source: null,
                      version: null,
                      revision: null,
                    }
                  : {}),
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
      {perspective === "organization" ? (
        <OrganizationPanel
          key={`${brain.id}:${JSON.stringify(scope)}`}
          brain={brain}
          scope={scope}
          onScopeChange={(next) =>
            patch({
              kind: "knowledge",
              repositories: next.selection.repository_ids,
              repository: null,
              areas: next.selection.area_ids,
              area: null,
              environment: next.selection.environment_id,
              collection: next.collection_id,
              manifest: next.manifest_revision_id,
              fact: next.fact_at,
              mode: next.mode,
              center: null,
              claim: null,
              source: null,
              version: null,
              revision: null,
              tab: "organization",
            })
          }
        />
      ) : perspective === "knowledge" ? (
        <GraphPanel key={brain.id} brain={brain} />
      ) : (
        <GraphProvenance key={brain.id} brain={brain} />
      )}
    </>
  );
}
