import { lazy, Suspense } from "react";
import { useBrain } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { LoadingState } from "../../components/AsyncState";

// Only the selected browser is mounted and fetched. These are distinct record
// catalogues, not an inferred cross-type search or a duplicated memory store.
// The Knowledge surface renders the Explore header and the view switcher; this
// route renders only the selected browser's body.
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
  const [search] = useBrainSearch();
  const view = search.view ?? "memory";
  return (
    <div className="flex min-w-0 flex-col gap-4" data-explore-view={view}>
      <Suspense fallback={<LoadingState label="Loading knowledge…" />}>
        {view === "memory" && <Memory key={brain.id} embedded />}
        {view === "sources" && <Sources key={brain.id} brain={brain} />}
        {view === "repositories" && <Repositories key={brain.id} embedded />}
      </Suspense>
    </div>
  );
}
