import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Code,
  Group,
  Loader,
  Modal,
  Select,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";

type Runner = components["schemas"]["McpPrivateRunner"];
export function usePrivateRunners(brain: string) {
  return useQuery({
    queryKey: ["mcp", brain, "private-runners"],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/private-runners", {
          params: { path: { brain } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
}
export function McpPrivateRunners({
  brain,
  canConfigure,
}: {
  brain: Brain;
  canConfigure: boolean;
}) {
  const runners = usePrivateRunners(brain.id);
  const cache = useQueryClient();
  const [editing, setEditing] = useState<Runner | "new" | null>(null);
  useEffect(() => {
    if (!canConfigure || brain.archived || runners.error) setEditing(null);
  }, [canConfigure, brain.archived, runners.error]);
  const refresh = () => {
    void cache.invalidateQueries({ queryKey: ["mcp", brain.id] });
    void cache.invalidateQueries({ queryKey: ["audit", brain.id] });
  };
  return (
    <Stack component="section" aria-label="Private runners">
      <Group justify="space-between">
        <Title order={4}>Private runners</Title>
        {canConfigure && (
          <Button
            variant="light"
            size="xs"
            disabled={brain.archived}
            onClick={() => setEditing("new")}
          >
            Register private runner
          </Button>
        )}
      </Group>
      <Text size="sm" c="dimmed">
        A private runner is the place your Brain chooses for targets on a
        network the central service cannot reach. Each registration binds this
        Brain to one paired device, and that device decides which network can
        reach the target. Registering or renaming a runner is metadata only: it
        starts nothing and proves no connectivity.
      </Text>
      <Text size="sm" c="dimmed">
        Manage the device itself in{" "}
        <Link to="/devices" search={{ code: undefined }}>
          Devices
        </Link>
        .
      </Text>
      {runners.isPending && <Loader size="sm" />}
      {runners.error && (
        <Alert color="red">
          {runners.error.message}
          <Button
            size="xs"
            variant="light"
            onClick={() => void runners.refetch()}
          >
            Refresh private runners
          </Button>
        </Alert>
      )}
      {!runners.error && runners.data?.length === 0 && (
        <Text size="sm">No private runners are registered for this Brain.</Text>
      )}
      {!runners.error &&
        runners.data?.map((runner) => (
          <Card withBorder key={runner.id} data-testid="mcp-private-runner">
            <Group justify="space-between">
              <Text fw={600}>{runner.name}</Text>
              <Badge color={runner.available ? "teal" : "gray"}>
                {!runner.enabled
                  ? "Disabled"
                  : !runner.eligible
                    ? "Host unavailable"
                    : runner.available
                      ? "Connected"
                      : "Offline"}
              </Badge>
            </Group>
            <Text size="sm" c="dimmed">
              {runner.lease_until
                ? `Last connection lease: ${new Date(runner.lease_until).toLocaleString()}`
                : "No runner connection has been established."}
            </Text>
            {!runner.eligible && runner.enabled && (
              <Text size="sm">
                The host needs an active paired device and current Brain
                administrator access.
              </Text>
            )}
            {canConfigure && (
              <>
                <Button
                  mt="xs"
                  size="xs"
                  variant="light"
                  disabled={brain.archived}
                  onClick={() => setEditing(runner)}
                >
                  Edit private runner
                </Button>
                <details>
                  <summary>Start on the registered device</summary>
                  <Text size="sm">
                    Use the companion profile paired to this device. Choose a
                    private directory for pending receipts.
                  </Text>
                  <Code
                    block
                  >{`recollect-agent private-runner ${runner.id} /path/to/private/receipts`}</Code>
                  <Text size="xs" c="dimmed">
                    Paired device: {runner.device_id} ·{" "}
                    <Link to="/devices" search={{ code: undefined }}>
                      open the device record in Devices
                    </Link>
                  </Text>
                </details>
              </>
            )}
          </Card>
        ))}
      {editing && canConfigure && !brain.archived && !runners.error && (
        <RunnerDialog
          key={editing === "new" ? "new" : editing.id}
          brain={brain.id}
          runner={editing === "new" ? undefined : editing}
          latest={runners.data?.find(
            (r) => editing !== "new" && r.id === editing.id,
          )}
          close={() => setEditing(null)}
          saved={() => {
            setEditing(null);
            refresh();
          }}
        />
      )}
    </Stack>
  );
}
function RunnerDialog({
  brain,
  runner,
  latest,
  close,
  saved,
}: {
  brain: string;
  runner?: Runner;
  latest?: Runner;
  close: () => void;
  saved: () => void;
}) {
  const [name, setName] = useState(runner?.name ?? "");
  const [device, setDevice] = useState<string | null>(
    runner?.device_id ?? null,
  );
  const [enabled, setEnabled] = useState(runner?.enabled ?? true);
  const command = useIdempotency();
  const devices = useQuery({
    queryKey: ["devices"],
    queryFn: async () => result(await client.GET("/api/devices")),
    enabled: !runner,
    retry: false,
    refetchInterval: 5000,
  });
  const active =
    devices.data?.filter(
      (d) =>
        d.claimed &&
        !d.revoked_at &&
        new Date(d.expires_at).getTime() > Date.now(),
    ) ?? [];
  const stale = !!runner && (!latest || runner.revision !== latest.revision);
  const save = useMutation({
    mutationFn: async () => {
      if (!device) throw new Error("Select your active paired device.");
      const body = {
        name,
        device_id: device,
        enabled,
        base_revision: runner?.revision ?? null,
      };
      const headers = { "Idempotency-Key": command.forInput(body) };
      return runner
        ? result(
            await client.PUT("/api/brains/{brain}/mcp/private-runners/{id}", {
              params: { path: { brain, id: runner.id } },
              headers,
              body,
            }),
          )
        : result(
            await client.POST("/api/brains/{brain}/mcp/private-runners", {
              params: { path: { brain } },
              headers,
              body,
            }),
          );
    },
    onSuccess: saved,
  });
  return (
    <Modal
      opened
      onClose={close}
      title={runner ? "Edit private runner" : "Register private runner"}
      size="lg"
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate();
        }}
      >
        <Stack>
          <Text size="sm">
            Registration binds this Brain's runner to one paired device. Start
            the companion there to connect.
          </Text>
          {stale && (
            <Alert color="yellow">
              This registration changed. Close and reopen it before saving.
            </Alert>
          )}
          <TextInput
            label="Private runner name"
            required
            maxLength={120}
            value={name}
            onChange={(e) => {
              setName(e.currentTarget.value);
              save.reset();
            }}
          />
          {runner ? (
            <Text size="sm">
              Paired device: {runner.device_id}. Register a new runner to use
              another device.
            </Text>
          ) : (
            <>
              {devices.isPending && <Loader size="sm" />}
              {devices.error && (
                <Alert color="red">
                  {devices.error.message}
                  <Button
                    size="xs"
                    variant="light"
                    onClick={() => void devices.refetch()}
                  >
                    Refresh paired devices
                  </Button>
                </Alert>
              )}
              {!devices.isPending && !devices.error && !active.length && (
                <Alert color="yellow">
                  Pair an active companion from Devices before registering a
                  private runner.
                </Alert>
              )}
              <Select
                label="Paired runner device"
                placeholder="Choose your active device"
                required
                data={active.map((d) => ({ value: d.id, label: d.name }))}
                value={device}
                onChange={(v) => {
                  setDevice(v);
                  save.reset();
                }}
              />
            </>
          )}
          <Checkbox
            label="Private runner enabled"
            checked={enabled}
            onChange={(e) => {
              setEnabled(e.currentTarget.checked);
              save.reset();
            }}
          />
          {save.error && (
            <Alert color="red" title="Private runner not saved">
              {save.error.message}
            </Alert>
          )}
          <Group justify="end">
            <Button variant="default" onClick={close}>
              Close
            </Button>
            <Button
              type="submit"
              loading={save.isPending}
              disabled={
                stale ||
                !device ||
                (!runner &&
                  (!!devices.error || !active.some((d) => d.id === device)))
              }
            >
              Save private runner
            </Button>
          </Group>
        </Stack>
      </form>
    </Modal>
  );
}
