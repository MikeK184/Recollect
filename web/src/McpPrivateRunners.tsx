import { useEffect, useState, type ReactNode } from "react";
import {
  Alert,
  Badge,
  Button,
  CopyButton,
  Input,
  Select,
  Spinner,
  StatusBadge,
  Switch,
  Tile,
} from "./components/ui";
import { staggerStyle } from "./components/Motion";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
  ArrowRight,
  Check,
  CircleCheck,
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
import { usePollInterval } from "./usePollInterval";

type Runner = components["schemas"]["McpPrivateRunner"];
type Device = components["schemas"]["Device"];
export function usePrivateRunners(brain: string) {
  const pollNormal = usePollInterval("normal");
  return useQuery({
    queryKey: ["mcp", brain, "private-runners"],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/private-runners", {
          params: { path: { brain } },
          signal,
        }),
      ),
    refetchInterval: pollNormal,
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
    <StatusBadge
      state={
        !runner.enabled
          ? "neutral"
          : !runner.eligible
            ? "negative"
            : runner.available
              ? "positive"
              : "attention"
      }
    >
      {runnerStatus(runner)}
    </StatusBadge>
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
          size="sm"
          variant="ghost"
          disabled={disabled}
          title="Start this runner on its registered device"
          onClick={copy}
        >
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
          {copied ? "Command copied" : "Copy setup command"}
        </Button>
      )}
    </CopyButton>
  );
}

const cardClass =
  "overflow-hidden rounded-lg border border-border bg-card transition-[border-color,box-shadow] duration-base ease-standard";
const devicesLink =
  "inline-flex items-center gap-1 rounded-sm text-label font-medium text-foreground-secondary transition-colors duration-fast hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring [&_svg]:size-3.5";
const deviceLabelClass = "text-caption text-muted-foreground";

export function McpPrivateRunners({
  brain,
  canConfigure,
  onInspectorChange,
}: {
  brain: Brain;
  canConfigure: boolean;
  onInspectorChange?: (open: boolean) => void;
}) {
  const pollNormal = usePollInterval("normal");
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
    refetchInterval: pollNormal,
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
    <section className="grid min-w-0 gap-3.5" aria-label="Private runners">
      <header className="flex min-h-8 items-center justify-between gap-3">
        <p className="m-0 text-label font-medium text-foreground-secondary">
          {runners.data && !runners.error ? (
            <>
              <span className="mono">{runners.data.length}</span> private runner
              {runners.data.length === 1 ? "" : "s"}
            </>
          ) : (
            "Private runners"
          )}
        </p>
        {canConfigure && (
          <Button
            disabled={
              !mayEdit || !runners.data || !!edit || runners.data.length >= 32
            }
            onClick={() => setEditing({ brain: brain.id, runner: "new" })}
          >
            <Plus aria-hidden="true" />
            Add runner
          </Button>
        )}
      </header>
      {runners.isPending && (
        <div
          role="status"
          className="flex items-center gap-2 rounded-lg border border-border bg-card px-4 py-3 text-label text-muted-foreground"
        >
          <Spinner size={14} /> Loading private runners…
        </div>
      )}
      {runners.error && (
        <Alert
          variant="destructive"
          title="Private runners unavailable"
          actions={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void runners.refetch()}
            >
              Refresh private runners
            </Button>
          }
        >
          {runners.error.message}
        </Alert>
      )}
      {edit === "new" && editor()}
      {!runners.error && runners.data?.length === 0 && edit !== "new" && (
        <div
          className={`${cardClass} flex animate-enter items-center gap-3.5 p-4`}
        >
          <Tile size="lg">
            <Network aria-hidden="true" />
          </Tile>
          <div className="min-w-0">
            <h3 className="m-0 text-title font-semibold text-foreground">
              No private runners yet
            </h3>
            <p className="m-0 mt-0.5 text-body text-muted-foreground">
              Add a runner to reach services inside your network.
            </p>
          </div>
        </div>
      )}
      {!runners.error &&
        runners.data?.map((runner, index) =>
          edit !== "new" && edit?.id === runner.id ? (
            editor(edit)
          ) : (
            <article
              key={runner.id}
              className={`${cardClass} animate-enter hover:border-border-strong`}
              style={staggerStyle(index)}
              data-testid="mcp-private-runner"
              aria-label={`Private runner ${runner.name}`}
            >
              <header className="flex items-start gap-3 border-b border-border px-4 py-3.5">
                <Tile className="mt-0.5">
                  <Network aria-hidden="true" />
                </Tile>
                <div className="min-w-0 flex-1">
                  <h3 className="m-0 truncate text-title font-semibold text-foreground">
                    {runner.name}
                  </h3>
                  <div className="mt-1 flex flex-wrap items-center gap-1.5">
                    <span className={deviceLabelClass}>Runs on</span>
                    <Badge variant="outline" title={runner.device_id}>
                      <Monitor aria-hidden="true" />
                      {deviceLabel(
                        runner,
                        devices.error ? undefined : devices.data,
                      )}
                    </Badge>
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-2 pt-0.5">
                  <RunnerStatus runner={runner} />
                  {runner.enabled && (
                    <span className="inline-flex items-center gap-1 text-caption text-muted-foreground">
                      <CircleCheck
                        aria-hidden="true"
                        className="size-3.5 text-brand-text"
                      />
                      Enabled
                    </span>
                  )}
                  {canConfigure && (
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label="Edit private runner"
                      disabled={!mayEdit || !!edit}
                      onClick={() => setEditing({ brain: brain.id, runner })}
                    >
                      <Pencil aria-hidden="true" />
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
              <footer className="flex min-h-11 items-center justify-between gap-3 border-t border-border bg-muted px-4 py-1.5">
                <Link className={devicesLink} to="/agents">
                  Manage paired devices
                  <ArrowRight aria-hidden="true" />
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
        <Link className={`${devicesLink} justify-self-start`} to="/agents">
          Manage paired devices
          <ArrowRight aria-hidden="true" />
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
      className={`${cardClass} animate-enter border-border-strong shadow-sm`}
      aria-label={runner ? "Edit private runner" : "Add private runner"}
      onSubmit={(e) => {
        e.preventDefault();
        if (valid && !save.isPending) save.mutate();
      }}
    >
      <fieldset className="m-0 min-w-0 border-0 p-0" disabled={save.isPending}>
        <header className="flex items-start gap-3 border-b border-border px-4 py-3.5">
          <Tile className="mt-0.5">
            <Network aria-hidden="true" />
          </Tile>
          <div className="grid min-w-0 flex-1 gap-1.5">
            <Input
              size="sm"
              className="max-w-[320px] font-medium"
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
            <div className="flex flex-wrap items-center gap-1.5">
              <span className={deviceLabelClass}>Runs on</span>
              {runner ? (
                <Badge
                  variant="outline"
                  title={`Fixed paired device: ${runner.device_id}`}
                >
                  <LockKeyhole aria-hidden="true" />
                  {deviceLabel(
                    runner,
                    devices.error ? undefined : devices.data,
                  )}
                </Badge>
              ) : (
                <Select
                  size="sm"
                  className="w-[260px]"
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
          <div className="flex shrink-0 items-center gap-3 pt-0.5">
            {runner ? (
              <RunnerStatus runner={latest ?? runner} />
            ) : (
              <StatusBadge state="neutral">Not connected</StatusBadge>
            )}
            <Switch
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
        {(stale ||
          (!runner && devices.error) ||
          (!runner && !devices.isPending && !devices.error && !active.length) ||
          save.error) && (
          <div className="grid gap-2 px-4 pb-3 pt-3">
            {stale && (
              <Alert variant="warning">
                This registration changed. Cancel and reopen it before saving.
              </Alert>
            )}
            {!runner && devices.error && (
              <Alert
                variant="destructive"
                title="Paired devices unavailable"
                actions={
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => void devices.refetch()}
                  >
                    Refresh paired devices
                  </Button>
                }
              >
                {devices.error.message}
              </Alert>
            )}
            {!runner &&
              !devices.isPending &&
              !devices.error &&
              !active.length && (
                <Alert variant="warning" role="status">
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
              <Alert variant="destructive" title="Private runner not saved">
                {save.error.message}
              </Alert>
            )}
          </div>
        )}
        <footer className="flex min-h-12 items-center justify-between gap-3 border-t border-border bg-muted px-4 py-2">
          {runner ? (
            <SetupCopy
              brain={brain}
              runner={runner}
              disabled={save.isPending}
            />
          ) : (
            <p className="m-0 text-caption text-muted-foreground">
              Save, then start the runner on its paired device.
            </p>
          )}
          <div className="flex items-center gap-2">
            <Button size="sm" variant="secondary" type="button" onClick={close}>
              Cancel
            </Button>
            <Button
              size="sm"
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
