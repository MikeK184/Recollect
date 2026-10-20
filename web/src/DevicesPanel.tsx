import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
  Check,
  Copy,
  Fingerprint,
  History,
  KeyRound,
  ShieldCheck,
  X,
} from "lucide-react";
import type { components } from "./api-schema";
import { client, result } from "./api";
import { HostIcon } from "./components/HostIcon";
import { EmptyState } from "./components/AsyncState";
import { SkeletonRows } from "./components/Skeleton";
import {
  Alert,
  Button,
  Checkbox,
  CopyButton,
  Count,
  Modal,
  SearchInput,
  StatusBadge,
  Tile,
  Tooltip,
} from "./components/ui";
import { usePollInterval } from "./usePollInterval";

const date = (value: string) =>
  new Date(value).toLocaleString("en-GB", {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
const hostLabels: Record<string, string> = {
  codex: "Codex",
  claude_code: "Claude Code",
  opencode: "OpenCode",
};

type Device = components["schemas"]["Device"];
type Mark = "positive" | "attention" | "negative" | "neutral";

/**
 * The device record carries no lifecycle enum, so these are the only states the
 * authorized read supports: `revoked_at`, the advertised `expires_at`, and
 * whether the approved pairing was ever `claimed` by the host. Historical
 * records are the ones that can no longer act, revoked or past advertised
 * expiry. Filtering is presentation only and never alters a credential.
 */
function statusOf(device: Device): {
  label: string;
  mark: Mark;
  historical: boolean;
} {
  if (device.revoked_at)
    return { label: "Revoked", mark: "negative", historical: true };
  if (new Date(device.expires_at).getTime() <= Date.now())
    return { label: "Expired", mark: "attention", historical: true };
  if (device.claimed)
    return { label: "Credential enabled", mark: "positive", historical: false };
  return { label: "Waiting for host", mark: "neutral", historical: false };
}

/** Active records first, then the pending pairing, then history. */
const order: Record<string, number> = {
  "Credential enabled": 0,
  "Waiting for host": 1,
  Expired: 2,
  Revoked: 3,
};

/** Subsection title with a small count, inside the Access tokens sheet. */
function ListHeading({
  icon,
  title,
  count,
}: {
  icon?: ReactNode;
  title: string;
  count: number;
}) {
  return (
    <h3 className="m-0 flex items-center gap-2 text-body font-semibold text-foreground [&_svg]:size-4 [&_svg]:text-muted-foreground">
      {icon}
      {title}
      <Count>{count}</Count>
    </h3>
  );
}

export function AgentAccessPanel({ code }: { code?: string }) {
  const pollNormal = usePollInterval("normal");
  const cache = useQueryClient();
  const [selected, setSelected] = useState<{ id: string; name: string } | null>(
    null,
  );
  const [showHistory, setShowHistory] = useState(false);
  const [search, setSearch] = useState("");
  const devices = useQuery({
    queryKey: ["devices"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/devices", { signal })),
    gcTime: 0,
    refetchInterval: pollNormal,
  });
  const revoke = useMutation({
    mutationFn: async (id: string) =>
      result(
        await client.DELETE("/api/devices/{id}", { params: { path: { id } } }),
      ),
    onSuccess: () => {
      setSelected(null);
      // Keep the effect of a revoke visible: the record moves to history, so
      // open the historical filter rather than hiding it again.
      setShowHistory(true);
      void cache.invalidateQueries({ queryKey: ["devices"] });
      void cache.invalidateQueries({ queryKey: ["account-agents"] });
      void cache.invalidateQueries({ queryKey: ["brain-agents"] });
    },
  });
  useEffect(() => {
    if (devices.isError) setSelected(null);
  }, [devices.isError]);
  const seen = useRef(new Map<string, boolean>());
  useEffect(() => {
    const list = devices.data;
    if (!list) return;
    const next = new Map<string, boolean>();
    let becameHistory = false;
    for (const device of list) {
      const historical = statusOf(device).historical;
      next.set(device.id, historical);
      // A record the user is already watching must not disappear mid-glance
      // when it becomes history somewhere else (native unpair, advertised
      // expiry). Reveal the filter — whose state stays visible and reversible —
      // rather than hiding it. A first load never auto-reveals, so history
      // still arrives collapsed.
      if (historical && seen.current.get(device.id) === false)
        becameHistory = true;
    }
    seen.current = next;
    if (becameHistory) setShowHistory(true);
  }, [devices.data]);
  const matches = (device: Device) =>
    `${device.name} ${device.host_kind ?? ""} ${(device.observed_hosts ?? []).map((host) => hostLabels[host] ?? host).join(" ")} ${device.integration ?? ""}`
      .toLowerCase()
      .includes(search.toLowerCase());
  const rows = useMemo(() => {
    const list = [...(devices.data ?? [])].sort(
      (a, b) => order[statusOf(a).label] - order[statusOf(b).label],
    );
    return {
      current: list.filter((device) => !statusOf(device).historical),
      history: list.filter((device) => statusOf(device).historical),
    };
  }, [devices.data]);
  const visibleCurrent = rows.current.filter(matches);
  const visibleHistory = rows.history.filter(matches);
  const onRevoke = (target: Device) => {
    revoke.reset();
    setSelected(target);
  };
  return (
    <div className="flex flex-col gap-6 text-body">
      {code && <PairingApproval key={code} code={code} />}
      {devices.error && (
        <Alert
          variant="destructive"
          title="Connections could not be loaded"
          actions={
            <Button
              variant="destructive-ghost"
              size="sm"
              onClick={() => void devices.refetch()}
            >
              Try again
            </Button>
          }
        >
          {devices.error.message}
        </Alert>
      )}
      {devices.isPending ? (
        <SkeletonRows rows={4} label="Loading access tokens…" />
      ) : (
        !devices.error && (
          <div className="flex flex-col gap-5">
            <div className="flex items-center gap-3">
              <ListHeading
                title="Your access tokens"
                count={devices.data?.length ?? 0}
              />
              {!!devices.data?.length && (
                <span className="mono ml-auto whitespace-nowrap text-caption text-muted-foreground">
                  {rows.current.length} enabled · {rows.history.length} in
                  history
                </span>
              )}
            </div>
            {!devices.data?.length ? (
              <div className="rounded-lg border border-border">
                <EmptyState
                  icon={KeyRound}
                  title="No access tokens yet"
                  description="Connect an agent from a Brain's Agents page."
                />
              </div>
            ) : (
              <>
                <div className="flex flex-col gap-3">
                  <SearchInput
                    placeholder="Find an access token"
                    aria-label="Find an access token"
                    value={search}
                    onChange={(event) => setSearch(event.currentTarget.value)}
                  />
                  <Checkbox
                    checked={showHistory}
                    onChange={(event) =>
                      setShowHistory(event.currentTarget.checked)
                    }
                    label="Show revoked and expired devices"
                  />
                </div>
                <section className="flex flex-col gap-2">
                  <ListHeading
                    title="Enabled credentials"
                    count={rows.current.length}
                  />
                  {!!visibleCurrent.length && (
                    <ul className="m-0 list-none rounded-lg border border-border bg-card px-3 py-0">
                      {visibleCurrent.map((device) => (
                        <DeviceCard
                          device={device}
                          key={device.id}
                          onRevoke={onRevoke}
                        />
                      ))}
                    </ul>
                  )}
                  {!rows.current.length && (
                    <div className="rounded-lg border border-border">
                      <EmptyState
                        icon={KeyRound}
                        title="No enabled device credentials"
                        description="Every device is revoked or expired and belongs to the history section below."
                      />
                    </div>
                  )}
                  {!!rows.current.length && !visibleCurrent.length && (
                    <p className="m-0 text-body text-muted-foreground">
                      No enabled access tokens match your search.
                    </p>
                  )}
                </section>
                {!!rows.history.length && (
                  <section className="flex flex-col gap-2">
                    <ListHeading
                      icon={<History aria-hidden="true" />}
                      title="Revoked and expired"
                      count={rows.history.length}
                    />
                    {showHistory ? (
                      !!visibleHistory.length && (
                        <ul className="m-0 list-none rounded-lg border border-border bg-card px-3 py-0">
                          {visibleHistory.map((device) => (
                            <DeviceCard
                              device={device}
                              key={device.id}
                              onRevoke={onRevoke}
                            />
                          ))}
                        </ul>
                      )
                    ) : (
                      <p className="m-0 text-caption text-muted-foreground">
                        {rows.history.length}{" "}
                        {rows.history.length === 1
                          ? "record is"
                          : "records are"}{" "}
                        collapsed. Turn on “Show revoked and expired devices”
                        above to list them.
                      </p>
                    )}
                  </section>
                )}
              </>
            )}
          </div>
        )
      )}
      <Modal
        opened={!!selected}
        onClose={() => !revoke.isPending && setSelected(null)}
        title="Revoke device access"
        size="sm"
        footer={
          <>
            <Button
              variant="secondary"
              disabled={revoke.isPending}
              onClick={() => setSelected(null)}
            >
              Keep device
            </Button>
            <Button
              variant="destructive"
              loading={revoke.isPending}
              onClick={() => selected && revoke.mutate(selected.id)}
            >
              Revoke device
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-3">
          <p className="m-0 text-body text-foreground-secondary">
            This revokes{" "}
            <strong className="text-foreground">{selected?.name}</strong>
            &apos;s keycard from{" "}
            <strong className="text-foreground">all Brains</strong>. Its token
            stops working at once; captured history stays in place.
          </p>
          {revoke.error && (
            <Alert variant="destructive">{revoke.error.message}</Alert>
          )}
        </div>
      </Modal>
    </div>
  );
}

/** One device row: name, a text status mark, human dates, and the machine
 * identifier only as a copyable field behind the row's identifier control. */
function DeviceCard({
  device,
  onRevoke,
}: {
  device: Device;
  onRevoke: (device: Device) => void;
}) {
  const status = statusOf(device);
  return (
    <li
      className="flex items-start gap-3 border-b border-border py-3 last:border-b-0"
      data-testid="device-card"
    >
      <Tile>
        <HostIcon host={device.host_kind} size={18} />
      </Tile>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <span
            className="min-w-0 text-body font-medium text-foreground [overflow-wrap:anywhere]"
            data-testid="device-row-name"
          >
            {device.name}
          </span>
          <StatusBadge state={status.mark}>{status.label}</StatusBadge>
        </div>
        <span className="text-caption text-muted-foreground">
          {(device.observed_hosts?.length
            ? device.observed_hosts
            : [device.host_kind ?? ""]
          )
            .map((host) => hostLabels[host])
            .filter(Boolean)
            .join(" · ") || "Host unreported"}
          {" · "}
          {device.integration === "plugin"
            ? "Recollect plugin"
            : device.integration === "mcp"
              ? "MCP token"
              : "Integration unreported"}
        </span>
        <span className="flex flex-wrap gap-x-3 text-caption text-muted-foreground">
          <span>
            {device.last_used_at ? (
              <>
                Last used{" "}
                <span className="mono text-foreground-secondary">
                  {date(device.last_used_at)}
                </span>
              </>
            ) : (
              "Not used yet"
            )}
          </span>
          <Tooltip
            label={`Paired ${new Date(device.created_at).toLocaleString()}`}
          >
            <span>
              {device.revoked_at ? "Revoked " : "Expires "}
              <span className="mono">
                {date(device.revoked_at ?? device.expires_at)}
              </span>
            </span>
          </Tooltip>
        </span>
      </div>
      <div className="flex shrink-0 items-center gap-1 self-center">
        <details className="relative">
          <summary
            title="Device identifier"
            className="grid size-7 cursor-pointer list-none place-items-center rounded-sm text-muted-foreground transition-colors duration-fast hover:bg-accent hover:text-foreground [&::-webkit-details-marker]:hidden"
          >
            <Fingerprint className="size-3.5" aria-hidden="true" />
            <span className="absolute size-px whitespace-nowrap [clip-path:inset(50%)]">
              Device identifier
            </span>
          </summary>
          <div className="absolute right-0 top-full z-10 mt-1 animate-pop rounded-md border border-border bg-popover p-1 shadow-md">
            <CopyButton value={device.id}>
              {({ copied, copy }) => (
                <Button size="sm" variant="ghost" onClick={copy}>
                  {copied ? (
                    <Check className="text-brand-text" aria-hidden="true" />
                  ) : (
                    <Copy aria-hidden="true" />
                  )}
                  {copied ? "Copied" : "Copy device ID"}
                </Button>
              )}
            </CopyButton>
          </div>
        </details>
        {!device.revoked_at && (
          <Button
            size="sm"
            variant="destructive-ghost"
            onClick={() => onRevoke(device)}
          >
            Revoke
          </Button>
        )}
      </div>
    </li>
  );
}

function PairingApproval({ code }: { code: string }) {
  const pollFast = usePollInterval("fast");
  const cache = useQueryClient();
  const navigate = useNavigate();
  const pairing = useQuery({
    queryKey: ["pairing", code],
    queryFn: async () =>
      result(
        await client.GET("/api/devices/pairings/{code}", {
          params: { path: { code } },
        }),
      ),
    refetchInterval: pollFast,
  });
  const decision = useMutation({
    mutationFn: async (approve: boolean) =>
      result(
        await client.POST("/api/devices/pairings/{code}/approve", {
          params: { path: { code } },
          body: {
            approve,
            confirm: approve && !!pairing.data?.reuses_device,
          },
        }),
      ),
    onSuccess: (value) => {
      cache.setQueryData(["pairing", code], value);
      void cache.invalidateQueries({ queryKey: ["devices"] });
      void cache.invalidateQueries({ queryKey: ["account-agents"] });
      void cache.invalidateQueries({ queryKey: ["brain-agents"] });
    },
  });
  const done =
    pairing.data?.state === "approved" || pairing.data?.state === "claimed";
  return (
    <section
      aria-labelledby="pairing-approval-title"
      className="flex animate-enter flex-col gap-4 rounded-lg border border-border-strong bg-card p-5 shadow-sm"
    >
      <div className="flex items-center gap-3">
        <Tile tone="warning">
          <ShieldCheck aria-hidden="true" />
        </Tile>
        <h2
          id="pairing-approval-title"
          className="m-0 text-section font-semibold text-foreground"
        >
          Approve this host
        </h2>
      </div>
      {pairing.isPending ? (
        <SkeletonRows rows={2} label="Loading pairing request…" />
      ) : pairing.error ? (
        <Alert variant="destructive">{pairing.error.message}</Alert>
      ) : (
        pairing.data && (
          <>
            <p className="m-0 text-body text-foreground-secondary">
              <strong className="text-foreground">{pairing.data.name}</strong>{" "}
              is requesting access as your account.
            </p>
            <div className="flex items-center justify-between gap-4 rounded-md bg-muted px-4 py-3">
              <span className="mono text-[26px] font-semibold leading-8 tracking-[0.15em] text-foreground">
                {pairing.data.user_code}
              </span>
              <span className="mono text-caption text-muted-foreground">
                Expires {date(pairing.data.expires_at)}
              </span>
            </div>
            {pairing.data.state === "pending" ? (
              <>
                <p className="m-0 text-label text-muted-foreground">
                  Check that this code matches the host you just started on your
                  own computer.
                </p>
                {pairing.data.reuses_device && (
                  <Alert variant="warning">
                    You already have a device named{" "}
                    <strong>{pairing.data.reuses_device}</strong>. Approving
                    replaces its token, so the host using it signs out.
                  </Alert>
                )}
                <div className="flex gap-2">
                  <Button
                    loading={decision.isPending && decision.variables === true}
                    disabled={decision.isPending}
                    onClick={() => decision.mutate(true)}
                  >
                    {!(decision.isPending && decision.variables === true) && (
                      <Check aria-hidden="true" />
                    )}
                    {pairing.data.reuses_device
                      ? "Replace and approve"
                      : "Approve device"}
                  </Button>
                  <Button
                    variant="secondary"
                    disabled={decision.isPending}
                    onClick={() => decision.mutate(false)}
                  >
                    <X aria-hidden="true" />
                    Decline
                  </Button>
                </div>
              </>
            ) : (
              <Alert variant={done ? "brand" : "default"} role="status">
                {pairing.data.state === "approved"
                  ? "Approved. Waiting for your host to finish pairing."
                  : pairing.data.state === "claimed"
                    ? "Pairing complete. Credential enabled."
                    : "This pairing was declined or cancelled. Start a new request to try again."}
              </Alert>
            )}
          </>
        )
      )}
      {decision.error && (
        <Alert variant="destructive">{decision.error.message}</Alert>
      )}
      <Button
        onClick={() =>
          void navigate({ to: "/agents", search: { access: true } })
        }
        variant="ghost"
        size="sm"
        className="self-start"
      >
        Close pairing request
      </Button>
    </section>
  );
}
