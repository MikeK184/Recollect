import { useEffect, useState, type ReactNode } from "react";
import {
  Alert,
  Badge,
  Button,
  CopyButton,
  Loader,
  Select,
  Switch,
  TextInput,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
  Check,
  Copy,
  LockKeyhole,
  Monitor,
  Network,
  Pencil,
  Plus,
} from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import {
  RunnerConnections,
  useRunnerCatalogue,
} from "./features/connections/RunnerConnections";
import "./features/connections/private-runners.css";

type Runner = components["schemas"]["McpPrivateRunner"];
type Device = components["schemas"]["Device"];
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

function runnerStatus(runner: Runner) {
  return !runner.enabled
    ? "Disabled"
    : !runner.eligible
      ? "Host unavailable"
      : runner.available
        ? "Connected"
        : "Offline";
}
function setupCommand(brain: string, runner: string) {
  return `recollect-plugin connect --url ${window.location.origin} --brain ${brain} --with-runner --runner-id ${runner}`;
}
function deviceLabel(runner: Runner, devices?: Device[]) {
  return (
    devices?.find((device) => device.id === runner.device_id)?.name ??
    `Paired device · ${runner.device_id.slice(0, 8)}`
  );
}
function RunnerStatus({ runner }: { runner: Runner }) {
  return (
    <Badge
      variant="light"
      color={
        runner.enabled && runner.eligible && runner.available ? "teal" : "gray"
      }
    >
      {runnerStatus(runner)}
    </Badge>
  );
}
function SetupCopy({
  brain,
  runner,
  disabled = false,
}: {
  brain: string;
  runner: Runner;
  disabled?: boolean;
}) {
  return (
    <CopyButton value={setupCommand(brain, runner.id)}>
      {({ copied, copy }) => (
        <Button
          type="button"
          size="xs"
          variant="subtle"
          disabled={disabled}
          leftSection={copied ? <Check size={13} /> : <Copy size={13} />}
          title="Start this runner on its registered device"
          onClick={copy}
        >
          {copied ? "Command copied" : "Copy setup command"}
        </Button>
      )}
    </CopyButton>
  );
}
export function McpPrivateRunners({
  brain,
  canConfigure,
  onInspectorChange,
}: {
  brain: Brain;
  canConfigure: boolean;
  onInspectorChange?: (open: boolean) => void;
}) {
  const runners = usePrivateRunners(brain.id);
  const catalogue = useRunnerCatalogue(brain.id);
  const cache = useQueryClient();
  const [editing, setEditing] = useState<{
    brain: string;
    runner: Runner | "new";
  } | null>(null);
  const mayEdit = canConfigure && !brain.archived && !runners.error;
  const edit = mayEdit && editing?.brain === brain.id ? editing.runner : null;
  const devices = useQuery({
    queryKey: ["devices"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/devices", { signal })),
    enabled: canConfigure,
    retry: false,
    refetchInterval: 5000,
  });
  useEffect(() => {
    onInspectorChange?.(!!edit);
    return () => onInspectorChange?.(false);
  }, [edit, onInspectorChange]);
  useEffect(() => {
    if (!mayEdit || editing?.brain !== brain.id) setEditing(null);
  }, [mayEdit, brain.id, editing?.brain]);
  const saved = () => {
    setEditing(null);
    void cache.invalidateQueries({ queryKey: ["mcp", brain.id] });
    void cache.invalidateQueries({ queryKey: ["audit", brain.id] });
  };
  const editor = (runner?: Runner) => (
    <RunnerEditor
      key={`${brain.id}:${runner?.id ?? "new"}`}
      brain={brain.id}
      runner={runner}
      latest={
        runner ? runners.data?.find((r) => r.id === runner.id) : undefined
      }
      devices={devices}
      registrations={runners.data ?? []}
      close={() => setEditing(null)}
      saved={saved}
      relationships={
        runner
          ? (disabled) => (
              <RunnerConnections
                brain={brain}
                runner={runner.id}
                catalogue={catalogue}
                disabled={disabled}
              />
            )
          : undefined
      }
    />
  );
  return (
    <section className="private-runners-view" aria-label="Private runners">
      <header className="private-runners-toolbar">
        <p className="private-runners-count">
          {runners.data && !runners.error
            ? `${runners.data.length} private runner${runners.data.length === 1 ? "" : "s"}`
            : "Private runners"}
        </p>
        {canConfigure && (
          <Button
            size="xs"
            leftSection={<Plus size={14} />}
            disabled={
              !mayEdit || !runners.data || !!edit || runners.data.length >= 32
            }
            onClick={() => setEditing({ brain: brain.id, runner: "new" })}
          >
            Add runner
          </Button>
        )}
      </header>
      {runners.isPending && (
        <div role="status" className="private-runner-notice">
          <Loader size="sm" /> Loading private runners…
        </div>
      )}
      {runners.error && (
        <Alert color="red" title="Private runners unavailable">
          {runners.error.message}
          <Button
            size="xs"
            variant="subtle"
            onClick={() => void runners.refetch()}
          >
            Refresh private runners
          </Button>
        </Alert>
      )}
      {edit === "new" && editor()}
      {!runners.error && runners.data?.length === 0 && edit !== "new" && (
        <div className="private-runner-empty">
          <span className="management-icon">
            <Network size={20} />
          </span>
          <div>
            <h3>No private runners yet</h3>
            <p>Add a runner to reach services inside your network.</p>
          </div>
        </div>
      )}
      {!runners.error &&
        runners.data?.map((runner) =>
          edit !== "new" && edit?.id === runner.id ? (
            editor(edit)
          ) : (
            <article
              key={runner.id}
              className="private-runner-card"
              data-testid="mcp-private-runner"
              aria-label={`Private runner ${runner.name}`}
            >
              <header className="private-runner-heading">
                <span className="management-icon private-runner-icon">
                  <Network size={20} />
                </span>
                <div className="private-runner-identity">
                  <h3>{runner.name}</h3>
                  <div className="private-runner-meta">
                    <span className="private-runner-device-label">Runs on</span>
                    <Badge
                      variant="light"
                      color="gray"
                      leftSection={<Monitor size={12} />}
                      title={runner.device_id}
                    >
                      {deviceLabel(
                        runner,
                        devices.error ? undefined : devices.data,
                      )}
                    </Badge>
                  </div>
                </div>
                <div className="private-runner-actions">
                  <RunnerStatus runner={runner} />
                  {runner.enabled && (
                    <span className="private-runner-enabled">Enabled</span>
                  )}
                  {canConfigure && (
                    <Button
                      size="xs"
                      variant="subtle"
                      aria-label="Edit private runner"
                      leftSection={<Pencil size={13} />}
                      disabled={!mayEdit || !!edit}
                      onClick={() => setEditing({ brain: brain.id, runner })}
                    >
                      Edit
                    </Button>
                  )}
                </div>
              </header>
              <RunnerConnections
                brain={brain}
                runner={runner.id}
                catalogue={catalogue}
              />
              <footer className="private-runner-footer">
                <Link className="private-runners-devices" to="/agents">
                  Manage paired devices →
                </Link>
                {canConfigure && (
                  <SetupCopy
                    brain={brain.id}
                    runner={runner}
                    disabled={brain.archived}
                  />
                )}
              </footer>
            </article>
          ),
        )}
      {edit &&
        edit !== "new" &&
        !runners.data?.some((r) => r.id === edit.id) &&
        editor(edit)}
      {canConfigure && runners.data?.length === 0 && (
        <Link className="private-runners-devices" to="/agents">
          Manage paired devices →
        </Link>
      )}
    </section>
  );
}
function RunnerEditor({
  brain,
  runner,
  latest,
  devices,
  registrations,
  close,
  saved,
  relationships,
}: {
  brain: string;
  runner?: Runner;
  latest?: Runner;
  devices: ReturnType<typeof useQuery<Device[], Error>>;
  registrations: Runner[];
  close: () => void;
  saved: () => void;
  relationships?: (disabled: boolean) => ReactNode;
}) {
  const [name, setName] = useState(runner?.name ?? "");
  const [device, setDevice] = useState<string | null>(
    runner?.device_id ?? null,
  );
  const [enabled, setEnabled] = useState(runner?.enabled ?? true);
  const command = useIdempotency();
  const active =
    devices.data?.filter(
      (d) =>
        d.claimed &&
        !d.revoked_at &&
        Date.parse(d.expires_at) > Date.now() &&
        !registrations.some((r) => r.device_id === d.id),
    ) ?? [];
  const stale = !!runner && (!latest || runner.revision !== latest.revision);
  const valid =
    !!name.trim() &&
    name.trim().length <= 120 &&
    !!device &&
    !stale &&
    (!!runner ||
      (!devices.error &&
        active.some((d) => d.id === device) &&
        registrations.length < 32));
  const save = useMutation({
    mutationFn: async () => {
      if (!valid)
        throw new Error(
          "Choose a name and an available paired device; reopen changed registrations.",
        );
      const body = {
        name: name.trim(),
        device_id: device!,
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
    <form
      className="private-runner-card private-runner-editor"
      aria-label={runner ? "Edit private runner" : "Add private runner"}
      onSubmit={(e) => {
        e.preventDefault();
        if (valid && !save.isPending) save.mutate();
      }}
    >
      <fieldset className="private-runner-fields" disabled={save.isPending}>
        <header className="private-runner-heading">
          <span className="management-icon private-runner-icon">
            <Network size={20} />
          </span>
          <div className="private-runner-identity">
            <TextInput
              size="xs"
              className="private-runner-name"
              aria-label="Private runner name"
              placeholder="Runner name"
              required
              maxLength={120}
              value={name}
              onChange={(e) => {
                setName(e.currentTarget.value);
                save.reset();
              }}
            />
            <div className="private-runner-meta">
              <span className="private-runner-device-label">Runs on</span>
              {runner ? (
                <Badge
                  variant="light"
                  color="gray"
                  leftSection={<LockKeyhole size={12} />}
                  title={`Fixed paired device: ${runner.device_id}`}
                >
                  {deviceLabel(
                    runner,
                    devices.error ? undefined : devices.data,
                  )}
                </Badge>
              ) : (
                <Select
                  size="xs"
                  className="private-runner-device"
                  aria-label="Paired runner device"
                  placeholder={
                    devices.isPending
                      ? "Loading devices…"
                      : "Choose paired device"
                  }
                  required
                  data={active.map((d) => ({ value: d.id, label: d.name }))}
                  value={device}
                  disabled={
                    save.isPending ||
                    devices.isPending ||
                    !!devices.error ||
                    !active.length
                  }
                  onChange={(v) => {
                    setDevice(v);
                    save.reset();
                  }}
                />
              )}
            </div>
          </div>
          <div className="private-runner-actions">
            {runner ? (
              <RunnerStatus runner={latest ?? runner} />
            ) : (
              <Badge variant="light" color="gray">
                Not connected
              </Badge>
            )}
            <Switch
              size="xs"
              label="Enabled"
              aria-label="Private runner enabled"
              checked={enabled}
              onChange={(e) => {
                setEnabled(e.currentTarget.checked);
                save.reset();
              }}
            />
          </div>
        </header>
        {relationships?.(save.isPending)}
        <div className="private-runner-editor-feedback">
          {stale && (
            <Alert color="yellow">
              This registration changed. Cancel and reopen it before saving.
            </Alert>
          )}
          {!runner && devices.error && (
            <Alert color="red" title="Paired devices unavailable">
              {devices.error.message}
              <Button
                size="xs"
                variant="subtle"
                onClick={() => void devices.refetch()}
              >
                Refresh paired devices
              </Button>
            </Alert>
          )}
          {!runner &&
            !devices.isPending &&
            !devices.error &&
            !active.length && (
              <Alert color="yellow">
                {devices.data?.some(
                  (d) =>
                    d.claimed &&
                    !d.revoked_at &&
                    Date.parse(d.expires_at) > Date.now(),
                )
                  ? "All active devices already have a runner in this Brain."
                  : "Connect the Recollect plugin on an active device first."}
              </Alert>
            )}
          {save.error && (
            <Alert color="red" title="Private runner not saved">
              {save.error.message}
            </Alert>
          )}
        </div>
        <footer className="private-runner-footer">
          {runner ? (
            <SetupCopy
              brain={brain}
              runner={runner}
              disabled={save.isPending}
            />
          ) : (
            <p>Save, then start the runner on its paired device.</p>
          )}
          <div className="private-runner-actions">
            <Button size="xs" variant="default" type="button" onClick={close}>
              Cancel
            </Button>
            <Button
              size="xs"
              type="submit"
              loading={save.isPending}
              disabled={!valid}
            >
              {runner ? "Save changes" : "Add runner"}
            </Button>
          </div>
        </footer>
      </fieldset>
    </form>
  );
}
