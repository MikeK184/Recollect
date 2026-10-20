import { useEffect, useState } from "react";
import { BookOpen, Play, ServerCog, Wrench } from "lucide-react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { StatusDot } from "./components/StatusDot";
import { staggerStyle } from "./components/Motion";
import { client, result, RequestError, type Brain } from "./api";
import type { components } from "./api-schema";
import { CodeBlock } from "./components/Markdown";
import { ToolDescription } from "./components/ToolDescription";
import {
  Alert,
  Badge,
  Button,
  Card,
  Modal,
  Select,
  Spinner,
} from "./components/ui";
import { Disclosure } from "./features/connections/ui";
import { cn } from "./lib/utils";
import type { McpTool } from "./McpRunDialog";
import type { McpCatalogue, McpProfile } from "./McpPanel";
import { usePollInterval } from "./usePollInterval";

export const availabilityLabel = (value: string) =>
  ({
    configured: "Configured",
    disabled: "Connection disabled",
    definition_disabled: "Definition disabled",
    configuration_invalid: "Settings need attention",
  })[value] ?? value;

// These reads return approved metadata only. Do not retry denied/invalid reads,
// mutations, inspections or tool execution.
export const retryMetadataRead = (failures: number, error: Error) =>
  failures < 2 &&
  (error instanceof RequestError
    ? [408, 429, 500, 502, 503, 504].includes(error.status)
    : error instanceof TypeError);
export const metadataRetryDelay = (attempt: number) =>
  Math.min(500 * 2 ** attempt, 2000);
export const placementLabel = (placement: string) =>
  (
    ({
      central: "Recollect service",
      local: "Paired device",
      private: "Private runner",
    }) as Record<string, string>
  )[placement] ?? placement;
export type ToolDescriptor = components["schemas"]["McpToolDescriptor"];
export type MetadataRead = {
  tools?: ToolDescriptor[];
  error: Error | null;
  pending: boolean;
  refreshing: boolean;
  reload: () => void;
};

export function ToolReadNotice({ read }: { read: MetadataRead }) {
  if (read.error) {
    const denied =
      read.error instanceof RequestError &&
      [401, 403].includes(read.error.status);
    return (
      <Alert
        variant={denied ? "default" : "destructive"}
        title={
          denied
            ? "Tool metadata access unavailable"
            : "Tool metadata unavailable"
        }
        actions={
          <Button
            size="sm"
            variant="secondary"
            disabled={read.refreshing}
            loading={read.refreshing}
            onClick={read.reload}
          >
            Reload tools
          </Button>
        }
      >
        {read.error.message}
      </Alert>
    );
  }
  if (read.pending || read.refreshing)
    return (
      <div
        className="flex items-center gap-2 text-caption text-muted-foreground"
        role="status"
      >
        <Spinner size={14} />
        {read.pending ? "Loading approved tools…" : "Refreshing approved tools…"}
      </div>
    );
  return null;
}

/** JSON schemas and hints, each behind its own disclosure. */
function SchemaDisclosures({
  tool,
  hints = false,
}: {
  tool: ToolDescriptor;
  hints?: boolean;
}) {
  return (
    <>
      <Disclosure summary="Input schema">
        <CodeBlock
          language="json"
          code={JSON.stringify(tool.inputSchema, null, 2)}
        />
      </Disclosure>
      {tool.outputSchema != null && (
        <Disclosure summary="Output schema">
          <CodeBlock
            language="json"
            code={JSON.stringify(tool.outputSchema, null, 2)}
          />
        </Disclosure>
      )}
      {hints && tool.annotations != null && (
        <Disclosure summary="Behavior hints">
          <p className="m-0 text-caption text-muted-foreground">
            Hints do not authorize use or establish actual behavior.
          </p>
          <CodeBlock
            language="json"
            code={JSON.stringify(tool.annotations, null, 2)}
          />
        </Disclosure>
      )}
    </>
  );
}

export function ApprovedToolDetails({ tool }: { tool: ToolDescriptor }) {
  return (
    <section className="grid min-w-0 gap-2 border-t border-border py-3 first:border-t-0 first:pt-0">
      <div className="flex min-w-0 items-center gap-2">
        <Wrench
          aria-hidden="true"
          className="size-3.5 shrink-0 text-muted-foreground"
        />
        <span className="mono min-w-0 truncate text-label font-medium text-foreground">
          {tool.name}
        </span>
      </div>
      <ToolDescription text={tool.description ?? ""} />
      <SchemaDisclosures tool={tool} />
    </section>
  );
}

export function InspectorToolGroup({
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
  const pollNormal = usePollInterval("normal");
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
    refetchInterval: (query) => (query.state.error ? false : pollNormal),
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
      className="grid min-w-0 gap-2 rounded-md border border-border bg-card p-3"
      aria-label={`Tool group ${profile.name}`}
    >
      <div className="flex min-w-0 items-center gap-2">
        <BookOpen
          aria-hidden="true"
          className="size-4 shrink-0 text-muted-foreground"
        />
        <strong className="min-w-0 flex-1 truncate text-label font-semibold text-foreground">
          {profile.name}
        </strong>
        <Badge className={cn(tools != null && !blocked && "mono")}>
          {count}
        </Badge>
      </div>
      {profile.description && (
        <p className="m-0 text-caption text-muted-foreground">
          {profile.description}
        </p>
      )}
      {blocked ? (
        <p className="m-0 text-caption text-muted-foreground">
          {blocked === "Use access required"
            ? "Use access is required to read this group’s tools."
            : "Tool discovery is paused for this configuration."}
        </p>
      ) : (
        <>
          <ToolReadNotice read={read} />
          {unavailable ? (
            <p className="m-0 text-label text-muted-foreground">
              {availabilityLabel(unavailable.reason)}
            </p>
          ) : (
            !read.error &&
            tools != null && (
              <>
                {detailed ? (
                  <div className="grid">
                    {tools.map((tool) => (
                      <ApprovedToolDetails key={tool.name} tool={tool} />
                    ))}
                  </div>
                ) : (
                  tools.length > 0 && (
                    <div className="flex flex-wrap gap-1.5">
                      {tools.map((tool) => (
                        <code
                          key={tool.name}
                          className="mono inline-flex h-[22px] max-w-full items-center truncate rounded-sm border border-border bg-muted px-1.5 text-caption text-foreground-secondary"
                        >
                          {tool.name}
                        </code>
                      ))}
                    </div>
                  )
                )}
                {tools.length === 0 && (
                  <p className="m-0 text-caption text-muted-foreground">
                    {canConfigure
                      ? "No approved tools in this connector."
                      : data?.total === 0
                        ? "No eligible approved tools in this group."
                        : "No tools from this connection on this page. Other pages may contain more."}
                  </p>
                )}
              </>
            )
          )}
          {!canConfigure && data && (data.total > 20 || offset > 0) && (
            <div className="flex items-center justify-between gap-2">
              <Button
                variant="secondary"
                size="sm"
                disabled={offset === 0}
                onClick={() => setOffset((value) => Math.max(0, value - 20))}
              >
                Previous tools
              </Button>
              <span className="mono text-caption text-muted-foreground">
                {Math.min(offset + 1, data.total)}–
                {Math.min(offset + 20, data.total)} of {data.total} group tools
              </span>
              <Button
                variant="secondary"
                size="sm"
                disabled={data.next_offset == null}
                onClick={() => setOffset(data.next_offset!)}
              >
                Next tools
              </Button>
            </div>
          )}
        </>
      )}
    </section>
  );
}

export function DiscoveryDialog({
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
  const pollNormal = usePollInterval("normal");
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
    refetchInterval: (query) => (query.state.error ? false : pollNormal),
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
      description="Available tools from this approved group. Choose a tool to run it."
      size="xl"
      bodyClassName="grid gap-4"
    >
      {environments.length > 0 && (
        <Select
          label="Tool environment"
          value={environment}
          clearable={!profile.environment_id}
          disabled={!!profile.environment_id}
          placeholder="Brain-wide"
          data={environments}
          className="max-w-[320px]"
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
          <p className="m-0 text-title font-semibold text-foreground">
            <span className="mono">{data.total}</span> tools · Approved
            catalogue
          </p>
          {connectionId && (
            <p className="m-0 text-caption text-muted-foreground">
              Showing {connectionName} tools on this page of {profile.name}.
              Other pages may contain more.
            </p>
          )}
          {data.unavailable_connections.map((c) => (
            <Alert variant="warning" role="status" key={c.connection_id}>
              {c.name}: {availabilityLabel(c.reason)}
            </Alert>
          ))}
          {data.total === 0 && (
            <p className="m-0 text-body text-muted-foreground">
              No eligible cached tools are available in this profile.
            </p>
          )}
          <div className="grid gap-3">
            {data.tools
              .filter(
                (item) => !connectionId || item.connection_id === connectionId,
              )
              .map((item, index) => (
                <Card
                  key={`${item.connection_id}:${item.tool.name}`}
                  data-testid="mcp-cached-tool"
                  className="grid animate-enter gap-2"
                  style={staggerStyle(index)}
                >
                  <div className="flex min-w-0 items-start justify-between gap-3">
                    <p className="m-0 min-w-0 text-label font-semibold text-foreground [overflow-wrap:anywhere]">
                      {item.connection_name} · {item.tool.name}
                    </p>
                    <Button
                      size="sm"
                      className="shrink-0"
                      onClick={() => run(item, environment)}
                    >
                      <Play aria-hidden="true" />
                      Run tool
                    </Button>
                  </div>
                  <ToolDescription text={item.tool.description ?? ""} />
                  <Disclosure summary="Tool details">
                    <p className="mono m-0 text-caption text-muted-foreground">
                      Connection {item.connection_id}
                    </p>
                    <SchemaDisclosures tool={item.tool} hints />
                  </Disclosure>
                </Card>
              ))}
          </div>
          {(data.total > 20 || offset > 0) && (
            <div className="flex items-center justify-between gap-2">
              <Button
                variant="secondary"
                disabled={offset === 0}
                onClick={() => setOffset((v) => Math.max(0, v - 20))}
              >
                Previous tools
              </Button>
              <span className="mono text-caption text-muted-foreground">
                {Math.min(offset + 1, data.total)}–
                {Math.min(offset + 20, data.total)} of {data.total}
              </span>
              <Button
                variant="secondary"
                disabled={data.next_offset == null}
                onClick={() => setOffset(data.next_offset!)}
              >
                Next tools
              </Button>
            </div>
          )}
        </>
      )}
    </Modal>
  );
}

export function ConnectionObservation({
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
      <p className="m-0 text-caption text-muted-foreground">
        Current session status unavailable
      </p>
    );
  if (pending || !status)
    return (
      <p className="m-0 text-caption text-muted-foreground">
        Checking observed sessions…
      </p>
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
    <div className="grid gap-1.5">
      <div className="flex items-center gap-2 text-label text-foreground">
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
      <span className="text-caption text-muted-foreground">
        {active.length ? (
          <span className="mono">
            {`${active.reduce((n, i) => n + i.active_calls, 0)} active calls · `}
          </span>
        ) : (
          ""
        )}
        Checked{" "}
        <span className="mono">{new Date(checkedAt).toLocaleTimeString()}</span>{" "}
        · up to 50 visible sessions, active first
      </span>
      {previous.length > 0 && (
        <p className="m-0 text-caption text-muted-foreground">
          {previous.length} earlier-configuration session
          {previous.length === 1 ? "" : "s"} still observed.
        </p>
      )}
      {!previous.length &&
        !active.length &&
        visible.some((i) => !i.current_configuration) && (
          <p className="m-0 text-caption text-muted-foreground">
            Previous configuration sessions are excluded.
          </p>
        )}
    </div>
  );
}

export function ConnectionTest({
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
  const pollNormal = usePollInterval("normal");
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
    refetchInterval: pollNormal,
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
      bodyClassName="grid gap-3"
    >
      {detail.isPending && <Spinner label="Loading connection" />}
      {detail.error && (
        <Alert variant="destructive">{detail.error.message}</Alert>
      )}
      {!detail.error &&
        detail.data &&
        (!eligible ? (
          <>
            <p className="m-0 text-body text-foreground-secondary">
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
            </p>
            {group &&
            summary.enabled &&
            summary.availability === "configured" &&
            definition?.enabled ? (
              <Button
                className="justify-self-start"
                onClick={() => tools(group.id)}
              >
                Open {group.name} tools
              </Button>
            ) : (
              <p className="m-0 text-label text-muted-foreground">
                {summary.enabled &&
                summary.availability === "configured" &&
                definition?.enabled
                  ? "Ask a tool-group administrator for Use access to this connection."
                  : "No tool test is available for this configuration."}
              </p>
            )}
          </>
        ) : (
          <>
            <p className="m-0 text-body text-foreground-secondary">
              Check the anonymous server handshake and list its tool metadata.
              No tool is executed, and approved tools and permissions stay
              unchanged.
            </p>
            {probe.isPending && (
              <p
                className="m-0 flex items-center gap-2 text-caption text-muted-foreground"
                role="status"
              >
                <StatusDot live />
                Checking server…
              </p>
            )}
            {probe.error && (
              <Alert variant="destructive" title="Server check failed">
                {probe.error.message}
              </Alert>
            )}
            {probe.data && !probe.isPending && !probe.error && (
              <Alert
                variant="brand"
                role="status"
                icon={<ServerCog aria-hidden="true" />}
                title={`Server responded · ${probe.data.count} tools listed`}
              >
                <p className="mono m-0 text-caption">{probe.data.checkedAt}</p>
                <p className="m-0 text-caption">
                  Anonymous check; no tools executed. This does not verify tool
                  permissions or future availability.
                </p>
              </Alert>
            )}
            <Button
              className="justify-self-start"
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
    </Modal>
  );
}
