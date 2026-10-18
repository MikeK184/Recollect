import { useSyncExternalStore } from "react";

// One cadence policy for every polling query. Intervals are paused while the
// document is hidden; focus refetching resumes the read when it becomes visible.
export const pollIntervals = {
  // Job, pipeline and call status that a user is watching progress.
  fast: 3_000,
  // Catalogues, lists and details.
  normal: 10_000,
  // Session, service status and rarely changing policy reads.
  slow: 30_000,
} as const;

export type PollKind = keyof typeof pollIntervals;

function subscribe(notify: () => void) {
  document.addEventListener("visibilitychange", notify);
  return () => document.removeEventListener("visibilitychange", notify);
}
const hidden = () => document.hidden;

export function usePollInterval(
  kind: PollKind,
  enabled = true,
): number | false {
  const isHidden = useSyncExternalStore(subscribe, hidden, () => false);
  return enabled && !isHidden ? pollIntervals[kind] : false;
}
