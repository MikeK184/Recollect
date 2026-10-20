import { useRef, type CSSProperties } from "react";
import { staggerStyle } from "../../components/Motion";

/**
 * Lists stagger in on their first load only (motion.md). Rows present in the
 * first loaded page get `animate-enter` with a 30ms step; rows that arrive on
 * a later refetch render still, so polling never replays the entrance.
 */
export function useFirstLoadStagger(keys: readonly string[] | undefined) {
  const first = useRef<Set<string> | null>(null);
  if (keys && !first.current) first.current = new Set(keys);
  return (key: string, index: number): { className: string; style?: CSSProperties } =>
    first.current?.has(key)
      ? { className: "animate-enter", style: staggerStyle(index) }
      : { className: "" };
}
