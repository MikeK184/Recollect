import type { ServerDraft } from "./features/connections/mcpConfig";
import { McpConnectorSetup } from "./McpConnectorSetup";
import { ConnectionPowerDialog } from "./McpConnectionPower";
import { StatusDot } from "./components/StatusDot";
import { iconSize } from "./design/tokens";
import { lazy, Suspense, useEffect, useState } from "react";
import {
  Alert,
  ActionIcon,
  Badge,
  Button,
  Card,
  Drawer,
  Code,
  Group,
  Loader,
  Modal,
  Menu,
  Select,
  Stack,
  Text,
  TextInput,
  Tabs,
  Title,
} from "@mantine/core";
import {
  Box,
  BookOpen,
  Copy,
  Database,
  FileText,
  Pencil,
  Play,
  Unplug,
  ShieldCheck,
  Plus,
  CirclePlus,
  Activity,
  Search,
  MoreHorizontal,
  Pause,
  X,
  Layers,
  Server,
  Monitor,
} from "lucide-react";
import { EmptyState } from "./components/AsyncState";
import { SkeletonRows } from "./components/Skeleton";
import { staggerStyle } from "./components/Motion";
import "./features/feature-views.css";
import "./features/guided-controls.css";
import "./features/connections/connections-inspection.css";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useWorkspace } from "./app/context";
import { client, result, RequestError, type Brain } from "./api";
import type { components } from "./api-schema";
import { McpConnectionDialog } from "./McpConnectionDialog";
import { McpProfileCard } from "./McpProfileCard";
import { McpConnectorIcon } from "./components/McpConnectorIcon";
import { CodeBlock } from "./components/Markdown";
import { ToolDescription } from "./components/ToolDescription";
import { McpRunDialog, type McpTool } from "./McpRunDialog";
const McpRuntimePanel = lazy(() =>
  import("./McpRuntimePanel").then((m) => ({ default: m.McpRuntimePanel })),
);
import { McpPrivateRunners } from "./McpPrivateRunners";

export type McpProfile = components["schemas"]["McpProfile"];
export type McpCatalogue = components["schemas"]["McpCatalogue"];
export type McpRights = components["schemas"]["McpRights"];
export const rightsLabel = (rights: McpRights) =>
  [
    rights.use_profile && "Use",
    rights.manage && "Manage",
    rights.share && "Share",
  ]
    .filter(Boolean)
    .join(" · ") || "No tool-group rights";
export const availabilityLabel = (value: string) =>
  ({
    configured: "Configured",
    disabled: "Connection disabled",
    definition_disabled: "Definition disabled",
    configuration_invalid: "Settings need attention",
  })[value] ?? value;

// These reads return approved metadata only. Do not retry denied/invalid reads,
// mutations, inspections or tool execution.
const retryMetadataRead = (failures: number, error: Error) =>
  failures < 2 &&
  (error instanceof RequestError
    ? [408, 429, 500, 502, 503, 504].includes(error.status)
    : error instanceof TypeError);
const metadataRetryDelay = (attempt: number) =>
  Math.min(500 * 2 ** attempt, 2000);
const placementLabel = (placement: string) =>
  (
    ({
      central: "Recollect service",
      local: "Paired device",
      private: "Private runner",
    }) as Record<string, string>
  )[placement] ?? placement;
type ToolDescriptor = components["schemas"]["McpToolDescriptor"];
type MetadataRead = {
  tools?: ToolDescriptor[];
  error: Error | null;
  pending: boolean;
  refreshing: boolean;
  reload: () => void;
};

export function McpPanel({
  brain,
  actor,
  section = "connections",
  onSectionChange,
  initialCall,
  initialConnection,
  initialProfile,
  onManageRunners,
  onInspectorChange,
}: {
  brain: Brain;
  actor: string;
  section?: "connections" | "profiles" | "runners" | "activity";
  onSectionChange?: (section: string | null) => void;
  initialCall?: string;
  initialConnection?: string;
  initialProfile?: string;
  onManageRunners?: () => void;
  onInspectorChange?: (open: boolean) => void;
}) {
  const cache = useQueryClient();
  const account = useWorkspace();
  const [testing, setTesting] = useState<string | null>(null);
  const [inspectedId, setInspectedId] = useState<string | null>(
    initialConnection ?? null,
  );
  const [inspectorDismissed, setInspectorDismissed] = useState(false);
  const [inspectorTab, setInspectorTab] = useState<string | null>(
    initialConnection ? "tools" : "overview",
  );
  const [connectionSearch, setConnectionSearch] = useState("");
  const [importedDraft, setImportedDraft] = useState<ServerDraft | undefined>();
  const [connection, setConnection] = useState<string | null>(null);
  const [connectionNameMount, setConnectionNameMount] =
    useState<HTMLDivElement | null>(null);
  const [power, setPower] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState(false);
  const [connectorSetup, setConnectorSetup] = useState(false);
  const [profile, setProfile] = useState<string | null>(null);
  const [discover, setDiscover] = useState<string | null>(null);
  const [discoverConnection, setDiscoverConnection] = useState<string | null>(
    null,
  );
  const [environment, setEnvironment] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [setupSaved, setSetupSaved] = useState(false);
  const [session, setSession] = useState(() => crypto.randomUUID());
  const [selectedCall, setSelectedCall] = useState<string | null>(
    initialCall ?? null,
  );
  const [run, setRun] = useState<{
    profile: string;
    tool: McpTool;
    environment: string | null;
  } | null>(null);
  useEffect(() => {
    onInspectorChange?.(
      !!(
        connection ||
        power ||
        profile ||
        discover ||
        run ||
        diagnostics ||
        connectorSetup ||
        testing
      ),
    );
    return () => onInspectorChange?.(false);
  }, [
    connection,
    power,
    profile,
    discover,
    run,
    diagnostics,
    connectorSetup,
    onInspectorChange,
    testing,
  ]);
  const catalogue = useQuery({
    queryKey: ["mcp", brain.id, "catalogue"],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  const runtime = useQuery({
    queryKey: ["mcp", brain.id, "runtime"],
    enabled: section === "connections" && !!catalogue.data && !catalogue.error,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/runtime", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  const workspace = useQuery({
    queryKey: ["mcp", brain.id, "environments"],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
  });
  const refresh = () => {
    void cache.invalidateQueries({ queryKey: ["mcp", brain.id] });
    void cache.invalidateQueries({ queryKey: ["audit", brain.id] });
  };
  const clear = (message: string) => {
    setConnection(null);
    setImportedDraft(undefined);
    setPower(null);
    setProfile(null);
    setDiscover(null);
    setRun(null);
    setNotice(message);
    refresh();
  };
  useEffect(() => {
    if (catalogue.error || workspace.error) {
      setTesting(null);
      setConnection(null);
      setImportedDraft(undefined);
      setPower(null);
      setConnectorSetup(false);
      setProfile(null);
      setDiscover(null);
      setRun(null);
      setNotice(
        "Catalogue access could not be refreshed. Reload it to continue.",
      );
      return;
    }
    if (!catalogue.data) return;
    if (
      testing &&
      (brain.archived ||
        !catalogue.data.can_configure ||
        !catalogue.data.connections.some((c) => c.id === testing))
    )
      setTesting(null);
    if (
      power &&
      (!catalogue.data.can_configure ||
        brain.archived ||
        !catalogue.data.connections.some((c) => c.id === power))
    )
      setPower(null);
    if (connectorSetup && (!catalogue.data.can_configure || brain.archived))
      setConnectorSetup(false);
    if (connection && !catalogue.data.can_configure) {
      setConnection(null);
      setImportedDraft(undefined);
      setNotice("Connection administration is no longer available.");
    }
    if (
      profile &&
      profile !== "new" &&
      !catalogue.data.profiles.some((p) => p.id === profile)
    ) {
      setProfile(null);
      setNotice("Tool-group access changed. The inspector was closed.");
    }
    if (profile === "new" && (!catalogue.data.can_configure || brain.archived))
      setProfile(null);
    if (
      discover &&
      (brain.archived ||
        !catalogue.data.profiles.some(
          (p) => p.id === discover && p.rights.use_profile && p.enabled,
        ))
    ) {
      setDiscover(null);
      setNotice("Tool-group use changed. Cached tool results were cleared.");
    }
  }, [
    catalogue.data,
    catalogue.error,
    workspace.error,
    brain.archived,
    connection,
    profile,
    discover,
    connectorSetup,
    power,
    testing,
  ]);
  const data = catalogue.error ? undefined : catalogue.data;
  const matchingConnections =
    data?.connections.filter(
      (c) =>
        (!environment ||
          !c.environment_id ||
          c.environment_id === environment) &&
        c.name
          .toLocaleLowerCase()
          .includes(connectionSearch.trim().toLocaleLowerCase()),
    ) ?? [];

  const inspected =
    connection && connection !== "new"
      ? data?.connections.find((item) => item.id === connection)
      : inspectorDismissed
        ? undefined
        : inspectedId
          ? matchingConnections.find((c) => c.id === inspectedId)
          : matchingConnections[0];
  useEffect(() => {
    setInspectedId(initialConnection ?? null);
    setInspectorDismissed(false);
    setInspectorTab(initialConnection ? "tools" : "overview");
  }, [brain.id, initialConnection]);
  const profileIds = data?.profiles.map((p) => p.id).join(",");
  useEffect(() => {
    if (section === "profiles" && initialProfile)
      document
        .getElementById(`tool-group-${initialProfile}`)
        ?.scrollIntoView({ block: "center" });
  }, [brain.id, section, initialProfile, profileIds]);
  const inspectedDetail = useQuery({
    queryKey: ["mcp", brain.id, "connection", inspected?.id],
    enabled:
      section === "connections" &&
      !!inspected &&
      !!data?.can_configure &&
      !workspace.error,
    gcTime: 0,
    retry: retryMetadataRead,
    retryDelay: metadataRetryDelay,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/connections/{id}", {
          params: { path: { brain: brain.id, id: inspected!.id } },
          signal,
        }),
      ),
  });
  const inspectedDefinition = useQuery({
    queryKey: ["mcp", brain.id, "definition", inspected?.definition_key],
    enabled:
      section === "connections" &&
      !!inspected &&
      !!data?.can_configure &&
      !workspace.error,
    gcTime: 0,
    retry: retryMetadataRead,
    retryDelay: metadataRetryDelay,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/definitions/{key}", {
          params: { path: { brain: brain.id, key: inspected!.definition_key } },
          signal,
        }),
      ),
  });
  const visibleDetail =
    data?.can_configure && !inspectedDetail.error
      ? inspectedDetail.data
      : undefined;
  const visibleDefinition =
    data?.can_configure && !inspectedDefinition.error
      ? inspectedDefinition.data
      : undefined;
  const approvedTools: MetadataRead = {
    tools: visibleDefinition?.tools,
    error: inspectedDefinition.error,
    pending: inspectedDefinition.isPending,
    refreshing: inspectedDefinition.isFetching,
    reload: () => void inspectedDefinition.refetch(),
  };
  const discoveryProfile = data?.profiles.find(
    (p) => p.id === discover && p.rights.use_profile && p.enabled,
  );
  const runProfile =
    !brain.archived &&
    data?.profiles.find(
      (p) => p.id === run?.profile && p.enabled && p.rights.use_profile,
    );
  const environments =
    workspace.data?.environments.map((e) => ({ value: e.id, label: e.name })) ??
    [];
  const environmentName = (id: string | null | undefined) =>
    id
      ? (environments.find((e) => e.value === id)?.label ??
        "Unavailable environment")
      : "Brain-wide";
  return (
    <section
      className={`feature-view mcp-management-view${section === "profiles" ? " mcp-tool-access-view" : ""}`}
      aria-label="MCP connections and tool groups"
    >
      <Stack gap="md">
        {connectorSetup &&
          data?.can_configure &&
          !brain.archived &&
          !workspace.error && (
            <McpConnectorSetup
              brain={brain}
              close={() => setConnectorSetup(false)}
              approvedConnectors={data.definitions.filter((d) => d.enabled)}
              reuse={(draft) => {
                setImportedDraft(draft);
                setConnectorSetup(false);
                setConnection("new");
              }}
              saved={(connected) => {
                setConnectorSetup(false);
                setNotice(
                  connected
                    ? "MCP server added. Include it in a tool group to make it available to your agents."
                    : "Connector registered. Add a connection to choose its target and execution location.",
                );
                refresh();
              }}
            />
          )}
        {notice && (
          <Alert
            title="Catalogue updated"
            withCloseButton
            onClose={() => setNotice(null)}
          >
            {notice}
          </Alert>
        )}
        {(catalogue.error || workspace.error) && (
          <Alert color="red" title="Catalogue unavailable">
            {(catalogue.error || workspace.error)?.message}
            <Button size="xs" variant="light" onClick={refresh}>
              Reload catalogue
            </Button>
          </Alert>
        )}
        {catalogue.isPending && (
          <SkeletonRows label="Loading catalogue…" rows={4} />
        )}
        {data && !workspace.error && (
          <>
            {section !== "activity" && (
              <div className="feature-toolbar">
                {section === "connections" && (
                  <TextInput
                    className="feature-search"
                    aria-label="Find a connection"
                    disabled={!!connection}
                    placeholder="Find a connection…"
                    leftSection={<Search size={16} />}
                    value={connectionSearch}
                    onChange={(event) =>
                      setConnectionSearch(event.currentTarget.value)
                    }
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
                {section === "connections" && data.can_configure && (
                  <Button
                    className="connection-add-action"
                    leftSection={<CirclePlus size={20} />}
                    disabled={brain.archived || !!connection}
                    onClick={() => setConnectorSetup(true)}
                  >
                    Add connection
                  </Button>
                )}
                {section === "profiles" && data.can_configure && (
                  <Button
                    leftSection={<Plus size={iconSize.small} />}
                    disabled={brain.archived}
                    onClick={() => setProfile("new")}
                  >
                    Create tool group
                  </Button>
                )}
              </div>
            )}
            {brain.archived && (
              <Alert color="gray">
                This Brain is archived. Configuration and tool use are paused.
              </Alert>
            )}
            {section === "connections" && (
              <>
                {setupSaved && (
                  <Alert title="Connection saved · not tested" color="brand">
                    <Text size="sm">
                      Saved without a tool call. Tool access is granted
                      separately.
                    </Text>
                    <Group mt="md">
                      {onSectionChange && (
                        <Button onClick={() => onSectionChange("profiles")}>
                          Manage tool access
                        </Button>
                      )}
                      <Button
                        variant="subtle"
                        onClick={() => setSetupSaved(false)}
                      >
                        Dismiss
                      </Button>
                    </Group>
                  </Alert>
                )}
                {data.can_configure && data.definitions.length === 0 && (
                  <Alert title="No approved connectors">
                    Start with Add connection to register an MCP connector. Then
                    choose where it runs and which agents may use it.
                  </Alert>
                )}
                {!data.connections.length && (
                  <EmptyState
                    icon={Unplug}
                    title="Tools, when you need them"
                    description="Add an approved connection, then include it in a tool group with explicit tool-use permissions."
                  />
                )}

                {runtime.error && (
                  <Alert color="gray" title="Session status unavailable">
                    {runtime.error.message}
                    <Button
                      size="xs"
                      variant="subtle"
                      onClick={() => void runtime.refetch()}
                    >
                      Retry status
                    </Button>
                  </Alert>
                )}
                {(connectionSearch || environment) && (
                  <Text size="xs" c="dimmed">
                    {matchingConnections.length} of {data.connections.length}{" "}
                    loaded connections
                  </Text>
                )}
                {!!data.connections.length && !matchingConnections.length && (
                  <Text size="sm" c="dimmed" role="status">
                    No connections match these filters.
                  </Text>
                )}
                {initialConnection &&
                  !data.connections.some((c) => c.id === initialConnection) && (
                    <Text size="sm" c="dimmed" role="status">
                      This connection is unavailable or no longer visible to
                      you.
                    </Text>
                  )}
                <div
                  className={
                    "connections-layout" + (!inspected ? " no-selection" : "")
                  }
                >
                  <div>
                    {matchingConnections.map((item) => (
                      <article
                        key={item.id}
                        className={
                          "connection-row" +
                          (inspected?.id === item.id ? " selected" : "")
                        }
                        data-testid="mcp-connection"
                      >
                        <McpConnectorIcon
                          definition={data.definitions.find(
                            (d) => d.key === item.definition_key,
                          )}
                        />
                        <div className="connection-row-content">
                          <button
                            className="connection-select"
                            disabled={
                              !!connection &&
                              connection !== "new" &&
                              connection !== item.id
                            }
                            onClick={() => {
                              setInspectedId(item.id);
                              setInspectorDismissed(false);
                              setInspectorTab("overview");
                            }}
                          >
                            <strong>{item.name}</strong>
                          </button>
                          <small>
                            {item.id === inspected?.id && visibleDetail
                              ? visibleDetail.target
                              : (data.definitions.find(
                                  (d) => d.key === item.definition_key,
                                )?.name ?? "Approved connector")}
                          </small>
                          <div className="connection-scope-badges">
                            <Badge
                              variant="light"
                              color="brand"
                              title={environmentName(item.environment_id)}
                              leftSection={<Layers size={12} />}
                            >
                              {environmentName(item.environment_id)}
                            </Badge>
                            <Badge
                              variant="outline"
                              color="gray"
                              leftSection={
                                item.placement === "local" ? (
                                  <Monitor size={12} />
                                ) : (
                                  <Server size={12} />
                                )
                              }
                            >
                              {placementLabel(item.placement)}
                            </Badge>
                          </div>
                        </div>
                        <div className="connection-row-status">
                          <Badge
                            leftSection={
                              <StatusDot
                                tone={
                                  item.availability === "configured"
                                    ? "accent"
                                    : "idle"
                                }
                                size={8}
                              />
                            }
                            variant="light"
                            color={
                              item.availability === "configured"
                                ? "brand"
                                : "gray"
                            }
                          >
                            {item.availability === "configured"
                              ? item.last_successful_call_at
                                ? "Last call succeeded"
                                : "Configured · not tested"
                              : availabilityLabel(item.availability)}
                          </Badge>
                          <small
                            title={
                              item.last_successful_call_at
                                ? new Date(
                                    item.last_successful_call_at,
                                  ).toLocaleString()
                                : undefined
                            }
                          >
                            {item.last_successful_call_at
                              ? "Last call · " +
                                new Date(
                                  item.last_successful_call_at,
                                ).toLocaleDateString()
                              : "No observed successful call"}
                          </small>
                        </div>
                        <div className="connection-row-actions">
                          <Button
                            variant="subtle"
                            size="compact-sm"
                            aria-label={"View " + item.name}
                            disabled={
                              !!connection &&
                              connection !== "new" &&
                              connection !== item.id
                            }
                            onClick={() => {
                              setInspectedId(item.id);
                              setInspectorDismissed(false);
                            }}
                          >
                            Inspect
                          </Button>
                          <Menu position="bottom-end" withinPortal>
                            <Menu.Target>
                              <ActionIcon
                                variant="subtle"
                                size={26}
                                aria-label={"Actions for " + item.name}
                                disabled={!!connection}
                              >
                                <MoreHorizontal size={20} />
                              </ActionIcon>
                            </Menu.Target>
                            <Menu.Dropdown>
                              <Menu.Item
                                leftSection={<Box size={16} />}
                                onClick={() => {
                                  if (connection) return;
                                  setInspectedId(item.id);
                                  setInspectorDismissed(false);
                                  setInspectorTab("overview");
                                }}
                              >
                                View connection
                              </Menu.Item>
                              {data.can_configure && (
                                <>
                                  <Menu.Divider />
                                  <Menu.Item
                                    leftSection={<Pencil size={16} />}
                                    disabled={brain.archived}
                                    onClick={() => {
                                      if (connection) return;
                                      setImportedDraft(undefined);
                                      setInspectedId(item.id);
                                      setInspectorDismissed(false);
                                      setInspectorTab("overview");
                                      setConnection(item.id);
                                    }}
                                  >
                                    Edit connection
                                  </Menu.Item>
                                  <Menu.Item
                                    leftSection={
                                      item.enabled ? (
                                        <Pause size={16} />
                                      ) : (
                                        <Play size={16} />
                                      )
                                    }
                                    disabled={brain.archived}
                                    onClick={() =>
                                      !connection && setPower(item.id)
                                    }
                                  >
                                    {item.enabled
                                      ? "Pause use…"
                                      : "Enable use…"}
                                  </Menu.Item>
                                </>
                              )}
                            </Menu.Dropdown>
                          </Menu>
                        </div>
                      </article>
                    ))}
                  </div>
                  {inspected && (
                    <aside
                      className="management-surface connection-inspector"
                      aria-label="Connection details"
                    >
                      <Group
                        gap="md"
                        mb="lg"
                        className="connection-inspector-heading"
                      >
                        <McpConnectorIcon
                          definition={data.definitions.find(
                            (d) => d.key === inspected.definition_key,
                          )}
                        />
                        <div className="connection-inspector-identity">
                          {connection === inspected.id ? (
                            <div ref={setConnectionNameMount} />
                          ) : (
                            <h2 className="management-title">
                              {inspected.name}
                            </h2>
                          )}
                          <p className="management-muted inspector-target">
                            {visibleDetail?.target ?? "Approved MCP connector"}
                          </p>
                        </div>
                        <div className="connection-inspector-actions">
                          <Menu position="bottom-end" withinPortal>
                            <Menu.Target>
                              <ActionIcon
                                variant="subtle"
                                size={26}
                                aria-label="Connection actions"
                                disabled={!!connection}
                              >
                                <MoreHorizontal size={20} />
                              </ActionIcon>
                            </Menu.Target>
                            <Menu.Dropdown>
                              <Menu.Item
                                leftSection={<FileText size={16} />}
                                disabled={!!connection}
                                onClick={() =>
                                  !connection && setInspectorTab("tools")
                                }
                              >
                                View approved tools
                              </Menu.Item>
                              <Menu.Item
                                leftSection={<Activity size={16} />}
                                disabled={!!connection}
                                onClick={() =>
                                  !connection && setInspectorTab("activity")
                                }
                              >
                                View connection history
                              </Menu.Item>
                              {data.can_configure && (
                                <>
                                  <Menu.Divider />
                                  <Menu.Item
                                    leftSection={<Pencil size={16} />}
                                    disabled={brain.archived}
                                    onClick={() => {
                                      if (connection) return;
                                      setImportedDraft(undefined);
                                      setInspectorTab("overview");
                                      setConnection(inspected.id);
                                    }}
                                  >
                                    Edit connection
                                  </Menu.Item>
                                  <Menu.Item
                                    leftSection={
                                      inspected.enabled ? (
                                        <Pause size={16} />
                                      ) : (
                                        <Play size={16} />
                                      )
                                    }
                                    disabled={brain.archived || !!connection}
                                    onClick={() =>
                                      !connection && setPower(inspected.id)
                                    }
                                  >
                                    {inspected.enabled
                                      ? "Pause use…"
                                      : "Enable use…"}
                                  </Menu.Item>
                                </>
                              )}
                            </Menu.Dropdown>
                          </Menu>
                          <ActionIcon
                            variant="subtle"
                            size={26}
                            aria-label="Close connection details"
                            disabled={!!connection}
                            onClick={() => setInspectorDismissed(true)}
                          >
                            <X size={20} />
                          </ActionIcon>
                        </div>
                      </Group>
                      <Tabs
                        value={inspectorTab}
                        onChange={(value) => {
                          if (!connection) setInspectorTab(value);
                        }}
                      >
                        <Tabs.List>
                          <Tabs.Tab value="overview">Overview</Tabs.Tab>
                          <Tabs.Tab value="tools" disabled={!!connection}>
                            Tools
                          </Tabs.Tab>
                          <Tabs.Tab value="activity" disabled={!!connection}>
                            Activity
                          </Tabs.Tab>
                        </Tabs.List>
                      </Tabs>
                      {inspectorTab === "overview" && (
                        <>
                          {connection === inspected.id ? (
                            <McpConnectionDialog
                              inline
                              nameMount={connectionNameMount}
                              brain={brain}
                              id={inspected.id}
                              catalogue={data}
                              environments={environments}
                              close={() => setConnection(null)}
                              saved={() => {
                                setConnection(null);
                                refresh();
                              }}
                              stale={clear}
                              manageRunners={onManageRunners}
                            />
                          ) : (
                            <>
                              <h3 className="management-title inspector-section-title">
                                Configuration
                              </h3>
                              <dl>
                                <dt>Target</dt>
                                <dd>
                                  {inspectedDetail.error ? (
                                    "Unavailable"
                                  ) : visibleDetail ? (
                                    <span className="inspector-target-value">
                                      <code>{visibleDetail.target}</code>
                                      <Button
                                        variant="subtle"
                                        size="compact-xs"
                                        aria-label="Copy target URL"
                                        onClick={() =>
                                          void navigator.clipboard.writeText(
                                            visibleDetail.target,
                                          )
                                        }
                                      >
                                        <Copy size={16} />
                                      </Button>
                                    </span>
                                  ) : data.can_configure ? (
                                    "Loading…"
                                  ) : (
                                    "Visible to managers"
                                  )}
                                </dd>
                                <dt>Scope</dt>
                                <dd>
                                  <code>
                                    {environmentName(inspected.environment_id)}
                                  </code>
                                </dd>
                                <dt>Runs on</dt>
                                <dd>
                                  <code>
                                    {placementLabel(inspected.placement)}
                                  </code>
                                </dd>
                              </dl>
                            </>
                          )}
                          <div className="inspector-test-section">
                            <Group justify="space-between" mb="sm">
                              <h3 className="management-title inspector-section-title">
                                Test status
                              </h3>
                              <Badge
                                color={
                                  inspected.last_successful_call_at
                                    ? "brand"
                                    : "yellow"
                                }
                              >
                                {inspected.last_successful_call_at
                                  ? "Successful call recorded"
                                  : "Configured · not tested"}
                              </Badge>
                            </Group>
                            <p className="management-muted">
                              {inspected.last_successful_call_at
                                ? "Last successful call: " +
                                  new Date(
                                    inspected.last_successful_call_at,
                                  ).toLocaleDateString()
                                : "No successful call recorded."}
                            </p>
                            <p className="connection-test-meaning">
                              Test checks the anonymous server handshake and
                              tool metadata. It does not run tools or update
                              approved metadata.
                            </p>
                            <details className="inspector-session-details">
                              <summary>Runtime sessions</summary>
                              <ConnectionObservation
                                id={inspected.id}
                                status={
                                  runtime.error ? undefined : runtime.data
                                }
                                pending={runtime.isPending}
                                failed={!!runtime.error}
                                checkedAt={runtime.dataUpdatedAt}
                              />
                            </details>
                            {data.can_configure && (
                              <Group
                                mt="md"
                                gap="xs"
                                className="connection-inspector-actions"
                              >
                                <Button
                                  size="sm"
                                  color="green"
                                  leftSection={<Play size={17} />}
                                  aria-label="Test connection"
                                  disabled={
                                    brain.archived ||
                                    !inspected.enabled ||
                                    !!connection
                                  }
                                  onClick={() => setTesting(inspected.id)}
                                >
                                  Test connection
                                </Button>
                                <Button
                                  size="sm"
                                  variant="default"
                                  leftSection={<Pencil size={17} />}
                                  aria-label="Edit connection"
                                  disabled={brain.archived || !!connection}
                                  onClick={() => {
                                    setImportedDraft(undefined);
                                    setInspectorTab("overview");
                                    setConnection(inspected.id);
                                  }}
                                >
                                  Edit
                                </Button>
                              </Group>
                            )}
                          </div>
                          <section className="inspector-tool-access">
                            <Group justify="space-between">
                              <h3 className="management-title inspector-section-title">
                                Tool access
                              </h3>
                              <Button
                                variant="subtle"
                                size="compact-sm"
                                onClick={() => onSectionChange?.("profiles")}
                              >
                                Manage access →
                              </Button>
                            </Group>
                            {data.profiles
                              .filter((p) =>
                                p.connection_ids.includes(inspected.id),
                              )
                              .map((p) => (
                                <InspectorToolGroup
                                  key={`${p.id}:${inspected.id}:${p.environment_id ?? "brain-wide"}`}
                                  brain={brain}
                                  profile={p}
                                  connection={inspected}
                                  canConfigure={data.can_configure}
                                  approved={approvedTools}
                                />
                              ))}
                            {!data.profiles.some((p) =>
                              p.connection_ids.includes(inspected.id),
                            ) && (
                              <p className="management-muted">
                                No visible tool groups.
                              </p>
                            )}
                            {data.can_configure && (
                              <Button
                                variant="subtle"
                                size="compact-xs"
                                mt="md"
                                disabled={brain.archived || !!connection}
                                onClick={() =>
                                  !connection && setPower(inspected.id)
                                }
                              >
                                {inspected.enabled
                                  ? "Pause use…"
                                  : "Enable use…"}
                              </Button>
                            )}
                          </section>
                        </>
                      )}
                      {inspectorTab === "tools" && (
                        <Stack mt="lg">
                          {data.can_configure ? (
                            <>
                              <Text size="xs" c="dimmed">
                                Approved metadata. Tool access is granted
                                independently.
                              </Text>
                              <ToolReadNotice read={approvedTools} />
                              {visibleDefinition?.tools.length === 0 && (
                                <Text size="sm" c="dimmed">
                                  No approved tools in this connector.
                                </Text>
                              )}
                              {visibleDefinition?.tools.map((tool) => (
                                <ApprovedToolDetails
                                  key={tool.name}
                                  tool={tool}
                                />
                              ))}
                            </>
                          ) : (
                            <>
                              <Text size="xs" c="dimmed">
                                Approved tools from your permitted groups. No
                                server connection is started.
                              </Text>
                              {data.profiles
                                .filter((p) =>
                                  p.connection_ids.includes(inspected.id),
                                )
                                .map((p) => (
                                  <InspectorToolGroup
                                    key={`${p.id}:${inspected.id}:${p.environment_id ?? "brain-wide"}`}
                                    brain={brain}
                                    profile={p}
                                    connection={inspected}
                                    canConfigure={false}
                                    approved={approvedTools}
                                    detailed
                                  />
                                ))}
                              {!data.profiles.some((p) =>
                                p.connection_ids.includes(inspected.id),
                              ) && (
                                <Text size="sm" c="dimmed">
                                  No visible tool groups. Use access is required
                                  to inspect tools.
                                </Text>
                              )}
                            </>
                          )}
                        </Stack>
                      )}
                      {inspectorTab === "activity" && (
                        <Stack mt="lg">
                          <Text size="sm">
                            {inspected.last_successful_call_at
                              ? "Last successful call: " +
                                new Date(
                                  inspected.last_successful_call_at,
                                ).toLocaleString()
                              : "No observed successful call for this configuration."}
                          </Text>
                          <Text size="xs" c="dimmed">
                            Inspect tool calls and session diagnostics across
                            this Brain’s permitted connections.
                          </Text>
                          <Button
                            variant="default"
                            onClick={() => setDiagnostics(true)}
                          >
                            Open tool activity
                          </Button>
                        </Stack>
                      )}
                    </aside>
                  )}
                </div>
                {onManageRunners && (
                  <div className="management-surface private-runner-note">
                    <div className="management-icon">
                      <Database />
                    </div>
                    <div>
                      <h3 className="management-title">
                        Need a private network?
                      </h3>
                      <p className="management-muted">
                        Use a private runner to connect to tools inside your
                        network.
                      </p>
                    </div>
                    <Button variant="subtle" onClick={onManageRunners}>
                      Manage runners →
                    </Button>
                  </div>
                )}
              </>
            )}
            {section === "profiles" && (
              <div className="mcp-tool-groups">
                {initialProfile &&
                  !data.profiles.some((p) => p.id === initialProfile) && (
                    <Text size="sm" c="dimmed" role="status">
                      This tool group is unavailable or no longer visible to
                      you.
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
                      tools={(id) => {
                        setDiscoverConnection(null);
                        setDiscover(id);
                      }}
                    />
                  ))}
              </div>
            )}
            {section === "runners" && (
              <McpPrivateRunners
                brain={brain}
                canConfigure={data.can_configure}
              />
            )}
            {section === "activity" && (
              <Suspense
                fallback={
                  <SkeletonRows rows={3} label="Loading tool activity…" />
                }
              >
                <McpRuntimePanel
                  brain={brain}
                  actor={actor}
                  catalogue={data}
                  selected={selectedCall}
                  select={setSelectedCall}
                  session={session}
                  released={() => {
                    setRun(null);
                    setSession(crypto.randomUUID());
                  }}
                />
              </Suspense>
            )}
            <Drawer
              className="feature-drawer"
              opened={diagnostics && section !== "activity"}
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
              {diagnostics && section !== "activity" && (
                <Suspense
                  fallback={
                    <SkeletonRows rows={3} label="Loading tool activity…" />
                  }
                >
                  <McpRuntimePanel
                    brain={brain}
                    actor={actor}
                    catalogue={data}
                    selected={selectedCall}
                    select={setSelectedCall}
                    session={session}
                    released={() => {
                      setRun(null);
                      setSession(crypto.randomUUID());
                    }}
                  />
                </Suspense>
              )}
            </Drawer>
          </>
        )}

        {data &&
          testing &&
          !workspace.error &&
          data.connections.some((c) => c.id === testing) && (
            <ConnectionTest
              key={`${testing}:${data.connections.find((c) => c.id === testing)?.revision}:${data.definitions.find((d) => d.key === data.connections.find((c) => c.id === testing)?.definition_key)?.updated_at}`}
              brain={brain}
              id={testing}
              catalogue={data}
              owner={account.user.installation_owner}
              close={() => setTesting(null)}
              tools={(profileId) => {
                setDiscoverConnection(testing);
                setTesting(null);
                setDiscover(profileId);
              }}
            />
          )}
        {data && power && !workspace.error && (
          <ConnectionPowerDialog
            key={power}
            brain={brain}
            id={power}
            catalogue={data}
            close={() => setPower(null)}
            saved={() => {
              setPower(null);
              refresh();
            }}
            stale={clear}
          />
        )}
        {data && connection === "new" && !workspace.error && (
          <McpConnectionDialog
            initialDraft={importedDraft}
            key={connection}
            brain={brain}
            id={connection}
            catalogue={data}
            environments={environments}
            manageRunners={
              onManageRunners
                ? () => {
                    setConnection(null);
                    setImportedDraft(undefined);
                    onManageRunners();
                  }
                : undefined
            }
            close={() => {
              setConnection(null);
              setImportedDraft(undefined);
            }}
            saved={() => {
              setSetupSaved(connection === "new");
              setConnection(null);
              setImportedDraft(undefined);
              refresh();
            }}
            stale={clear}
          />
        )}
        {discoveryProfile && !workspace.error && (
          <DiscoveryDialog
            key={`${discover}:${discoverConnection}`}
            connectionId={discoverConnection}
            connectionName={
              data?.connections.find((c) => c.id === discoverConnection)?.name
            }
            brain={brain}
            profile={discoveryProfile}
            environments={environments}
            close={() => setDiscover(null)}
            run={(tool, selectedEnvironment) => {
              setRun({
                profile: discoveryProfile.id,
                tool,
                environment: selectedEnvironment,
              });
              setDiscover(null);
            }}
          />
        )}
        {run && runProfile && !workspace.error && (
          <McpRunDialog
            brain={brain}
            profile={runProfile}
            tool={run.tool}
            environment={run.environment}
            session={session}
            close={() => setRun(null)}
            submitted={(id) => {
              setRun(null);
              setSelectedCall(id);
              setDiagnostics(true);
              refresh();
            }}
          />
        )}
      </Stack>
    </section>
  );
}

function ToolReadNotice({ read }: { read: MetadataRead }) {
  if (read.error) {
    const denied =
      read.error instanceof RequestError &&
      [401, 403].includes(read.error.status);
    return (
      <Alert
        color={denied ? "gray" : "red"}
        title={
          denied
            ? "Tool metadata access unavailable"
            : "Tool metadata unavailable"
        }
      >
        <Text size="sm">{read.error.message}</Text>
        <Button
          size="xs"
          variant="light"
          mt="xs"
          disabled={read.refreshing}
          loading={read.refreshing}
          onClick={read.reload}
        >
          Reload tools
        </Button>
      </Alert>
    );
  }
  if (read.pending || read.refreshing)
    return (
      <Group gap="xs" className="connection-tools-loading" role="status">
        <Loader size="xs" />
        <Text size="xs" c="dimmed">
          {read.pending
            ? "Loading approved tools…"
            : "Refreshing approved tools…"}
        </Text>
      </Group>
    );
  return null;
}

function ApprovedToolDetails({ tool }: { tool: ToolDescriptor }) {
  return (
    <section className="inspector-approved-tool">
      <Text fw={600} size="sm">
        {tool.name}
      </Text>
      <ToolDescription text={tool.description ?? ""} />
      <details className="tool-metadata">
        <summary>Input schema</summary>
        <CodeBlock
          language="json"
          code={JSON.stringify(tool.inputSchema, null, 2)}
        />
      </details>
      {tool.outputSchema != null && (
        <details className="tool-metadata">
          <summary>Output schema</summary>
          <CodeBlock
            language="json"
            code={JSON.stringify(tool.outputSchema, null, 2)}
          />
        </details>
      )}
    </section>
  );
}

function InspectorToolGroup({
  brain,
  profile,
  connection,
  canConfigure,
  approved,
  detailed = false,
}: {
  brain: Brain;
  profile: McpProfile;
  connection: components["schemas"]["McpConnectionSummary"];
  canConfigure: boolean;
  approved: MetadataRead;
  detailed?: boolean;
}) {
  const [offset, setOffset] = useState(0);
  const selectedEnvironment =
    profile.environment_id ?? connection.environment_id ?? null;
  useEffect(() => {
    setOffset(0);
  }, [
    profile.id,
    profile.revision,
    connection.id,
    connection.revision,
    selectedEnvironment,
  ]);
  const usable =
    profile.enabled &&
    profile.rights.use_profile &&
    !brain.archived &&
    connection.enabled &&
    connection.availability === "configured";
  const query = useQuery({
    queryKey: [
      "mcp",
      brain.id,
      "inspector-discovery",
      profile.id,
      profile.revision,
      connection.id,
      connection.revision,
      selectedEnvironment,
      offset,
    ],
    enabled: !canConfigure && usable,
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/mcp/discover", {
          params: { path: { brain: brain.id } },
          body: {
            profile_id: profile.id,
            environment_id: selectedEnvironment,
            operation_id: null,
            offset,
          },
          signal,
        }),
      ),
    refetchInterval: (query) => (query.state.error ? false : 5000),
    retry: retryMetadataRead,
    retryDelay: metadataRetryDelay,
    gcTime: 0,
  });
  const data = !canConfigure && usable && !query.error ? query.data : undefined;
  useEffect(() => {
    if (data && offset > 0 && offset >= data.total) setOffset(0);
  }, [data, offset]);
  const unavailable = data?.unavailable_connections.find(
    (item) => item.connection_id === connection.id,
  );
  const tools = canConfigure
    ? approved.tools
    : data?.tools
        .filter((item) => item.connection_id === connection.id)
        .map((item) => item.tool);
  const read = canConfigure
    ? approved
    : {
        tools,
        error: query.error,
        pending: query.isPending,
        refreshing: query.isFetching,
        reload: () => void query.refetch(),
      };
  const blocked = canConfigure
    ? null
    : !profile.rights.use_profile
      ? "Use access required"
      : !profile.enabled
        ? "Tool group disabled"
        : brain.archived
          ? "Brain archived"
          : !connection.enabled || connection.availability !== "configured"
            ? availabilityLabel(connection.availability)
            : null;
  const count =
    blocked ??
    (unavailable
      ? "Unavailable"
      : read.error
        ? read.error instanceof RequestError &&
          [401, 403].includes(read.error.status)
          ? "Access unavailable"
          : "Unavailable"
        : tools == null
          ? "Loading…"
          : canConfigure
            ? `${tools.length} approved tools`
            : `${tools.length} on this page`);
  return (
    <section
      className="inspector-tool-group"
      aria-label={`Tool group ${profile.name}`}
    >
      <Group justify="space-between">
        <BookOpen size={18} />
        <strong>{profile.name}</strong>
        <Badge color="gray" variant="light">
          {count}
        </Badge>
      </Group>
      {profile.description && (
        <p className="management-muted">{profile.description}</p>
      )}
      {blocked ? (
        <Text size="xs" c="dimmed" mt="sm">
          {blocked === "Use access required"
            ? "Use access is required to read this group’s tools."
            : "Tool discovery is paused for this configuration."}
        </Text>
      ) : (
        <>
          <ToolReadNotice read={read} />
          {unavailable ? (
            <Text size="sm" c="dimmed" mt="sm">
              {availabilityLabel(unavailable.reason)}
            </Text>
          ) : (
            !read.error &&
            tools != null && (
              <>
                {tools.map((tool) =>
                  detailed ? (
                    <ApprovedToolDetails key={tool.name} tool={tool} />
                  ) : (
                    <div className="inspector-tool-name" key={tool.name}>
                      <FileText size={14} />
                      <code>{tool.name}</code>
                    </div>
                  ),
                )}
                {tools.length === 0 && (
                  <Text size="xs" c="dimmed" mt="sm">
                    {canConfigure
                      ? "No approved tools in this connector."
                      : data?.total === 0
                        ? "No eligible approved tools in this group."
                        : "No tools from this connection on this page. Other pages may contain more."}
                  </Text>
                )}
              </>
            )
          )}
          {!canConfigure && data && (data.total > 20 || offset > 0) && (
            <Group mt="sm" gap="xs" className="inspector-tool-pagination">
              <Button
                variant="default"
                size="compact-xs"
                disabled={offset === 0}
                onClick={() => setOffset((value) => Math.max(0, value - 20))}
              >
                Previous tools
              </Button>
              <Text size="xs" c="dimmed">
                {Math.min(offset + 1, data.total)}–
                {Math.min(offset + 20, data.total)} of {data.total} group tools
              </Text>
              <Button
                variant="default"
                size="compact-xs"
                disabled={data.next_offset == null}
                onClick={() => setOffset(data.next_offset!)}
              >
                Next tools
              </Button>
            </Group>
          )}
        </>
      )}
    </section>
  );
}

function DiscoveryDialog({
  brain,
  profile,
  environments,
  close,
  run,
  connectionId,
  connectionName,
}: {
  connectionId?: string | null;
  connectionName?: string;
  brain: Brain;
  profile: McpProfile;
  environments: { value: string; label: string }[];
  close: () => void;
  run: (tool: McpTool, environment: string | null) => void;
}) {
  const [environment, setEnvironment] = useState(
    profile.environment_id ?? null,
  );
  const [offset, setOffset] = useState(0);
  const query = useQuery({
    queryKey: ["mcp", brain.id, "discovery", profile.id, environment, offset],
    queryFn: async ({ signal }) =>
      result(
        await client.POST("/api/brains/{brain}/mcp/discover", {
          params: { path: { brain: brain.id } },
          body: {
            profile_id: profile.id,
            environment_id: environment,
            operation_id: null,
            offset,
          },
          signal,
        }),
      ),
    refetchInterval: (query) => (query.state.error ? false : 5000),
    retry: retryMetadataRead,
    retryDelay: metadataRetryDelay,
    gcTime: 0,
  });
  const data = query.error ? undefined : query.data;
  return (
    <Modal
      opened
      onClose={close}
      title={`Tools · ${connectionName ?? profile.name}`}
      size="xl"
    >
      <Stack>
        <Text size="sm">
          Available tools from this approved group. Choose a tool to run it.
        </Text>
        {environments.length > 0 && (
          <Select
            label="Tool environment"
            value={environment}
            clearable={!profile.environment_id}
            disabled={!!profile.environment_id}
            placeholder="Brain-wide"
            data={environments}
            onChange={(v) => {
              setEnvironment(v);
              setOffset(0);
            }}
          />
        )}
        <ToolReadNotice
          read={{
            error: query.error,
            pending: query.isPending,
            refreshing: query.isFetching,
            reload: () => void query.refetch(),
          }}
        />
        {data && (
          <>
            <Text fw={600}>{data.total} tools · Approved catalogue</Text>
            {connectionId && (
              <Text size="xs" c="dimmed">
                Showing {connectionName} tools on this page of {profile.name}.
                Other pages may contain more.
              </Text>
            )}
            {data.unavailable_connections.map((c) => (
              <Alert color="yellow" key={c.connection_id}>
                {c.name}: {availabilityLabel(c.reason)}
              </Alert>
            ))}
            {data.total === 0 && (
              <Text>
                No eligible cached tools are available in this profile.
              </Text>
            )}
            {data.tools
              .filter(
                (item) => !connectionId || item.connection_id === connectionId,
              )
              .map((item) => (
                <Card
                  withBorder
                  key={`${item.connection_id}:${item.tool.name}`}
                  data-testid="mcp-cached-tool"
                >
                  <Text fw={600}>
                    {item.connection_name} · {item.tool.name}
                  </Text>
                  <ToolDescription text={item.tool.description ?? ""} />
                  <Button
                    size="xs"
                    mt="sm"
                    onClick={() => run(item, environment)}
                  >
                    Run tool
                  </Button>
                  <details className="tool-metadata">
                    <summary>Tool details</summary>
                    <Text size="xs" c="dimmed">
                      Connection {item.connection_id}
                    </Text>
                    <details>
                      <summary>Input schema</summary>
                      <CodeBlock
                        language="json"
                        code={JSON.stringify(item.tool.inputSchema, null, 2)}
                      />
                    </details>
                    {item.tool.outputSchema != null && (
                      <details>
                        <summary>Output schema</summary>
                        <CodeBlock
                          language="json"
                          code={JSON.stringify(item.tool.outputSchema, null, 2)}
                        />
                      </details>
                    )}
                    {item.tool.annotations != null && (
                      <details>
                        <summary>Behavior hints</summary>
                        <Text size="xs">
                          Hints do not authorize use or establish actual
                          behavior.
                        </Text>
                        <CodeBlock
                          language="json"
                          code={JSON.stringify(item.tool.annotations, null, 2)}
                        />
                      </details>
                    )}
                  </details>
                </Card>
              ))}
            {(data.total > 20 || offset > 0) && (
              <Group>
                <Button
                  variant="light"
                  disabled={offset === 0}
                  onClick={() => setOffset((v) => Math.max(0, v - 20))}
                >
                  Previous tools
                </Button>
                <Text size="sm">
                  {Math.min(offset + 1, data.total)}–
                  {Math.min(offset + 20, data.total)} of {data.total}
                </Text>
                <Button
                  variant="light"
                  disabled={data.next_offset == null}
                  onClick={() => setOffset(data.next_offset!)}
                >
                  Next tools
                </Button>
              </Group>
            )}
          </>
        )}
      </Stack>
    </Modal>
  );
}

function ConnectionObservation({
  id,
  status,
  pending,
  failed,
  checkedAt,
}: {
  id: string;
  status?: components["schemas"]["McpRuntimeStatus"];
  pending: boolean;
  failed: boolean;
  checkedAt: number;
}) {
  if (failed)
    return (
      <Text size="xs" c="dimmed">
        Current session status unavailable
      </Text>
    );
  if (pending || !status)
    return (
      <Text size="xs" c="dimmed">
        Checking observed sessions…
      </Text>
    );
  const visible = status.instances.filter((i) => i.connection_id === id);
  const active = visible.filter(
    (i) =>
      i.current_configuration &&
      ["starting", "ready", "draining"].includes(i.state),
  );
  const states = [...new Set(active.map((i) => i.state))];
  const previous = visible.filter(
    (i) =>
      !i.current_configuration &&
      ["starting", "ready", "draining"].includes(i.state),
  );
  return (
    <Stack gap="xs">
      <div className="connection-runtime">
        <StatusDot
          tone={active.length ? "accent" : "muted"}
          live={active.some(
            (i) => i.state === "starting" || i.active_calls > 0,
          )}
        />
        <span>
          {active.length
            ? `Observed session${active.length === 1 ? "" : "s"} · ${states.map((s) => s[0].toUpperCase() + s.slice(1)).join(" / ")}`
            : "No current session observed"}
        </span>
      </div>
      <span className="connection-runtime-note">
        {active.length
          ? `${active.reduce((n, i) => n + i.active_calls, 0)} active calls · `
          : ""}
        Checked {new Date(checkedAt).toLocaleTimeString()} · up to 50 visible
        sessions, active first
      </span>
      {previous.length > 0 && (
        <Text size="xs" c="dimmed">
          {previous.length} earlier-configuration session
          {previous.length === 1 ? "" : "s"} still observed.
        </Text>
      )}
      {!previous.length &&
        !active.length &&
        visible.some((i) => !i.current_configuration) && (
          <Text size="xs" c="dimmed">
            Previous configuration sessions are excluded.
          </Text>
        )}
    </Stack>
  );
}

function ConnectionTest({
  brain,
  id,
  catalogue,
  owner,
  close,
  tools,
}: {
  brain: Brain;
  id: string;
  catalogue: McpCatalogue;
  owner: boolean;
  close: () => void;
  tools: (profile: string) => void;
}) {
  const summary = catalogue.connections.find((c) => c.id === id)!;
  const definition = catalogue.definitions.find(
    (d) => d.key === summary.definition_key,
  );
  const detail = useQuery({
    queryKey: ["mcp", brain.id, "test-detail", id, summary.revision],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/connections/{id}", {
          params: { path: { brain: brain.id, id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  const eligible =
    owner &&
    summary.enabled &&
    summary.availability === "configured" &&
    definition?.enabled &&
    definition.transport === "streamable_http" &&
    summary.placement === "central" &&
    detail.data?.summary.revision === summary.revision &&
    !detail.data.credential_alias &&
    Object.keys(detail.data.configuration ?? {}).length === 0 &&
    !detail.error;
  const probe = useMutation({
    mutationFn: async () => {
      if (!eligible || !detail.data)
        throw new Error("Refresh this connection before testing.");
      const value = result(
        await client.POST("/api/mcp/definitions/inspect-http", {
          body: { name: summary.name, url: detail.data.target },
        }),
      );
      return {
        count: value.tools.length,
        checkedAt: new Date().toLocaleString(),
      };
    },
    retry: false,
  });
  const group = catalogue.profiles.find(
    (p) => p.enabled && p.rights.use_profile && p.connection_ids.includes(id),
  );
  return (
    <Modal
      opened
      onClose={close}
      title={`Test connection · ${summary.name}`}
      size="md"
    >
      <Stack>
        {detail.isPending && <Loader size="sm" />}
        {detail.error && <Alert color="red">{detail.error.message}</Alert>}
        {!detail.error &&
          detail.data &&
          (!eligible ? (
            <>
              <Text size="sm">
                {!summary.enabled ||
                summary.availability !== "configured" ||
                !definition?.enabled
                  ? "This connection is paused or unavailable. Resolve its configuration before testing."
                  : !owner
                    ? "Anonymous server checks are available to the installation owner."
                    : summary.placement !== "central"
                      ? "This connection runs on a device. Test it with a permitted tool call."
                      : detail.data.credential_alias ||
                          Object.keys(detail.data.configuration ?? {}).length
                        ? "This connection uses credentials or custom settings. Test it with a permitted tool call."
                        : "This connector requires a permitted tool call to test it."}
              </Text>
              {group &&
              summary.enabled &&
              summary.availability === "configured" &&
              definition?.enabled ? (
                <Button onClick={() => tools(group.id)}>
                  Open {group.name} tools
                </Button>
              ) : (
                <Text size="sm" c="dimmed">
                  {summary.enabled &&
                  summary.availability === "configured" &&
                  definition?.enabled
                    ? "Ask a tool-group administrator for Use access to this connection."
                    : "No tool test is available for this configuration."}
                </Text>
              )}
            </>
          ) : (
            <>
              <Text size="sm">
                Check the anonymous server handshake and list its tool metadata.
                No tool is executed, and approved tools and permissions stay
                unchanged.
              </Text>
              {probe.isPending && (
                <Text size="xs" role="status">
                  Checking server…
                </Text>
              )}
              {probe.error && (
                <Alert color="red" title="Server check failed">
                  {probe.error.message}
                </Alert>
              )}
              {probe.data && !probe.isPending && !probe.error && (
                <Alert
                  color="brand"
                  title={`Server responded · ${probe.data.count} tools listed`}
                >
                  <Text size="sm">{probe.data.checkedAt}</Text>
                  <Text size="xs">
                    Anonymous check; no tools executed. This does not verify
                    tool permissions or future availability.
                  </Text>
                </Alert>
              )}
              <Button
                loading={probe.isPending}
                onClick={() => {
                  probe.reset();
                  probe.mutate();
                }}
              >
                Check server
              </Button>
            </>
          ))}
      </Stack>
    </Modal>
  );
}
