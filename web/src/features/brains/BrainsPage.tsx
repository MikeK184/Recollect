import { useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import {
  Archive,
  ArrowRight,
  Boxes,
  Check,
  Copy,
  Fingerprint,
  Plus,
} from "lucide-react";
import { client, result, type Brain } from "../../api";
import { useWorkspace } from "../../app/context";
import { BrainForm } from "../../components/BrainForm";
import { Page } from "../../components/Page";
import { PageHeader } from "../../components/PageHeader";
import { DeletionNotice } from "../settings/DeletionNotice";
import { BrainIcon } from "../../components/BrainIcon";
import { staggerStyle } from "../../components/Motion";
import { Skeleton } from "../../components/Skeleton";
import { EmptyState, ErrorState } from "../../components/AsyncState";
import {
  Badge,
  Button,
  CopyButton,
  SearchInput,
  SegmentedControl,
} from "../../components/ui";
import { cn } from "../../lib/utils";
import { usePollInterval } from "../../usePollInterval";

/**
 * The view filter is also the historical collapse: archived Brains are only
 * ever listed by their own view, so no active list can interleave history, and
 * the active list stays active-first.
 */
const views = [
  {
    value: "active",
    segment: "Active",
    label: "Active Brains",
    emptyTitle: "No active Brains",
    emptyDescription:
      "Every Brain you can access is archived. Archived Brains stay out of this list until you switch the view.",
  },
  {
    value: "mine",
    segment: "Owned",
    label: "Owned by you",
    emptyTitle: "No Brains owned by you",
    emptyDescription:
      "Brains you created appear here. Brains other people share with you stay in the shared view.",
  },
  {
    value: "shared",
    segment: "Shared",
    label: "Shared with you",
    emptyTitle: "No Brains shared with you",
    emptyDescription:
      "Only Brains created by other people appear here. Brains you own stay in the owned view.",
  },
  {
    value: "archived",
    segment: "Archived",
    label: "Archived Brains (history)",
    emptyTitle: "No archived Brains",
    emptyDescription:
      "Archived Brains are history and never mix into the active list. Nothing has been archived yet.",
  },
] as const;
type View = (typeof views)[number]["value"];

const createdDate = (value: string) =>
  new Date(value).toLocaleDateString(undefined, {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });

export function BrainsPage() {
  const pollNormal = usePollInterval("normal");
  const user = useWorkspace();
  const navigate = useNavigate();
  const cache = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [search, setSearch] = useState("");
  const [view, setView] = useState<View>("active");
  const query = useQuery({
    queryKey: ["brains"],
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/brains", { signal })),
  });
  // Cards present on the first successful read stagger in; Brains that
  // arrive with a later refetch appear without an entrance.
  const firstLoad = useRef<Set<string> | null>(null);
  if (!firstLoad.current && query.data)
    firstLoad.current = new Set(query.data.map((brain) => brain.id));
  const brains = query.isError ? [] : (query.data ?? []);
  const selected = views.find((v) => v.value === view) ?? views[0];
  const term = search.trim().toLocaleLowerCase();
  const searching = term.length > 0;
  const visible = brains.filter((b) => {
    const matches =
      !term || `${b.name} ${b.description}`.toLocaleLowerCase().includes(term);
    const inView =
      view === "archived"
        ? b.archived
        : view === "mine"
          ? !b.archived && b.owner_id === user.user.id
          : view === "shared"
            ? !b.archived && b.owner_id !== user.user.id
            : !b.archived;
    return matches && inView;
  });
  // Active records first; the stable sort keeps the authorized read's own order
  // inside each group.
  const ordered = [...visible].sort(
    (a, b) => Number(a.archived) - Number(b.archived),
  );
  const archivedHidden = brains.filter((b) => b.archived).length;
  const archivedCount = `${archivedHidden} archived ${archivedHidden === 1 ? "Brain" : "Brains"}`;
  // The filter states its own meaning, including what it currently collapses.
  const collapseNotice =
    view === "archived"
      ? archivedHidden
        ? `Reading history: ${archivedCount} listed here and absent from every active view.`
        : "Reading history: nothing has been archived yet."
      : archivedHidden
        ? `${archivedCount} hidden from this list; switch the view to read them.`
        : "Archived Brains stay collapsed behind this view.";
  // Genuinely empty, filtered-empty and no-results stay distinct.
  const empty = visible.length
    ? null
    : !brains.length
      ? {
          title: "Room for your first idea",
          description:
            "Create a Brain for your personal knowledge, a project, or your team.",
          action: (
            <Button onClick={() => setCreating(true)}>
              <Plus aria-hidden="true" />
              Create your first Brain
            </Button>
          ),
        }
      : searching
        ? {
            title: `No Brains match “${search.trim()}”`,
            description: `Search covers the Brain name and description in the ${selected.label.toLowerCase()} view. Clear the search or choose another view.`,
            action: (
              <Button variant="secondary" onClick={() => setSearch("")}>
                Clear search
              </Button>
            ),
          }
        : {
            title: selected.emptyTitle,
            description: selected.emptyDescription,
            action: (
              <Button variant="secondary" onClick={() => setView("active")}>
                Show active Brains
              </Button>
            ),
          };
  return (
    <Page className="text-body">
      <DeletionNotice />
      <PageHeader
        title="Your Brains"
        description="Every Brain you can open on this installation."
        actions={
          <Button onClick={() => setCreating(true)}>
            <Plus aria-hidden="true" />
            Create Brain
          </Button>
        }
      />
      <div className="-mt-1 flex flex-col gap-2">
        <div className="flex flex-wrap items-center gap-3">
          <div className="w-80">
            <SearchInput
              aria-label="Search Brains"
              placeholder="Find a Brain"
              value={search}
              onChange={(e) => setSearch(e.currentTarget.value)}
            />
          </div>
          <SegmentedControl
            label="Brain view"
            value={selected.value}
            onChange={setView}
            data={views.map(({ value, segment }) => ({
              value,
              label: segment,
            }))}
          />
          <span className="mono ml-auto text-label text-muted-foreground">
            {visible.length} {visible.length === 1 ? "Brain" : "Brains"}
          </span>
        </div>
        {/* The filter's own meaning, stated as text beside the list it governs. */}
        <p
          className="m-0 flex items-center gap-1.5 text-caption text-muted-foreground"
          role="status"
        >
          <Archive className="size-3.5" aria-hidden="true" />
          {collapseNotice}
        </p>
      </div>
      <ErrorState error={query.error} retry={() => void query.refetch()} />
      {query.isPending ? (
        <div
          className="grid grid-cols-3 gap-3"
          role="status"
          aria-label="Loading your Brains…"
        >
          {Array.from({ length: 6 }, (_, index) => (
            <div
              key={index}
              className="flex h-[118px] gap-3 rounded-lg border border-border bg-card px-4 py-3.5"
            >
              <Skeleton width={36} height={36} className="rounded-md" />
              <div className="flex flex-1 flex-col gap-2 pt-1">
                <Skeleton width={`${50 + ((index * 17) % 35)}%`} height={12} />
                <Skeleton width="80%" height={10} />
                <Skeleton width="40%" height={10} className="mt-auto" />
              </div>
            </div>
          ))}
        </div>
      ) : empty ? (
        <EmptyState
          icon={Boxes}
          title={empty.title}
          description={empty.description}
          action={empty.action}
        />
      ) : (
        <div className="grid grid-cols-3 gap-3">
          {ordered.map((brain, index) => (
            <BrainCard
              key={brain.id}
              brain={brain}
              owned={brain.owner_id === user.user.id}
              entrance={firstLoad.current?.has(brain.id) ? index : undefined}
            />
          ))}
        </div>
      )}
      <BrainForm
        opened={creating}
        close={() => setCreating(false)}
        saved={(brain) => {
          setCreating(false);
          void cache.invalidateQueries({ queryKey: ["brains"] });
          void navigate({
            to: "/brains/$brainId/dashboard",
            params: { brainId: brain.id },
            search: {},
          });
        }}
      />
    </Page>
  );
}

function BrainCard({
  brain,
  owned,
  entrance,
}: {
  brain: Brain;
  owned: boolean;
  /** Stagger index on first load; undefined for later arrivals. */
  entrance?: number;
}) {
  const role = owned
    ? "Owner"
    : brain.role.charAt(0).toLocaleUpperCase() + brain.role.slice(1);
  return (
    <div
      data-testid="brain-card"
      className={cn(
        "group relative flex gap-3 rounded-lg border border-border bg-card px-4 py-3.5 text-card-foreground transition-[border-color,box-shadow,transform] duration-fast ease-standard hover:-translate-y-px hover:border-border-strong hover:shadow-sm has-[details[open]]:z-10 has-[a:focus-visible]:border-ring has-[a:focus-visible]:shadow-focus",
        entrance !== undefined && "animate-enter",
      )}
      style={entrance !== undefined ? staggerStyle(entrance) : undefined}
    >
      <BrainIcon id={brain.id} revision={brain.icon_revision} size={36} />
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <div className="flex items-center gap-2">
          <h3 className="m-0 min-w-0 truncate text-body font-semibold text-foreground">
            <Link
              to="/brains/$brainId/dashboard"
              params={{ brainId: brain.id }}
              search={{}}
              aria-label={`Open ${brain.name}`}
              className="text-inherit no-underline outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
            >
              <span data-testid="brain-row-name">{brain.name}</span>
            </Link>
          </h3>
          <ArrowRight
            className="ml-auto size-4 text-muted-foreground transition-[color,transform] duration-fast ease-standard group-hover:translate-x-0.5 group-hover:text-foreground"
            aria-hidden="true"
          />
        </div>
        <p className="m-0 line-clamp-2 h-9 text-label text-muted-foreground">
          {brain.description || "No description"}
        </p>
        <div className="mt-1.5 flex items-center gap-2">
          <Badge variant="outline">
            {brain.archived && <Archive aria-hidden="true" />}
            {brain.archived ? "Archived · " : ""}
            {role}
            <span className="absolute size-px whitespace-nowrap [clip-path:inset(50%)]">
              {" "}
              access
            </span>
          </Badge>
          {/* The Brain id is a machine identifier: it stays out of the row
              and is offered as a copyable field instead. */}
          <details className="relative z-[1]">
            <summary
              title="Brain identifier"
              className="grid size-5 cursor-pointer list-none place-items-center rounded-xs text-muted-foreground transition-colors duration-fast hover:bg-accent hover:text-foreground [&::-webkit-details-marker]:hidden"
            >
              <Fingerprint className="size-3.5" aria-hidden="true" />
              <span className="absolute size-px whitespace-nowrap [clip-path:inset(50%)]">
                Brain identifier
              </span>
            </summary>
            <div className="absolute left-0 top-full z-10 mt-1 animate-pop rounded-md border border-border bg-popover p-1 shadow-md">
              <CopyButton value={brain.id}>
                {({ copied, copy }) => (
                  <Button size="sm" variant="ghost" onClick={copy}>
                    {copied ? (
                      <Check className="text-brand-text" aria-hidden="true" />
                    ) : (
                      <Copy aria-hidden="true" />
                    )}
                    {copied ? "Copied" : "Copy Brain ID"}
                  </Button>
                )}
              </CopyButton>
            </div>
          </details>
          <span className="ml-auto whitespace-nowrap text-caption text-muted-foreground">
            Created{" "}
            <span className="mono">{createdDate(brain.created_at)}</span>
          </span>
        </div>
      </div>
    </div>
  );
}
