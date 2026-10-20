import { useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useRouterState } from "@tanstack/react-router";
import { Bot, ChevronRight, Info, KeyRound, ShieldAlert } from "lucide-react";
import { client, result } from "../../api";
import { EmptyState, ErrorState } from "../../components/AsyncState";
import { Page } from "../../components/Page";
import { PageHeader } from "../../components/PageHeader";
import { BrainIcon } from "../../components/BrainIcon";
import { HostIcon } from "../../components/HostIcon";
import { staggerStyle } from "../../components/Motion";
import { Skeleton } from "../../components/Skeleton";
import {
  DetailInspector,
  InspectorSection,
  PropList,
} from "../../components/Inspector";
import {
  Alert,
  Button,
  Card,
  CardBar,
  CardDescription,
  CardTitle,
  Drawer,
  SearchInput,
  StatusBadge,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Tile,
} from "../../components/ui";
import { cn } from "../../lib/utils";
import { AgentAccessPanel } from "../../DevicesPanel";
import { TeamPanel } from "../../TeamPanel";
import { useWorkspace } from "../../app/context";
import { usePollInterval } from "../../usePollInterval";

const hostLabel = (kind?: string | null) =>
  ({ codex: "Codex", opencode: "OpenCode", claude_code: "Claude Code" })[
    kind ?? ""
  ] ?? "Host unreported";
const integrationLabel = (value: string) =>
  value === "plugin" ? "Recollect plugin" : "MCP token";
const time = (value?: string | null) =>
  value ? new Date(value).toLocaleString() : "No recorded use";

export function AgentsGlobalPage() {
  const pollNormal = usePollInterval("normal");
  const navigate = useNavigate();
  const searchStr = useRouterState({
    select: (state) => state.location.searchStr,
  });
  const params = new URLSearchParams(searchStr),
    code = params.get("code") ?? undefined;
  const access = params.get("access") === "true" || !!code;
  const [selected, setSelected] = useState<string | null>(null),
    [search, setSearch] = useState("");
  const roster = useQuery({
    queryKey: ["account-agents"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/agents", { signal })),
    gcTime: 0,
    refetchInterval: pollNormal,
    retry: false,
  });
  // Groups on the first successful read stagger in; later reads never do.
  const firstLoad = useRef<Set<string> | null>(null);
  const agents = roster.isError
    ? []
    : (roster.data?.groups ?? []).flatMap((group) =>
        group.agents.map((agent) => ({ ...agent, user_name: group.user_name })),
      );
  const inspected = agents.find((agent) => agent.device_id === selected);
  type Agent = (typeof agents)[number];
  type Usage = Agent["brains"][number];
  const byBrain = new Map<
    string,
    { brain: Usage; agents: { agent: Agent; usage: Usage }[] }
  >();
  const term = search.trim().toLocaleLowerCase();
  for (const agent of agents)
    for (const brain of agent.brains) {
      if (
        !`${brain.name} ${agent.name} ${hostLabel(agent.host_kind)} ${(agent.observed_hosts ?? []).map(hostLabel).join(" ")}`
          .toLocaleLowerCase()
          .includes(term)
      )
        continue;
      if (!byBrain.has(brain.brain_id))
        byBrain.set(brain.brain_id, { brain, agents: [] });
      byBrain.get(brain.brain_id)!.agents.push({ agent, usage: brain });
    }
  const groups = [...byBrain.values()].sort((a, b) =>
    a.brain.name.localeCompare(b.brain.name),
  );
  if (!firstLoad.current && roster.data)
    firstLoad.current = new Set(groups.map(({ brain }) => brain.brain_id));
  const openAccess = () =>
    void navigate({ to: "/agents", search: { access: true } });
  return (
    <Page className="text-body">
      <div className="flex flex-col gap-5" data-testid="account-agent-roster">
        <PageHeader
          title="Agents"
          description="Agents connected to your account, across every Brain."
          actions={
            <Button variant="secondary" onClick={openAccess}>
              <KeyRound aria-hidden="true" />
              Access tokens
            </Button>
          }
        />
        {/* The rail sits beside the roster when the panel is wide enough and
            drops below it when the docked inspector narrows the panel. */}
        <div className="@container">
          <div className="grid items-start gap-4 @min-[960px]:grid-cols-[minmax(0,1fr)_340px] @min-[1100px]:grid-cols-[minmax(0,1fr)_360px]">
            <div className="flex min-w-0 flex-col gap-3">
              <div className="flex items-center gap-3">
                <div className="w-80">
                  <SearchInput
                    aria-label="Search agents or Brains"
                    placeholder="Find an agent or Brain"
                    value={search}
                    onChange={(event) => setSearch(event.currentTarget.value)}
                  />
                </div>
                {!!roster.data && !roster.isError && (
                  <span className="mono ml-auto whitespace-nowrap text-label text-muted-foreground">
                    {groups.length} {groups.length === 1 ? "Brain" : "Brains"}
                  </span>
                )}
              </div>
              {roster.isPending ? (
                <Card
                  className="p-0"
                  role="status"
                  aria-label="Loading agents…"
                >
                  <CardBar>
                    <Skeleton width={24} height={24} className="rounded-md" />
                    <Skeleton width={160} height={12} />
                  </CardBar>
                  <div className="px-4">
                    {[46, 58, 38].map((width, index) => (
                      <div
                        key={index}
                        className="flex h-12 items-center gap-3 border-b border-border last:border-b-0"
                      >
                        <Skeleton
                          width={28}
                          height={28}
                          className="rounded-md"
                        />
                        <Skeleton width={`${width}%`} height={12} />
                        <Skeleton width="16%" height={10} className="ml-auto" />
                      </div>
                    ))}
                  </div>
                </Card>
              ) : roster.isError ? (
                <ErrorState
                  error={roster.error}
                  retry={() => void roster.refetch()}
                />
              ) : !groups.length ? (
                <Card className="p-0">
                  <EmptyState
                    icon={Bot}
                    title={
                      term
                        ? "No matching agents or Brains"
                        : "No recorded Brain activity yet"
                    }
                    description={
                      term
                        ? "Try a different name."
                        : "Connect an agent from a Brain’s Agents page. Your credentials are available in Access tokens."
                    }
                  />
                </Card>
              ) : (
                groups.map(({ brain, agents: rows }, index) => (
                  <section
                    key={brain.brain_id}
                    aria-label={brain.name}
                    data-testid="agent-brain-group"
                    className={cn(
                      "overflow-hidden rounded-lg border border-border bg-card",
                      firstLoad.current?.has(brain.brain_id) && "animate-enter",
                    )}
                    style={
                      firstLoad.current?.has(brain.brain_id)
                        ? staggerStyle(index)
                        : undefined
                    }
                  >
                    <CardBar>
                      <BrainIcon
                        id={brain.brain_id}
                        revision={brain.icon_revision}
                        size={24}
                      />
                      <h2>
                        <Link
                          to="/brains/$brainId/agents"
                          params={{ brainId: brain.brain_id }}
                          search={{}}
                          className="text-foreground no-underline hover:underline"
                        >
                          {brain.name}
                        </Link>
                      </h2>
                      <span className="mono ml-auto text-caption text-muted-foreground">
                        {rows.length} {rows.length === 1 ? "agent" : "agents"}
                      </span>
                    </CardBar>
                    <Table>
                      <TableHead>
                        <tr>
                          <TableHeader>Agent</TableHeader>
                          <TableHeader className="w-[220px]">
                            Connection
                          </TableHeader>
                          <TableHeader className="w-[200px]">
                            Last used
                          </TableHeader>
                          <TableHeader className="w-10" aria-label="Details" />
                        </tr>
                      </TableHead>
                      <TableBody>
                        {rows
                          .sort(
                            (a, b) =>
                              Date.parse(b.usage.last_used_at) -
                              Date.parse(a.usage.last_used_at),
                          )
                          .map(({ agent, usage }) => (
                            <TableRow
                              key={agent.device_id}
                              data-testid={`account-agent-row-${agent.device_id}`}
                              selected={selected === agent.device_id}
                              className="cursor-pointer"
                              onClick={() => setSelected(agent.device_id)}
                            >
                              <TableCell primary className="py-2">
                                <button
                                  type="button"
                                  className="flex min-w-0 items-center gap-3 rounded-sm text-left outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                                  onClick={(event) => {
                                    event.stopPropagation();
                                    setSelected(agent.device_id);
                                  }}
                                  aria-label={`Inspect ${agent.name}`}
                                >
                                  <Tile size="sm">
                                    <HostIcon
                                      host={agent.host_kind}
                                      size={16}
                                    />
                                  </Tile>
                                  <span className="truncate text-body font-medium text-foreground">
                                    {agent.name}
                                  </span>
                                </button>
                              </TableCell>
                              <TableCell>
                                <span className="block truncate text-label text-foreground-secondary">
                                  {agent.observed_hosts?.length
                                    ? agent.observed_hosts
                                        .map(hostLabel)
                                        .join(" · ")
                                    : hostLabel(agent.host_kind)}
                                </span>
                                <span className="block text-caption text-muted-foreground">
                                  {integrationLabel(agent.integration)}
                                </span>
                              </TableCell>
                              <TableCell>
                                <time
                                  dateTime={usage.last_used_at}
                                  className="mono text-caption text-muted-foreground"
                                >
                                  {time(usage.last_used_at)}
                                </time>
                              </TableCell>
                              <TableCell className="text-right">
                                <ChevronRight
                                  className="inline size-4 text-muted-foreground transition-transform duration-fast group-hover/row:translate-x-0.5"
                                  aria-hidden="true"
                                />
                              </TableCell>
                            </TableRow>
                          ))}
                      </TableBody>
                    </Table>
                  </section>
                ))
              )}
            </div>
            <aside
              aria-label="About agent access"
              className="grid grid-cols-2 items-start gap-4 @min-[960px]:grid-cols-1"
            >
              <Card className="flex flex-col gap-4">
                <div className="flex items-start gap-3">
                  <Tile>
                    <KeyRound aria-hidden="true" />
                  </Tile>
                  <div className="min-w-0">
                    <CardTitle as="h2">Your access tokens</CardTitle>
                    <CardDescription>
                      Every credential your agents use, pairing approvals and
                      revocation live in Access tokens.
                    </CardDescription>
                  </div>
                </div>
                <Button
                  variant="secondary"
                  className="w-full"
                  onClick={openAccess}
                >
                  Manage access tokens
                </Button>
              </Card>
              <Alert
                variant="info"
                role="note"
                icon={<Info aria-hidden="true" />}
                title="Each Brain lists its own agents"
              >
                Open a Brain and go to Agents to see what its agents
                contributed.
              </Alert>
            </aside>
          </div>
        </div>
      </div>
      <Drawer
        opened={access}
        onClose={() => void navigate({ to: "/agents", search: {} })}
        title="Access tokens"
        description="Credentials your agents use to reach your Brains."
        size={560}
      >
        {access && <AgentAccessPanel code={code} />}
      </Drawer>
      <DetailInspector
        opened={!!inspected && !access}
        onClose={() => setSelected(null)}
        title="Agent record"
      >
        {inspected && (
          <>
            <InspectorSection>
              <div className="flex items-center gap-3">
                <Tile>
                  <HostIcon host={inspected.host_kind} size={18} />
                </Tile>
                <div className="min-w-0">
                  <p className="m-0 truncate text-title font-semibold text-foreground">
                    {inspected.name}
                  </p>
                  <p className="m-0 text-label text-muted-foreground">
                    {inspected.observed_hosts?.length
                      ? inspected.observed_hosts.map(hostLabel).join(" · ")
                      : hostLabel(inspected.host_kind)}{" "}
                    · {integrationLabel(inspected.integration)}
                  </p>
                </div>
              </div>
            </InspectorSection>
            <InspectorSection label="Details">
              <PropList
                items={[
                  { term: "Account", value: inspected.user_name },
                  {
                    term: "Status",
                    value: (
                      <StatusBadge
                        state={inspected.claimed ? "positive" : "neutral"}
                      >
                        {inspected.claimed
                          ? "Credential enabled"
                          : "Waiting for host"}
                      </StatusBadge>
                    ),
                  },
                ]}
              />
            </InspectorSection>
            <InspectorSection label="Brains">
              <ul className="m-0 flex list-none flex-col gap-3 p-0">
                {inspected.brains.map((brain) => (
                  <li
                    key={brain.brain_id}
                    className="flex flex-col gap-2 rounded-md border border-border p-3"
                  >
                    <div className="flex items-center gap-2.5">
                      <BrainIcon
                        id={brain.brain_id}
                        revision={brain.icon_revision}
                        size={24}
                      />
                      <strong className="min-w-0 truncate text-body font-semibold text-foreground">
                        {brain.name}
                      </strong>
                    </div>
                    <p className="m-0 text-caption text-muted-foreground">
                      Last observed use{" "}
                      <span className="mono">{time(brain.last_used_at)}</span>
                    </p>
                    <Button
                      asChild
                      variant="secondary"
                      size="sm"
                      className="self-start"
                    >
                      <Link
                        to="/brains/$brainId/activity"
                        params={{ brainId: brain.brain_id }}
                        search={{ tab: "pipeline" }}
                      >
                        Open Brain activity
                      </Link>
                    </Button>
                  </li>
                ))}
              </ul>
            </InspectorSection>
          </>
        )}
      </DetailInspector>
    </Page>
  );
}

export function TeamPage() {
  const session = useWorkspace();
  return session.user.installation_owner ? (
    <TeamPanel />
  ) : (
    <Page className="text-body">
      <Alert
        variant="default"
        role="status"
        icon={<ShieldAlert aria-hidden="true" />}
        title="Installation owner access required"
      >
        Your Brain access is managed in each Brain’s Settings.
      </Alert>
    </Page>
  );
}
