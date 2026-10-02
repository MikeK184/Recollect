import { useState, type ReactNode } from "react";
import {
  Alert,
  Button,
  Code,
  CopyButton,
  Modal,
  PasswordInput,
  Select,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useMutation } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { client, result, type Brain } from "./api";
import { PluginAgentSetup } from "./PluginAgentSetup";
import { ErrorState } from "./components/AsyncState";

const shell = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;

const hostName = (host: string) =>
  host === "codex" ? "Codex" : host === "opencode" ? "OpenCode" : "Claude Code";

// The connect flow ends here: a real memory read is the only proof of a live
// agent connection. One definition, used by the setup modal and the Agents page.
export function agentMemoryReadCheck(brain: { id: string; name: string }) {
  return `Use Recollect workspace.list to verify access to Brain ${brain.name} (${brain.id}). Then start a task for this workspace and use memory.recall to find a piece of knowledge from it. Report the actual result and its citations, and the exact error text if any step fails. Do not change permissions or call connected tools.`;
}

/** One numbered stage of the connect flow. Headings only: no state machine, so
 * every stage stays readable while an earlier one is still incomplete. */
function Stage({
  step,
  title,
  children,
}: {
  step: number;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <Stack gap="sm" data-testid={`agent-setup-stage-${step}`}>
      <Title order={4}>{`Step ${step} · ${title}`}</Title>
      {children}
    </Stack>
  );
}

function DirectAgentSetup({
  brain,
  initialHost,
  buttonLabel = "Connect coding agent",
}: {
  brain: Brain;
  initialHost?: string;
  buttonLabel?: string;
}) {
  const [opened, setOpened] = useState(false);
  const [host, setHost] = useState(initialHost ?? "codex");
  const [token, setToken] = useState<string | null>(null);
  const [credentialStore, setCredentialStore] = useState(
    /Mac/.test(navigator.platform) ? "keychain" : "environment",
  );
  const url = `${window.location.origin}/api/brains/${brain.id}/mcp/agent`;
  const hostLabel = hostName(host);
  const hostConfigFile =
    host === "codex"
      ? ".codex/config.toml"
      : host === "opencode"
        ? "opencode.json"
        : ".mcp.json";
  const keychain = host === "codex" && credentialStore === "keychain";
  const keychainService = `Recollect MCP: ${url}`;
  const keychainRead = `/usr/bin/security find-generic-password -a recollect -s ${shell(keychainService)} -w`;
  const connectionSecret = token
    ? keychain
      ? JSON.stringify({ Authorization: `Bearer ${token}` })
      : token
    : "";
  const config =
    host === "codex"
      ? `[mcp_servers.recollect]\nurl = ${JSON.stringify(url)}\n${keychain ? `http_headers_helper = ${JSON.stringify(keychainRead)}` : 'bearer_token_env_var = "RECOLLECT_MCP_TOKEN"'}`
      : host === "opencode"
        ? JSON.stringify(
            {
              mcp: {
                servers: {
                  recollect: {
                    type: "remote",
                    url,
                    oauth: false,
                    headers: {
                      Authorization: "Bearer {env:RECOLLECT_MCP_TOKEN}",
                    },
                  },
                },
              },
            },
            null,
            2,
          )
        : JSON.stringify(
            {
              mcpServers: {
                recollect: {
                  type: "http",
                  url,
                  headers: { Authorization: "Bearer ${RECOLLECT_MCP_TOKEN}" },
                },
              },
            },
            null,
            2,
          );
  const check = agentMemoryReadCheck(brain);
  const create = useMutation({
    mutationFn: async () => {
      const pairing = result(
        await client.POST("/api/devices/pairings", {
          body: {
            name: `${hostLabel} MCP · ${brain.name}`.slice(0, 120),
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
            "Could not finish creating this token. Check Devices before retrying.",
          );
        return credential.token;
      } catch (error) {
        await client.POST("/api/devices/pairings/cancel", {
          body: { device_code: pairing.device_code },
        });
        throw error;
      }
    },
    onSuccess: (value) => {
      setToken(value);
      create.reset();
    },
    gcTime: 0,
  });
  const close = () => {
    if (create.isPending) return;
    setToken(null);
    create.reset();
    setOpened(false);
  };
  return (
    <>
      <Button
        variant="light"
        onClick={() => setOpened(true)}
        disabled={brain.archived}
      >
        {buttonLabel}
      </Button>
      <Modal
        opened={opened}
        onClose={close}
        title={
          initialHost
            ? `Connect ${hostName(initialHost)}`
            : "Connect a coding agent"
        }
        size="lg"
      >
        <Stack gap="lg">
          <Stage step={1} title="Choose your coding host">
            <Text size="sm">
              Memory tools work over a standard MCP connection.
            </Text>
            {!initialHost && (
              <Select
                label="Coding host"
                value={host}
                onChange={(v) => v && setHost(v)}
                data={[
                  { value: "codex", label: "Codex" },
                  { value: "claude", label: "Claude Code" },
                  { value: "opencode", label: "OpenCode" },
                ]}
                allowDeselect={false}
              />
            )}
            {host === "codex" && (
              <Select
                label="Store your access token"
                value={credentialStore}
                onChange={(value) => value && setCredentialStore(value)}
                data={[
                  { value: "keychain", label: "macOS Keychain" },
                  { value: "environment", label: "Environment variable" },
                ]}
                allowDeselect={false}
                description={
                  keychain
                    ? "Codex reads Keychain directly. Start Codex normally after setup."
                    : "The token must be available in the terminal that starts your agent."
                }
              />
            )}
          </Stage>
          <Stage step={2} title="Bind this Brain and create your access token">
            <Text size="sm">
              The server URL below selects {brain.name}, so this connection
              cannot drift onto another Brain. Creating the access token pairs
              it as a device record that acts as you, with your current Brain
              permissions. Manage or revoke it later in{" "}
              <Link
                to="/brains/$brainId/agents"
                params={{ brainId: brain.id }}
              >
                this Brain&apos;s Agents list
              </Link>
              ; nothing here revokes anything.
            </Text>
            {token ? (
              <>
                <PasswordInput
                  label={
                    keychain
                      ? "Keychain secret · shown once"
                      : "Access token · shown once"
                  }
                  readOnly
                  value={connectionSecret}
                  autoComplete="off"
                />
                <CopyButton value={connectionSecret}>
                  {({ copied, copy }) => (
                    <Button variant="light" onClick={copy}>
                      {copied
                        ? "Copied"
                        : keychain
                          ? "Copy Keychain secret"
                          : "Copy access token"}
                    </Button>
                  )}
                </CopyButton>
                <Alert color="brand" title="Token created · not yet connected">
                  <Text size="sm">
                    Saving this token in your host configures the connection. It
                    proves nothing on its own: only a successful Recollect tool
                    call in step 5 does.
                  </Text>
                </Alert>
              </>
            ) : (
              <Button
                loading={create.isPending}
                onClick={() => create.mutate()}
              >
                Create access token
              </Button>
            )}
            <ErrorState error={create.error} />
          </Stage>
          <Stage step={3} title="Add it to your host">
            <Text size="sm">
              Add this to <code>{hostConfigFile}</code> in your project, keeping
              any existing servers.
              {host === "opencode"
                ? " Merge its mcp.servers entry into your opencode.json."
                : ""}
            </Text>
            <Code block data-testid="agent-setup-command">
              {config}
            </Code>
            <CopyButton value={config}>
              {({ copied, copy }) => (
                <Button variant="default" onClick={copy}>
                  {copied ? "Copied" : "Copy MCP configuration"}
                </Button>
              )}
            </CopyButton>
            {!keychain && (
              <Text size="sm">
                Keep <code>RECOLLECT_MCP_TOKEN</code> exactly as written in the
                configuration. It is a variable name; your access token goes
                into the hidden terminal prompt below.
              </Text>
            )}
            {keychain ? (
              <>
                <Text size="sm">
                  Copy the Keychain secret, run this command in Terminal, and
                  paste at the hidden password prompt. Paste it again if asked
                  to confirm. This saves the credential for this Brain in
                  Keychain.
                </Text>
                <Code block data-testid="agent-credential-command">
                  {`/usr/bin/security add-generic-password -U -a recollect -s ${shell(keychainService)} -w`}
                </Code>
                <Text size="sm">
                  Start {hostLabel} normally in your project. {hostLabel}{" "}
                  0.157.1 is verified with this Keychain setup.
                </Text>
              </>
            ) : (
              <>
                <Text size="sm">
                  Copy the access token. In a bash or zsh terminal in your
                  project, run this block and paste the token at the hidden
                  prompt. The agent starts with the token; repeat this step for
                  a new terminal.
                </Text>
                <Code block data-testid="agent-credential-command">
                  {`printf 'Recollect access token: '\nread -r -s RECOLLECT_MCP_TOKEN\nprintf '\\n'\nexport RECOLLECT_MCP_TOKEN\n${host}`}
                </Code>
                <Text size="sm">
                  An already running agent does not receive this variable. Start
                  it again from that terminal.
                </Text>
              </>
            )}
          </Stage>
          <Stage step={4} title="Set your working context">
            <Text size="sm">
              Ask your restarted agent to call <code>workspace.list</code>. It
              returns this Brain&apos;s repositories, areas, environments and
              your own task inventory. The agent then starts a task with{" "}
              <code>workspace.start_task</code> and narrows future work with{" "}
              <code>workspace.set_scope</code>. Those task scopes are private to
              your account and appear under Agents → Your working contexts.
            </Text>
          </Stage>
          <Stage step={5} title="Verify with a real memory read">
            <Text size="sm">
              Run this prompt in your agent. Until it returns a real result,
              this connection is configured, not connected.
            </Text>
            <CopyButton value={check}>
              {({ copied, copy }) => (
                <Button variant="default" onClick={copy}>
                  {copied ? "Copied" : "Copy verification prompt"}
                </Button>
              )}
            </CopyButton>
            <details className="feature-advanced">
              <summary>Verification prompt</summary>
              <Code block data-testid="agent-verification-prompt">
                {check}
              </Code>
            </details>
          </Stage>
          <Text size="xs" c="dimmed">
            The token uses your current Brain permissions. Revoke it in this
            Brain&apos;s Agents list. This connection selects {brain.name}.
          </Text>
        </Stack>
      </Modal>
    </>
  );
}
export function McpAgentSetup(props: {
  brain: Brain;
  initialHost?: string;
  buttonLabel?: string;
}) {
  return (
    <PluginAgentSetup
      {...props}
      advanced={<DirectAgentSetup {...props} buttonLabel="Set up direct MCP" />}
    />
  );
}
