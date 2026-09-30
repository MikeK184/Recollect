import { Text } from "@mantine/core";
import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import "../feature-views.css";
import {
  brainNavigation,
  defaultKnowledgeView,
  knowledgeSections,
  resolveKnowledgeView,
  type KnowledgeSection,
} from "../../app/navigation";
import { useBrain } from "../../app/context";

// The surface's view value is the contracted route suffix, so `/memory`,
// `/sources`, `/graph` and `/repositories` stay deep-linkable, reloadable and
// Back/forward safe while one shell hosts the switcher and the inspector.
export type KnowledgeView = KnowledgeSection;

export { defaultKnowledgeView, resolveKnowledgeView };

/** Switcher order and labels, sourced from the sidebar so they cannot drift. */
export const knowledgeViews: readonly { view: KnowledgeView; label: string }[] =
  knowledgeSections.map((view) => ({
    view,
    label: brainNavigation.find((entry) => entry.section === view)!.label,
  }));

/** The view the current route selects, resolved against the four known values. */
export function useKnowledgeView(): KnowledgeView {
  const path = useRouterState({ select: (state) => state.location.pathname });
  return resolveKnowledgeView(path.split("/")[3]);
}

/**
 * One shared lineage inspector region for every Knowledge view. The surface
 * renders it exactly once; the lineage inspector unit replaces its body.
 */
export function KnowledgeInspectorRegion({ view }: { view: KnowledgeView }) {
  return (
    <aside
      className="knowledge-inspector"
      aria-label="Lineage inspector"
      data-knowledge-view={view}
    >
      <Text size="sm" c="dimmed">
        The lineage inspector for this Brain appears here.
      </Text>
    </aside>
  );
}

/**
 * Pathless layout route component wrapping the four Knowledge routes. It stays
 * mounted across view switches, so the switcher and the inspector region are a
 * single instance while each view keeps its own URL and lazy bundle.
 */
export function KnowledgeSurface() {
  const brain = useBrain();
  const view = useKnowledgeView();
  return (
    <div className="knowledge-surface" data-knowledge-view={view}>
      <div
        className="knowledge-switcher"
        role="group"
        aria-label="Knowledge views"
      >
        {knowledgeViews.map((entry) => (
          <Link
            key={entry.view}
            to={`/brains/$brainId/${entry.view}`}
            params={{ brainId: brain.id }}
            search={{}}
            className={`knowledge-switcher-item ${
              view === entry.view ? "selected" : ""
            }`}
            aria-current={view === entry.view ? "page" : undefined}
          >
            {entry.label}
          </Link>
        ))}
      </div>
      <div className="knowledge-view-region">
        <Outlet />
      </div>
      <KnowledgeInspectorRegion view={view} />
    </div>
  );
}
