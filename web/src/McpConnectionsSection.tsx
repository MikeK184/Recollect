import type { Dispatch, SetStateAction } from "react";
import {
  Activity,
  ArrowRight,
  Box,
  CircleCheck,
  CircleDashed,
  FileText,
  Globe,
  HardDrive,
  Layers,
  MoreHorizontal,
  Monitor,
  Pause,
  Pencil,
  Play,
  Plus,
  Server,
  Unplug,
  X,
} from "lucide-react";
import type { UseQueryResult } from "@tanstack/react-query";
import type { Brain } from "./api";
import type { components } from "./api-schema";
import type { ServerDraft } from "./features/connections/mcpConfig";
import { EmptyState } from "./components/AsyncState";
import { McpConnectorIcon } from "./components/McpConnectorIcon";
import { staggerStyle } from "./components/Motion";
import {
  Alert,
  Badge,
  Button,
  CopyAction,
  Count,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  IconButton,
  StatusBadge,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableWrap,
  Tabs,
  TabsList,
  TabsTrigger,
  Tile,
} from "./components/ui";
import { Disclosure, FactRows, SectionLabel } from "./features/connections/ui";
import { cn } from "./lib/utils";
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

const mutedText = "m-0 text-label text-muted-foreground";

/** Configured status: colour, icon and word together; history is separate. */
function ConnectionStatus({ item }: { item: ConnectionSummary }) {
  if (item.availability !== "configured")
    return (
      <StatusBadge state="attention">
        {availabilityLabel(item.availability)}
      </StatusBadge>
    );
  return item.last_successful_call_at ? (
    <Badge variant="brand">
      <CircleCheck aria-hidden="true" />
      Last call succeeded
    </Badge>
  ) : (
    <Badge>
      <CircleDashed aria-hidden="true" />
      Configured · not tested
    </Badge>
  );
}

function ScopeBadges({
  item,
  environmentName,
}: {
  item: ConnectionSummary;
  environmentName: (id: string | null | undefined) => string;
}) {
  const PlacementIcon = item.placement === "local" ? Monitor : Server;
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-1.5">
      <Badge
        variant="outline"
        className="max-w-[200px]"
        title={environmentName(item.environment_id)}
      >
        <Layers aria-hidden="true" />
        <span className="truncate">{environmentName(item.environment_id)}</span>
      </Badge>
      <Badge variant="outline">
        <PlacementIcon aria-hidden="true" />
        {placementLabel(item.placement)}
      </Badge>
    </div>
  );
}

function ConnectionRow({
  item,
  index,
  brain,
  data,
  inspected,
  visibleDetail,
  controls,
  environmentName,
}: {
  item: ConnectionSummary;
  index: number;
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
  const selected = inspected?.id === item.id;
  const locked = !!connection && connection !== "new" && connection !== item.id;
  return (
    <article
      className={cn(
        "group/row relative flex animate-enter items-center gap-3 rounded-lg border bg-card p-3.5 transition-[background-color,border-color,box-shadow,transform] duration-fast ease-standard",
        selected
          ? "selected border-brand/40 bg-brand-soft/35 shadow-xs"
          : "border-border hover:-translate-y-px hover:border-border-strong hover:shadow-sm",
      )}
      style={staggerStyle(index)}
      data-testid="mcp-connection"
    >
      <McpConnectorIcon
        definition={data.definitions.find((d) => d.key === item.definition_key)}
      />
      <div className="grid min-w-0 flex-1 gap-1">
        <button
          type="button"
          className="w-fit max-w-full truncate rounded-sm text-left text-body font-semibold text-foreground outline-none after:absolute after:inset-0 after:rounded-lg after:content-[''] hover:underline hover:underline-offset-[3px] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:cursor-not-allowed disabled:opacity-60 disabled:hover:no-underline"
          disabled={locked}
          aria-current={selected ? "true" : undefined}
          onClick={() => {
            setInspectedId(item.id);
            setInspectorDismissed(false);
            setInspectorTab("overview");
          }}
        >
          {item.name}
        </button>
        <small
          className={cn(
            "truncate text-caption text-muted-foreground",
            item.id === inspected?.id && visibleDetail && "mono",
          )}
        >
          {item.id === inspected?.id && visibleDetail
            ? visibleDetail.target
            : (data.definitions.find((d) => d.key === item.definition_key)
                ?.name ?? "Approved connector")}
        </small>
        <ScopeBadges item={item} environmentName={environmentName} />
      </div>
      <div className="relative z-[1] flex shrink-0 flex-col items-end gap-1">
        <ConnectionStatus item={item} />
        <small
          className="text-caption text-muted-foreground"
          title={
            item.last_successful_call_at
              ? new Date(item.last_successful_call_at).toLocaleString()
              : undefined
          }
        >
          {item.last_successful_call_at ? (
            <>
              Last call ·{" "}
              <span className="mono">
                {new Date(item.last_successful_call_at).toLocaleDateString()}
              </span>
            </>
          ) : (
            "No observed successful call"
          )}
        </small>
      </div>
      <div className="relative z-[1] flex shrink-0 items-center gap-1">
        <Button
          variant="ghost"
          size="sm"
          aria-label={"View " + item.name}
          disabled={locked}
          onClick={() => {
            setInspectedId(item.id);
            setInspectorDismissed(false);
          }}
        >
          Inspect
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <IconButton
              size="icon-sm"
              label={"Actions for " + item.name}
              disabled={!!connection}
            >
              <MoreHorizontal aria-hidden="true" />
            </IconButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              onSelect={() => {
                if (connection) return;
                setInspectedId(item.id);
                setInspectorDismissed(false);
                setInspectorTab("overview");
              }}
            >
              <Box aria-hidden="true" />
              View connection
            </DropdownMenuItem>
            {data.can_configure && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  disabled={brain.archived}
                  onSelect={() => {
                    if (connection) return;
                    setImportedDraft(undefined);
                    setInspectedId(item.id);
                    setInspectorDismissed(false);
                    setInspectorTab("overview");
                    setConnection(item.id);
                  }}
                >
                  <Pencil aria-hidden="true" />
                  Edit connection
                </DropdownMenuItem>
                <DropdownMenuItem
                  disabled={brain.archived}
                  onSelect={() => !connection && setPower(item.id)}
                >
                  {item.enabled ? (
                    <Pause aria-hidden="true" />
                  ) : (
                    <Play aria-hidden="true" />
                  )}
                  {item.enabled ? "Pause use…" : "Enable use…"}
                </DropdownMenuItem>
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
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
    <header className="flex items-start gap-3 border-b border-border px-4 py-3.5">
      <McpConnectorIcon
        definition={data.definitions.find(
          (d) => d.key === inspected.definition_key,
        )}
      />
      <div className="min-w-0 flex-1">
        {connection === inspected.id ? (
          <div ref={setConnectionNameMount} />
        ) : (
          <h2 className="m-0 truncate text-title font-semibold text-foreground">
            {inspected.name}
          </h2>
        )}
        <p
          className={cn(
            "m-0 mt-0.5 truncate text-caption text-muted-foreground",
            visibleDetail && "mono",
          )}
        >
          {visibleDetail?.target ?? "Approved MCP connector"}
        </p>
      </div>
      <div className="-mr-1 flex shrink-0 items-center gap-0.5">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <IconButton
              size="icon-sm"
              label="Connection actions"
              disabled={!!connection}
            >
              <MoreHorizontal aria-hidden="true" />
            </IconButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              disabled={!!connection}
              onSelect={() => !connection && setInspectorTab("tools")}
            >
              <FileText aria-hidden="true" />
              View approved tools
            </DropdownMenuItem>
            <DropdownMenuItem
              disabled={!!connection}
              onSelect={() => !connection && setInspectorTab("activity")}
            >
              <Activity aria-hidden="true" />
              View connection history
            </DropdownMenuItem>
            {data.can_configure && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  disabled={brain.archived}
                  onSelect={() => {
                    if (connection) return;
                    setImportedDraft(undefined);
                    setInspectorTab("overview");
                    setConnection(inspected.id);
                  }}
                >
                  <Pencil aria-hidden="true" />
                  Edit connection
                </DropdownMenuItem>
                <DropdownMenuItem
                  disabled={brain.archived || !!connection}
                  onSelect={() => !connection && setPower(inspected.id)}
                >
                  {inspected.enabled ? (
                    <Pause aria-hidden="true" />
                  ) : (
                    <Play aria-hidden="true" />
                  )}
                  {inspected.enabled ? "Pause use…" : "Enable use…"}
                </DropdownMenuItem>
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
        <IconButton
          size="icon-sm"
          label="Close connection details"
          disabled={!!connection}
          onClick={() => setInspectorDismissed(true)}
        >
          <X aria-hidden="true" />
        </IconButton>
      </div>
    </header>
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
    <section className="grid gap-2.5 border-t border-border pt-4">
      <SectionLabel
        actions={
          inspected.last_successful_call_at ? (
            <StatusBadge state="positive">Successful call recorded</StatusBadge>
          ) : (
            <StatusBadge state="attention">Configured · not tested</StatusBadge>
          )
        }
      >
        Test status
      </SectionLabel>
      <p className={mutedText}>
        {inspected.last_successful_call_at ? (
          <>
            Last successful call:{" "}
            <span className="mono text-foreground">
              {new Date(inspected.last_successful_call_at).toLocaleDateString()}
            </span>
          </>
        ) : (
          "No successful call recorded."
        )}
      </p>
      <p className="m-0 text-caption text-muted-foreground">
        Test checks the anonymous server handshake and tool metadata. It does
        not run tools or update approved metadata.
      </p>
      <Disclosure summary="Runtime sessions">
        <ConnectionObservation
          id={inspected.id}
          status={runtime.error ? undefined : runtime.data}
          pending={runtime.isPending}
          failed={!!runtime.error}
          checkedAt={runtime.dataUpdatedAt}
        />
      </Disclosure>
      {data.can_configure && (
        <div className="flex flex-wrap gap-2 pt-1">
          <Button
            size="sm"
            aria-label="Test connection"
            disabled={brain.archived || !inspected.enabled || !!connection}
            onClick={() => setTesting(inspected.id)}
          >
            <Play aria-hidden="true" />
            Test connection
          </Button>
          <Button
            size="sm"
            variant="secondary"
            aria-label="Edit connection"
            disabled={brain.archived || !!connection}
            onClick={() => {
              setImportedDraft(undefined);
              setInspectorTab("overview");
              setConnection(inspected.id);
            }}
          >
            <Pencil aria-hidden="true" />
            Edit
          </Button>
        </div>
      )}
    </section>
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
    <section className="grid gap-2.5 border-t border-border pt-4">
      <SectionLabel
        actions={
          <Button
            variant="ghost"
            size="sm"
            className="-mr-2"
            onClick={() => onSectionChange?.("profiles")}
          >
            Manage access
            <ArrowRight aria-hidden="true" />
          </Button>
        }
      >
        Tool access
      </SectionLabel>
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
        <p className={mutedText}>No visible tool groups.</p>
      )}
      {data.can_configure && (
        <Button
          variant="ghost"
          size="sm"
          className="-ml-2.5 justify-self-start"
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
        <section className="grid gap-1">
          <SectionLabel>Configuration</SectionLabel>
          <FactRows
            items={[
              {
                term: "Target",
                value: detailError ? (
                  "Unavailable"
                ) : visibleDetail ? (
                  <span className="flex min-w-0 items-center justify-end gap-1">
                    <code className="mono min-w-0 text-caption [overflow-wrap:anywhere]">
                      {visibleDetail.target}
                    </code>
                    <CopyAction
                      iconOnly
                      variant="ghost"
                      size="icon-sm"
                      label="Copy target URL"
                      value={visibleDetail.target}
                    />
                  </span>
                ) : data.can_configure ? (
                  "Loading…"
                ) : (
                  "Visible to managers"
                ),
              },
              {
                term: "Scope",
                value: environmentName(inspected.environment_id),
              },
              {
                term: "Runs on",
                value: placementLabel(inspected.placement),
              },
            ]}
          />
        </section>
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
  return data.can_configure ? (
    <div className="grid gap-3">
      <p className="m-0 text-caption text-muted-foreground">
        Approved metadata. Tool access is granted independently.
      </p>
      <ToolReadNotice read={approvedTools} />
      {visibleDefinition?.tools.length === 0 && (
        <p className={mutedText}>No approved tools in this connector.</p>
      )}
      {!!visibleDefinition?.tools.length && (
        <div className="grid">
          {visibleDefinition.tools.map((tool) => (
            <ApprovedToolDetails key={tool.name} tool={tool} />
          ))}
        </div>
      )}
    </div>
  ) : (
    <div className="grid gap-3">
      <p className="m-0 text-caption text-muted-foreground">
        Approved tools from your permitted groups. No server connection is
        started.
      </p>
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
      {!data.profiles.some((p) => p.connection_ids.includes(inspected.id)) && (
        <p className={mutedText}>
          No visible tool groups. Use access is required to inspect tools.
        </p>
      )}
    </div>
  );
}

function InspectorActivity({
  inspected,
  controls,
}: Pick<InspectorProps, "inspected" | "controls">) {
  return (
    <div className="grid gap-3">
      <p className="m-0 text-body text-foreground">
        {inspected.last_successful_call_at ? (
          <>
            Last successful call:{" "}
            <span className="mono">
              {new Date(inspected.last_successful_call_at).toLocaleString()}
            </span>
          </>
        ) : (
          "No observed successful call for this configuration."
        )}
      </p>
      <p className="m-0 text-caption text-muted-foreground">
        Inspect tool calls and session diagnostics across this Brain’s permitted
        connections.
      </p>
      <Button
        variant="secondary"
        className="justify-self-start"
        onClick={() => controls.setDiagnostics(true)}
      >
        <Activity aria-hidden="true" />
        Open tool activity
      </Button>
    </div>
  );
}

function ConnectionInspector(props: InspectorProps) {
  const { controls } = props;
  const { connection, inspectorTab, setInspectorTab } = controls;
  return (
    <aside
      className="sticky top-4 flex max-h-[calc(100vh-96px)] min-w-0 animate-pane-in flex-col overflow-hidden rounded-lg border border-border bg-card"
      aria-label="Connection details"
    >
      <InspectorHeading {...props} />
      <Tabs
        value={inspectorTab ?? "overview"}
        onValueChange={(value) => {
          if (!connection) setInspectorTab(value);
        }}
        className="min-h-0 flex-1 gap-0"
      >
        <TabsList className="shrink-0 px-2">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="tools" disabled={!!connection}>
            Tools
          </TabsTrigger>
          <TabsTrigger value="activity" disabled={!!connection}>
            Activity
          </TabsTrigger>
        </TabsList>
        <div
          key={inspectorTab}
          className="grid min-h-0 animate-enter content-start gap-4 overflow-auto p-4 scrollbar-thin"
        >
          {inspectorTab === "overview" && <InspectorOverview {...props} />}
          {inspectorTab === "tools" && <InspectorTools {...props} />}
          {inspectorTab === "activity" && <InspectorActivity {...props} />}
        </div>
      </Tabs>
    </aside>
  );
}

function transportLabel(transport: string) {
  return transport === "streamable_http"
    ? "HTTP"
    : transport === "stdio"
      ? "Local"
      : transport;
}

/** Approved connectors listed inside the empty state for configurers. */
function ApprovedConnectors({
  definitions,
  onAdd,
}: {
  definitions: McpCatalogue["definitions"];
  onAdd?: (key: string) => void;
}) {
  return (
    <div className="mx-auto mt-2 grid w-full max-w-[760px] gap-2 text-left">
      <div className="flex items-center gap-2">
        <h3 className="m-0 text-label font-medium text-foreground">
          Approved connectors you can add
        </h3>
        <Count>{definitions.length}</Count>
      </div>
      <TableWrap>
        <Table>
          <TableHead>
            <tr>
              <TableHeader>Connector</TableHeader>
              <TableHeader className="w-24">Transport</TableHeader>
              <TableHeader className="w-32">Status</TableHeader>
              {onAdd && (
                <TableHeader className="w-[88px] text-right">Action</TableHeader>
              )}
            </tr>
          </TableHead>
          <TableBody>
            {definitions.map((definition) => {
              const TransportIcon =
                definition.transport === "stdio" ? HardDrive : Globe;
              return (
                <TableRow key={definition.key}>
                  <TableCell className="h-auto py-2">
                    <div className="flex min-w-0 items-center gap-3">
                      <Tile size="sm">
                        <TransportIcon aria-hidden="true" />
                      </Tile>
                      <div className="min-w-0">
                        <div className="truncate font-medium text-foreground">
                          {definition.name}
                        </div>
                        {definition.description && (
                          <div className="line-clamp-2 text-caption text-muted-foreground">
                            {definition.description}
                          </div>
                        )}
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline">
                      {transportLabel(definition.transport)}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <StatusBadge state="positive">Approved</StatusBadge>
                  </TableCell>
                  {onAdd && (
                    <TableCell className="text-right">
                      <Button
                        size="sm"
                        variant="secondary"
                        aria-label={`Add ${definition.name}`}
                        onClick={() => onAdd(definition.key)}
                      >
                        <Plus aria-hidden="true" />
                        Add
                      </Button>
                    </TableCell>
                  )}
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </TableWrap>
      <p className="m-0 max-w-[400px] text-caption text-muted-foreground">
        Connector approval controls what can be configured. Brain access
        controls who can use it.
      </p>
    </div>
  );
}

export function ConnectionsSection({
  matchingConnections,
  initialConnection,
  connectionSearch,
  environment,
  setupSaved,
  setSetupSaved,
  onAddConnector,
  ...inspector
}: Omit<InspectorProps, "inspected"> & {
  inspected: ConnectionSummary | undefined;
  matchingConnections: ConnectionSummary[];
  initialConnection?: string;
  connectionSearch: string;
  environment: string | null;
  setupSaved: boolean;
  setSetupSaved: Setter<boolean>;
  /** Start a connection from an approved connector in the empty state. */
  onAddConnector?: (definitionKey: string) => void;
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
  const approved = data.definitions.filter((d) => d.enabled);
  return (
    <>
      {setupSaved && (
        <Alert
          variant="brand"
          role="status"
          icon={<CircleCheck aria-hidden="true" />}
          title="Connection saved · not tested"
          className="animate-enter"
          actions={
            <>
              {onSectionChange && (
                <Button size="sm" onClick={() => onSectionChange("profiles")}>
                  Manage tool access
                </Button>
              )}
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setSetupSaved(false)}
              >
                Dismiss
              </Button>
            </>
          }
        >
          Saved without a tool call. Tool access is granted separately.
        </Alert>
      )}
      {data.can_configure && data.definitions.length === 0 && (
        <Alert role="status" title="No approved connectors">
          Start with Add connection to register an MCP connector. Then choose
          where it runs and which agents may use it.
        </Alert>
      )}
      {!data.connections.length && (
        <section
          className="rounded-lg border border-border bg-card pb-6"
          aria-label="No connections yet"
        >
          <EmptyState
            icon={Unplug}
            title="No connections yet"
            description="Add an approved connector, then include it in a tool group with explicit tool-use permissions."
            className="pb-4 [&_p]:max-w-[440px]"
          />
          {data.can_configure && approved.length > 0 && (
            <div className="px-6">
              <ApprovedConnectors
                definitions={approved}
                onAdd={onAddConnector}
              />
            </div>
          )}
        </section>
      )}

      {runtime.error && (
        <Alert
          role="status"
          title="Session status unavailable"
          actions={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void runtime.refetch()}
            >
              Retry status
            </Button>
          }
        >
          {runtime.error.message}
        </Alert>
      )}
      {(connectionSearch || environment) && (
        <p className="mono m-0 text-caption text-muted-foreground">
          {matchingConnections.length} of {data.connections.length} loaded
          connections
        </p>
      )}
      {!!data.connections.length && !matchingConnections.length && (
        <p className={mutedText} role="status">
          No connections match these filters.
        </p>
      )}
      {initialConnection &&
        !data.connections.some((c) => c.id === initialConnection) && (
          <p className={mutedText} role="status">
            This connection is unavailable or no longer visible to you.
          </p>
        )}
      {(!!matchingConnections.length || inspected) && (
        <div
          className={cn(
            "grid items-start gap-4",
            inspected && "grid-cols-[minmax(0,1fr)_400px]",
          )}
        >
          <div className="grid min-w-0 gap-2.5">
            {matchingConnections.map((item, index) => (
              <ConnectionRow
                key={item.id}
                index={index}
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
            <ConnectionInspector
              {...inspector}
              inspected={inspected}
            />
          )}
        </div>
      )}
      {onManageRunners && (
        <section
          className="flex items-center gap-3.5 rounded-lg border border-border bg-card p-4"
          aria-label="Need a private network?"
        >
          <Tile>
            <Server aria-hidden="true" />
          </Tile>
          <div className="min-w-0 flex-1">
            <h2 className="m-0 text-title font-semibold text-foreground">
              Need a private network?
            </h2>
            <p className="m-0 mt-0.5 text-body text-muted-foreground">
              Use a Private Runner to reach tools inside your network.
            </p>
          </div>
          <Button variant="ghost" onClick={onManageRunners}>
            Manage runners
            <ArrowRight aria-hidden="true" />
          </Button>
        </section>
      )}
    </>
  );
}

