import { useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Drawer,
  Group,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useRouterState } from "@tanstack/react-router";
import { ArrowRight, Search, Terminal } from "lucide-react";
import { motion, useReducedMotion } from "motion/react";
import { client, result } from "../../api";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
import { PageHeader } from "../../components/PageHeader";
import { BrainIcon } from "../../components/BrainIcon";
import { AgentAccessPanel } from "../../DevicesPanel";
import { TeamPanel } from "../../TeamPanel";
import { useWorkspace } from "../../app/context";
import "./control-panel.css";

const hostLabel = (kind?: string | null) =>
  ({ codex: "Codex", opencode: "OpenCode", claude_code: "Claude Code" })[
    kind ?? ""
  ] ?? "Host unreported";
const integrationLabel = (value: string) =>
  value === "plugin" ? "Recollect plugin" : "MCP token";
const time = (value?: string | null) =>
  value ? new Date(value).toLocaleString() : "No recorded use";

export function AgentsGlobalPage() {
  const reduced = useReducedMotion(),
    navigate = useNavigate();
  const searchStr = useRouterState({
    select: (state) => state.location.searchStr,
  });
  const params = new URLSearchParams(searchStr),
    code = params.get("code") ?? undefined;
  const access = params.get("access") === "true" || !!code;
  const [selected, setSelected] = useState<string | null>(null),
    [search, setSearch] = useState("");
  const roster = useQuery({
    queryKey: ["account-agents"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/agents", { signal })),
    gcTime: 0,
    refetchInterval: 5000,
    retry: false,
  });
  const agents = roster.isError
    ? []
    : (roster.data?.groups ?? []).flatMap((group) =>
        group.agents.map((agent) => ({ ...agent, user_name: group.user_name })),
      );
  const inspected = agents.find((agent) => agent.device_id === selected);
  type Agent = (typeof agents)[number];
  type Usage = Agent["brains"][number];
  const byBrain = new Map<
    string,
    { brain: Usage; agents: { agent: Agent; usage: Usage }[] }
  >();
  const term = search.trim().toLocaleLowerCase();
  for (const agent of agents)
    for (const brain of agent.brains) {
      if (
        !`${brain.name} ${agent.name} ${hostLabel(agent.host_kind)} ${(agent.observed_hosts ?? []).map(hostLabel).join(" ")}`
          .toLocaleLowerCase()
          .includes(term)
      )
        continue;
      if (!byBrain.has(brain.brain_id))
        byBrain.set(brain.brain_id, { brain, agents: [] });
      byBrain.get(brain.brain_id)!.agents.push({ agent, usage: brain });
    }
  const groups = [...byBrain.values()].sort((a, b) =>
    a.brain.name.localeCompare(b.brain.name),
  );
  return (
    <Stack gap="lg" data-testid="account-agent-roster">
      <PageHeader
        title="Agents"
        actions={
          <Button
            variant="default"
            onClick={() =>
              void navigate({ to: "/agents", search: { access: true } })
            }
          >
            Access tokens
          </Button>
        }
      />
      <TextInput
        aria-label="Search agents or Brains"
        placeholder="Find an agent or Brain…"
        leftSection={<Search size={16} />}
        value={search}
        onChange={(event) => setSearch(event.currentTarget.value)}
        maw={480}
      />
      {roster.isPending ? (
        <LoadingState label="Loading agents…" />
      ) : roster.isError ? (
        <ErrorState error={roster.error} retry={() => void roster.refetch()} />
      ) : !groups.length ? (
        <EmptyState
          title={
            term
              ? "No matching agents or Brains"
              : "No recorded Brain activity yet"
          }
          description={
            term
              ? "Try a different name."
              : "Connect an agent from a Brain’s Agents page. Your credentials are available in Access tokens."
          }
        />
      ) : (
        groups.map(({ brain, agents: rows }) => (
          <motion.section
            key={brain.brain_id}
            layout={!reduced}
            transition={{ duration: reduced ? 0 : 0.22 }}
            className="control-surface agent-brain-group"
            data-testid="agent-brain-group"
          >
            <div className="control-surface-head">
              <BrainIcon id={brain.brain_id} revision={brain.icon_revision} />
              <div>
                <Link
                  to="/brains/$brainId/agents"
                  params={{ brainId: brain.brain_id }}
                  search={{}}
                >
                  <strong>{brain.name}</strong>
                </Link>
                <small>
                  {rows.length} {rows.length === 1 ? "agent" : "agents"}
                </small>
              </div>
            </div>
            <table className="agent-group-table">
              <thead>
                <tr>
                  <th>Agent</th>
                  <th>Connection</th>
                  <th>Last used</th>
                  <th>
                    <span className="sr-only">Details</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {rows
                  .sort(
                    (a, b) =>
                      Date.parse(b.usage.last_used_at) -
                      Date.parse(a.usage.last_used_at),
                  )
                  .map(({ agent, usage }) => (
                    <tr
                      key={agent.device_id}
                      data-testid={`account-agent-row-${agent.device_id}`}
                    >
                      <td>
                        <button
                          className="agent-name-button"
                          onClick={() => setSelected(agent.device_id)}
                          aria-label={`Inspect ${agent.name}`}
                        >
                          <span className="control-agent-icon">
                            <Terminal size={18} />
                          </span>
                          <strong>{agent.name}</strong>
                        </button>
                      </td>
                      <td>
                        <span>
                          {agent.observed_hosts?.length
                            ? agent.observed_hosts.map(hostLabel).join(" · ")
                            : hostLabel(agent.host_kind)}
                        </span>
                        <small>{integrationLabel(agent.integration)}</small>
                      </td>
                      <td>
                        <time dateTime={usage.last_used_at}>
                          {time(usage.last_used_at)}
                        </time>
                      </td>
                      <td>
                        <ArrowRight size={16} aria-hidden="true" />
                      </td>
                    </tr>
                  ))}
              </tbody>
            </table>
          </motion.section>
        ))
      )}
      <Drawer
        opened={access}
        onClose={() => void navigate({ to: "/agents", search: {} })}
        title="Access tokens"
        position="right"
        size="xl"
        className="control-drawer"
      >
        {access && <AgentAccessPanel code={code} />}
      </Drawer>
      <Drawer
        opened={!!inspected && !access}
        onClose={() => setSelected(null)}
        title="Agent record"
        position="right"
        size="lg"
        className="control-drawer"
      >
        {inspected && (
          <Stack gap="lg">
            <div className="creation-preview">
              <Terminal size={26} />
              <div>
                <strong>{inspected.name}</strong>
                <p>
                  {inspected.observed_hosts?.length
                    ? inspected.observed_hosts.map(hostLabel).join(" · ")
                    : hostLabel(inspected.host_kind)}{" "}
                  · {integrationLabel(inspected.integration)}
                </p>
              </div>
            </div>
            <Group>
              <Text>{inspected.user_name}</Text>
              <Badge variant="light">
                {inspected.claimed ? "Credential enabled" : "Waiting for host"}
              </Badge>
            </Group>
            {inspected.brains.map((brain) => (
              <div className="control-detail-section" key={brain.brain_id}>
                <strong>{brain.name}</strong>
                <Text size="sm">
                  Last observed use {time(brain.last_used_at)}
                </Text>
                <Button
                  variant="light"
                  renderRoot={(props) => (
                    <Link
                      {...props}
                      to="/brains/$brainId/activity"
                      params={{ brainId: brain.brain_id }}
                      search={{ tab: "pipeline" }}
                    />
                  )}
                >
                  Open Brain activity
                </Button>
              </div>
            ))}
          </Stack>
        )}
      </Drawer>
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
