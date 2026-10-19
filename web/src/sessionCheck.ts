import type { QueryClient } from "@tanstack/react-query";
import { session, type Session } from "./api";

// What the browser held is gone once the server stops accepting the session:
// drop every cached read of that session, keeping the session query itself.
async function dropSessionData(cache: QueryClient) {
  const other = (query: { queryKey: readonly unknown[] }) =>
    query.queryKey[0] !== "session";
  await cache.cancelQueries({ predicate: other });
  cache.removeQueries({ predicate: other });
}

// One /api/auth/me read decides what a session check means. It returns the
// current session, or null once the server no longer accepts it. When a session
// this browser held has ended (revoked, expired or its account disabled), the
// sign-in view explains it; an explicit sign-out clears the session first, so it
// never reaches here with a session to lose and shows no notice.
export function checkSession(cache: QueryClient, ended: () => void) {
  return async () => {
    const value = await session();
    if (!value && cache.getQueryData(["session"])) {
      await dropSessionData(cache);
      ended();
    }
    return value;
  };
}

// A 401 from any other request only says that request was not accepted; a stale
// in-flight call can fail after the session was replaced. Revalidate with /me
// and end the session only if /me also rejects it. Concurrent 401s share one
// check.
let revalidation: Promise<void> | null = null;
export function revalidateSession(
  cache: QueryClient,
  check: () => Promise<Session | null>,
) {
  revalidation ??= check()
    .then((value) => {
      cache.setQueryData(["session"], value);
    })
    .catch(() => {
      // /me itself failed (network or server error): the session is unknown,
      // so keep the user signed in; the next scheduled check tries again.
    })
    .finally(() => {
      revalidation = null;
    });
  return revalidation;
}
