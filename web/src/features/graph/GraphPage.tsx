import { useEffect } from "react";
import { GraphPanel } from "../../GraphPanel";
import { GraphProvenance } from "../../GraphProvenance";
import { useBrain } from "../../app/context";
import { Page } from "../../components/Page";
import { SegmentedControl } from "../../components/ui";
import { cn } from "../../lib/utils";
import { useKnowledgeSelection } from "../knowledge/selection";
import { useBrainSearch } from "../../app/useBrainSearch";
import { OrganizationPanel } from "./OrganizationPanel";
import { organizationScope } from "./organization-scope";
import {
  ChromeSlot,
  GraphDockContext,
  GraphStage,
  GraphZoneProvider,
  chromeCard,
  useGraphStage,
  useZoneRefs,
} from "./GraphStage";

type Perspective = "organization" | "knowledge" | "provenance";

// The panel below the 52px topbar inside the 8px-inset main panel. Used only
// as a floor so the canvas fills the panel even before its surface stretches.
const panelHeight = "min-h-[calc(100dvh-70px)]";

/**
 * The page header card floating on the canvas: the h1, the perspective
 * switch, and zones where each perspective adds its status, tools and scope.
 */
function GraphHeader({
  perspective,
  onPerspective,
}: {
  perspective: Perspective;
  onPerspective: (next: Perspective) => void;
}) {
  const refs = useZoneRefs(useGraphStage()?.register);
  return (
    <div
      className={cn(chromeCard, "grid w-max max-w-full gap-2.5 p-3")}
      data-graph-chrome=""
    >
      <div className="flex min-w-0 flex-wrap items-center gap-x-2.5 gap-y-1.5">
        <h1 className="m-0 text-title font-semibold tracking-[-0.011em] text-foreground">
          Graph
        </h1>
        <div
          ref={refs["header-meta"]}
          className="contents"
        />
      </div>
      <div className="flex min-w-0 max-w-[620px] flex-wrap items-center gap-1.5">
        <SegmentedControl
          size="sm"
          label="Graph perspective"
          value={perspective}
          onChange={onPerspective}
          data={[
            { value: "organization", label: "Topics" },
            { value: "knowledge", label: "Evidence" },
            { value: "provenance", label: "Contributions" },
          ]}
        />
        <div
          ref={refs["header-tools"]}
          className="contents"
        />
      </div>
      <div
        ref={refs["header-extra"]}
        className="flex min-w-0 max-w-[620px] flex-wrap items-center gap-1.5 empty:hidden"
      />
    </div>
  );
}

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
  const perspective: Perspective =
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
  function changePerspective(next: Perspective) {
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
            repositories: search.repository ? [search.repository] : null,
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
  }
  const header = (
    <GraphHeader perspective={perspective} onPerspective={changePerspective} />
  );
  return (
    <GraphDockContext.Provider value={true}>
      <Page width="full" className={panelHeight}>
        {perspective === "organization" ? (
          // Topics is a card overview that flows as a document; its header
          // card sits where the floating canvas header sits in the other
          // perspectives, so switching never moves the perspective control.
          <GraphZoneProvider>
            <div className="flex min-h-0 flex-1 flex-col gap-5 bg-background p-3 pb-10">
              <div className="flex">{header}</div>
              <div className="w-full max-w-[1600px]">
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
              </div>
            </div>
          </GraphZoneProvider>
        ) : (
          <GraphStage header>
            <ChromeSlot zone="top-left" order={0}>
              {header}
            </ChromeSlot>
            {perspective === "knowledge" ? (
              <GraphPanel key={brain.id} brain={brain} />
            ) : (
              <GraphProvenance key={brain.id} brain={brain} />
            )}
          </GraphStage>
        )}
      </Page>
    </GraphDockContext.Provider>
  );
}
