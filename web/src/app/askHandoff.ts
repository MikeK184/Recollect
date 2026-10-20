// ⌘K → Ask hand-off. The typed question travels in memory, never in the URL
// (desktop contract: questions never enter URLs). The palette stores it and
// navigates to the Brain's Ask page, which takes it once and submits it as an
// explicit Ask request. A hand-off expires after a minute or on Brain change.
type Pending = { brainId: string; question: string; at: number };
let pending: Pending | null = null;
const listeners = new Set<() => void>();

export function handOffToAsk(brainId: string, question: string) {
  const trimmed = question.trim();
  if (!trimmed) return;
  pending = { brainId, question: trimmed, at: Date.now() };
  for (const listener of listeners) listener();
}

/** Take (and clear) a pending question for this Brain, if one is fresh. */
export function takeAskHandoff(brainId: string): string | null {
  const value = pending;
  if (!value || value.brainId !== brainId || Date.now() - value.at > 60_000)
    return null;
  pending = null;
  return value.question;
}

/** Notify an already-mounted Ask page that a new hand-off arrived. */
export function subscribeAskHandoff(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
