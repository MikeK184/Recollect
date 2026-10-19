import { lazy, Suspense, type Dispatch, type SetStateAction } from "react";
import { Drawer, Button, Select, Text, TextInput } from "@mantine/core";
import { BookOpen, CirclePlus, Plus, Search } from "lucide-react";
import { iconSize } from "./design/tokens";
import type { Brain } from "./api";
import { EmptyState } from "./components/AsyncState";
import { SkeletonRows } from "./components/Skeleton";
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
  connection,
  connectionSearch,
  setConnectionSearch,
  environments,
  environment,
  setEnvironment,
  setDiscover,
  setConnectorSetup,
  setProfile,
}: {
  section: "connections" | "profiles" | "runners" | "activity";
  canConfigure: boolean;
  archived: boolean;
  connection: string | null;
  connectionSearch: string;
  setConnectionSearch: Setter<string>;
  environments: EnvironmentOption[];
  environment: string | null;
  setEnvironment: Setter<string | null>;
  setDiscover: Setter<string | null>;
  setConnectorSetup: Setter<boolean>;
  setProfile: Setter<string | null>;
}) {
  return (
    <div className="feature-toolbar">
      {section === "connections" && (
        <TextInput
          className="feature-search"
          aria-label="Find a connection"
          disabled={!!connection}
          placeholder="Find a connection…"
          leftSection={<Search size={16} />}
          value={connectionSearch}
          onChange={(event) => setConnectionSearch(event.currentTarget.value)}
        />
      )}
      {environments.length > 0 && (
        <Select
          className="feature-search"
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
      {section === "connections" && canConfigure && (
        <Button
          className="connection-add-action"
          leftSection={<CirclePlus size={20} />}
          disabled={archived || !!connection}
          onClick={() => setConnectorSetup(true)}
        >
          Add connection
        </Button>
      )}
      {section === "profiles" && canConfigure && (
        <Button
          leftSection={<Plus size={iconSize.small} />}
          disabled={archived}
          onClick={() => setProfile("new")}
        >
          Create tool group
        </Button>
      )}
    </div>
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
    <div className="mcp-tool-groups">
      {initialProfile &&
        !data.profiles.some((p) => p.id === initialProfile) && (
          <Text size="sm" c="dimmed" role="status">
            This tool group is unavailable or no longer visible to you.
          </Text>
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
        <EmptyState
          icon={BookOpen}
          title="No tool groups yet"
          description="Add MCPs and people to a tool group."
        />
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
}: {
  brain: Brain;
  actor: string;
  catalogue: McpCatalogue;
  selected: string | null;
  select: (id: string | null) => void;
  session: string;
  released: () => void;
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
  return (
    <Drawer
      className="feature-drawer"
      opened={open}
      onClose={() => {
        setSelectedCall(null);
        setDiagnostics(false);
      }}
      closeOnEscape={selectedCall === null}
      closeOnClickOutside={selectedCall === null}
      trapFocus={selectedCall === null}
      position="right"
      title="Tool runtime diagnostics"
      size="lg"
    >
      {open && (
        <McpRuntimeView
          {...runtime}
          selected={selectedCall}
          select={setSelectedCall}
        />
      )}
    </Drawer>
  );
}
