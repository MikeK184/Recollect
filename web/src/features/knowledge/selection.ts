import { createContext, useContext, useEffect, type ReactNode } from "react";

/**
 * The Knowledge surface's shared selection. One value drives the single
 * lineage inspector region for every view; views never keep private copies.
 * Identifiers are opaque server IDs or server-generated graph keys, so a
 * selection is only meaningful in the view that can resolve it.
 */
export type KnowledgeSelection =
  | { kind: "memory"; id: string }
  | { kind: "source"; id: string; version?: string }
  | { kind: "graph-node"; key: string; claimId?: string; revision?: string }
  | {
      kind: "graph-edge";
      relation: string;
      evidenceKind: string;
      evidenceId: string;
    }
  | { kind: "repository"; id: string }
  | null;

export interface KnowledgeSelectionContextValue {
  available: boolean;
  selection: KnowledgeSelection;
  select: (selection: KnowledgeSelection, actions?: ReactNode) => void;
  actions?: ReactNode;
  nestedOpen: boolean;
  setNestedOpen: (opened: boolean) => void;
}

/** Defaults to "nothing selected" so views outside the surface stay inert. */
export const KnowledgeSelectionContext =
  createContext<KnowledgeSelectionContextValue>({
    available: false,
    selection: null,
    select: () => {},
    nestedOpen: false,
    setNestedOpen: () => {},
  });

export function useKnowledgeSelection() {
  return useContext(KnowledgeSelectionContext);
}

/** Suspend the parent focus trap and Escape handler while a child dialog opens. */
export function useLineageOverlay(opened: boolean) {
  const { setNestedOpen } = useKnowledgeSelection();
  useEffect(() => {
    if (!opened) return;
    setNestedOpen(true);
    return () => setNestedOpen(false);
  }, [opened, setNestedOpen]);
}
