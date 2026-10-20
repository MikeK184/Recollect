import { useCallback, useEffect, useSyncExternalStore } from "react";

// Light and Ink (ADR 0025). The preference is a non-sensitive browser setting:
// "system" follows prefers-color-scheme; public/theme-init.js applies it before
// first paint and this module keeps <html data-theme> current afterwards.
export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

const storageKey = "recollect-theme";
const query = "(prefers-color-scheme: dark)";
const listeners = new Set<() => void>();

function readPreference(): ThemePreference {
  try {
    const stored = window.localStorage.getItem(storageKey);
    return stored === "light" || stored === "dark" ? stored : "system";
  } catch {
    return "system";
  }
}
function systemTheme(): ResolvedTheme {
  return window.matchMedia?.(query).matches ? "dark" : "light";
}
function resolve(preference: ThemePreference): ResolvedTheme {
  return preference === "system" ? systemTheme() : preference;
}

let preference: ThemePreference =
  typeof window === "undefined" ? "system" : readPreference();

function apply() {
  const resolved = resolve(preference);
  const root = document.documentElement;
  if (root.dataset.theme !== resolved) root.dataset.theme = resolved;
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute("content", resolved === "dark" ? "#0b1018" : "#f4f6f8");
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function setThemePreference(next: ThemePreference) {
  preference = next;
  try {
    if (next === "system") window.localStorage.removeItem(storageKey);
    else window.localStorage.setItem(storageKey, next);
  } catch {
    /* Storage may be unavailable; the choice still applies to this page. */
  }
  apply();
}

/** Keep the attribute in step with the system setting and other tabs. */
export function useThemeSync() {
  useEffect(() => {
    const media = window.matchMedia?.(query);
    const onSystem = () => {
      if (preference === "system") apply();
    };
    const onStorage = (event: StorageEvent) => {
      if (event.key !== storageKey) return;
      preference = readPreference();
      apply();
    };
    media?.addEventListener("change", onSystem);
    window.addEventListener("storage", onStorage);
    apply();
    return () => {
      media?.removeEventListener("change", onSystem);
      window.removeEventListener("storage", onStorage);
    };
  }, []);
}

/** The person's preference and the theme currently applied. */
export function useThemeMode() {
  const snapshot = useSyncExternalStore(
    subscribe,
    () => `${preference}:${resolve(preference)}`,
    () => "system:light",
  );
  const [pref, resolved] = snapshot.split(":") as [
    ThemePreference,
    ResolvedTheme,
  ];
  const setPreference = useCallback(
    (next: ThemePreference) => setThemePreference(next),
    [],
  );
  return { preference: pref, resolved, setPreference };
}
