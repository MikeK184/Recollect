import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Loader,
  Modal,
  Select,
  Stack,
  Text,
} from "@mantine/core";
import { BookOpen, FileText } from "lucide-react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { StatusDot } from "./components/StatusDot";
import { client, result, RequestError, type Brain } from "./api";
import type { components } from "./api-schema";
import { CodeBlock } from "./components/Markdown";
import { ToolDescription } from "./components/ToolDescription";
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

export function ApprovedToolDetails({ tool }: { tool: ToolDescriptor }) {
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
