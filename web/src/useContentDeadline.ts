import { useEffect, useState } from "react";

// A deadline is a display limit, not a promise that authority cannot change
// earlier. Canonical invalidation and fresh authorized reads remain necessary.
export function useContentDeadline(deadline?: string | null) {
  const [expired, setExpired] = useState<string | null>(null);
  useEffect(() => {
    if (!deadline) return;
    let timer: ReturnType<typeof setTimeout>;
    const check = () => {
      const remaining = Date.parse(deadline) - Date.now();
      if (!Number.isFinite(remaining) || remaining <= 0) setExpired(deadline);
      else timer = setTimeout(check, Math.min(remaining + 1, 2_147_483_647));
    };
    check();
    return () => clearTimeout(timer);
  }, [deadline]);
  return (
    !!deadline && (expired === deadline || Date.parse(deadline) <= Date.now())
  );
}
