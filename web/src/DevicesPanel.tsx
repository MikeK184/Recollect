import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Code,
  Group,
  Loader,
  Modal,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { Check, Laptop, ShieldCheck } from "lucide-react";
import { client, result } from "./api";

const date = (value: string) => new Date(value).toLocaleString();

export function DevicesPanel() {
  const cache = useQueryClient();
  const search = useRouterState({
    select: (state) => state.location.searchStr,
  });
  const code = new URLSearchParams(search).get("code") ?? "";
  const [selected, setSelected] = useState<{ id: string; name: string } | null>(
    null,
  );
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
      void cache.invalidateQueries({ queryKey: ["devices"] });
    },
  });
  useEffect(() => {
    if (devices.isError) setSelected(null);
  }, [devices.isError]);
  return (
    <Stack gap="xl">
      <div className="page-heading">
        <div>
          <span className="eyebrow">YOUR COMPANIONS</span>
          <Title order={1}>Devices</Title>
          <Text c="dimmed" mt="xs">
            Connect your tools to Recollect and manage each device’s access.
          </Text>
        </div>
        <Badge
          variant="light"
          color="teal"
          leftSection={<ShieldCheck size={13} />}
        >
          Browser approved
        </Badge>
      </div>
      {code && <PairingApproval key={code} code={code} />}
      <Card withBorder p="xl" bg="var(--rc-surface)">
        <Group align="flex-start" wrap="nowrap">
          <Laptop size={24} style={{ flexShrink: 0 }} />
          <Stack gap="xs">
            <Text fw={600}>Pair a companion</Text>
            <Text size="sm" c="dimmed">
              On your computer, run <Code>recollect-agent pair</Code> and open
              the link it displays. Compare the code, then approve the device
              here.
            </Text>
            <Text size="xs" c="dimmed">
              A companion acts as you and uses your current Brain permissions.
              You can revoke it independently at any time.
            </Text>
          </Stack>
        </Group>
      </Card>
      {devices.error && (
        <Alert color="red" title="Devices could not be loaded">
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
          <Stack gap="md">
            <Text fw={600}>
              Your devices{" "}
              <span className="count-pill">{devices.data?.length ?? 0}</span>
            </Text>
            {!devices.data?.length && (
              <Card withBorder className="empty-state">
                <div className="empty-icon">
                  <Laptop size={26} />
                </div>
                <Title order={3}>No devices paired yet</Title>
                <Text c="dimmed" size="sm" ta="center">
                  Approve a companion to bring your context with you.
                </Text>
              </Card>
            )}
            {devices.data?.map((device) => {
              const expired =
                new Date(device.expires_at).getTime() <= Date.now();
              const status = device.revoked_at
                ? "Revoked"
                : expired
                  ? "Expired"
                  : device.claimed
                    ? "Active"
                    : "Waiting for companion";
              return (
                <Card
                  withBorder
                  p="lg"
                  key={device.id}
                  data-testid="device-card"
                >
                  <Group justify="space-between" align="flex-start" gap="md">
                    <Stack gap={7} style={{ minWidth: 0, flex: "1 1 230px" }}>
                      <Group>
                        <Laptop size={18} />
                        <Text fw={600} style={{ overflowWrap: "anywhere" }}>
                          {device.name}
                        </Text>
                        <Badge color={status === "Active" ? "teal" : "gray"}>
                          {status}
                        </Badge>
                      </Group>
                      <Text size="xs" c="dimmed">
                        Paired {date(device.created_at)}
                      </Text>
                      <Text size="xs" c="dimmed">
                        Last used{" "}
                        {device.last_used_at
                          ? date(device.last_used_at)
                          : "— not used yet"}
                      </Text>
                      <Text size="xs" c="dimmed">
                        {device.revoked_at
                          ? `Revoked ${date(device.revoked_at)}`
                          : `Expires ${date(device.expires_at)}`}
                      </Text>
                    </Stack>
                    {!device.revoked_at && (
                      <Button
                        size="xs"
                        color="red"
                        variant="light"
                        onClick={() => {
                          revoke.reset();
                          setSelected(device);
                        }}
                      >
                        Revoke
                      </Button>
                    )}
                  </Group>
                </Card>
              );
            })}
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
            <strong>{selected?.name}</strong> will lose access to Recollect.
            Pair it again when you want to reconnect.
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
    },
  });
  return (
    <Card withBorder p="xl" style={{ borderColor: "var(--rc-accent)" }}>
      <Stack>
        <Group>
          <ShieldCheck size={22} />
          <Title order={2} fz={22}>
            Approve a companion
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
                    Check that this code matches the companion you just started
                    on your own computer.
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
                    ? "Approved. Waiting for your companion to finish pairing."
                    : pairing.data.state === "claimed"
                      ? "Your companion is connected."
                      : "This pairing was declined or cancelled. Start a new request to try again."}
                </Alert>
              )}
            </>
          )
        )}
        {decision.error && <Alert color="red">{decision.error.message}</Alert>}
        <Button
          onClick={() =>
            void navigate({ to: "/devices", search: { code: undefined } })
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
