import { createContext, useContext } from "react";
import type { Brain, Session } from "../api";

export const WorkspaceContext = createContext<Session | null>(null);
export const BrainContext = createContext<Brain | null>(null);
export function useWorkspace() {
  const session = useContext(WorkspaceContext);
  if (!session) throw new Error("Workspace session is unavailable");
  return session;
}
export function useBrain() {
  const brain = useContext(BrainContext);
  if (!brain) throw new Error("Brain context is unavailable");
  return brain;
}
