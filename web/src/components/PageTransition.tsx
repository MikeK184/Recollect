import { useEffect, useRef } from "react";
import { Outlet, useRouterState } from "@tanstack/react-router";
import { useAnimate, useReducedMotion } from "motion/react";

// The four Knowledge views share one surface; moving between them is not a
// page change. Graph keeps its own key because its layout is full-bleed.
const knowledge = new Set(["explore", "memory", "sources", "repositories"]);

function pageKey(path: string) {
  const parts = path.split("/");
  if (parts[1] !== "brains") return path;
  const section = parts[3] ?? "";
  return `brains/${parts[2]}/${knowledge.has(section) ? "knowledge" : section}`;
}

/**
 * Page content fades and rises 4px when the page changes; the sidebar and
 * topbar stay still. The route tree is never re-keyed, so Brain-scoped
 * caches, the Knowledge surface and its inspector survive navigation.
 */
export function PageTransition() {
  const key = useRouterState({
    select: (state) => pageKey(state.location.pathname),
  });
  const [scope, animate] = useAnimate<HTMLDivElement>();
  const reduced = useReducedMotion();
  const first = useRef(true);
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    const element = scope.current;
    if (reduced || !element) return;
    // Clear the transform afterwards: a lingering transform would become the
    // containing block for fixed-position page content (the ambient display).
    const reset = () => {
      element.style.transform = "";
      element.style.opacity = "";
    };
    const controls = animate(
      element,
      { opacity: [0, 1], transform: ["translateY(4px)", "translateY(0px)"] },
      { duration: 0.22, ease: [0.16, 1, 0.3, 1] },
    );
    void controls.then(reset);
    return () => {
      controls.stop();
      reset();
    };
  }, [key, animate, reduced, scope]);
  return (
    <div ref={scope} className="flex min-h-full flex-1 flex-col">
      <Outlet />
    </div>
  );
}
