import type { Dispatch, SetStateAction } from "react";
import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Group,
  Menu,
  Stack,
  Tabs,
  Text,
} from "@mantine/core";
import {
  Activity,
  Box,
  Copy,
  Database,
  FileText,
  Layers,
  MoreHorizontal,
  Monitor,
  Pause,
  Pencil,
  Play,
  Server,
  Unplug,
  X,
} from "lucide-react";
import type { UseQueryResult } from "@tanstack/react-query";
import type { Brain } from "./api";
import type { components } from "./api-schema";
import type { ServerDraft } from "./features/connections/mcpConfig";
import { EmptyState } from "./components/AsyncState";
import { StatusDot } from "./components/StatusDot";
import { McpConnectorIcon } from "./components/McpConnectorIcon";
import { McpConnectionDialog } from "./McpConnectionDialog";
import type { McpCatalogue } from "./McpPanel";
import {
  ApprovedToolDetails,
  availabilityLabel,
  ConnectionObservation,
  InspectorToolGroup,
  placementLabel,
  ToolReadNotice,
  type MetadataRead,
} from "./McpPanelParts";

type Setter<T> = Dispatch<SetStateAction<T>>;
type ConnectionSummary = components["schemas"]["McpConnectionSummary"];
type ConnectionDetail = components["schemas"]["McpConnectionDetail"];
type DefinitionDetail = components["schemas"]["McpDefinitionDetail"];
type RuntimeStatus = components["schemas"]["McpRuntimeStatus"];

export type ConnectionControls = {
  connection: string | null;
  setConnection: Setter<string | null>;
  setImportedDraft: Setter<ServerDraft | undefined>;
  setPower: Setter<string | null>;
  setTesting: Setter<string | null>;
  setDiagnostics: Setter<boolean>;
  setInspectedId: Setter<string | null>;
  setInspectorDismissed: Setter<boolean>;
  inspectorTab: string | null;
  setInspectorTab: Setter<string | null>;
};

type InspectorProps = {
  brain: Brain;
  data: McpCatalogue;
  inspected: ConnectionSummary;
  controls: ConnectionControls;
  runtime: UseQueryResult<RuntimeStatus, Error>;
  detailError: Error | null;
  visibleDetail: ConnectionDetail | undefined;
  visibleDefinition: DefinitionDetail | undefined;
  approvedTools: MetadataRead;
  environments: { value: string; label: string }[];
  environmentName: (id: string | null | undefined) => string;
  connectionNameMount: HTMLDivElement | null;
  setConnectionNameMount: (element: HTMLDivElement | null) => void;
  onSectionChange?: (section: string | null) => void;
  onManageRunners?: () => void;
  refresh: () => void;
  clear: (message: string) => void;
};

function ConnectionRow({
  item,
  brain,
  data,
  inspected,
  visibleDetail,
  controls,
  environmentName,
}: {
  item: ConnectionSummary;
  brain: Brain;
  data: McpCatalogue;
  inspected: ConnectionSummary | undefined;
  visibleDetail: ConnectionDetail | undefined;
  controls: ConnectionControls;
  environmentName: (id: string | null | undefined) => string;
}) {
  const {
    connection,
    setConnection,
    setImportedDraft,
    setPower,
    setInspectedId,
    setInspectorDismissed,
    setInspectorTab,
  } = controls;
  return (
    <article
      className={
        "connection-row" + (inspected?.id === item.id ? " selected" : "")
      }
      data-testid="mcp-connection"
    >
      <McpConnectorIcon
        definition={data.definitions.find((d) => d.key === item.definition_key)}
      />
      <div className="connection-row-content">
        <button
          className="connection-select"
          disabled={
            !!connection && connection !== "new" && connection !== item.id
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
            : (data.definitions.find((d) => d.key === item.definition_key)
                ?.name ?? "Approved connector")}
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
              tone={item.availability === "configured" ? "accent" : "idle"}
              size={8}
            />
          }
          variant="light"
          color={item.availability === "configured" ? "brand" : "gray"}
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
              ? new Date(item.last_successful_call_at).toLocaleString()
              : undefined
          }
        >
          {item.last_successful_call_at
            ? "Last call · " +
              new Date(item.last_successful_call_at).toLocaleDateString()
            : "No observed successful call"}
        </small>
      </div>
      <div className="connection-row-actions">
        <Button
          variant="subtle"
          size="compact-sm"
          aria-label={"View " + item.name}
          disabled={
            !!connection && connection !== "new" && connection !== item.id
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
                    item.enabled ? <Pause size={16} /> : <Play size={16} />
                  }
                  disabled={brain.archived}
                  onClick={() => !connection && setPower(item.id)}
                >
                  {item.enabled ? "Pause use…" : "Enable use…"}
                </Menu.Item>
              </>
            )}
          </Menu.Dropdown>
        </Menu>
      </div>
    </article>
  );
}

function InspectorHeading({
  brain,
  data,
  inspected,
  controls,
  visibleDetail,
  setConnectionNameMount,
}: Pick<
  InspectorProps,
  | "brain"
  | "data"
  | "inspected"
  | "controls"
  | "visibleDetail"
  | "setConnectionNameMount"
>) {
  const {
    connection,
    setConnection,
    setImportedDraft,
    setPower,
    setInspectorDismissed,
    setInspectorTab,
  } = controls;
  return (
    <Group gap="md" mb="lg" className="connection-inspector-heading">
      <McpConnectorIcon
        definition={data.definitions.find(
          (d) => d.key === inspected.definition_key,
        )}
      />
      <div className="connection-inspector-identity">
        {connection === inspected.id ? (
          <div ref={setConnectionNameMount} />
        ) : (
          <h2 className="management-title">{inspected.name}</h2>
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
              onClick={() => !connection && setInspectorTab("tools")}
            >
              View approved tools
            </Menu.Item>
            <Menu.Item
              leftSection={<Activity size={16} />}
              disabled={!!connection}
              onClick={() => !connection && setInspectorTab("activity")}
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
                    inspected.enabled ? <Pause size={16} /> : <Play size={16} />
                  }
                  disabled={brain.archived || !!connection}
                  onClick={() => !connection && setPower(inspected.id)}
                >
                  {inspected.enabled ? "Pause use…" : "Enable use…"}
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
  );
}

function InspectorTestStatus({
  brain,
  data,
  inspected,
  controls,
  runtime,
}: Pick<
  InspectorProps,
  "brain" | "data" | "inspected" | "controls" | "runtime"
>) {
  const {
    connection,
    setConnection,
    setImportedDraft,
    setTesting,
    setInspectorTab,
  } = controls;
  return (
    <div className="inspector-test-section">
      <Group justify="space-between" mb="sm">
        <h3 className="management-title inspector-section-title">
          Test status
        </h3>
        <Badge color={inspected.last_successful_call_at ? "brand" : "yellow"}>
          {inspected.last_successful_call_at
            ? "Successful call recorded"
            : "Configured · not tested"}
        </Badge>
      </Group>
      <p className="management-muted">
        {inspected.last_successful_call_at
          ? "Last successful call: " +
            new Date(inspected.last_successful_call_at).toLocaleDateString()
          : "No successful call recorded."}
      </p>
      <p className="connection-test-meaning">
        Test checks the anonymous server handshake and tool metadata. It does
        not run tools or update approved metadata.
      </p>
      <details className="inspector-session-details">
        <summary>Runtime sessions</summary>
        <ConnectionObservation
          id={inspected.id}
          status={runtime.error ? undefined : runtime.data}
          pending={runtime.isPending}
          failed={!!runtime.error}
          checkedAt={runtime.dataUpdatedAt}
        />
      </details>
      {data.can_configure && (
        <Group mt="md" gap="xs" className="connection-inspector-actions">
          <Button
            size="sm"
            color="green"
            leftSection={<Play size={17} />}
            aria-label="Test connection"
            disabled={brain.archived || !inspected.enabled || !!connection}
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
  );
}

function InspectorToolAccess({
  brain,
  data,
  inspected,
  controls,
  approvedTools,
  onSectionChange,
}: Pick<
  InspectorProps,
  | "brain"
  | "data"
  | "inspected"
  | "controls"
  | "approvedTools"
  | "onSectionChange"
>) {
  const { connection, setPower } = controls;
  return (
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
        .filter((p) => p.connection_ids.includes(inspected.id))
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
      {!data.profiles.some((p) => p.connection_ids.includes(inspected.id)) && (
        <p className="management-muted">No visible tool groups.</p>
      )}
      {data.can_configure && (
        <Button
          variant="subtle"
          size="compact-xs"
          mt="md"
          disabled={brain.archived || !!connection}
          onClick={() => !connection && setPower(inspected.id)}
        >
          {inspected.enabled ? "Pause use…" : "Enable use…"}
        </Button>
      )}
    </section>
  );
}

function InspectorOverview(props: InspectorProps) {
  const {
    brain,
    data,
    inspected,
    controls,
    detailError,
    visibleDetail,
    environments,
    environmentName,
    connectionNameMount,
    refresh,
    clear,
    onManageRunners,
  } = props;
  const { connection, setConnection } = controls;
  return (
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
              {detailError ? (
                "Unavailable"
              ) : visibleDetail ? (
                <span className="inspector-target-value">
                  <code>{visibleDetail.target}</code>
                  <Button
                    variant="subtle"
                    size="compact-xs"
                    aria-label="Copy target URL"
                    onClick={() =>
                      void navigator.clipboard.writeText(visibleDetail.target)
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
              <code>{environmentName(inspected.environment_id)}</code>
            </dd>
            <dt>Runs on</dt>
            <dd>
              <code>{placementLabel(inspected.placement)}</code>
            </dd>
          </dl>
        </>
      )}
      <InspectorTestStatus
        brain={brain}
        data={data}
        inspected={inspected}
        controls={controls}
        runtime={props.runtime}
      />
      <InspectorToolAccess
        brain={brain}
        data={data}
        inspected={inspected}
        controls={controls}
        approvedTools={props.approvedTools}
        onSectionChange={props.onSectionChange}
      />
    </>
  );
}

function InspectorTools({
  brain,
  data,
  inspected,
  visibleDefinition,
  approvedTools,
}: Pick<
  InspectorProps,
  "brain" | "data" | "inspected" | "visibleDefinition" | "approvedTools"
>) {
  return (
    <Stack mt="lg">
      {data.can_configure ? (
        <>
          <Text size="xs" c="dimmed">
            Approved metadata. Tool access is granted independently.
          </Text>
          <ToolReadNotice read={approvedTools} />
          {visibleDefinition?.tools.length === 0 && (
            <Text size="sm" c="dimmed">
              No approved tools in this connector.
            </Text>
          )}
          {visibleDefinition?.tools.map((tool) => (
            <ApprovedToolDetails key={tool.name} tool={tool} />
          ))}
        </>
      ) : (
        <>
          <Text size="xs" c="dimmed">
            Approved tools from your permitted groups. No server connection is
            started.
          </Text>
          {data.profiles
            .filter((p) => p.connection_ids.includes(inspected.id))
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
              No visible tool groups. Use access is required to inspect tools.
            </Text>
          )}
        </>
      )}
    </Stack>
  );
}

function InspectorActivity({
  inspected,
  controls,
}: Pick<InspectorProps, "inspected" | "controls">) {
  return (
    <Stack mt="lg">
      <Text size="sm">
        {inspected.last_successful_call_at
          ? "Last successful call: " +
            new Date(inspected.last_successful_call_at).toLocaleString()
          : "No observed successful call for this configuration."}
      </Text>
      <Text size="xs" c="dimmed">
        Inspect tool calls and session diagnostics across this Brain’s permitted
        connections.
      </Text>
      <Button variant="default" onClick={() => controls.setDiagnostics(true)}>
        Open tool activity
      </Button>
    </Stack>
  );
}

function ConnectionInspector(props: InspectorProps) {
  const { controls } = props;
  const { connection, inspectorTab, setInspectorTab } = controls;
  return (
    <aside
      className="management-surface connection-inspector"
      aria-label="Connection details"
    >
      <InspectorHeading {...props} />
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
      {inspectorTab === "overview" && <InspectorOverview {...props} />}
      {inspectorTab === "tools" && <InspectorTools {...props} />}
      {inspectorTab === "activity" && <InspectorActivity {...props} />}
    </aside>
  );
}

export function ConnectionsSection({
  matchingConnections,
  initialConnection,
  connectionSearch,
  environment,
  setupSaved,
  setSetupSaved,
  ...inspector
}: Omit<InspectorProps, "inspected"> & {
  inspected: ConnectionSummary | undefined;
  matchingConnections: ConnectionSummary[];
  initialConnection?: string;
  connectionSearch: string;
  environment: string | null;
  setupSaved: boolean;
  setSetupSaved: Setter<boolean>;
}) {
  const {
    brain,
    data,
    inspected,
    controls,
    runtime,
    visibleDetail,
    environmentName,
    onSectionChange,
    onManageRunners,
  } = inspector;
  return (
    <>
      {setupSaved && (
        <Alert title="Connection saved · not tested" color="brand">
          <Text size="sm">
            Saved without a tool call. Tool access is granted separately.
          </Text>
          <Group mt="md">
            {onSectionChange && (
              <Button onClick={() => onSectionChange("profiles")}>
                Manage tool access
              </Button>
            )}
            <Button variant="subtle" onClick={() => setSetupSaved(false)}>
              Dismiss
            </Button>
          </Group>
        </Alert>
      )}
      {data.can_configure && data.definitions.length === 0 && (
        <Alert title="No approved connectors">
          Start with Add connection to register an MCP connector. Then choose
          where it runs and which agents may use it.
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
          {matchingConnections.length} of {data.connections.length} loaded
          connections
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
            This connection is unavailable or no longer visible to you.
          </Text>
        )}
      <div
        className={"connections-layout" + (!inspected ? " no-selection" : "")}
      >
        <div>
          {matchingConnections.map((item) => (
            <ConnectionRow
              key={item.id}
              item={item}
              brain={brain}
              data={data}
              inspected={inspected}
              visibleDetail={visibleDetail}
              controls={controls}
              environmentName={environmentName}
            />
          ))}
        </div>
        {inspected && (
          <ConnectionInspector {...inspector} inspected={inspected} />
        )}
      </div>
      {onManageRunners && (
        <div className="management-surface private-runner-note">
          <div className="management-icon">
            <Database />
          </div>
          <div>
            <h3 className="management-title">Need a private network?</h3>
            <p className="management-muted">
              Use a private runner to connect to tools inside your network.
            </p>
          </div>
          <Button variant="subtle" onClick={onManageRunners}>
            Manage runners →
          </Button>
        </div>
      )}
    </>
  );
}
