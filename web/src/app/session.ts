import type { QueryClient } from "@tanstack/react-query";
import { setCsrf, type Session } from "../api";

/** Swap the signed-in session, dropping every protected cached payload. */
export async function replaceSession(cache: QueryClient, value: Session | null) {
  await cache.cancelQueries();
  cache.removeQueries({
    predicate: (query) => query.queryKey[0] !== "session",
  });
  setCsrf(value?.csrf_token ?? "");
  cache.setQueryData(["session"], value);
}
