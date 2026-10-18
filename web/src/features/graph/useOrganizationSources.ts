import { useEffect, useRef, useState } from "react";
import { client, result } from "../../api";
import type { components } from "../../api-schema";
import { useContentDeadline } from "../../useContentDeadline";

type View = components["schemas"]["OrganizationView"];
type Center = components["schemas"]["OrganizationCenter"];
type Cursor = components["schemas"]["OrganizationSourceCursor"];
type Page = components["schemas"]["OrganizationSourcesView"];

/** One independently paginated, bounded source page; never retain whole streams. */
export function useOrganizationSources({
  brain,
  center,
  view,
  qualificationKey,
  onClear,
  onFailure,
}: {
  brain: string;
  center: Center | null;
  view?: View;
  qualificationKey: string | null;
  onClear: () => void;
  onFailure: (error: Error) => void;
}) {
  const [read, setRead] = useState<{
    key: string | null;
    page?: Page;
    pending: boolean;
    cursor?: Cursor;
    history: (Cursor | null)[];
  }>({ key: null, pending: false, history: [] });
  const active = useRef(qualificationKey);
  active.current = qualificationKey;
  const request = useRef<AbortController | null>(null);
  const callbacks = useRef({ onClear, onFailure });
  callbacks.current = { onClear, onFailure };
  useEffect(() => {
    request.current?.abort();
    request.current = null;
    setRead({ key: qualificationKey, pending: false, history: [] });
    return () => request.current?.abort();
  }, [qualificationKey]);
  const current =
    read.key === qualificationKey && qualificationKey !== null
      ? read
      : undefined;
  const page = current?.page;
  const expired = useContentDeadline(page?.expires_at);
  useEffect(() => {
    if (!expired || !qualificationKey) return;
    request.current?.abort();
    setRead({ key: qualificationKey, pending: false, history: [] });
    callbacks.current.onFailure(
      new Error(
        "Source memberships reached their retention deadline. Read current organization.",
      ),
    );
  }, [expired, qualificationKey]);
  const enabled = !!view && !!qualificationKey && center?.kind !== "entity";
  const nextCursor = page ? page.next_cursor : view?.source_next_cursor;
  async function load(cursor: Cursor, history: (Cursor | null)[]) {
    if (!enabled || !qualificationKey || !view || current?.pending) return;
    const key = qualificationKey,
      epoch = view.epoch,
      canonicalScope = view.scope;
    const controller = new AbortController();
    request.current?.abort();
    request.current = controller;
    callbacks.current.onClear();
    setRead({ key, pending: true, history });
    try {
      const next = result(
        await client.POST("/api/brains/{brain}/organization/sources/view", {
          params: { path: { brain } },
          body: {
            scope: canonicalScope,
            topic_id: center?.kind === "topic" ? center.id : null,
            cursor,
          },
          signal: controller.signal,
        }),
      );
      if (controller.signal.aborted || active.current !== key) return;
      if (
        next.brain_id !== brain ||
        next.epoch !== epoch ||
        JSON.stringify(next.scope) !== JSON.stringify(canonicalScope)
      )
        throw new Error(
          "Source organization changed. Read current organization.",
        );
      setRead({ key, page: next, pending: false, cursor, history });
    } catch (error) {
      if (!controller.signal.aborted && active.current === key) {
        setRead({ key, pending: false, history: [] });
        callbacks.current.onFailure(
          error instanceof Error
            ? error
            : new Error("Source memberships could not be read."),
        );
      }
    }
  }
  return {
    page: expired ? undefined : page,
    pending: current?.pending ?? false,
    pageNumber: (current?.history.length ?? 0) + 1,
    hasNext: enabled && !!nextCursor,
    hasPrevious: enabled && !!current?.history.length,
    next: () =>
      nextCursor &&
      void load(nextCursor, [
        ...(current?.history ?? []),
        current?.cursor ?? null,
      ]),
    previous: () => {
      if (!current?.history.length) return;
      const previous = current.history.at(-1),
        history = current.history.slice(0, -1);
      if (previous) void load(previous, history);
      else {
        callbacks.current.onClear();
        setRead({ key: qualificationKey, pending: false, history: [] });
      }
    },
  };
}
