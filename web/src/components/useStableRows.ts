import { useRef, useState } from "react";

/** Preserve order while inspecting, retaining only IDs. Every rendered payload
 * comes from the latest read; removed or inaccessible rows disappear immediately. */
export function useStableRows<T>(
  rows: readonly T[] | undefined,
  scope: string,
  inspecting: boolean,
  identity: (row: T) => string,
) {
  const order = useRef<{ scope: string; ids: string[] }>({ scope, ids: [] });
  const [, update] = useState(0);
  if (
    order.current.scope !== scope ||
    !inspecting ||
    !rows ||
    !order.current.ids.length
  ) {
    order.current = { scope, ids: rows?.map(identity) ?? [] };
  }
  const current = new Map(rows?.map((row) => [identity(row), row]));
  const visible = order.current.ids.flatMap((id) => {
    const row = current.get(id);
    return row ? [row] : [];
  });
  const known = new Set(order.current.ids);
  const pending = rows?.filter((row) => !known.has(identity(row))).length ?? 0;
  return {
    rows: visible,
    pending,
    reveal: () => {
      order.current = { scope, ids: rows?.map(identity) ?? [] };
      update((value) => value + 1);
    },
  };
}
