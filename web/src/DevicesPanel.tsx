import { useEffect, useMemo, useRef, useState } from "react";
import {
  ActionIcon,
  TextInput,
  Tooltip,
  Alert,
  Button,
  Card,
  Checkbox,
  Code,
  CopyButton,
  Group,
  Loader,
  Modal,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
  Check,
  Copy,
  History,
  Laptop,
  Search,
  ShieldCheck,
} from "lucide-react";
import type { components } from "./api-schema";
import { client, result } from "./api";
import { StatusBadge } from "./components/StatusBadge";
import { HostIcon } from "./components/HostIcon";
import "./features/agents/access-tokens.css";

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

export function AgentAccessPanel({ code }: { code?: string }) {
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
    refetchInterval: 3000,
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
  return (
    <Stack gap="xl">
      {code && <PairingApproval key={code} code={code} />}
      {devices.error && (
        <Alert color="red" title="Connections could not be loaded">
          {devices.error.message}
          <Button variant="subtle" onClick={() => void devices.refetch()}>
            Try again
          </Button>
        </Alert>
      )}
      {devices.isPending ? (
        <Loader />
      ) : (
        !devices.error && (
          <Stack gap="lg">
            <Text fw={600}>
              Your access tokens{" "}
              <span className="count-pill">{devices.data?.length ?? 0}</span>
            </Text>
            {!devices.data?.length ? (
              <Card withBorder className="empty-state">
                <div className="empty-icon">
                  <Laptop size={26} />
                </div>
                <Title order={3}>No access tokens yet</Title>
                <Text c="dimmed" size="sm" ta="center">
                  Connect an agent from a Brain&apos;s Agents page.
                </Text>
              </Card>
            ) : (
              <>
                <Group align="flex-start" justify="space-between" gap="md">
                  <Checkbox
                    maw={520}
                    style={{ minWidth: 0 }}
                    checked={showHistory}
                    onChange={(event) =>
                      setShowHistory(event.currentTarget.checked)
                    }
                    label="Include revoked and expired"
                  />
                  <Text size="xs" c="dimmed" style={{ whiteSpace: "nowrap" }}>
                    {rows.current.length} enabled credentials ·{" "}
                    {rows.history.length} in history
                  </Text>
                </Group>
                <TextInput
                  placeholder="Find an access token…"
                  aria-label="Find an access token"
                  value={search}
                  onChange={(event) => setSearch(event.currentTarget.value)}
                  leftSection={<Search size={16} />}
                />
                <Stack gap="xs">
                  <Text fw={600}>
                    Enabled credentials{" "}
                    <span className="count-pill">{rows.current.length}</span>
                  </Text>
                  {visibleCurrent
                    .map((device) => (
                      <DeviceCard
                        device={device}
                        key={device.id}
                        onRevoke={(target) => {
                          revoke.reset();
                          setSelected(target);
                        }}
                      />
                    ))}
                  {!rows.current.length && (
                    <Card withBorder className="empty-state">
                      <div className="empty-icon">
                        <Laptop size={26} />
                      </div>
                      <Title order={3}>No enabled device credentials</Title>
                      <Text c="dimmed" size="sm" ta="center">
                        Every device is revoked or expired and belongs to the
                        history section below.
                      </Text>
                    </Card>
                  )}
                  {!!rows.current.length && !visibleCurrent.length && (
                    <Text size="sm" c="dimmed">No enabled access tokens match your search.</Text>
                  )}
                </Stack>
                {!!rows.history.length && (
                  <Stack gap="md">
                    <Group gap="xs">
                      <History size={16} />
                      <Text fw={600}>Revoked and expired</Text>
                      <span className="count-pill">{rows.history.length}</span>
                    </Group>
                    {showHistory ? (
                      visibleHistory
                        .map((device) => (
                          <DeviceCard
                            device={device}
                            key={device.id}
                            onRevoke={(target) => {
                              revoke.reset();
                              setSelected(target);
                            }}
                          />
                        ))
                    ) : (
                      <Text size="xs" c="dimmed">
                        {rows.history.length}{" "}
                        {rows.history.length === 1
                          ? "record is"
                          : "records are"}{" "}
                        collapsed. Use “Include revoked and expired” above
                        to list them.
                      </Text>
                    )}
                  </Stack>
                )}
              </>
            )}
          </Stack>
        )
      )}
      <Modal
        opened={!!selected}
        onClose={() => !revoke.isPending && setSelected(null)}
        title="Revoke device access"
        centered
      >
        <Stack>
          <Text size="sm">
            This revokes <strong>{selected?.name}</strong>&apos;s keycard from{" "}
            <b>all Brains</b>. Its token stops working at once; captured history
            stays in place.
          </Text>
          {revoke.error && <Alert color="red">{revoke.error.message}</Alert>}
          <Group justify="flex-end">
            <Button
              variant="default"
              disabled={revoke.isPending}
              onClick={() => setSelected(null)}
            >
              Keep device
            </Button>
            <Button
              color="red"
              loading={revoke.isPending}
              onClick={() => selected && revoke.mutate(selected.id)}
            >
              Revoke device
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}

/** One device row: name, a text status mark, human dates, and the machine
 * identifier only as a copyable field in the detail region below the row. */
function DeviceCard({
  device,
  onRevoke,
}: {
  device: Device;
  onRevoke: (device: Device) => void;
}) {
  const status = statusOf(device);
  return (
    <div className="access-token-row" data-testid="device-card">
      <div className="access-token-identity">
        <div className="access-token-icon">
          <HostIcon host={device.host_kind} size={20} />
        </div>
        <div>
          <Text fw={600} size="sm" data-testid="device-row-name">
            {device.name}
          </Text>
          <Text size="xs" c="dimmed">
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
          </Text>
          <StatusBadge state={status.mark}>
            {status.label === "Credential enabled" ? "Enabled" : status.label}
          </StatusBadge>
        </div>
      </div>
      <div className="access-token-dates">
        <Text size="xs" c="dimmed">
          Last used
        </Text>
        <Text size="xs">
          {device.last_used_at ? date(device.last_used_at) : "Not used yet"}
        </Text>
        <Tooltip
          label={`Paired ${new Date(device.created_at).toLocaleString()}`}
        >
          <Text size="xs" c="dimmed">
            {device.revoked_at
              ? `Revoked ${date(device.revoked_at)}`
              : `Expires ${date(device.expires_at)}`}
          </Text>
        </Tooltip>
      </div>
      <Group gap={4} wrap="nowrap">
        <CopyButton value={device.id}>
          {({ copied, copy }) => (
            <Tooltip label={copied ? "Copied" : "Copy device ID"}>
              <ActionIcon
                variant="subtle"
                aria-label="Copy device ID"
                onClick={copy}
              >
                {copied ? <Check size={16} /> : <Copy size={16} />}
              </ActionIcon>
            </Tooltip>
          )}
        </CopyButton>
        {!device.revoked_at && (
          <Button
            size="compact-xs"
            color="red"
            variant="subtle"
            onClick={() => onRevoke(device)}
          >
            Revoke
          </Button>
        )}
      </Group>
    </div>
  );
}

function PairingApproval({ code }: { code: string }) {
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
    refetchInterval: 2000,
  });
  const decision = useMutation({
    mutationFn: async (approve: boolean) =>
      result(
        await client.POST("/api/devices/pairings/{code}/approve", {
          params: { path: { code } },
          body: { approve },
        }),
      ),
    onSuccess: (value) => {
      cache.setQueryData(["pairing", code], value);
      void cache.invalidateQueries({ queryKey: ["devices"] });
      void cache.invalidateQueries({ queryKey: ["account-agents"] });
      void cache.invalidateQueries({ queryKey: ["brain-agents"] });
    },
  });
  return (
    <Card withBorder p="xl" style={{ borderColor: "var(--rc-accent)" }}>
      <Stack>
        <Group>
          <ShieldCheck size={22} />
          <Title order={2} fz={22}>
            Approve this host
          </Title>
        </Group>
        {pairing.isPending ? (
          <Loader />
        ) : pairing.error ? (
          <Alert color="red">{pairing.error.message}</Alert>
        ) : (
          pairing.data && (
            <>
              <Text>
                <strong>{pairing.data.name}</strong> is requesting access as
                your account.
              </Text>
              <Group justify="space-between">
                <Code fz={26} fw={600} style={{ letterSpacing: "0.15em" }}>
                  {pairing.data.user_code}
                </Code>
                <Text size="xs" c="dimmed">
                  Expires {date(pairing.data.expires_at)}
                </Text>
              </Group>
              {pairing.data.state === "pending" ? (
                <>
                  <Text size="sm" c="dimmed">
                    Check that this code matches the host you just started on
                    your own computer.
                  </Text>
                  <Group>
                    <Button
                      leftSection={<Check size={16} />}
                      loading={
                        decision.isPending && decision.variables === true
                      }
                      disabled={decision.isPending}
                      onClick={() => decision.mutate(true)}
                    >
                      Approve device
                    </Button>
                    <Button
                      variant="default"
                      disabled={decision.isPending}
                      onClick={() => decision.mutate(false)}
                    >
                      Decline
                    </Button>
                  </Group>
                </>
              ) : (
                <Alert
                  color={
                    pairing.data.state === "approved" ||
                    pairing.data.state === "claimed"
                      ? "teal"
                      : "gray"
                  }
                >
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
        {decision.error && <Alert color="red">{decision.error.message}</Alert>}
        <Button
          onClick={() =>
            void navigate({ to: "/agents", search: { access: true } })
          }
          variant="subtle"
          size="xs"
          style={{ alignSelf: "flex-start" }}
        >
          Close pairing request
        </Button>
      </Stack>
    </Card>
  );
}
