import { lazy, Suspense, type Dispatch, type SetStateAction } from "react";
import { createPortal } from "react-dom";
import { Link } from "@tanstack/react-router";
import { ArrowRight, BookOpen, Plus } from "lucide-react";
import type { Brain } from "./api";
import { EmptyState } from "./components/AsyncState";
import { SkeletonRows } from "./components/Skeleton";
import { Button, Drawer, SearchInput, Select } from "./components/ui";
import { McpProfileCard } from "./McpProfileCard";
import type { McpCatalogue } from "./McpPanel";
const McpRuntimePanel = lazy(() =>
  import("./McpRuntimePanel").then((m) => ({ default: m.McpRuntimePanel })),
);

type Setter<T> = Dispatch<SetStateAction<T>>;
type EnvironmentOption = { value: string; label: string };

export function McpToolbar({
  section,
  canConfigure,
  archived,
  owner = false,
  connection,
  connectionSearch,
  setConnectionSearch,
  environments,
  environment,
  setEnvironment,
  setDiscover,
  setConnectorSetup,
  setProfile,
  actionsMount,
}: {
  section: "connections" | "profiles" | "runners" | "activity";
  canConfigure: boolean;
  archived: boolean;
  owner?: boolean;
  connection: string | null;
  connectionSearch: string;
  setConnectionSearch: Setter<string>;
  environments: EnvironmentOption[];
  environment: string | null;
  setEnvironment: Setter<string | null>;
  setDiscover: Setter<string | null>;
  setConnectorSetup: Setter<boolean>;
  setProfile: Setter<string | null>;
  /**
   * The page header's action slot; the primary action renders there. `null`
   * while the slot mounts; `undefined` keeps the action in this toolbar.
   */
  actionsMount?: HTMLElement | null;
}) {
  const primary =
    section === "connections" && canConfigure ? (
      <Button
        disabled={archived || !!connection}
        onClick={() => setConnectorSetup(true)}
      >
        <Plus aria-hidden="true" />
        Add connection
      </Button>
    ) : section === "profiles" && canConfigure ? (
      <Button disabled={archived} onClick={() => setProfile("new")}>
        <Plus aria-hidden="true" />
        Create tool group
      </Button>
    ) : null;
  const filters = section === "connections" || environments.length > 0;
  const inline = actionsMount === undefined;
  return (
    <>
      {primary && actionsMount && createPortal(primary, actionsMount)}
      {(filters || (primary && inline)) && (
        <div className="flex flex-wrap items-center gap-2">
          {section === "connections" && (
            <div className="w-[320px]">
              <SearchInput
                aria-label="Find a connection"
                disabled={!!connection}
                placeholder="Find a connection"
                value={connectionSearch}
                onChange={(event) =>
                  setConnectionSearch(event.currentTarget.value)
                }
              />
            </div>
          )}
          {environments.length > 0 && (
            <Select
              className="w-[220px]"
              aria-label="Environment"
              disabled={!!connection}
              placeholder="All environments"
              clearable
              data={environments}
              value={environment}
              onChange={(value) => {
                setEnvironment(value);
                setDiscover(null);
              }}
            />
          )}
          <div className="ml-auto flex items-center gap-2">
            {section === "connections" && owner && (
              <Button variant="ghost" size="sm" asChild>
                <Link to="/connectors">
                  Approved connectors
                  <ArrowRight aria-hidden="true" />
                </Link>
              </Button>
            )}
            {inline && primary}
          </div>
        </div>
      )}
    </>
  );
}

export function ProfilesSection({
  brain,
  actor,
  data,
  initialProfile,
  profile,
  setProfile,
  environments,
  environment,
  refresh,
  openTools,
}: {
  brain: Brain;
  actor: string;
  data: McpCatalogue;
  initialProfile?: string;
  profile: string | null;
  setProfile: Setter<string | null>;
  environments: EnvironmentOption[];
  environment: string | null;
  refresh: () => void;
  openTools: (profileId: string) => void;
}) {
  return (
    <div className="grid gap-3.5">
      {initialProfile &&
        !data.profiles.some((p) => p.id === initialProfile) && (
          <p className="m-0 text-label text-muted-foreground" role="status">
            This tool group is unavailable or no longer visible to you.
          </p>
        )}
      {profile === "new" && (
        <McpProfileCard
          brain={brain}
          actor={actor}
          id="new"
          catalogue={data}
          environments={environments}
          saved={refresh}
          close={() => setProfile(null)}
          tools={() => {}}
        />
      )}
      {!data.profiles.length && profile !== "new" && (
        <div className="rounded-lg border border-border bg-card">
          <EmptyState
            icon={BookOpen}
            title="No tool groups yet"
            description="Add MCPs and people to a tool group."
          />
        </div>
      )}
      {data.profiles
        .filter(
          (p) =>
            !environment ||
            !p.environment_id ||
            p.environment_id === environment,
        )
        .map((p) => (
          <McpProfileCard
            key={p.id}
            brain={brain}
            actor={actor}
            id={p.id}
            catalogue={data}
            environments={environments}
            saved={refresh}
            tools={openTools}
          />
        ))}
    </div>
  );
}

export function McpRuntimeView({
  brain,
  actor,
  catalogue,
  selected,
  select,
  session,
  released,
  overlay = false,
}: {
  brain: Brain;
  actor: string;
  catalogue: McpCatalogue;
  selected: string | null;
  select: (id: string | null) => void;
  session: string;
  released: () => void;
  /** Inside a modal drawer: the call opens above it instead of docking. */
  overlay?: boolean;
}) {
  return (
    <Suspense
      fallback={<SkeletonRows rows={3} label="Loading tool activity…" />}
    >
      <McpRuntimePanel
        brain={brain}
        actor={actor}
        catalogue={catalogue}
        selected={selected}
        select={select}
        session={session}
        released={released}
        overlay={overlay}
      />
    </Suspense>
  );
}

export function DiagnosticsDrawer({
  open,
  selectedCall,
  setSelectedCall,
  setDiagnostics,
  ...runtime
}: {
  open: boolean;
  selectedCall: string | null;
  setSelectedCall: Setter<string | null>;
  setDiagnostics: Setter<boolean>;
  brain: Brain;
  actor: string;
  catalogue: McpCatalogue;
  session: string;
  released: () => void;
}) {
  // A call opened from here is a dialog above this drawer; Escape and outside
  // clicks dismiss only the top layer, so the drawer stays while it is open.
  return (
    <Drawer
      opened={open}
      onClose={() => {
        setSelectedCall(null);
        setDiagnostics(false);
      }}
      title="Tool runtime diagnostics"
      size={640}
    >
      {open && (
        <McpRuntimeView
          {...runtime}
          selected={selectedCall}
          select={setSelectedCall}
          overlay
        />
      )}
    </Drawer>
  );
}
