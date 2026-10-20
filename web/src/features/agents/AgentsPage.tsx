import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import {
  ArrowRight,
  BookOpen,
  Check,
  Copy,
  KeyRound,
  Plus,
} from "lucide-react";
import { PluginInstall, PluginConnect } from "../../PluginAgentSetup";
import { client, result } from "../../api";
import { useBrain } from "../../app/context";
import { McpAgentSetup, agentMemoryReadCheck } from "../../McpAgentSetup";
import { CapturePanel } from "../../CapturePanel";
import { Page } from "../../components/Page";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs } from "../../components/FeatureTabs";
import { DetailInspector } from "../../components/DetailInspector";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
import { HostIcon } from "../../components/HostIcon";
import {
  Badge,
  Button,
  Code,
  CopyButton,
  SegmentedControl,
  StatusBadge,
  Tile,
} from "../../components/ui";
import { Disclosure } from "../connections/ui";
import { AgentsRoster } from "./AgentsRoster";
import { useBrainSearch } from "../../app/useBrainSearch";
import { SkillsPanel } from "../skills/SkillsPanel";
import { usePollInterval } from "../../usePollInterval";

const hosts = [
  { value: "codex", label: "Codex" },
  { value: "claude", label: "Claude Code" },
  { value: "opencode", label: "OpenCode" },
] as const;
const tabs = [
  { value: "agents", label: "Agent setup" },
  { value: "skills", label: "Skills" },
] as const;

const cardClass =
  "flex min-w-0 animate-enter flex-col gap-4 rounded-lg border border-border bg-card p-4";
const cardTitle = "m-0 text-title font-semibold text-foreground";
const cardText = "m-0 mt-0.5 text-body text-muted-foreground";

export function AgentsPage() {
  const pollNormal = usePollInterval("normal");
  const brain = useBrain();
  const [host, setHost] = useState("codex");
  const [directory, setDirectory] = useState("/path/to/recollect-plugin");
  const [inspecting, setInspecting] = useState(false);
  const [composeSkill, setComposeSkill] = useState(false);
  const [search, updateSearch] = useBrainSearch();
  const navigate = useNavigate();
  const device = search.device;
  const admin = brain.role === "admin" && !brain.archived;
  const selected = useQuery({
    queryKey: ["brain-agents", brain.id, true],
    enabled: !!device,
    gcTime: 0,
    retry: false,
    refetchInterval: pollNormal,
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
  const hostName = hosts.find((entry) => entry.value === host)?.label ?? "Codex";
  const tab = search.tab === "skills" ? "skills" : "agents";
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
    <div className="grid min-w-0 gap-4">
      <div className="flex min-w-0 items-center gap-3">
        <Tile size="lg" className="text-foreground">
          <HostIcon host={agent.host_kind} size={22} />
        </Tile>
        <div className="min-w-0">
          <h3 className="m-0 truncate text-title font-semibold text-foreground">
            {agent.name}
          </h3>
          <p className="m-0 truncate text-label text-muted-foreground">
            {agent.user_name}
          </p>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2 border-b border-border pb-4">
        {agent.active ? (
          <Badge variant="brand">
            <KeyRound aria-hidden="true" />
            Credential enabled
          </Badge>
        ) : (
          <StatusBadge state="neutral">Revoked or expired</StatusBadge>
        )}
        <small className="text-caption text-muted-foreground">
          {agent.last_used_on_brain_at ? (
            <>
              Observed{" "}
              <span className="mono">
                {new Date(agent.last_used_on_brain_at).toLocaleString()}
              </span>
            </>
          ) : (
            "No observed use"
          )}
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
    </div>
  );
  return (
    <Page>
      <PageHeader
        title="Agents"
        description="Connect your coding agents and share their working guidance."
        actions={
          <>
            <Button variant="ghost" asChild>
              <Link to="/agents">
                My agents
                <ArrowRight aria-hidden="true" />
              </Link>
            </Button>
            <Button variant="secondary" asChild>
              <Link to="/agents" search={{ access: true }}>
                <KeyRound aria-hidden="true" />
                Access tokens
              </Link>
            </Button>
            <McpAgentSetup brain={brain} buttonLabel="Connect agent" />
          </>
        }
      />
      <FeatureTabs
        tabs={tabs}
        value={tab}
        onChange={(next) =>
          updateSearch({
            tab: next === "skills" ? "skills" : undefined,
            device: undefined,
          })
        }
      >
        {tab === "skills" ? (
          <SkillsPanel
            key={brain.id}
            startPublishing={composeSkill}
            onStartPublishing={() => setComposeSkill(false)}
          />
        ) : (
          <div className="@container grid min-w-0 gap-4">
            <div className="grid min-w-0 gap-4 @[880px]:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
              <section
                className={cardClass}
                aria-labelledby="agent-connect-title"
              >
                <div>
                  <h2 className={cardTitle} id="agent-connect-title">
                    Connect your coding tool
                  </h2>
                  <p className={cardText}>
                    Install the plugin, choose a Brain, and start working.
                  </p>
                </div>
                <SegmentedControl
                  label="Coding tool"
                  className="self-start"
                  value={host}
                  onChange={setHost}
                  data={hosts.map((entry) => ({
                    value: entry.value,
                    label: (
                      <>
                        <HostIcon host={entry.value} size={14} />
                        {entry.label}
                      </>
                    ),
                  }))}
                />
                <PluginConnect brain={brain} compact hostLabel={hostName} />
                <Disclosure summary={`View setup for ${hostName}`}>
                  <PluginInstall
                    host={host}
                    directory={directory}
                    setDirectory={setDirectory}
                  />
                </Disclosure>
              </section>
              <section
                className={cardClass}
                aria-labelledby="agent-check-title"
                style={{ animationDelay: "30ms" }}
              >
                <div>
                  <h2 className={cardTitle} id="agent-check-title">
                    Check memory access
                  </h2>
                  <p className={cardText}>
                    Verify a real memory read from your agent.
                  </p>
                </div>
                <Disclosure summary="View memory-read prompt">
                  <Code block>{agentMemoryReadCheck(brain)}</Code>
                </Disclosure>
                <CopyButton value={agentMemoryReadCheck(brain)}>
                  {({ copied, copy }) => (
                    <Button
                      variant="secondary"
                      className="w-full"
                      onClick={copy}
                    >
                      {copied ? (
                        <Check aria-hidden="true" />
                      ) : (
                        <Copy aria-hidden="true" />
                      )}
                      {copied ? "Copied" : "Copy memory-read check"}
                    </Button>
                  )}
                </CopyButton>
              </section>
            </div>
            <div className="grid min-w-0 gap-4 @[880px]:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
              <AgentsRoster
                brain={brain}
                selectedDevice={device}
                onSelect={(id) => updateSearch({ device: id, tab: undefined })}
              />
              <section
                className={`${cardClass} gap-3.5`}
                aria-labelledby="agent-skills-title"
                style={{ animationDelay: "60ms" }}
              >
                <div>
                  <span className="flex items-center gap-1.5 text-caption text-muted-foreground">
                    <BookOpen aria-hidden="true" className="size-3.5" />
                    Skills · shared guidance
                  </span>
                  <h2
                    className={`${cardTitle} mt-1.5`}
                    id="agent-skills-title"
                  >
                    One library for your agents
                  </h2>
                  <p className={cardText}>
                    Publish skills and working instructions for this Brain.
                    Agents fetch what they need, when they need it.
                  </p>
                </div>
                <ol className="m-0 grid list-none gap-2 p-0 text-body text-foreground">
                  {[
                    "Manager publishes",
                    "Agent discovers",
                    "Instructions fetched on demand",
                  ].map((step, index) => (
                    <li key={step} className="flex items-baseline gap-2.5">
                      <span className="mono text-caption text-muted-foreground">
                        0{index + 1}
                      </span>
                      {step}
                    </li>
                  ))}
                </ol>
                <div className="mt-auto flex flex-wrap items-center gap-2 pt-1">
                  {admin && (
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => {
                        setComposeSkill(true);
                        updateSearch({ tab: "skills", device: undefined });
                      }}
                    >
                      <Plus aria-hidden="true" />
                      Publish skill
                    </Button>
                  )}
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() =>
                      updateSearch({ tab: "skills", device: undefined })
                    }
                  >
                    Open Skills
                    <ArrowRight aria-hidden="true" />
                  </Button>
                </div>
              </section>
            </div>
          </div>
        )}
      </FeatureTabs>
      <DetailInspector
        opened={!!device && tab === "agents"}
        onClose={close}
        title="Agent activity"
        closeLabel="Close agent activity"
        closeOnEscape={!inspecting}
      >
        {!!device && activity}
      </DetailInspector>
    </Page>
  );
}
