import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
  ArrowRight,
  Bot,
  Check,
  Copy,
  Eye,
  KeyRound,
  MoreHorizontal,
} from "lucide-react";
import { client, result, type Brain } from "../../api";
import { ErrorState } from "../../components/AsyncState";
import { HostIcon } from "../../components/HostIcon";
import { staggerStyle } from "../../components/Motion";
import { SkeletonRows } from "../../components/Skeleton";
import {
  Badge,
  Button,
  CardBar,
  ConfirmDialog,
  CopyButton,
  Count,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  IconButton,
  StatusBadge,
  Tile,
} from "../../components/ui";
import { cn } from "../../lib/utils";
import { pluginConnectPrompt } from "../../PluginAgentSetup";
import { usePollInterval } from "../../usePollInterval";

const hostLabel = (kind?: string | null) =>
  kind === "codex"
    ? "Codex"
    : kind === "opencode"
      ? "OpenCode"
      : kind === "claude_code"
        ? "Claude Code"
        : null;

const integrationLabel = (value: string) =>
  value === "plugin" ? "Recollect plugin" : "MCP token";

const time = (value?: string | null) =>
  value
    ? new Date(value).toLocaleDateString("en-GB", {
        day: "numeric",
        month: "short",
      })
    : "No observed use";

/** Observed contributors to this Brain, grouped
 * by user, each row one connection of exactly two kinds. A row proves the
 * configuration exists; only real calls prove it works. */
export function AgentsRoster({
  brain,
  onSelect,
  selectedDevice,
}: {
  brain: Brain;
  onSelect: (id: string) => void;
  selectedDevice?: string;
}) {
  const pollNormal = usePollInterval("normal");
  const [showHidden, setShowHidden] = useState(false);
  const [pendingRevoke, setPendingRevoke] = useState<string | null>(null);
  const cache = useQueryClient();
  const roster = useQuery({
    queryKey: ["brain-agents", brain.id, showHidden],
    gcTime: 0,
    refetchInterval: pollNormal,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/agents", {
          signal,
          params: {
            path: { brain: brain.id },
            query: showHidden ? { include_hidden: true } : {},
          },
        }),
      ),
  });
  const revoke = useMutation({
    mutationFn: async (id: string) =>
      result(
        await client.DELETE("/api/devices/{id}", { params: { path: { id } } }),
      ),
    onSuccess: async () => {
      await cache.invalidateQueries({ queryKey: ["brain-agents", brain.id] });
      await cache.invalidateQueries({ queryKey: ["account-agents"] });
      await cache.invalidateQueries({ queryKey: ["devices"] });
    },
  });
  // This Brain's answer to "who is connected here" is the set of agents that
  // have actually been used on it. The complete account list lives on the
  // global Agents page.
  const usedGroups = (roster.data?.groups ?? [])
    .map((group) => ({
      ...group,
      agents: group.agents.filter((a) => a.last_used_on_brain_at),
    }))
    .filter((group) => group.agents.length > 0);
  const visible = usedGroups.flatMap((g) => g.agents);
  const target = pendingRevoke
    ? visible.find((a) => a.device_id === pendingRevoke)
    : null;
  return (
    <section
      className="@container flex min-w-0 flex-col overflow-hidden rounded-lg border border-border bg-card"
      aria-labelledby="brain-agents-title"
      data-testid="brain-agent-roster"
    >
      <CardBar>
        <h2 id="brain-agents-title">Agents on this Brain</h2>
        {roster.data && <Count>{visible.length}</Count>}
        <Link
          to="/agents"
          className="ml-auto inline-flex items-center gap-1 rounded-sm text-label font-medium text-brand-text underline-offset-[3px] hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
        >
          My agents
        </Link>
      </CardBar>
      {roster.isPending ? (
        <div className="px-4 py-2">
          <SkeletonRows rows={3} label="Loading agents…" />
        </div>
      ) : roster.isError ? (
        <div className="p-4">
          <ErrorState error={roster.error} />
        </div>
      ) : (
        <>
          {visible.length === 0 && (
            <div className="grid animate-enter justify-items-center gap-2 px-6 py-8 text-center">
              <Tile size="lg" className="mb-2">
                <Bot aria-hidden="true" />
              </Tile>
              <h3 className="m-0 text-title font-semibold text-foreground">
                No agents have used this Brain yet
              </h3>
              <p className="m-0 max-w-[360px] text-body text-muted-foreground">
                Connect an agent to start contributing. Your full list is in My
                agents.
              </p>
              <CopyButton value={pluginConnectPrompt(brain)}>
                {({ copied, copy }) => (
                  <Button
                    size="sm"
                    variant="secondary"
                    className="mt-2"
                    onClick={copy}
                  >
                    {copied ? (
                      <Check aria-hidden="true" />
                    ) : (
                      <Copy aria-hidden="true" />
                    )}
                    {copied ? "Copied" : "Copy connect prompt"}
                  </Button>
                )}
              </CopyButton>
            </div>
          )}
          {usedGroups.map((group, index) => (
            <div
              key={group.user_name}
              className="animate-enter border-b border-border last:border-b-0"
              style={staggerStyle(index)}
            >
              <div className="flex items-center gap-2.5 px-4 pb-1 pt-3">
                <span
                  aria-hidden="true"
                  className="grid size-6 place-items-center rounded-full bg-secondary text-caption font-semibold text-foreground-secondary"
                >
                  {group.user_name.slice(0, 1).toUpperCase()}
                </span>
                <strong className="text-label font-semibold text-foreground">
                  {group.user_name}
                </strong>
                <small className="text-caption text-muted-foreground">
                  <span className="mono">{group.agents.length}</span>{" "}
                  {group.agents.length === 1 ? "agent" : "agents"}
                </small>
              </div>
              <ul className="m-0 grid list-none p-0 pb-1.5">
                {group.agents.map((agent) => {
                  const selected = selectedDevice === agent.device_id;
                  return (
                    <li
                      key={agent.device_id}
                      className={cn(
                        "group/row relative mx-1.5 flex min-h-14 items-center gap-3 rounded-md px-2.5 py-2 transition-colors duration-fast ease-standard",
                        selected
                          ? "selected bg-brand-soft/45"
                          : "hover:bg-accent",
                      )}
                      data-testid={`brain-agent-row-${agent.device_id}`}
                    >
                      <Tile className="text-foreground">
                        <HostIcon host={agent.host_kind} size={18} />
                      </Tile>
                      <div className="min-w-0 flex-1">
                        <button
                          type="button"
                          className="block max-w-full truncate rounded-sm text-left text-body font-medium text-foreground outline-none after:absolute after:inset-0 after:rounded-md after:content-[''] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                          onClick={() => onSelect(agent.device_id)}
                          aria-pressed={selected}
                        >
                          {agent.name}
                        </button>
                        <p className="m-0 truncate text-caption text-muted-foreground">
                          {(agent.observed_hosts?.length
                            ? agent.observed_hosts
                                .map(hostLabel)
                                .filter(Boolean)
                                .join(" · ")
                            : hostLabel(agent.host_kind)) ?? "Coding host"}{" "}
                          · {integrationLabel(agent.integration)}
                          {!agent.active && " · revoked or expired"}
                        </p>
                      </div>
                      {agent.active ? (
                        <Badge variant="brand" className="hidden @[520px]:inline-flex">
                          <KeyRound aria-hidden="true" />
                          Credential enabled
                        </Badge>
                      ) : (
                        <StatusBadge
                          state="neutral"
                          className="hidden @[520px]:inline-flex"
                        >
                          Revoked or expired
                        </StatusBadge>
                      )}
                      <time
                        className="mono hidden shrink-0 whitespace-nowrap text-right text-caption text-muted-foreground @[620px]:block"
                        title={
                          agent.last_used_on_brain_at
                            ? new Date(
                                agent.last_used_on_brain_at,
                              ).toLocaleString()
                            : undefined
                        }
                        dateTime={agent.last_used_on_brain_at ?? undefined}
                      >
                        {"Observed "}
                        {time(agent.last_used_on_brain_at)}
                      </time>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <IconButton
                            size="icon-sm"
                            className="relative z-[1]"
                            label={`Actions for ${agent.name}`}
                          >
                            <MoreHorizontal aria-hidden="true" />
                          </IconButton>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem
                            onSelect={() => onSelect(agent.device_id)}
                          >
                            <Eye aria-hidden="true" />
                            View activity
                          </DropdownMenuItem>
                          {agent.can_revoke && (
                            <>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem
                                destructive
                                onSelect={() =>
                                  setPendingRevoke(agent.device_id)
                                }
                              >
                                <KeyRound aria-hidden="true" />
                                Revoke credential across all Brains
                              </DropdownMenuItem>
                            </>
                          )}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
          {roster.data.hidden_count > 0 && (
            <div className="border-t border-border px-2.5 py-1.5">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowHidden((v) => !v)}
              >
                {showHidden
                  ? "Hide revoked and expired"
                  : "Show revoked and expired"}
                <ArrowRight
                  aria-hidden="true"
                  className={cn(
                    "transition-transform duration-fast",
                    showHidden ? "-rotate-90" : "rotate-90",
                  )}
                />
              </Button>
            </div>
          )}
        </>
      )}
      <ConfirmDialog
        opened={!!target?.can_revoke}
        onClose={() => setPendingRevoke(null)}
        title="Revoke this credential across all Brains?"
        description={
          <>
            This revokes {target?.name ?? "the agent"}&apos;s keycard from{" "}
            <b>all Brains</b>, not just this one. Its token stops working at
            once; captured history stays in place.
          </>
        }
        confirmLabel="Revoke from all Brains"
        loading={revoke.isPending}
        onConfirm={() => {
          if (!pendingRevoke || !target?.can_revoke) return;
          revoke.mutate(pendingRevoke, {
            onSuccess: () => setPendingRevoke(null),
          });
        }}
      >
        <ErrorState error={revoke.error} />
      </ConfirmDialog>
    </section>
  );
}
