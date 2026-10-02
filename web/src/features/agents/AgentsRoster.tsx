import { useState } from "react";
import { Button, Card, Group, Modal, Stack, Text } from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { client, result, type Brain } from "../../api";
import { palette, tokens } from "../../design/tokens";
import { EmptyState, ErrorState, LoadingState } from "../../components/AsyncState";

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
  value ? new Date(value).toLocaleString() : "Never used on this Brain";

/** Per-Brain answer to "who is connected here": the account's agents grouped
 * by user, each row one connection of exactly two kinds. A row proves the
 * configuration exists; only real calls prove it works. */
export function AgentsRoster({ brain }: { brain: Brain }) {
  const [showHidden, setShowHidden] = useState(false);
  const [pendingRevoke, setPendingRevoke] = useState<string | null>(null);
  const cache = useQueryClient();
  const roster = useQuery({
    queryKey: ["brain-agents", brain.id, showHidden],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/agents", {
          params: {
            path: { brain: brain.id },
            query: showHidden ? { include_hidden: true } : {},
          },
        }),
      ),
  });
  const revoke = useMutation({
    mutationFn: async (id: string) =>
      result(await client.DELETE("/api/devices/{id}", { params: { path: { id } } })),
    onSuccess: () =>
      cache.invalidateQueries({ queryKey: ["brain-agents", brain.id] }),
  });
  if (roster.isPending) return <LoadingState label="Loading agents…" />;
  if (roster.isError) return <ErrorState error={roster.error} />;
  const { groups, hidden_count } = roster.data;
  // This Brain's answer to "who is connected here" is the set of agents that
  // have actually been used on it. The complete account list lives on the
  // global Agents page.
  const usedGroups = groups
    .map((group) => ({
      ...group,
      agents: group.agents.filter((a) => a.last_used_on_brain_at),
    }))
    .filter((group) => group.agents.length > 0);
  const visible = usedGroups.flatMap((g) => g.agents);
  const target = pendingRevoke
    ? visible.find((a) => a.device_id === pendingRevoke)
    : null;
  return (
    <Stack gap="sm" data-testid="brain-agent-roster">
      {visible.length === 0 && (
        <EmptyState
          title="No agents have been used on this Brain yet"
          description="Connect the Recollect plugin or a direct MCP access token below, or review the full account list on the Agents page."
        />
      )}
      {usedGroups.map((group) => (
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
                data-testid={`brain-agent-row-${agent.device_id}`}
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
                <Stack gap={0} style={{ flex: 1, minWidth: 180 }}>
                  <Text size="sm" fw={500}>
                    {agent.name}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {hostLabel(agent.host_kind) ?? "Coding host"} ·{" "}
                    {integrationLabel(agent.integration)}
                    {!agent.active && " · revoked or expired"}
                  </Text>
                </Stack>
                <Text size="xs" c="dimmed" style={{ whiteSpace: "nowrap" }}>
                  Last used {time(agent.last_used_on_brain_at).toLowerCase()}
                </Text>
                <Button
                  variant="subtle"
                  color="red"
                  size="xs"
                  onClick={() => setPendingRevoke(agent.device_id)}
                >
                  Revoke
                </Button>
              </Group>
            ))}
          </Stack>
        </Card>
      ))}
      <Group gap="sm">
        {hidden_count > 0 && (
          <Button
            variant="subtle"
            size="xs"
            fw={400}
            onClick={() => setShowHidden((v) => !v)}
          >
            {showHidden
              ? "Hide revoked and expired"
              : "Show revoked and expired"}
          </Button>
        )}
        <Link to="/agents" style={{ fontSize: "var(--mantine-font-size-xs)" }}>
          All account agents
        </Link>
      </Group>
      <Modal
        opened={pendingRevoke !== null}
        onClose={() => setPendingRevoke(null)}
        title="Remove this agent?"
        size="sm"
      >
        <Stack gap="md">
          <Text size="sm">
            This revokes {target?.name ?? "the agent"}&apos;s keycard from{" "}
            <b>all Brains</b>, not just this one. Its token stops working at
            once; captured history stays in place.
          </Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setPendingRevoke(null)}>
              Cancel
            </Button>
            <Button
              color="red"
              loading={revoke.isPending}
              onClick={() => {
                if (!pendingRevoke) return;
                revoke.mutate(pendingRevoke, {
                  onSuccess: () => setPendingRevoke(null),
                });
              }}
            >
              Revoke from all Brains
            </Button>
          </Group>
          <ErrorState error={revoke.error} />
        </Stack>
      </Modal>
    </Stack>
  );
}
