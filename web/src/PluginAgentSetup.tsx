import { useState, type ReactNode } from "react";
import {
  Alert,
  Button,
  Code,
  CopyButton,
  Modal,
  Select,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import type { Brain } from "./api";

const quote = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;
function Copy({ value, label }: { value: string; label: string }) {
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

export function PluginAgentSetup({
  brain,
  initialHost,
  buttonLabel = "Connect coding agent",
  advanced,
}: {
  brain: Brain;
  initialHost?: string;
  buttonLabel?: string;
  advanced: ReactNode;
}) {
  const [opened, setOpened] = useState(false);
  const [host, setHost] = useState(initialHost ?? "codex");
  const [directory, setDirectory] = useState("/path/to/recollect-plugin");
  const plugin = `${directory.replace(/\/$/, "")}/plugins/recollect-memory`;
  const install =
    host === "codex"
      ? `codex plugin marketplace add ${quote(directory)}\ncodex plugin add recollect-memory@recollect`
      : host === "opencode"
        ? JSON.stringify({ plugins: [`${plugin}/opencode/index.mjs`] }, null, 2)
        : `claude plugin marketplace add ${quote(directory)}\nclaude plugin install recollect-memory@recollect`;
  const connect = `Use the Recollect connect skill to connect to ${window.location.origin}, Brain ${brain.name} (${brain.id}). Use browser authorization and the OS credential store. Keep the optional execution runner disabled.`;
  const verify = `What relevant knowledge does Recollect already have for this workspace? Cite its sources and say if no relevant memory was available. Do not change any permissions.`;
  return (
    <>
      <Button
        variant="light"
        disabled={brain.archived}
        onClick={() => setOpened(true)}
      >
        {buttonLabel}
      </Button>
      <Modal
        opened={opened}
        onClose={() => setOpened(false)}
        title="Connect a coding agent"
        size="lg"
      >
        <Stack gap="lg">
          <Stack gap="sm" data-testid="agent-setup-stage-1">
            <Title order={4}>Step 1 · Install the Recollect plugin</Title>
            <Text size="sm">
              Use the Recollect plugin package for your computer. It includes
              memory tools, automatic recall and session capture. Keep the
              package in a permanent folder.
            </Text>
            <Select
              label="Coding host"
              value={host}
              onChange={(value) => value && setHost(value)}
              allowDeselect={false}
              data={[
                { value: "codex", label: "Codex" },
                { value: "claude", label: "Claude Code" },
                { value: "opencode", label: "OpenCode" },
              ]}
            />
            <TextInput
              label="Plugin package folder"
              value={directory}
              onChange={(event) => setDirectory(event.currentTarget.value)}
            />
            {host === "opencode" && (
              <Text size="sm">
                Add this entry to the plugins list in your opencode.json.
                Preserve your existing entries.
              </Text>
            )}
            <Code
              block
              style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}
            >
              {install}
            </Code>
            <Copy
              value={install}
              label={
                host === "opencode"
                  ? "Copy plugin entry"
                  : "Copy install commands"
              }
            />
          </Stack>
          <Stack gap="sm" data-testid="agent-setup-stage-2">
            <Title order={4}>Step 2 · Connect once</Title>
            <Text size="sm">
              Restart your coding host, trust the installed plugin when asked,
              and send this prompt. Approve the connection in your browser. Your
              credential stays in the operating system credential store.
            </Text>
            <Code block style={{ whiteSpace: "pre-wrap" }}>
              {connect}
            </Code>
            <Copy value={connect} label="Copy connection prompt" />
          </Stack>
          <Stack gap="sm" data-testid="agent-setup-stage-3">
            <Title order={4}>Step 3 · Start a normal session</Title>
            <Text size="sm">
              The plugin gives your agent relevant, cited memory before it
              answers. Permitted session evidence uploads in the background.
              Check Captured sessions for actual delivery and coverage gaps.
            </Text>
            <Copy value={verify} label="Copy verification prompt" />
            <Text size="sm" c="dimmed">
              An empty Brain may have no relevant memory yet. Installation alone
              does not prove a successful memory read or upload.
            </Text>
          </Stack>
          <Alert color="gray" title="Existing Recollect setup">
            Drain pending captures with their original setup, then remove the
            previous first-party MCP entry before enabling this plugin. Verify
            the new connection before retiring the old setup files. Preserve
            other plugins and credentials; do not run both capture paths for the
            same session.
          </Alert>
          <details className="feature-advanced">
            <summary>Optional execution runner</summary>
            <Text size="sm">
              Enable this only if Recollect should independently run approved
              tools on this computer or its private network. Memory and ordinary
              coding tools work without it. Ask the connect skill to include{" "}
              <Code>--with-runner</Code>; an existing private registration can
              also use <Code>--runner-id UUID</Code>. Reconnect without the flag
              to disable it.
            </Text>
          </details>
          <details className="feature-advanced">
            <summary>Advanced · Direct MCP connection</summary>
            <Text size="sm" mb="sm">
              Use direct HTTP when you only want memory tools. Automatic session
              capture and prompt recall are provided by the plugin.
            </Text>
            {advanced}
          </details>
        </Stack>
      </Modal>
    </>
  );
}
