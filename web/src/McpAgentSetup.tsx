import { useEffect, useRef, useState } from "react";
import {
  Badge,
  Button,
  Code,
  Modal,
  PasswordField,
  Select,
  Stepper,
  TextField,
} from "./components/ui";
import { cn } from "./lib/utils";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Bot, Plug, Plus, ArrowLeft, ArrowRight, Check } from "lucide-react";
import { Disclosure } from "./features/connections/ui";
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
  const stepTitle = "m-0 text-title font-semibold text-foreground";
  const stepText = "m-0 text-body text-foreground-secondary";
  const stepNote = "m-0 text-caption text-muted-foreground";
  const choice = (selected: boolean) =>
    cn(
      "relative grid min-w-0 gap-1 rounded-lg border bg-card p-4 text-left transition-[border-color,background-color,box-shadow,transform] duration-fast ease-standard focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:cursor-not-allowed disabled:opacity-60 [&>svg]:size-5",
      selected
        ? "border-brand bg-brand-soft/40 shadow-xs [&>svg]:text-brand-text"
        : "border-border hover:-translate-y-px hover:border-border-strong hover:shadow-sm [&>svg]:text-muted-foreground",
    );
  return (
    <>
      <Button disabled={brain.archived} onClick={() => setOpened(true)}>
        <Plus aria-hidden="true" />
        {buttonLabel}
      </Button>
      <Modal
        opened={opened}
        onClose={close}
        title="Connect a coding agent"
        size="lg"
        closeOnEscape={!create.isPending}
        closeOnClickOutside={!create.isPending}
        bodyClassName="grid gap-5"
        footer={
          <div className="flex w-full items-center justify-between gap-3">
            <Button
              variant="secondary"
              disabled={step === 0 || create.isPending}
              onClick={() => setStep((v) => v - 1)}
            >
              <ArrowLeft aria-hidden="true" />
              Back
            </Button>
            <span className="mono text-caption text-muted-foreground">
              {step + 1} of 4
            </span>
            {step < 3 ? (
              <Button
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
                <ArrowRight aria-hidden="true" />
              </Button>
            ) : (
              <Button asChild>
                <Link
                  to="/brains/$brainId/activity"
                  params={{ brainId: brain.id }}
                  search={{ tab: "capture" }}
                  onClick={close}
                >
                  View recorded activity
                  <ArrowRight aria-hidden="true" />
                </Link>
              </Button>
            )}
          </div>
        }
      >
        <Stepper steps={labels.map((label) => ({ label }))} active={step} />
        {step > 0 && (
          <div className="flex gap-1.5">
            <Badge variant="outline">{hostLabel}</Badge>
            <Badge variant="outline">{plugin ? "Plugin" : "Direct MCP"}</Badge>
          </div>
        )}
        <div
          key={`${method}-${step}`}
          className="grid min-w-0 animate-enter gap-3"
          data-testid={`agent-setup-stage-${step + 1}`}
        >
          {step === 0 && (
            <div className="grid gap-3">
              <h3 className={stepTitle}>Choose your coding host</h3>
              <Select
                label="Coding host"
                className="max-w-[320px]"
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
                className="grid grid-cols-2 gap-3"
                role="group"
                aria-label="Connection method"
              >
                <button
                  type="button"
                  className={choice(plugin)}
                  aria-pressed={plugin}
                  disabled={!!token}
                  onClick={() => setMethod("plugin")}
                >
                  <Bot aria-hidden="true" />
                  <strong className="flex items-center gap-2 text-body font-semibold text-foreground">
                    Recollect plugin{" "}
                    <span className="rounded-full bg-secondary px-2 py-px text-caption font-medium text-foreground-secondary">
                      Recommended
                    </span>
                  </strong>
                  <small className="text-label text-muted-foreground">
                    Automatic recall and session capture.
                  </small>
                  {plugin && (
                    <Check
                      aria-hidden="true"
                      className="absolute right-3 top-3 size-4 text-brand-text"
                    />
                  )}
                </button>
                <button
                  type="button"
                  className={choice(!plugin)}
                  aria-pressed={!plugin}
                  disabled={!!token}
                  onClick={() => setMethod("direct")}
                >
                  <Plug aria-hidden="true" />
                  <strong className="text-body font-semibold text-foreground">
                    Direct MCP
                  </strong>
                  <small className="text-label text-muted-foreground">
                    Memory tools. Configure your host manually.
                  </small>
                  {!plugin && (
                    <Check
                      aria-hidden="true"
                      className="absolute right-3 top-3 size-4 text-brand-text"
                    />
                  )}
                </button>
              </div>
              {!!token && (
                <p className={stepNote}>
                  Host and method are fixed for this token. Close to begin a
                  different setup.
                </p>
              )}
            </div>
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
            <div className="grid gap-3">
              <h3 className={stepTitle}>Create your access token</h3>
              <p className={stepText}>
                Acts as you across Brains you can access. This URL selects{" "}
                {brain.name}; it does not restrict the token to this Brain.
                Expires after 30 days. Manage it in{" "}
                <Link
                  to="/agents"
                  search={{ access: true }}
                  className="text-brand-text underline-offset-[3px] hover:underline"
                >
                  Access tokens
                </Link>
                .
              </p>
              <TextField
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
                  <PasswordField
                    label="Access token · shown once"
                    inputClassName="mono text-label"
                    value={token}
                    readOnly
                    autoComplete="off"
                  />
                  <SetupCopy value={token} label="Copy access token" />
                  <p className={stepNote}>
                    Kept here while you go Back or Next. Closing clears this
                    copy.
                  </p>
                </>
              ) : (
                <Button
                  className="justify-self-start"
                  loading={create.isPending}
                  disabled={!credentialName.trim()}
                  onClick={() => create.mutate()}
                >
                  Create access token
                </Button>
              )}
              <ErrorState error={create.error} />
            </div>
          )}
          {!plugin && step === 2 && (
            <div className="grid gap-3">
              <h3 className={stepTitle}>Configure {hostLabel}</h3>
              <p className={stepText}>
                Merge this into <Code>{hostConfigFile}</Code>, preserving your
                existing servers.
              </p>
              <div data-testid="agent-setup-command">
                {revealed ? (
                  <CodeBlock code={config} copyLabel="Copy MCP configuration" />
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
                variant="ghost"
                className="-ml-3 justify-self-start"
                onClick={() => setRevealed((value) => !value)}
              >
                {revealed
                  ? "Hide token in configuration"
                  : "Reveal token in configuration"}
              </Button>
              <Disclosure summary="Show the one-time credential">
                <PasswordField
                  label="Credential"
                  inputClassName="mono text-label"
                  readOnly
                  value={token ?? ""}
                  autoComplete="off"
                />
                <SetupCopy value={token ?? ""} label="Copy credential" />
              </Disclosure>
              <p className={stepText}>
                Copy includes your token. Save in a private, untracked
                configuration file, then start your host normally.
              </p>
            </div>
          )}
          {!plugin && step === 3 && (
            <div className="grid gap-3">
              <h3 className={stepTitle}>Instructions complete</h3>
              <p className={stepText}>
                Ask your agent to read memory. Configuration alone does not
                verify the connection.
              </p>
              <SetupCopy
                value={agentMemoryReadCheck(brain)}
                label="Copy verification prompt"
              />
              <Disclosure summary="Verification prompt">
                <Code block data-testid="agent-verification-prompt">
                  {agentMemoryReadCheck(brain)}
                </Code>
              </Disclosure>
            </div>
          )}
        </div>
      </Modal>
    </>
  );
}
