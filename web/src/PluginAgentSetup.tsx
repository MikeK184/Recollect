import { Check, Copy } from "lucide-react";
import type { Brain } from "./api";
import { TerminalCard } from "./components/TerminalCard";
import { Button, Code, CopyButton, TextField } from "./components/ui";
import { Disclosure } from "./features/connections/ui";
const quote = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;

const stepTitle = "m-0 text-title font-semibold text-foreground";
const stepText = "m-0 text-body text-foreground-secondary";
const stepNote = "m-0 text-caption text-muted-foreground";

export function SetupCopy({ value, label }: { value: string; label: string }) {
  return (
    <CopyButton value={value}>
      {({ copied, copy }) => (
        <Button
          variant="secondary"
          className="justify-self-start"
          onClick={copy}
        >
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
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
    <div className="grid gap-3">
      <h3 className={stepTitle}>Install the plugin</h3>
      <p className={stepText}>
        Automatic recall and session capture. Keep the plugin package in a
        permanent folder.
      </p>
      <TextField
        label="Plugin package folder"
        inputClassName="mono text-label"
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
        <p className={stepNote}>
          Merge into the plugins list in opencode.json; keep existing entries.
        </p>
      )}
      <Disclosure summary="Replacing an existing Recollect setup">
        <p className={stepText}>
          Drain pending captures using the original setup, then remove the
          previous first-party MCP entry before enabling the plugin. Verify the
          new connection before retiring old setup files. Preserve other plugins
          and credentials; do not run both capture paths for the same session.
        </p>
      </Disclosure>
    </div>
  );
}
/** The prompt that asks a coding agent to connect itself to this Brain. */
export const pluginConnectPrompt = (brain: { id: string; name: string }) =>
  `Use the Recollect connect skill to connect to ${window.location.origin}, Brain ${brain.name} (${brain.id}). Use browser authorization and the OS credential store. Keep the optional execution runner disabled.`;

export function PluginConnect({
  brain,
  compact = false,
  hostLabel,
}: {
  brain: Brain;
  compact?: boolean;
  /** Names the coding host in the compact terminal bar. */
  hostLabel?: string;
}) {
  const connect = pluginConnectPrompt(brain);
  if (compact)
    return (
      <div className="grid min-w-0 gap-3">
        <div className="relative min-w-0">
          <TerminalCard
            title={`Connect prompt${hostLabel ? ` · ${hostLabel}` : ""}`}
            text={`\u001b[2mConnect to this Brain using your agent:\u001b[0m\nUse the Recollect connect skill for ${brain.name}`}
          />
          <CopyButton value={connect}>
            {({ copied, copy }) => (
              <button
                type="button"
                onClick={copy}
                aria-label="Copy connection prompt"
                title={copied ? "Copied" : "Copy connection prompt"}
                className="absolute right-1.5 top-1 grid size-7 place-items-center rounded-md text-terminal-muted transition-colors duration-fast hover:bg-terminal-border hover:text-terminal-foreground focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring"
              >
                {copied ? (
                  <Check aria-hidden="true" className="size-4 text-ansi-green" />
                ) : (
                  <Copy aria-hidden="true" className="size-4" />
                )}
              </button>
            )}
          </CopyButton>
        </div>
        <Disclosure summary="View connection prompt">
          <Code block>{connect}</Code>
          <p className={stepNote}>
            Restart your host, trust the plugin, then send this prompt. Approve
            the connection in your browser. Credentials stay in the operating
            system credential store.
          </p>
        </Disclosure>
      </div>
    );
  return (
    <div className="grid gap-3">
      <h3 className={stepTitle}>Connect once</h3>
      <p className={stepText}>
        Restart your host, trust the plugin, then send this prompt. Approve the
        connection in your browser.
      </p>
      <Code block>{connect}</Code>
      <SetupCopy value={connect} label="Copy connection prompt" />
      <p className={stepNote}>
        Your credential stays in the operating system credential store.
      </p>
    </div>
  );
}

export function PluginFinish({ brain }: { brain: Brain }) {
  const verify = `What relevant knowledge does Recollect already have for this workspace? Cite its sources and say if no relevant memory was available. Do not change any permissions.`;
  return (
    <div className="grid gap-3">
      <h3 className={stepTitle}>Start a normal session</h3>
      <p className={stepText}>
        The plugin recalls relevant memory and captures permitted evidence in
        the background. Check recorded activity in {brain.name}.
      </p>
      <SetupCopy value={verify} label="Copy verification prompt" />
      <p className={stepNote}>
        The next view shows Brain activity. Installation alone does not prove a
        read or upload from this agent. An empty Brain may have no relevant
        memory yet.
      </p>
      <Disclosure summary="Optional tool execution">
        <p className={stepText}>
          The execution runner lets Recollect execute separately authorized
          connected tools on this computer or its private network. Automatic
          memory and your host’s ordinary coding tools do not require it. It
          stays off by default.
        </p>
        <p className={stepText}>
          To enable it explicitly, ask the connect skill to reconnect with{" "}
          <Code>--with-runner</Code>. Tool access still requires its own grants.
        </p>
      </Disclosure>
    </div>
  );
}
