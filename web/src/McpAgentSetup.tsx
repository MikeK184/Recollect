import { useState } from "react";
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
} from "@mantine/core";
import { useMutation } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { ErrorState } from "./components/AsyncState";

const shell = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;

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
  const [host, setHost] = useState(initialHost ?? "codex");
  const [token, setToken] = useState<string | null>(null);
  const [credentialStore, setCredentialStore] = useState(
    /Mac/.test(navigator.platform) ? "keychain" : "environment",
  );
  const url = `${window.location.origin}/api/brains/${brain.id}/mcp/agent`;
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
  const create = useMutation({
    mutationFn: async () => {
      const pairing = result(
        await client.POST("/api/devices/pairings", {
          body: {
            name: `${host === "codex" ? "Codex" : "Claude Code"} MCP · ${brain.name}`.slice(
              0,
              120,
            ),
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
            ? `Connect ${initialHost === "codex" ? "Codex" : "Claude Code"}`
            : "Connect a coding agent"
        }
        size="lg"
      >
        <Stack gap="lg">
          <Text>
            Connect directly over MCP to recall and contribute memory. No
            Recollect companion is required.
          </Text>
          {!initialHost && (
            <Select
              label="Coding host"
              value={host}
              onChange={(v) => v && setHost(v)}
              data={[
                { value: "codex", label: "Codex" },
                { value: "claude", label: "Claude Code" },
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
          <Text size="sm">
            Add this to{" "}
            <code>{host === "codex" ? ".codex/config.toml" : ".mcp.json"}</code>{" "}
            in your project, keeping any existing servers.
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
              configuration. It is a variable name; your access token goes into
              the hidden terminal prompt below.
            </Text>
          )}
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
              <Alert color="teal">
                Token created. Save it using the steps below, then restart your
                agent. A successful Recollect tool call confirms the connection.
              </Alert>
            </>
          ) : (
            <Button loading={create.isPending} onClick={() => create.mutate()}>
              Create access token
            </Button>
          )}
          {keychain ? (
            <>
              <Text size="sm">
                Copy the Keychain secret, run this command in Terminal, and
                paste at the hidden password prompt. Paste it again if asked to
                confirm. This saves the credential for this Brain in Keychain.
              </Text>
              <Code block data-testid="agent-credential-command">
                {`/usr/bin/security add-generic-password -U -a recollect -s ${shell(keychainService)} -w`}
              </Code>
              <Text size="sm">
                Start Codex normally in your project and ask it to use
                Recollect&apos;s <code>workspace.list</code>. Codex 0.157.1 is
                verified with this Keychain setup.
              </Text>
            </>
          ) : (
            <>
              <Text size="sm">
                Copy the access token. In a bash or zsh terminal in your
                project, run this block and paste the token at the hidden
                prompt. The agent starts with the token; repeat this step for a
                new terminal.
              </Text>
              <Code block data-testid="agent-credential-command">
                {`printf 'Recollect access token: '\nread -r -s RECOLLECT_MCP_TOKEN\nprintf '\\n'\nexport RECOLLECT_MCP_TOKEN\n${host === "codex" ? "codex" : "claude"}`}
              </Code>
              <Text size="sm">
                An already running agent does not receive this variable. Ask the
                newly started agent to use Recollect&apos;s{" "}
                <code>workspace.list</code>.
              </Text>
            </>
          )}
          <ErrorState error={create.error} />
          <Text size="xs" c="dimmed">
            The token uses your current Brain permissions. Revoke it under
            Devices. This connection selects {brain.name}.
          </Text>
          <details className="feature-advanced">
            <summary>Automatic session capture</summary>
            <Stack gap="sm">
              <Text size="sm">
                MCP gives your agent memory tools. Capturing supported
                conversations and discovering local repositories also needs the
                Recollect companion and its host hooks.
              </Text>
              <Code
                block
              >{`RECOLLECT_URL=${shell(window.location.origin)} recollect-agent pair\nRECOLLECT_URL=${shell(window.location.origin)} recollect-agent capture setup ${host} . --brain ${brain.id}`}</Code>
              <Text size="sm">
                Pairing uses a short browser approval code. A packaged plugin
                and native MCP OAuth sign-in are not available yet.
              </Text>
            </Stack>
          </details>
        </Stack>
      </Modal>
    </>
  );
}
