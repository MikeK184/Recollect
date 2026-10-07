import { useState } from "react";
import {
  Badge,
  Button,
  Card,
  Group,
  Menu,
  Modal,
  Stack,
  Text,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { MoreHorizontal } from "lucide-react";
import { client, result, type Brain } from "../../api";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
import { HostIcon } from "../../components/HostIcon";
import { staggerStyle } from "../../components/Motion";

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
  value
    ? new Date(value).toLocaleDateString("en-GB", {
        day: "numeric",
        month: "short",
      })
    : "No observed use";

/** Observed contributors to this Brain, grouped
 * by user, each row one connection of exactly two kinds. A row proves the
 * configuration exists; only real calls prove it works. */
export function AgentsRoster({
  brain,
  onSelect,
  selectedDevice,
}: {
  brain: Brain;
  onSelect: (id: string) => void;
  selectedDevice?: string;
}) {
  const [showHidden, setShowHidden] = useState(false);
  const [pendingRevoke, setPendingRevoke] = useState<string | null>(null);
  const cache = useQueryClient();
  const roster = useQuery({
    queryKey: ["brain-agents", brain.id, showHidden],
    gcTime: 0,
    refetchInterval: 5000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/agents", {
          signal,
          params: {
            path: { brain: brain.id },
            query: showHidden ? { include_hidden: true } : {},
          },
        }),
      ),
  });
  const revoke = useMutation({
    mutationFn: async (id: string) =>
      result(
        await client.DELETE("/api/devices/{id}", { params: { path: { id } } }),
      ),
    onSuccess: async () => {
      await cache.invalidateQueries({ queryKey: ["brain-agents", brain.id] });
      await cache.invalidateQueries({ queryKey: ["account-agents"] });
      await cache.invalidateQueries({ queryKey: ["devices"] });
    },
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
          description="Connect an agent to start contributing. Your complete account list is in My agents."
        />
      )}
      {usedGroups.map((group, index) => (
        <Card
          key={group.user_name}
          withBorder
          padding={24}
          className="agent-roster-group rc-enter"
          style={staggerStyle(index)}
        >
          <div className="agent-roster-heading">
            <div className="agent-owner-avatar">
              {group.user_name.slice(0, 1).toUpperCase()}
            </div>
            <div>
              <strong>{group.user_name}</strong>
              <small>
                {group.agents.length}{" "}
                {group.agents.length === 1 ? "agent" : "agents"}
              </small>
            </div>
          </div>
          <Stack gap="xs">
            {group.agents.map((agent) => (
              <div
                key={agent.device_id}
                className={`agent-roster-row${selectedDevice === agent.device_id ? " selected" : ""}`}
                data-testid={`brain-agent-row-${agent.device_id}`}
              >
                <div className="management-icon agent-host-icon">
                  <HostIcon host={agent.host_kind} size={32} />
                </div>
                <Stack gap={0} style={{ flex: 1, minWidth: 180 }}>
                  <Button
                    variant="subtle"
                    className="agent-name"
                    px={0}
                    h="auto"
                    w="fit-content"
                    onClick={() => onSelect(agent.device_id)}
                    aria-pressed={selectedDevice === agent.device_id}
                  >
                    {agent.name}
                  </Button>
                  <Text size="md" c="dimmed">
                    {(agent.observed_hosts?.length
                      ? agent.observed_hosts
                          .map(hostLabel)
                          .filter(Boolean)
                          .join(" · ")
                      : hostLabel(agent.host_kind)) ?? "Coding host"}{" "}
                    · {integrationLabel(agent.integration)}
                    {!agent.active && " · revoked or expired"}
                  </Text>
                </Stack>
                <Badge
                  className="agent-access-badge"
                  variant="light"
                  color={agent.active ? "brand" : "gray"}
                  leftSection={
                    <span
                      className={
                        "agent-access-dot" + (agent.active ? "" : " inactive")
                      }
                    />
                  }
                >
                  {agent.active ? "Credential enabled" : "Revoked or expired"}
                </Badge>
                <time
                  title={
                    agent.last_used_on_brain_at
                      ? new Date(agent.last_used_on_brain_at).toLocaleString()
                      : undefined
                  }
                  dateTime={agent.last_used_on_brain_at ?? undefined}
                >
                  {"Observed "}
                  {time(agent.last_used_on_brain_at)}
                </time>
                <Menu position="bottom-end" withinPortal>
                  <Menu.Target>
                    <Button
                      variant="subtle"
                      size="compact-sm"
                      aria-label={`Actions for ${agent.name}`}
                    >
                      <MoreHorizontal size={16} />
                    </Button>
                  </Menu.Target>
                  <Menu.Dropdown>
                    <Menu.Item onClick={() => onSelect(agent.device_id)}>
                      View activity
                    </Menu.Item>
                    {agent.can_revoke && (
                      <Menu.Item
                        color="red"
                        onClick={() => setPendingRevoke(agent.device_id)}
                      >
                        Revoke credential across all Brains
                      </Menu.Item>
                    )}
                  </Menu.Dropdown>
                </Menu>
              </div>
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
      </Group>
      <Modal
        opened={!!target?.can_revoke}
        onClose={() => setPendingRevoke(null)}
        title="Revoke this credential across all Brains?"
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
                if (!pendingRevoke || !target?.can_revoke) return;
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
