import { useRef, type MouseEvent } from "react";
import { BookMarked, History } from "lucide-react";
import { EmptyState } from "./components/AsyncState";
import { SkeletonRows } from "./components/Skeleton";
import { staggerStyle } from "./components/Motion";
import { StatusDot } from "./components/StatusDot";
import {
  Button,
  KindChip,
  StatusBadge,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableWrap,
} from "./components/ui";
import { cn } from "./lib/utils";
import { CompositeStatus, Pages, label, memoryKindChip } from "./claimsShared";
import type { ClaimsRouteState } from "./useClaimsRoute";

/** Two-letter contributor mark; the name always stays beside it. */
function Initials({ name }: { name: string }) {
  const letters =
    name
      .split(/[\s._@-]+/)
      .filter(Boolean)
      .map((part) => part[0])
      .join("")
      .slice(0, 2) || name.slice(0, 2);
  return (
    <span
      aria-hidden="true"
      className="grid size-6 shrink-0 place-items-center rounded-full bg-kind-claim-soft text-[11px] font-semibold uppercase leading-none text-kind-claim"
    >
      {letters.length === 1 ? name.slice(0, 2) : letters}
    </span>
  );
}

/**
 * Rows present when a list first loads enter with a short stagger; rows that
 * arrive later in the same list mark themselves as newly learned. A refetch
 * that returns the same rows never animates them again.
 */
function useRowEntrance(scope: string, keys: readonly string[] | undefined) {
  const memo = useRef({
    scope,
    loaded: false,
    rows: new Map<string, { className: string; index: number }>(),
  });
  if (memo.current.scope !== scope)
    memo.current = { scope, loaded: false, rows: new Map() };
  if (keys) {
    const current = memo.current;
    keys.forEach((key, index) => {
      if (!current.rows.has(key))
        current.rows.set(
          key,
          current.loaded
            ? { className: "animate-learned", index: 0 }
            : { className: "animate-enter", index },
        );
    });
    current.loaded = true;
  }
  return (key: string) => memo.current.rows.get(key);
}

const ignoreInner = (event: MouseEvent) =>
  !!(event.target as HTMLElement).closest("button, a, input, summary");

// The loading skeleton, empty state, record rows and pager for the memory list.
export function ClaimsList({
  state,
  onOpenHandovers,
}: {
  state: ClaimsRouteState;
  onOpenHandovers?: () => void;
}) {
  const {
    claims,
    data,
    query,
    filtered,
    stable,
    selected,
    setSelected,
    offset,
    setOffset,
    memoryKind,
    mode,
    environment,
    repository,
    times,
  } = state;
  const entrance = useRowEntrance(
    JSON.stringify([
      memoryKind,
      mode,
      environment,
      repository,
      times,
      query,
      offset,
    ]),
    data ? stable.rows.map((view) => view.revision.claim_id) : undefined,
  );
  const accepted = stable.rows.filter(
    (view) => view.revision.review === "accepted",
  ).length;
  const proposed = stable.rows.filter(
    (view) => view.revision.review === "proposed",
  ).length;
  const unavailable = data?.unavailable ?? [];
  return (
    <>
      {claims.isPending && (
        <div className="rounded-lg border border-border bg-card px-3">
          <SkeletonRows label="Loading memory…" />
        </div>
      )}
      {data && (
        <>
          {data.items.length === 0 && !unavailable.length && (
            <div className="rounded-lg border border-dashed border-border">
              <EmptyState
                icon={BookMarked}
                title={
                  query || filtered
                    ? "No matching memory"
                    : "Your knowledge will gather here"
                }
                description={
                  query || filtered
                    ? "Try another phrase or adjust the filters. Historical and qualified records can be inspected separately."
                    : "Recollect learns from permitted sources and agent activity. You can also add an evidence-backed memory yourself."
                }
              />
            </div>
          )}
          {stable.pending > 0 && (
            <div>
              <Button variant="secondary" size="sm" onClick={stable.reveal}>
                <StatusDot tone="accent" size={6} />
                <span className="mono">{stable.pending}</span> new records ·
                Show updates
              </Button>
            </div>
          )}
          {(!!stable.rows.length || !!unavailable.length) && (
            <TableWrap>
              <Table>
                <TableHead>
                  <tr>
                    <TableHeader scope="col">Memory</TableHeader>
                    <TableHeader scope="col" className="w-px">
                      Kind
                    </TableHeader>
                    <TableHeader scope="col" className="w-px">
                      Status
                    </TableHeader>
                    <TableHeader scope="col" className="w-px">
                      Contributor
                    </TableHeader>
                    <TableHeader scope="col" className="w-px text-right">
                      Updated
                    </TableHeader>
                  </tr>
                </TableHead>
                <TableBody>
                  {stable.rows.map((view) => {
                    const id = view.revision.claim_id;
                    const content = view.revision.content;
                    const chip = memoryKindChip(content.kind);
                    // Handover and procedure records repeat their kind as the
                    // predicate; the kind chip already says it.
                    const predicate = label(content.predicate);
                    const redundant =
                      predicate.toLowerCase() === content.kind.toLowerCase();
                    const enter = entrance(id);
                    const isSelected = selected === id;
                    return (
                      <TableRow
                        key={id}
                        data-claim={id}
                        selected={isSelected}
                        className={cn(
                          "memory-record cursor-pointer",
                          enter?.className,
                          isSelected && "[&>td]:bg-brand-soft/55",
                        )}
                        style={
                          enter?.className === "animate-enter"
                            ? staggerStyle(enter.index)
                            : undefined
                        }
                        onClick={(event) => {
                          if (!ignoreInner(event)) setSelected(id);
                        }}
                      >
                        {/* Dense row: one primary identifier line plus one
                            composite status mark. Evidence, time and every
                            dimension move to the shared lineage inspector. */}
                        <TableCell primary className="w-full max-w-0">
                          {/* The row button names the whole assertion
                              ("subject · predicate value"), as before. */}
                          <button
                            type="button"
                            className="memory-record-title group/title flex w-full min-w-0 items-baseline gap-1.5 rounded-xs text-left outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                            onClick={() => setSelected(id)}
                          >
                            <span className="max-w-[70%] shrink-0 truncate font-medium text-foreground group-hover/title:underline group-hover/title:underline-offset-[3px]">
                              {redundant
                                ? content.subject
                                : `${content.subject} · ${predicate}`}
                            </span>{" "}
                            <span className="min-w-0 flex-1 truncate font-normal text-foreground-secondary">
                              {content.value}
                            </span>
                          </button>
                        </TableCell>
                        <TableCell className="whitespace-nowrap">
                          <KindChip kind={chip.kind}>{chip.label}</KindChip>
                        </TableCell>
                        <TableCell className="max-w-[220px] whitespace-nowrap">
                          <CompositeStatus view={view} />
                        </TableCell>
                        <TableCell className="whitespace-nowrap">
                          <span className="flex items-center gap-2">
                            <Initials name={view.revision.actor_name} />
                            <span className="max-w-[160px] truncate">
                              {view.revision.actor_name}
                            </span>
                          </span>
                        </TableCell>
                        <TableCell num className="whitespace-nowrap">
                          <time dateTime={view.revision.recorded_at}>
                            {new Date(
                              view.revision.recorded_at,
                            ).toLocaleDateString()}
                          </time>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                  {unavailable.map((item) => (
                    <TableRow
                      key={item.claim_id}
                      data-claim={item.claim_id}
                      selected={selected === item.claim_id}
                      className={cn(
                        "cursor-pointer",
                        selected === item.claim_id && "[&>td]:bg-brand-soft/55",
                      )}
                      onClick={(event) => {
                        if (!ignoreInner(event)) setSelected(item.claim_id);
                      }}
                    >
                      <TableCell primary className="w-full max-w-0">
                        <div className="flex min-w-0 items-baseline gap-1.5">
                          <button
                            type="button"
                            className="shrink-0 rounded-xs text-left font-medium text-foreground outline-none hover:underline hover:underline-offset-[3px] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                            onClick={() => setSelected(item.claim_id)}
                          >
                            Unavailable memory ·{" "}
                            <span className="mono text-caption">
                              {item.claim_id.slice(0, 8)}
                            </span>
                          </button>
                          <span className="min-w-0 flex-1 truncate font-normal text-muted-foreground">
                            Content is unavailable. Inspect its remaining
                            history and erased intervals.
                          </span>
                        </div>
                      </TableCell>
                      <TableCell className="text-muted-foreground">—</TableCell>
                      <TableCell className="whitespace-nowrap">
                        <StatusBadge state="attention">Unavailable</StatusBadge>
                      </TableCell>
                      <TableCell className="text-muted-foreground">—</TableCell>
                      <TableCell num>—</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableWrap>
          )}
          {(!!stable.rows.length || onOpenHandovers) && (
            <div className="flex flex-wrap items-center gap-3">
              {!!stable.rows.length && (
                <p className="m-0 text-caption text-muted-foreground">
                  <span className="mono">{stable.rows.length}</span>{" "}
                  {stable.rows.length === 1 ? "memory" : "memories"}
                  {offset > 0 || data.next_offset != null
                    ? " on this page"
                    : ""}{" "}
                  · <span className="mono">{accepted}</span> accepted ·{" "}
                  <span className="mono">{proposed}</span> proposed
                </p>
              )}
              {onOpenHandovers && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="ml-auto"
                  onClick={onOpenHandovers}
                >
                  <History aria-hidden="true" />
                  Handover tools &amp; history
                </Button>
              )}
            </div>
          )}
          <Pages
            offset={offset}
            total={data.total_candidates}
            nextOffset={data.next_offset}
            onChange={setOffset}
          />
        </>
      )}
    </>
  );
}
