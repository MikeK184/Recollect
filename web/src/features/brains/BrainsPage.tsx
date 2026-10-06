import { useState } from "react";
import {
  Button,
  CopyButton,
  SegmentedControl,
  Text,
  TextInput,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { ArrowRight, Boxes, Check, Copy, Plus, Search } from "lucide-react";
import { client, result } from "../../api";
import { useWorkspace } from "../../app/context";
import { BrainForm } from "../../components/BrainForm";
import { PageHeader } from "../../components/PageHeader";
import { DeletionNotice } from "../settings/DeletionNotice";
import { BrainIcon } from "../../components/BrainIcon";
import { motion, useReducedMotion } from "motion/react";
import "../workspace/control-panel.css";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";

/**
 * The view filter is also the historical collapse: archived Brains are only
 * ever listed by their own view, so no active list can interleave history, and
 * the active list stays active-first.
 */
const views = [
  {
    value: "active",
    label: "Active Brains",
    emptyTitle: "No active Brains",
    emptyDescription:
      "Every Brain you can access is archived. Archived Brains stay out of this list until you switch the view.",
  },
  {
    value: "mine",
    label: "Owned by you",
    emptyTitle: "No Brains owned by you",
    emptyDescription:
      "Brains you created appear here. Brains other people share with you stay in the shared view.",
  },
  {
    value: "shared",
    label: "Shared with you",
    emptyTitle: "No Brains shared with you",
    emptyDescription:
      "Only Brains created by other people appear here. Brains you own stay in the owned view.",
  },
  {
    value: "archived",
    label: "Archived Brains (history)",
    emptyTitle: "No archived Brains",
    emptyDescription:
      "Archived Brains are history and never mix into the active list. Nothing has been archived yet.",
  },
] as const;

export function BrainsPage() {
  const user = useWorkspace();
  const navigate = useNavigate();
  const cache = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [search, setSearch] = useState("");
  const [view, setView] = useState<string>("active");
  const reduced = useReducedMotion();
  const query = useQuery({
    queryKey: ["brains"],
    refetchInterval: 5000,
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/brains", { signal })),
  });
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
            <Button variant="light" onClick={() => setCreating(true)}>
              Create your first Brain
            </Button>
          ),
        }
      : searching
        ? {
            title: `No Brains match “${search.trim()}”`,
            description: `Search covers the Brain name and description in the ${selected.label.toLowerCase()} view. Clear the search or choose another view.`,
            action: (
              <Button variant="light" onClick={() => setSearch("")}>
                Clear search
              </Button>
            ),
          }
        : {
            title: selected.emptyTitle,
            description: selected.emptyDescription,
            action: (
              <Button variant="light" onClick={() => setView("active")}>
                Show active Brains
              </Button>
            ),
          };
  return (
    <>
      <DeletionNotice />
      <PageHeader
        title="Your Brains"
        actions={
          <Button
            leftSection={<Plus size={18} />}
            onClick={() => setCreating(true)}
          >
            Create Brain
          </Button>
        }
      />
      <div className="filter-bar brain-filter-bar control-brain-filters">
        <TextInput
          aria-label="Search Brains"
          placeholder="Find a Brain…"
          leftSection={<Search size={16} />}
          value={search}
          onChange={(e) => setSearch(e.currentTarget.value)}
        />
        <SegmentedControl
          aria-label="Brain view"
          value={selected.value}
          onChange={setView}
          data={views.map(({ value }) => ({
            value,
            label:
              value === "mine"
                ? "Owned"
                : value === "shared"
                  ? "Shared"
                  : value === "archived"
                    ? "Archived"
                    : "Active",
          }))}
        />
        <Text size="xs" c="dimmed" ml="auto">
          {visible.length} {visible.length === 1 ? "Brain" : "Brains"} · Only
          spaces you can access
        </Text>
      </div>
      {/* The filter's own meaning, stated as text beside the list it governs. */}
      <Text className="collapse-notice" size="xs" c="dimmed" role="status">
        {collapseNotice}
      </Text>
      <ErrorState error={query.error} retry={() => void query.refetch()} />
      {query.isPending ? (
        <LoadingState label="Loading your Brains…" />
      ) : empty ? (
        <EmptyState
          icon={Boxes}
          title={empty.title}
          description={empty.description}
          action={empty.action}
        />
      ) : (
        <div className="brain-grid control-brain-grid">
          {ordered.map((brain) => (
            <motion.div
              layout={!reduced}
              transition={{ duration: reduced ? 0 : 0.25 }}
              className="brain-cell control-brain-cell"
              key={brain.id}
              data-testid="brain-card"
            >
              <Link
                to="/brains/$brainId/dashboard"
                params={{ brainId: brain.id }}
                search={{}}
                className="control-brain-select"
                aria-label={`Open ${brain.name}`}
              >
                <BrainIcon
                  id={brain.id}
                  revision={brain.icon_revision}
                  size={44}
                />
                <span>
                  <strong data-testid="brain-row-name">{brain.name}</strong>
                  <span className="control-brain-description">
                    {brain.description || "A space for knowledge and context."}
                  </span>
                  <span className="control-access">
                    {brain.archived ? "Archived · " : ""}
                    {brain.owner_id === user.user.id
                      ? "Owner"
                      : brain.role}{" "}
                    access
                  </span>
                </span>
                <ArrowRight className="brain-card-arrow" size={17} />
              </Link>
              {/* The Brain id is a machine identifier: it stays out of the row
                  and is offered as a copyable field instead. */}
              <details className="record-identity">
                <summary>Brain identifier</summary>
                <CopyButton value={brain.id}>
                  {({ copied, copy }) => (
                    <Button
                      size="xs"
                      variant="default"
                      onClick={copy}
                      leftSection={
                        copied ? <Check size={14} /> : <Copy size={14} />
                      }
                    >
                      {copied ? "Copied" : "Copy Brain ID"}
                    </Button>
                  )}
                </CopyButton>
              </details>
            </motion.div>
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
    </>
  );
}
