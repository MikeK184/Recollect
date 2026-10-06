import { Outlet, useRouterState } from "@tanstack/react-router";

/** Page-content transition wrapper: every pathname change remounts the route
 * subtree so it enters with one fade+slide (rc-page-enter). */
export function PageTransition() {
  const path = useRouterState({ select: (state) => state.location.pathname });
  return (
    <div key={path} className="rc-page-enter">
      <Outlet />
    </div>
  );
}
