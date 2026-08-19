import { useState } from "react";
import {
  Alert,
  Button,
  Code,
  CopyButton,
  Group,
  Modal,
  Select,
  Stack,
  Text,
} from "@mantine/core";
import type { Brain } from "./api";

const shell = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;

export function McpAgentSetup({ brain }: { brain: Brain }) {
  const [opened, setOpened] = useState(false);
  const [host, setHost] = useState("codex");
  const [capture, setCapture] = useState("tools");
  const command = `RECOLLECT_URL=${shell(window.location.origin)} recollect-agent ${capture === "capture" ? "capture setup" : "mcp-config"} ${host} ${capture === "capture" ? ". " : ""}--brain ${brain.id}${capture === "tools" ? " --directory ." : ""}`;
  return (
    <>
      <Button
        size="xs"
        variant="light"
        onClick={() => setOpened(true)}
        disabled={brain.archived}
      >
        Connect coding agent
      </Button>
      <Modal
        opened={opened}
        onClose={() => setOpened(false)}
        title="Connect a coding agent"
        size="lg"
      >
        <Stack>
          <Text size="sm">
            Use Recollect memory, task scopes and approved tools from your
            coding agent. Run these commands from the workspace you want to use.
          </Text>
          <Text size="sm">
            Install the companion binaries together, then pair this device
            through Devices:
          </Text>
          <Code
            block
          >{`RECOLLECT_URL=${shell(window.location.origin)} recollect-agent pair`}</Code>
          <Select
            label="Coding host"
            value={host}
            onChange={(value) => value && setHost(value)}
            data={[
              { value: "codex", label: "Codex" },
              { value: "claude", label: "Claude Code" },
            ]}
            allowDeselect={false}
          />
          <Select
            label="Session integration"
            value={capture}
            onChange={(value) => value && setCapture(value)}
            data={[
              { value: "tools", label: "Memory and workspace tools" },
              {
                value: "capture",
                label: "Tools with automatic session capture",
              },
            ]}
            allowDeselect={false}
          />
          <Code block data-testid="agent-setup-command">
            {command}
          </Code>
          <Group justify="space-between">
            <Text size="xs" c="dimmed">
              Brain: {brain.name}
            </Text>
            <CopyButton value={command}>
              {({ copied, copy }) => (
                <Button size="xs" variant="light" onClick={copy}>
                  {copied ? "Copied" : "Copy setup command"}
                </Button>
              )}
            </CopyButton>
          </Group>
          <Text size="sm">
            {capture === "tools"
              ? "The command prints settings and launch arguments for this Brain. Add them to your host’s project settings or pass the arguments when launching it."
              : "The command creates a capture setup and prints its launch command. Use that command to start the host with tools and automatic capture under the Brain’s capture policy."}
          </Text>
          <Text size="sm">
            Pairing credentials stay in your OS credential store. Your OS may
            request access when the bridge first starts. Set
            RECOLLECT_DEVICE_PROFILE if you use a named pairing.
          </Text>
          <Alert color="blue" title="Connection options">
            Vault is optional for each managed MCP connection. Environment
            credentials, OS-stored credentials and anonymous access work
            independently. Managed tools still require profile Use permission.
          </Alert>
          <Text size="xs" c="dimmed">
            A scope change returns fresh context. With managed capture, later
            turns adopt the new scope; existing turns and delayed tool results
            retain their original scope. Creating settings alone does not prove
            a connection.
          </Text>
        </Stack>
      </Modal>
    </>
  );
}
