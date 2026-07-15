import { useRef } from "react";

// Preserve a request key across network retries, and replace it for edited input
// or a new deliberate command. No payload hashing or client version gates.
export function useIdempotency() {
  const attempt = useRef({ key: crypto.randomUUID(), input: "" });
  return {
    forInput(input: unknown) {
      const encoded = JSON.stringify(input);
      if (encoded !== attempt.current.input)
        attempt.current = { key: crypto.randomUUID(), input: encoded };
      return attempt.current.key;
    },
    reset() {
      attempt.current = { key: crypto.randomUUID(), input: "" };
    },
  };
}
