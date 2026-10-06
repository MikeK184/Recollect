import { lazy, Suspense } from "react";
import { SegmentedControl } from "@mantine/core";
import { useBrain } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { PageHeader } from "../../components/PageHeader";
import { LoadingState } from "../../components/AsyncState";
import "./explore.css";

// Only the selected browser is mounted and fetched. These are distinct record
// catalogues, not an inferred cross-type search or a duplicated memory store.
const Memory = lazy(() =>
  import("../memory/MemoryPage").then((module) => ({
    default: module.MemoryPage,
  })),
);
const Sources = lazy(() =>
  import("../../EvidencePanel").then((module) => ({
    default: module.EvidencePanel,
  })),
);
const Repositories = lazy(() =>
  import("../repositories/RepositoriesPage").then((module) => ({
    default: module.RepositoriesPage,
  })),
);

export function ExplorePage() {
  const brain = useBrain();
  const [search, patch] = useBrainSearch();
  const view = search.view ?? "memory";
  return (
    <section className="explore-workspace" data-explore-view={view}>
      <PageHeader title="Explore" />
      <div className="explore-view-switch">
        <SegmentedControl
          aria-label="Explore browsing scope"
          value={view}
          data={[
            { value: "memory", label: "Memory" },
            { value: "sources", label: "Sources" },
            { value: "repositories", label: "Repositories" },
          ]}
          onChange={(next) => {
            if (
              next !== "memory" &&
              next !== "sources" &&
              next !== "repositories"
            )
              return;
            patch({
              view: next,
              claim: null,
              source: null,
              version: null,
              repository: null,
              snapshot: null,
              revision: null,
              detail: null,
              center: null,
              tab: null,
              kind: null,
              mode: null,
              environment: null,
              knowledge: null,
              fact: null,
            });
          }}
        />
      </div>
      <div className="explore-browser-region">
        <Suspense fallback={<LoadingState label="Loading knowledge…" />}>
          {view === "memory" && <Memory key={brain.id} embedded />}
          {view === "sources" && <Sources key={brain.id} brain={brain} />}
          {view === "repositories" && <Repositories key={brain.id} embedded />}
        </Suspense>
      </div>
    </section>
  );
}
