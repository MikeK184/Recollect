import { useEffect, useState } from "react";
import {
  Button,
  CopyButton,
  Drawer,
  Group,
  SegmentedControl,
  Text,
  Code,
  ActionIcon,
  Badge,
} from "@mantine/core";
import { useMediaQuery } from "@mantine/hooks";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { PluginInstall, PluginConnect } from "../../PluginAgentSetup";
import { Check, Clipboard, ArrowRight, KeyRound, X } from "lucide-react";
import { client, result } from "../../api";
import { useBrain } from "../../app/context";
import { McpAgentSetup, agentMemoryReadCheck } from "../../McpAgentSetup";
import { CapturePanel } from "../../CapturePanel";
import { PageHeader } from "../../components/PageHeader";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
import { HostIcon } from "../../components/HostIcon";
import { AgentsRoster } from "./AgentsRoster";
import { useBrainSearch } from "../../app/useBrainSearch";
import "./agents.css";

export function AgentsPage() {
  const brain = useBrain();
  const [host, setHost] = useState("codex");
  const [directory, setDirectory] = useState("/path/to/recollect-plugin");
  const [inspecting, setInspecting] = useState(false);
  const [search, updateSearch] = useBrainSearch();
  const navigate = useNavigate();
  const wide = useMediaQuery("(min-width: 1180px)");
  const device = search.device;
  const selected = useQuery({
    queryKey: ["brain-agents", brain.id, true],
    enabled: !!device,
    gcTime: 0,
    retry: false,
    refetchInterval: 5000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/agents", {
          signal,
          params: {
            path: { brain: brain.id },
            query: { include_hidden: true },
          },
        }),
      ),
  });
  const agent = selected.isError
    ? undefined
    : selected.data?.groups
        .flatMap((group) =>
          group.agents.map((value) => ({
            ...value,
            user_name: group.user_name,
          })),
        )
        .find((value) => value.device_id === device);
  useEffect(() => {
    if (search.tab === "contexts") {
      void navigate({
        to: ".",
        search: { ...search, tab: undefined, device: undefined },
        replace: true,
        resetScroll: false,
      });
      return;
    }
    if (search.tab === "sessions")
      void navigate({
        to: "/brains/$brainId/activity",
        params: { brainId: brain.id },
        search: { ...search, tab: "capture" },
        replace: true,
      });
  }, [brain.id, navigate, search]);
  const close = () => updateSearch({ device: undefined, tab: undefined });
  const activity = selected.isPending ? (
    <LoadingState label="Loading agent…" />
  ) : selected.isError ? (
    <ErrorState error={selected.error} retry={() => void selected.refetch()} />
  ) : !agent ? (
    <EmptyState
      title="Agent unavailable"
      description="This agent is no longer visible on this Brain."
    />
  ) : (
    <>
      <div className="agent-inspector-heading">
        <div className="management-icon agent-inspector-icon">
          <HostIcon host={agent.host_kind} size={24} />
        </div>
        <div className="agent-inspector-name">
          <h2>{agent.name}</h2>
          <small>{agent.user_name}</small>
        </div>
        {wide && (
          <ActionIcon
            variant="subtle"
            aria-label="Close agent activity"
            onClick={close}
          >
            <X size={18} />
          </ActionIcon>
        )}
      </div>
      <div className="agent-inspector-status">
        <Badge variant="light" color={agent.active ? "brand" : "gray"}>
          {agent.active ? "Credential enabled" : "Revoked or expired"}
        </Badge>
        <small>
          {agent.last_used_on_brain_at
            ? `Observed ${new Date(agent.last_used_on_brain_at).toLocaleString()}`
            : "No observed use"}
        </small>
      </div>
      <CapturePanel
        key={`${brain.id}:${device}`}
        brain={brain}
        section="sessions"
        deviceId={device}
        compactTimeline
        onInspectorChange={setInspecting}
      />
    </>
  );
  return (
    <div className="agents-management-page">
      <PageHeader
        title="Agents"
        description="Coding agents that have used this Brain."
        actions={
          <div className="agents-header-actions">
            <McpAgentSetup brain={brain} buttonLabel="Connect agent" />
            <Link to="/agents" search={{ access: true }}>
              <KeyRound size={16} /> Access tokens
            </Link>
            <Link to="/agents">
              My agents <ArrowRight size={18} />
            </Link>
          </div>
        }
      />
      <div className={`agents-layout${device && wide ? " inspecting" : ""}`}>
        <div className="agents-main">
          <AgentsRoster
            brain={brain}
            selectedDevice={device}
            onSelect={(id) => updateSearch({ device: id, tab: undefined })}
          />
          <div className="agents-utilities">
            <section className="management-surface agent-connect-utility">
              <h2 className="management-title">Connect your coding tool</h2>
              <Text size="sm" c="dimmed" mb="md">
                Install the plugin, choose a Brain, and start working.
              </Text>
              <SegmentedControl
                fullWidth
                className="vision-segment agent-host-tabs"
                value={host}
                onChange={setHost}
                data={[
                  {
                    value: "codex",
                    label: (
                      <span>
                        <HostIcon host="codex" size={24} />
                        Codex
                      </span>
                    ),
                  },
                  {
                    value: "claude",
                    label: (
                      <span>
                        <HostIcon host="claude" size={24} />
                        Claude Code
                      </span>
                    ),
                  },
                  {
                    value: "opencode",
                    label: (
                      <span>
                        <HostIcon host="opencode" size={24} />
                        OpenCode
                      </span>
                    ),
                  },
                ]}
              />
              <div className="agent-setup-prompt">
                <PluginConnect brain={brain} compact />
              </div>
              <details className="feature-advanced">
                <summary>
                  View setup for{" "}
                  {host === "claude"
                    ? "Claude Code"
                    : host === "opencode"
                      ? "OpenCode"
                      : "Codex"}
                </summary>
                <div className="feature-advanced-content">
                  <PluginInstall
                    host={host}
                    directory={directory}
                    setDirectory={setDirectory}
                  />
                </div>
              </details>
            </section>
            <section className="management-surface agent-memory-utility">
              <h2 className="management-title">Check memory access</h2>
              <Text size="md" c="dimmed" mb="md">
                Verify a real memory read from your agent.
              </Text>
              <details className="agent-prompt-details">
                <summary>View memory-read prompt</summary>
                <Code block className="agent-read-check">
                  {agentMemoryReadCheck(brain)}
                </Code>
              </details>
              <Group mt="lg">
                <CopyButton value={agentMemoryReadCheck(brain)}>
                  {({ copied, copy }) => (
                    <Button
                      variant="outline"
                      fullWidth
                      onClick={copy}
                      leftSection={
                        copied ? <Check size={21} /> : <Clipboard size={21} />
                      }
                    >
                      {copied ? "Copied" : "Copy memory-read check"}
                    </Button>
                  )}
                </CopyButton>
              </Group>
            </section>
          </div>
        </div>
        {!!device && wide && (
          <aside
            className="management-surface agent-activity-inspector"
            aria-label="Agent activity"
          >
            {!agent && (
              <Group justify="flex-end">
                <ActionIcon
                  variant="subtle"
                  aria-label="Close agent activity"
                  onClick={close}
                >
                  <X size={18} />
                </ActionIcon>
              </Group>
            )}
            {activity}
          </aside>
        )}
      </div>
      <Drawer
        closeOnEscape={!inspecting}
        closeOnClickOutside={!inspecting}
        trapFocus={!inspecting}
        className="feature-drawer"
        opened={!!device && !wide}
        onClose={close}
        title={agent ? `${agent.name} · ${agent.user_name}` : "Agent activity"}
        position="right"
        size="md"
      >
        {!!device && !wide && activity}
      </Drawer>
    </div>
  );
}
