import { Card, Group, Stack, Text } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { Alert } from "@mantine/core";
import { client, result } from "../../api";
import { tokens } from "../../design/tokens";
import { palette } from "../../design/tokens";
import { EmptyState, ErrorState, LoadingState } from "../../components/AsyncState";
import { PageHeader } from "../../components/PageHeader";
import { TeamPanel } from "../../TeamPanel";
import { useWorkspace } from "../../app/context";

const hostLabel = (kind?: string | null) =>
  kind === "codex"
    ? "Codex"
    : kind === "opencode"
      ? "OpenCode"
      : kind === "claude_code"
        ? "Claude Code"
        : null;

const integrationLabel = (value: string) =>
  value === "plugin" ? "Recollect plugin" : "MCP token";

const time = (value?: string | null) =>
  value ? new Date(value).toLocaleString() : null;

/** Account-level answer to "which agents exist and where are they used":
 * every agent of the signed-in user, grouped by user, with the Brains each is
 * used in and when it was last used there. */
export function AgentsGlobalPage() {
  const roster = useQuery({
    queryKey: ["account-agents"],
    queryFn: async () => result(await client.GET("/api/agents")),
  });
  if (roster.isPending) return <LoadingState label="Loading agents…" />;
  if (roster.isError) return <ErrorState error={roster.error} />;
  const { groups, hidden_count } = roster.data;
  const total = groups.reduce((n, g) => n + g.agents.length, 0);
  return (
    <Stack gap="md" data-testid="account-agent-roster">
      <PageHeader
        title="Agents"
        description="Every agent of this account, and the Brains it is used in."
      />
      {total === 0 && (
        <EmptyState
          title="No agents connected yet"
          description="Connect an agent from any Brain's Agents page."
        />
      )}
      {groups.map((group) => (
        <Card key={group.user_name} withBorder padding="sm">
          <Text fw={600} size="sm" mb="xs">
            {group.user_name} — {group.agents.length}{" "}
            {group.agents.length === 1 ? "agent" : "agents"}
          </Text>
          <Stack gap="xs">
            {group.agents.map((agent) => (
              <Group
                key={agent.device_id}
                gap="sm"
                wrap="wrap"
                data-testid={`account-agent-row-${agent.device_id}`}
              >
                <span
                  aria-hidden
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: "50%",
                    background: agent.active ? tokens.accent : palette.lineDark,
                    flexShrink: 0,
                    marginTop: 6,
                  }}
                />
                <Stack gap={0} style={{ flex: 1, minWidth: 200 }}>
                  <Text size="sm" fw={500}>
                    {agent.name}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {hostLabel(agent.host_kind) ?? "Coding host"} ·{" "}
                    {integrationLabel(agent.integration)}
                  </Text>
                </Stack>
                <Stack gap={0} style={{ minWidth: 220, flexShrink: 0 }}>
                  {agent.brains.length === 0 ? (
                    <Text size="xs" c="dimmed">
                      Not used on any Brain yet
                    </Text>
                  ) : (
                    agent.brains.map((usage) => (
                      <Text key={usage.brain_id} size="xs" c="dimmed">
                        {usage.name} — last used{" "}
                        {time(usage.last_used_at)?.toLowerCase()}
                      </Text>
                    ))
                  )}
                </Stack>
              </Group>
            ))}
          </Stack>
        </Card>
      ))}
      {hidden_count > 0 && (
        <Text size="xs" c="dimmed">
          {hidden_count} revoked or expired{" "}
          {hidden_count === 1 ? "agent is" : "agents are"} hidden. The complete
          device history stays on the direct /devices surface.
        </Text>
      )}
    </Stack>
  );
}

export function TeamPage() {
  const session = useWorkspace();
  return session.user.installation_owner ? (
    <TeamPanel />
  ) : (
    <Alert title="Installation owner access required">
      Your Brain access is managed in each Brain’s Settings.
    </Alert>
  );
}
