import { useEffect, useRef, useState } from "react";

/** Debounce a value: [debounced, cancel]. */
export function useDebouncedValue<T>(value: T, wait: number) {
  const [debounced, setDebounced] = useState(value);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => {
    timer.current = setTimeout(() => setDebounced(value), wait);
    return () => clearTimeout(timer.current);
  }, [value, wait]);
  return [debounced, () => clearTimeout(timer.current)] as const;
}

/** Track a media query, e.g. useMediaQuery("(min-width: 1440px)"). */
export function useMediaQuery(query: string, initial = false) {
  const [matches, setMatches] = useState(() =>
    typeof window === "undefined" ? initial : window.matchMedia(query).matches,
  );
  useEffect(() => {
    const media = window.matchMedia(query);
    const update = () => setMatches(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, [query]);
  return matches;
}

/** True from 1440px, where the inspector docks and the sidebar is full. */
export const wideQuery = "(min-width: 1440px)";
export const useWideLayout = () => useMediaQuery(wideQuery, true);

/**
 * Return focus to the element that was focused when `opened` became true,
 * once it becomes false again (for custom overlays outside Radix).
 */
export function useFocusReturn({ opened }: { opened: boolean }) {
  const last = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (opened) {
      last.current = document.activeElement as HTMLElement | null;
      return;
    }
    const target = last.current;
    if (target?.isConnected) target.focus({ preventScroll: true });
    last.current = null;
  }, [opened]);
}
