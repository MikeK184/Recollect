import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Code,
  Group,
  Loader,
  Modal,
  Select,
  Stack,
  Text,
  Title,
} from "@mantine/core";
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

export function McpPanel({ brain, actor }: { brain: Brain; actor: string }) {
  const cache = useQueryClient();
  const [connection, setConnection] = useState<string | null>(null);
  const [profile, setProfile] = useState<string | null>(null);
  const [discover, setDiscover] = useState<string | null>(null);
  const [environment, setEnvironment] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
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
    <Card
      withBorder
      padding="lg"
      component="section"
      aria-label="MCP connections and profiles"
    >
      <Stack>
        <Group justify="space-between">
          <Title order={3}>MCP connections and profiles</Title>
          <Badge variant="light" color="gray">
            Catalogue
          </Badge>
        </Group>
        <Text size="sm" c="dimmed">
          Organize approved tools by environment and decide who can use, manage
          or share each profile. Brain access alone does not grant tool use.
        </Text>
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
            <Group>
              <Select
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
              {data.can_configure && (
                <>
                  <Button
                    disabled={
                      brain.archived || !data.definitions.some((d) => d.enabled)
                    }
                    onClick={() => setConnection("new")}
                  >
                    Add connection
                  </Button>
                  <Button
                    variant="light"
                    disabled={brain.archived}
                    onClick={() => setProfile("new")}
                  >
                    Create execution profile
                  </Button>
                </>
              )}
            </Group>
            {brain.archived && (
              <Alert color="gray">
                This Brain is archived. Configuration and profile use are
                paused.
              </Alert>
            )}
            {data.can_configure && data.definitions.length === 0 && (
              <Alert title="No approved connectors">
                The installation operator must approve a connector definition
                before you can configure a connection. Catalogue approval does
                not start it.
              </Alert>
            )}
            {data.profiles.filter(
              (p) =>
                !environment ||
                !p.environment_id ||
                p.environment_id === environment,
            ).length === 0 && (
              <Text size="sm">
                No visible execution profiles in this selection. A Brain
                administrator can create one or grant profile access.
              </Text>
            )}
            {data.profiles
              .filter(
                (p) =>
                  !environment ||
                  !p.environment_id ||
                  p.environment_id === environment,
              )
              .map((p) => (
                <Card withBorder key={p.id} data-testid="mcp-profile">
                  <Group justify="space-between">
                    <Text fw={600}>{p.name}</Text>
                    <Badge color={p.enabled ? "teal" : "gray"}>
                      {p.enabled ? "Configured" : "Disabled"}
                    </Badge>
                  </Group>
                  {p.description && <Text size="sm">{p.description}</Text>}
                  <Text size="sm">
                    {environmentName(p.environment_id)} ·{" "}
                    {p.connection_ids.length} connections
                  </Text>
                  <Text size="sm">Your rights: {rightsLabel(p.rights)}</Text>
                  {!p.rights.use_profile && (
                    <Text size="xs" c="dimmed">
                      An explicit Use grant is required, including for
                      administrators.
                    </Text>
                  )}
                  <Group mt="sm">
                    <Button
                      size="xs"
                      variant="light"
                      onClick={() => setProfile(p.id)}
                    >
                      Inspect profile
                    </Button>
                    <Button
                      size="xs"
                      disabled={
                        !p.rights.use_profile || !p.enabled || brain.archived
                      }
                      onClick={() => setDiscover(p.id)}
                    >
                      Inspect cached tools
                    </Button>
                  </Group>
                </Card>
              ))}
            {data.connections.length > 0 && (
              <>
                <Title order={4}>Connections</Title>
                {data.connections
                  .filter(
                    (c) =>
                      !environment ||
                      !c.environment_id ||
                      c.environment_id === environment,
                  )
                  .map((c) => (
                    <Card withBorder key={c.id} data-testid="mcp-connection">
                      <Group justify="space-between">
                        <Text fw={600}>{c.name}</Text>
                        <Badge
                          color={
                            c.availability === "configured" ? "gray" : "yellow"
                          }
                        >
                          {availabilityLabel(c.availability)}
                        </Badge>
                      </Group>
                      <Text size="sm">
                        {environmentName(c.environment_id)} · {c.placement}{" "}
                        placement
                      </Text>
                      <Text size="xs" c="dimmed">
                        {c.description}
                      </Text>
                      {data.can_configure && (
                        <Button
                          size="xs"
                          mt="xs"
                          variant="light"
                          onClick={() => setConnection(c.id)}
                        >
                          Inspect connection
                        </Button>
                      )}
                    </Card>
                  ))}
              </>
            )}
            <Text size="xs" c="dimmed">
              These records are approved configuration. No backend connection,
              credential delivery or successful tool call is established by this
              catalogue.
            </Text>
            <McpPrivateRunners
              brain={brain}
              canConfigure={data.can_configure}
            />
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
              refresh();
            }}
          />
        )}
      </Stack>
    </Card>
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
