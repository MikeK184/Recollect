import {
  Button,
  Code,
  CopyButton,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { Copy, Check } from "lucide-react";
import type { Brain } from "./api";
const quote = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;
export function SetupCopy({ value, label }: { value: string; label: string }) {
  return (
    <CopyButton value={value}>
      {({ copied, copy }) => (
        <Button variant="default" onClick={copy}>
          {copied ? "Copied" : label}
        </Button>
      )}
    </CopyButton>
  );
}
export function PluginInstall({
  host,
  directory,
  setDirectory,
}: {
  host: string;
  directory: string;
  setDirectory: (value: string) => void;
}) {
  const plugin = `${directory.replace(/\/$/, "")}/plugins/recollect-memory`;
  const install =
    host === "codex"
      ? `codex plugin marketplace add ${quote(directory)}\ncodex plugin add recollect-memory@recollect`
      : host === "opencode"
        ? JSON.stringify({ plugins: [`${plugin}/opencode/index.mjs`] }, null, 2)
        : `claude plugin marketplace add ${quote(directory)}\nclaude plugin install recollect-memory@recollect`;
  return (
    <Stack gap="md">
      <Title order={3}>Install the plugin</Title>
      <Text size="sm">
        Automatic recall and session capture. Keep the plugin package in a
        permanent folder.
      </Text>
      <TextInput
        label="Plugin package folder"
        value={directory}
        onChange={(e) => setDirectory(e.currentTarget.value)}
      />
      <Code block data-testid="agent-plugin-command">
        {install}
      </Code>
      <SetupCopy
        value={install}
        label={
          host === "opencode" ? "Copy plugin entry" : "Copy install commands"
        }
      />
      {host === "opencode" && (
        <Text size="xs" c="dimmed">
          Merge into the plugins list in opencode.json; keep existing entries.
        </Text>
      )}
      <details className="feature-advanced">
        <summary>Replacing an existing Recollect setup</summary>
        <Text size="sm" mt="sm">
          Drain pending captures using the original setup, then remove the
          previous first-party MCP entry before enabling the plugin. Verify the
          new connection before retiring old setup files. Preserve other plugins
          and credentials; do not run both capture paths for the same session.
        </Text>
      </details>
    </Stack>
  );
}
export function PluginConnect({
  brain,
  compact = false,
}: {
  brain: Brain;
  compact?: boolean;
}) {
  const connect = `Use the Recollect connect skill to connect to ${window.location.origin}, Brain ${brain.name} (${brain.id}). Use browser authorization and the OS credential store. Keep the optional execution runner disabled.`;
  if (compact)
    return (
      <div className="agent-compact-connect">
        <span>Connect to this Brain using your agent:</span>
        <div>
          <code>Use the Recollect connect skill for {brain.name}</code>
          <CopyButton value={connect}>
            {({ copied, copy }) => (
              <Button
                variant="subtle"
                size="compact-sm"
                onClick={copy}
                aria-label="Copy connection prompt"
              >
                {copied ? <Check size={18} /> : <Copy size={18} />}
              </Button>
            )}
          </CopyButton>
        </div>
        <details className="agent-prompt-details">
          <summary>View connection prompt</summary>
          <Code block>{connect}</Code>
          <Text size="xs">
            Restart your host, trust the plugin, then send this prompt. Approve
            the connection in your browser. Credentials stay in the operating
            system credential store.
          </Text>
        </details>
      </div>
    );
  return (
    <Stack gap="md">
      <Title order={3}>Connect once</Title>
      <Text size="sm">
        Restart your host, trust the plugin, then send this prompt. Approve the
        connection in your browser.
      </Text>
      <Code block>{connect}</Code>
      <SetupCopy value={connect} label="Copy connection prompt" />
      <Text size="xs" c="dimmed">
        Your credential stays in the operating system credential store.
      </Text>
    </Stack>
  );
}

export function PluginFinish({ brain }: { brain: Brain }) {
  const verify = `What relevant knowledge does Recollect already have for this workspace? Cite its sources and say if no relevant memory was available. Do not change any permissions.`;
  return (
    <Stack gap="md">
      <Title order={3}>Start a normal session</Title>
      <Text size="sm">
        The plugin recalls relevant memory and captures permitted evidence in
        the background. Check recorded activity in {brain.name}.
      </Text>
      <SetupCopy value={verify} label="Copy verification prompt" />
      <Text size="xs" c="dimmed">
        The next view shows Brain activity. Installation alone does not prove a
        read or upload from this agent. An empty Brain may have no relevant
        memory yet.
      </Text>
      <details className="feature-advanced">
        <summary>Optional tool execution</summary>
        <Text size="sm" mt="sm">
          The execution runner lets Recollect execute separately authorized
          connected tools on this computer or its private network. Automatic
          memory and your host’s ordinary coding tools do not require it. It
          stays off by default.
        </Text>
        <Text size="sm" mt="sm">
          To enable it explicitly, ask the connect skill to reconnect with{" "}
          <code>--with-runner</code>. Tool access still requires its own grants.
        </Text>
      </details>
    </Stack>
  );
}
