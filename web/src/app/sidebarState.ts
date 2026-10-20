import { useCallback, useSyncExternalStore } from "react";
import { useWideLayout } from "../lib/hooks";

// The sidebar is 240px, or a 56px rail. Below 1440px it collapses to the rail
// unless the person expanded it; an explicit choice is remembered.
type Preference = "auto" | "full" | "rail";
const key = "recollect-sidebar";
const listeners = new Set<() => void>();
let preference: Preference = read();

function read(): Preference {
  try {
    const value = window.localStorage.getItem(key);
    return value === "full" || value === "rail" ? value : "auto";
  } catch {
    return "auto";
  }
}

export function useSidebar() {
  const wide = useWideLayout();
  const pref = useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => preference,
    () => "auto" as Preference,
  );
  const rail = pref === "rail" || (pref === "auto" && !wide);
  const toggle = useCallback(() => {
    preference = rail ? "full" : "rail";
    try {
      window.localStorage.setItem(key, preference);
    } catch {
      /* The choice still applies to this page. */
    }
    for (const listener of listeners) listener();
  }, [rail]);
  return { rail, toggle };
}
