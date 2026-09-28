import { McpConnectorSetup } from "./McpConnectorSetup";
import { iconSize } from "./design/tokens";
import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Drawer,
  Code,
  Group,
  Loader,
  Modal,
  Select,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { Unplug, ShieldCheck, Plus, Activity } from "lucide-react";
import { EmptyState } from "./components/AsyncState";
import "./features/feature-views.css";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { McpConnectionDialog } from "./McpConnectionDialog";
import { McpProfileDialog } from "./McpProfileDialog";
import { McpRunDialog, type McpTool } from "./McpRunDialog";
import { McpRuntimePanel } from "./McpRuntimePanel";
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
    .join(" · ") || "No profile rights";
export const availabilityLabel = (value: string) =>
  ({
    configured: "Configured · connection not checked",
    disabled: "Connection disabled",
    definition_disabled: "Definition disabled",
    configuration_invalid: "Settings need attention",
  })[value] ?? value;

export function McpPanel({
  brain,
  actor,
  section = "connections",
  onSectionChange,
}: {
  brain: Brain;
  actor: string;
  section?: "connections" | "profiles" | "runners" | "activity";
  onSectionChange?: (section: string | null) => void;
}) {
  const cache = useQueryClient();
  const [connection, setConnection] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState(false);
  const [connectorSetup, setConnectorSetup] = useState(false);
  const [profile, setProfile] = useState<string | null>(null);
  const [discover, setDiscover] = useState<string | null>(null);
  const [environment, setEnvironment] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [setupSaved, setSetupSaved] = useState(false);
  const [session, setSession] = useState(() => crypto.randomUUID());
  const [selectedCall, setSelectedCall] = useState<string | null>(null);
  const [run, setRun] = useState<{
    profile: string;
    tool: McpTool;
    environment: string | null;
  } | null>(null);
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
    setProfile(null);
    setDiscover(null);
    setRun(null);
    setNotice(message);
    refresh();
  };
  useEffect(() => {
    if (catalogue.error || workspace.error) {
      setConnection(null);
      setProfile(null);
      setDiscover(null);
      setRun(null);
      setNotice(
        "Catalogue access could not be refreshed. Reload it to continue.",
      );
      return;
    }
    if (!catalogue.data) return;
    if (connection && !catalogue.data.can_configure) {
      setConnection(null);
      setNotice("Connection administration is no longer available.");
    }
    if (
      profile &&
      profile !== "new" &&
      !catalogue.data.profiles.some((p) => p.id === profile)
    ) {
      setProfile(null);
      setNotice("Profile access changed. The inspector was closed.");
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
      setNotice("Profile use changed. Cached tool results were cleared.");
    }
  }, [
    catalogue.data,
    catalogue.error,
    workspace.error,
    brain.archived,
    connection,
    profile,
    discover,
  ]);
  const data = catalogue.error ? undefined : catalogue.data;
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
    <section className="feature-view" aria-label="MCP connections and profiles">
      <Stack gap="lg">
        {connectorSetup && (
          <McpConnectorSetup
            brain={brain}
            close={() => setConnectorSetup(false)}
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
        {section === "profiles" && (
          <Text size="sm">
            Profiles are tool groups: choose which MCP connections an agent can
            use together. For example, a Documentation profile could contain
            GitLab and Confluence. Use permission is separate from changing or
            sharing the group.
          </Text>
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
        {catalogue.isPending && <Loader size="sm" />}
        {data && !workspace.error && (
          <>
            {section !== "activity" && (
              <div className="feature-toolbar">
                <Select
                  className="feature-search"
                  label="Catalogue environment"
                  placeholder="All environments"
                  clearable
                  data={environments}
                  value={environment}
                  onChange={(value) => {
                    setEnvironment(value);
                    setDiscover(null);
                  }}
                />
                {section === "connections" && data.can_configure && (
                  <Button
                    leftSection={<Plus size={iconSize.small} />}
                    disabled={brain.archived}
                    onClick={() => setConnectorSetup(true)}
                  >
                    Add connection
                  </Button>
                )}
                {section === "connections" &&
                  data.can_configure &&
                  data.definitions.length > 0 && (
                    <Button
                      variant="subtle"
                      disabled={brain.archived}
                      onClick={() => setConnection("new")}
                    >
                      Use registered connector
                    </Button>
                  )}
                {section === "profiles" && data.can_configure && (
                  <Button
                    leftSection={<Plus size={iconSize.small} />}
                    disabled={brain.archived}
                    onClick={() => setProfile("new")}
                  >
                    Create execution profile
                  </Button>
                )}
                {section !== "runners" && (
                  <Button
                    variant="subtle"
                    leftSection={<Activity size={iconSize.small} />}
                    onClick={() => setDiagnostics(true)}
                  >
                    Runtime diagnostics
                  </Button>
                )}
              </div>
            )}
            {brain.archived && (
              <Alert color="gray">
                This Brain is archived. Configuration and profile use are
                paused.
              </Alert>
            )}
            {section === "connections" && (
              <>
                {setupSaved && (
                  <Alert title="Connection saved · not tested" color="brand">
                    <Text size="sm">
                      Add this connection to an execution profile and grant Use
                      separately. Then choose a tool, review its inputs and
                      effects, and explicitly run it. Saving has made no tool
                      call.
                    </Text>
                    <Group mt="md">
                      {onSectionChange && (
                        <Button onClick={() => onSectionChange("profiles")}>
                          Configure profile &amp; test
                        </Button>
                      )}
                      <Button
                        variant="subtle"
                        onClick={() => setSetupSaved(false)}
                      >
                        Dismiss setup guidance
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
                    description="Add an approved connection, then include it in a profile with explicit tool-use permissions."
                  />
                )}
                <div className="connection-grid">
                  {data.connections
                    .filter(
                      (connection) =>
                        !environment ||
                        !connection.environment_id ||
                        connection.environment_id === environment,
                    )
                    .map((connection) => (
                      <Card
                        withBorder
                        p="lg"
                        key={connection.id}
                        className="connection-record"
                        data-testid="mcp-connection"
                      >
                        <Stack gap="md">
                          <Group gap="sm">
                            <Unplug size={iconSize.action} />
                            <Text fw={600}>{connection.name}</Text>
                          </Group>
                          <Text size="sm" c="dimmed">
                            {connection.description ||
                              "Approved tool connection"}
                          </Text>
                          <Badge
                            variant="light"
                            color={
                              connection.availability === "configured"
                                ? "gray"
                                : "yellow"
                            }
                          >
                            {availabilityLabel(connection.availability)}
                          </Badge>
                          <Text size="xs" c="dimmed">
                            {environmentName(connection.environment_id)} ·{" "}
                            {connection.placement} placement
                          </Text>
                          {data.can_configure && (
                            <Button
                              variant="default"
                              onClick={() => setConnection(connection.id)}
                            >
                              Inspect connection
                            </Button>
                          )}
                        </Stack>
                      </Card>
                    ))}
                </div>
                <Text size="xs" c="dimmed">
                  Saving a connection does not contact its server. A real test
                  uses a profile with an explicit Use grant and may have
                  effects.
                </Text>
                {onSectionChange && (
                  <Button
                    variant="subtle"
                    w="fit-content"
                    onClick={() => onSectionChange("profiles")}
                  >
                    Manage profiles and test tools
                  </Button>
                )}
              </>
            )}
            {section === "profiles" && (
              <>
                <Text size="sm" c="dimmed">
                  Use, Manage and Share are independent grants. Brain membership
                  and administration do not grant tool use.
                </Text>
                {!data.profiles.length && (
                  <EmptyState
                    icon={ShieldCheck}
                    title="Choose who can use your tools"
                    description="Execution profiles group connections and grant only the permissions each person or group needs."
                  />
                )}
                <div className="connection-grid">
                  {data.profiles
                    .filter(
                      (profile) =>
                        !environment ||
                        !profile.environment_id ||
                        profile.environment_id === environment,
                    )
                    .map((profile) => (
                      <Card
                        withBorder
                        p="lg"
                        key={profile.id}
                        className="connection-record"
                        data-testid="mcp-profile"
                      >
                        <Stack gap="sm">
                          <Group justify="space-between">
                            <Text fw={600}>{profile.name}</Text>
                            <Badge
                              variant="light"
                              color={profile.enabled ? "teal" : "gray"}
                            >
                              {profile.enabled ? "Enabled" : "Disabled"}
                            </Badge>
                          </Group>
                          <Text size="sm" c="dimmed">
                            {profile.description}
                          </Text>
                          <Text size="xs">
                            {environmentName(profile.environment_id)} ·{" "}
                            {profile.connection_ids.length} connections
                          </Text>
                          <Text size="sm">
                            Your rights: {rightsLabel(profile.rights)}
                          </Text>
                          {!profile.rights.use_profile && (
                            <Text size="xs" c="dimmed">
                              An explicit Use grant is required, including for
                              administrators.
                            </Text>
                          )}
                          <Group>
                            <Button
                              size="sm"
                              variant="default"
                              onClick={() => setProfile(profile.id)}
                            >
                              Inspect profile
                            </Button>
                            <Button
                              size="sm"
                              variant="light"
                              disabled={
                                !profile.rights.use_profile ||
                                !profile.enabled ||
                                brain.archived
                              }
                              onClick={() => setDiscover(profile.id)}
                            >
                              Inspect cached tools
                            </Button>
                          </Group>
                        </Stack>
                      </Card>
                    ))}
                </div>
              </>
            )}
            {section === "runners" && (
              <McpPrivateRunners
                brain={brain}
                canConfigure={data.can_configure}
              />
            )}
            {section === "activity" && (
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
              )}
            </Drawer>
          </>
        )}
        {data && connection && !workspace.error && (
          <McpConnectionDialog
            key={connection}
            brain={brain}
            id={connection}
            catalogue={data}
            environments={environments}
            close={() => setConnection(null)}
            saved={() => {
              setSetupSaved(connection === "new");
              setConnection(null);
              refresh();
            }}
            stale={clear}
          />
        )}
        {data && profile && !workspace.error && (
          <McpProfileDialog
            key={profile}
            brain={brain}
            actor={actor}
            id={profile}
            catalogue={data}
            environments={environments}
            close={() => setProfile(null)}
            saved={refresh}
            stale={clear}
          />
        )}
        {discoveryProfile && !workspace.error && (
          <DiscoveryDialog
            key={discover}
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

function DiscoveryDialog({
  brain,
  profile,
  environments,
  close,
  run,
}: {
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
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  const data = query.error ? undefined : query.data;
  return (
    <Modal
      opened
      onClose={close}
      title={`Cached tools · ${profile.name}`}
      size="xl"
    >
      <Stack>
        <Text size="sm">
          Approved catalogue metadata only. Inspecting these schemas does not
          start a backend or resolve credentials.
        </Text>
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
        {query.isPending && <Loader size="sm" />}
        {query.error && (
          <Alert color="red">
            {query.error.message}
            <Button
              variant="light"
              size="xs"
              onClick={() => void query.refetch()}
            >
              Refresh cached tools
            </Button>
          </Alert>
        )}
        {data && (
          <>
            <Text fw={600}>{data.total} approved tools · Cached metadata</Text>
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
            {data.tools.map((item) => (
              <Card
                withBorder
                key={`${item.connection_id}:${item.tool.name}`}
                data-testid="mcp-cached-tool"
              >
                <Text fw={600}>
                  {item.connection_name} · {item.tool.name}
                </Text>
                <Text size="sm">{item.tool.description}</Text>
                <Button
                  size="xs"
                  mt="sm"
                  onClick={() => run(item, environment)}
                >
                  Run tool
                </Button>
                <Text size="xs" c="dimmed">
                  Connection {item.connection_id}
                </Text>
                <details>
                  <summary>Input schema</summary>
                  <Code block>
                    {JSON.stringify(item.tool.inputSchema, null, 2)}
                  </Code>
                </details>
                {item.tool.outputSchema != null && (
                  <details>
                    <summary>Output schema</summary>
                    <Code block>
                      {JSON.stringify(item.tool.outputSchema, null, 2)}
                    </Code>
                  </details>
                )}
                {item.tool.annotations != null && (
                  <details>
                    <summary>Behavior hints</summary>
                    <Text size="xs">
                      Hints do not authorize use or establish actual behavior.
                    </Text>
                    <Code block>
                      {JSON.stringify(item.tool.annotations, null, 2)}
                    </Code>
                  </details>
                )}
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
