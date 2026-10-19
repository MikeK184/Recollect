import {
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";

export type QualifiedRead<T> = {
  key: string;
  view?: T;
  error?: Error;
  pending: boolean;
};

/**
 * One qualified read per `key`: a key change aborts the previous request and
 * discards its response, a response is accepted only while its key is still the
 * latest rendered key, and `read` validates epoch/brain bindings by throwing.
 * The state is returned as `[read, setRead]` so a caller can reset or fail the
 * read after a command, exactly like the hand-written state it replaces.
 */
export function useQualifiedRead<T>({
  key,
  enabled = true,
  initialPending = false,
  fallback,
  read,
  onError,
}: {
  key: string;
  enabled?: boolean;
  initialPending?: boolean;
  fallback: string;
  read: (signal: AbortSignal) => Promise<T>;
  onError?: (error: Error) => void;
}): [QualifiedRead<T>, Dispatch<SetStateAction<QualifiedRead<T>>>] {
  const [state, setState] = useState<QualifiedRead<T>>({
    key: "",
    pending: initialPending,
  });
  const latest = useRef({ key, read, onError, fallback });
  latest.current = { key, read, onError, fallback };
  useEffect(() => {
    const controller = new AbortController();
    setState({ key, pending: true });
    if (!enabled) return () => controller.abort();
    const current = () =>
      !controller.signal.aborted && latest.current.key === key;
    latest.current
      .read(controller.signal)
      .then((view) => {
        if (current()) setState({ key, view, pending: false });
      })
      .catch((cause) => {
        if (!current()) return;
        const error =
          cause instanceof Error ? cause : new Error(latest.current.fallback);
        setState({ key, error, pending: false });
        latest.current.onError?.(error);
      });
    return () => controller.abort();
  }, [key, enabled]);
  return [state, setState];
}
