import { useEffect, useRef, useState } from "react";
import {
  Badge,
  Button,
  Code,
  Group,
  Modal,
  PasswordInput,
  Select,
  Stack,
  Stepper,
  Text,
  Title,
  TextInput,
} from "@mantine/core";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Bot, Plug, Plus, ArrowRight } from "lucide-react";
import { client, result, type Brain } from "./api";
import {
  PluginInstall,
  PluginConnect,
  PluginFinish,
  SetupCopy,
} from "./PluginAgentSetup";
import { ErrorState } from "./components/AsyncState";
import { CodeBlock } from "./components/CodeBlock";
import {
  directAgentConfig,
  directAgentCredentialName,
} from "./features/agents/agentConfig";
import "./features/guided-controls.css";
const hostName = (host: string) =>
  host === "codex" ? "Codex" : host === "opencode" ? "OpenCode" : "Claude Code";
export function agentMemoryReadCheck(brain: { id: string; name: string }) {
  return `Use Recollect workspace.list to verify access to Brain ${brain.name} (${brain.id}). Then start a task for this workspace and use memory.recall to find a piece of knowledge from it. Report the actual result and its citations, and the exact error text if any step fails. Do not change permissions or call connected tools.`;
}
export function McpAgentSetup({
  brain,
  initialHost,
  buttonLabel = "Connect coding agent",
}: {
  brain: Brain;
  initialHost?: string;
  buttonLabel?: string;
}) {
  const [opened, setOpened] = useState(false);
  const [step, setStep] = useState(0);
  const [host, setHost] = useState(initialHost ?? "codex");
  const [method, setMethod] = useState("plugin");
  const [directory, setDirectory] = useState("/path/to/recollect-plugin");
  const [credential, setCredential] = useState<{
    token: string;
    brainId: string;
  } | null>(null);
  const token = credential?.brainId === brain.id ? credential.token : null;
  const [revealed, setRevealed] = useState(false);
  const [credentialName, setCredentialName] = useState("");
  const cache = useQueryClient();
  const activeBrain = useRef(brain.id);
  activeBrain.current = brain.id;
  const url = `${window.location.origin}/api/brains/${brain.id}/mcp/agent`;
  const hostLabel = hostName(host);
  const hostConfigFile =
    host === "codex"
      ? ".codex/config.toml"
      : host === "opencode"
        ? "opencode.json"
        : ".mcp.json";
  const config = token ? directAgentConfig(host, url, token) : "";
  const maskedConfig = directAgentConfig(host, url, "••••••••");
  useEffect(() => {
    if (!token)
      setCredentialName(`${hostLabel} MCP · ${brain.name}`.slice(0, 80));
  }, [hostLabel, brain.name, token]);
  useEffect(() => {
    setCredential(null);
    setRevealed(false);
    setOpened(false);
    setStep(0);
  }, [brain.id]);
  const create = useMutation({
    mutationFn: async () => {
      const pairing = result(
        await client.POST("/api/devices/pairings", {
          body: {
            name: directAgentCredentialName(
              credentialName,
              crypto.randomUUID(),
            ),
            host_kind: host === "claude" ? "claude_code" : host,
            integration: "mcp",
          },
        }),
      );
      try {
        result(
          await client.POST("/api/devices/pairings/{code}/approve", {
            params: { path: { code: pairing.user_code } },
            body: { approve: true },
          }),
        );
        const credential = result(
          await client.POST("/api/devices/pairings/poll", {
            body: { device_code: pairing.device_code },
          }),
        );
        if (!credential.token)
          throw new Error("No access token was returned. Try again.");
        const finished = await client.POST("/api/devices/pairings/finish", {
          body: { device_code: pairing.device_code },
        });
        if (!finished.response.ok)
          throw new Error(
            "Could not finish creating this token. Check Access tokens before retrying.",
          );
        return { token: credential.token, brainId: brain.id };
      } catch (error) {
        await client.POST("/api/devices/pairings/cancel", {
          body: { device_code: pairing.device_code },
        });
        throw error;
      }
    },
    onSuccess: (value) => {
      if (activeBrain.current === value.brainId) setCredential(value);
      create.reset();
      void cache.invalidateQueries({ queryKey: ["devices"] });
      void cache.invalidateQueries({ queryKey: ["account-agents"] });
    },
    gcTime: 0,
  });
  const close = () => {
    if (create.isPending) return;
    setCredential(null);
    setRevealed(false);
    create.reset();
    setOpened(false);
    setStep(0);
  };
  const plugin = method === "plugin";
  const labels = plugin
    ? ["Host", "Install", "Connect", "Finish"]
    : ["Host", "Token", "Configure", "Finish"];
  return (
    <>
      <Button
        variant="filled"
        leftSection={<Plus size={20} />}
        disabled={brain.archived}
        onClick={() => setOpened(true)}
      >
        {buttonLabel}
      </Button>
      <Modal
        opened={opened}
        onClose={close}
        title="Connect a coding agent"
        size="lg"
        centered
        closeOnEscape={!create.isPending}
        closeOnClickOutside={!create.isPending}
      >
        <Stack gap="lg">
          <Stepper
            active={step}
            size="xs"
            allowNextStepsSelect={false}
            className="setup-progress"
          >
            {labels.map((label) => (
              <Stepper.Step key={label} label={label} allowStepSelect={false} />
            ))}
          </Stepper>
          {step > 0 && (
            <Group gap="xs">
              <Badge variant="light" color="gray">
                {hostLabel}
              </Badge>
              <Badge variant="light" color="brand">
                {plugin ? "Plugin" : "Direct MCP"}
              </Badge>
            </Group>
          )}
          <div
            key={`${method}-${step}`}
            className="setup-stage rc-page-enter"
            data-testid={`agent-setup-stage-${step + 1}`}
          >
            {step === 0 && (
              <Stack gap="md">
                <Title order={3}>Choose your coding host</Title>
                <Select
                  label="Coding host"
                  value={host}
                  onChange={(v) => v && setHost(v)}
                  disabled={!!token || create.isPending}
                  allowDeselect={false}
                  data={[
                    { value: "codex", label: "Codex" },
                    { value: "claude", label: "Claude Code" },
                    { value: "opencode", label: "OpenCode" },
                  ]}
                />
                <div
                  className="setup-choices"
                  role="group"
                  aria-label="Connection method"
                >
                  <button
                    type="button"
                    className={`setup-choice ${plugin ? "selected" : ""}`}
                    aria-pressed={plugin}
                    disabled={!!token}
                    onClick={() => setMethod("plugin")}
                  >
                    <Bot size={22} />
                    <strong>
                      Recollect plugin <span>Recommended</span>
                    </strong>
                    <small>Automatic recall and session capture.</small>
                  </button>
                  <button
                    type="button"
                    className={`setup-choice ${!plugin ? "selected" : ""}`}
                    aria-pressed={!plugin}
                    disabled={!!token}
                    onClick={() => setMethod("direct")}
                  >
                    <Plug size={22} />
                    <strong>Direct MCP</strong>
                    <small>Memory tools. Configure your host manually.</small>
                  </button>
                </div>
                {!!token && (
                  <Text size="xs" c="dimmed">
                    Host and method are fixed for this token. Close to begin a
                    different setup.
                  </Text>
                )}
              </Stack>
            )}
            {plugin && step === 1 && (
              <PluginInstall
                host={host}
                directory={directory}
                setDirectory={setDirectory}
              />
            )}
            {plugin && step === 2 && <PluginConnect brain={brain} />}
            {plugin && step === 3 && <PluginFinish brain={brain} />}
            {!plugin && step === 1 && (
              <Stack gap="md">
                <Title order={3}>Create your access token</Title>
                <Text size="sm">
                  Acts as you across Brains you can access. This URL selects{" "}
                  {brain.name}; it does not restrict the token to this Brain.
                  Expires after 30 days. Manage it in{" "}
                  <Link to="/agents" search={{ access: true }}>
                    Access tokens
                  </Link>
                  .
                </Text>
                <TextInput
                  label="Token name"
                  value={credentialName}
                  maxLength={80}
                  disabled={!!token || create.isPending}
                  onChange={(event) =>
                    setCredentialName(event.currentTarget.value)
                  }
                />
                {token ? (
                  <>
                    <PasswordInput
                      label="Access token · shown once"
                      value={token}
                      readOnly
                      autoComplete="off"
                    />
                    <SetupCopy value={token} label="Copy access token" />
                    <Text size="xs" c="dimmed">
                      Kept here while you go Back or Next. Closing clears this
                      copy.
                    </Text>
                  </>
                ) : (
                  <Button
                    loading={create.isPending}
                    disabled={!credentialName.trim()}
                    onClick={() => create.mutate()}
                  >
                    Create access token
                  </Button>
                )}
                <ErrorState error={create.error} />
              </Stack>
            )}
            {!plugin && step === 2 && (
              <Stack gap="md">
                <Title order={3}>Configure {hostLabel}</Title>
                <Text size="sm">
                  Merge this into <code>{hostConfigFile}</code>, preserving your
                  existing servers.
                </Text>
                <div data-testid="agent-setup-command">
                  {revealed ? (
                    <CodeBlock
                      code={config}
                      copyLabel="Copy MCP configuration"
                    />
                  ) : (
                    <CodeBlock
                      code={maskedConfig}
                      language={host === "codex" ? "toml" : "json"}
                      copyCode={config}
                      copyLabel="Copy MCP configuration"
                    />
                  )}
                </div>
                <Button
                  variant="subtle"
                  onClick={() => setRevealed((value) => !value)}
                >
                  {revealed
                    ? "Hide token in configuration"
                    : "Reveal token in configuration"}
                </Button>
                <details className="feature-advanced">
                  <summary>Show the one-time credential</summary>
                  <PasswordInput
                    label="Credential"
                    readOnly
                    value={token ?? ""}
                    autoComplete="off"
                  />
                  <SetupCopy value={token ?? ""} label="Copy credential" />
                </details>
                <Text size="sm">
                  Copy includes your token. Save in a private, untracked
                  configuration file, then start your host normally.
                </Text>
              </Stack>
            )}
            {!plugin && step === 3 && (
              <Stack gap="md">
                <Title order={3}>Instructions complete</Title>
                <Text size="sm">
                  Ask your agent to read memory. Configuration alone does not
                  verify the connection.
                </Text>
                <SetupCopy
                  value={agentMemoryReadCheck(brain)}
                  label="Copy verification prompt"
                />
                <details className="feature-advanced">
                  <summary>Verification prompt</summary>
                  <Code block data-testid="agent-verification-prompt">
                    {agentMemoryReadCheck(brain)}
                  </Code>
                </details>
              </Stack>
            )}
          </div>
          <Group justify="space-between" className="setup-footer">
            <Button
              variant="default"
              disabled={step === 0 || create.isPending}
              onClick={() => setStep((v) => v - 1)}
            >
              Back
            </Button>
            <Text size="xs" c="dimmed">
              {step + 1} of 4
            </Text>
            {step < 3 ? (
              <Button
                rightSection={<ArrowRight size={16} />}
                disabled={
                  create.isPending ||
                  (!plugin && step === 1 && !token) ||
                  (plugin &&
                    step === 1 &&
                    (!directory.trim() ||
                      directory === "/path/to/recollect-plugin"))
                }
                onClick={() => setStep((v) => v + 1)}
              >
                Next
              </Button>
            ) : (
              <Button
                renderRoot={(props) => (
                  <Link
                    {...props}
                    to="/brains/$brainId/activity"
                    params={{ brainId: brain.id }}
                    search={{ tab: "capture" }}
                    onClick={close}
                  />
                )}
                rightSection={<ArrowRight size={16} />}
              >
                View recorded activity
              </Button>
            )}
          </Group>
        </Stack>
      </Modal>
    </>
  );
}
