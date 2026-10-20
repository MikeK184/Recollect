import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { ArrowRight, BookOpen, Layers } from "lucide-react";
import { Badge, Button, Spinner } from "../../components/ui";
import type { MouseEvent } from "react";
import { client, result, RequestError, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { McpConnectorIcon } from "../../components/McpConnectorIcon";
import { usePollInterval } from "../../usePollInterval";
import { useWorkspaceCatalogue } from "../../catalogueQueries";

type Catalogue = components["schemas"]["McpCatalogue"];
type Connection = Catalogue["connections"][number];
type Profile = Catalogue["profiles"][number];
export type RunnerCatalogue = ReturnType<typeof useRunnerCatalogue>;

export function useRunnerCatalogue(brain: string) {
  const pollNormal = usePollInterval("normal");
  return useQuery({
    queryKey: ["mcp", brain, "catalogue"],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp", {
          params: { path: { brain } },
          signal,
        }),
      ),
    refetchInterval: pollNormal,
    retry: false,
    gcTime: 0,
  });
}

export function RunnerConnections({
  brain,
  runner,
  catalogue,
  disabled = false,
}: {
  brain: Brain;
  runner: string;
  catalogue: RunnerCatalogue;
  disabled?: boolean;
}) {
  const workspace = useWorkspaceCatalogue(brain.id, undefined, { poll: "normal" });
  const data = catalogue.error ? undefined : catalogue.data;
  const assigned =
    data?.connections.filter((c) => c.private_runner_id === runner) ?? [];
  const missingBinding = data?.connections.some(
    (c) => c.placement === "private" && c.private_runner_id === undefined,
  );
  return (
    <section
      className="grid gap-2 px-4 py-3"
      aria-label="Assigned connections"
    >
      <header className="flex items-center gap-2">
        <h4 className="section-label m-0">
          {data?.can_configure ? "Assigned connections" : "Visible connections"}
        </h4>
        {data && !missingBinding && (
          <Badge className="mono">
            {assigned.length} MCP{assigned.length === 1 ? "" : "s"}
          </Badge>
        )}
      </header>
      {catalogue.error ? (
        <div className={notice} role="alert">
          Connections unavailable.
          <Button
            size="sm"
            variant="ghost"
            disabled={disabled || catalogue.isFetching}
            onClick={() => void catalogue.refetch()}
          >
            Retry connections
          </Button>
        </div>
      ) : !data ? (
        <p className={notice} role="status">
          <Spinner size={12} /> Loading connections…
        </p>
      ) : missingBinding ? (
        <p className={notice}>
          Runner assignments are unavailable from this server.
        </p>
      ) : !assigned.length ? (
        <p className={notice}>
          {data.can_configure
            ? "No assigned connections."
            : "No connections visible to you."}
        </p>
      ) : (
        <div className="grid gap-2">
          {assigned.map((connection) => (
            <RunnerConnection
              key={connection.id}
              brain={brain}
              connection={connection}
              groups={data.profiles.filter((p) =>
                p.connection_ids.includes(connection.id),
              )}
              definition={data.definitions.find(
                (d) => d.key === connection.definition_key,
              )}
              canConfigure={data.can_configure}
              metadataRevision={data.connections
                .map((c) => `${c.id}:${c.revision}`)
                .join(",")}
              disabled={disabled}
              scope={
                !connection.environment_id
                  ? "Brain-wide"
                  : workspace.error
                    ? "Environment unavailable"
                    : workspace.isPending
                      ? "Loading environment…"
                      : (workspace.data?.environments.find(
                          (e) => e.id === connection.environment_id,
                        )?.name ?? "Environment unavailable")
              }
            />
          ))}
        </div>
      )}
      {!!assigned.some((c) => c.environment_id) && workspace.error && (
        <div className={notice} role="alert">
          Environments unavailable.
          <Button
            size="sm"
            variant="ghost"
            disabled={disabled || workspace.isFetching}
            onClick={() => void workspace.refetch()}
          >
            Retry environments
          </Button>
        </div>
      )}
    </section>
  );
}

const notice =
  "m-0 flex flex-wrap items-center gap-2 text-label text-muted-foreground";
const linkClass =
  "inline-flex w-fit items-center gap-1 rounded-sm text-label font-medium text-foreground-secondary transition-colors duration-fast hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring aria-disabled:pointer-events-none aria-disabled:opacity-50 [&_svg]:size-3.5";

function RunnerConnection({
  brain,
  connection,
  groups,
  definition,
  canConfigure,
  metadataRevision,
  scope,
  disabled,
}: {
  brain: Brain;
  connection: Connection;
  groups: Profile[];
  definition?: Catalogue["definitions"][number];
  canConfigure: boolean;
  metadataRevision: string;
  scope: string;
  disabled: boolean;
}) {
  const pollNormal = usePollInterval("normal");
  const group = groups.find((p) => p.enabled && p.rights.use_profile);
  const enabled =
    canConfigure ||
    (!!group &&
      !brain.archived &&
      connection.enabled &&
      connection.availability === "configured");
  const tools = useQuery({
    queryKey: canConfigure
      ? [
          "mcp",
          brain.id,
          "runner-definition-tools",
          connection.definition_key,
          definition?.updated_at,
        ]
      : [
          "mcp",
          brain.id,
          "runner-profile-tools",
          group?.id,
          group?.revision,
          group?.environment_id,
          group?.rights.use_profile,
          metadataRevision,
        ],
    enabled,
    queryFn: async ({ signal }) => {
      if (canConfigure)
        return (
          await result(
            await client.GET("/api/brains/{brain}/mcp/definitions/{key}", {
              params: {
                path: { brain: brain.id, key: connection.definition_key },
              },
              signal,
            }),
          )
        ).tools.map((t) => ({ connectionId: null, name: t.name }));
      const names = new Map<
        string,
        { connectionId: string | null; name: string }
      >();
      const unavailable = new Set<string>();
      let offset = 0;
      for (;;) {
        const page = await result(
          await client.POST("/api/brains/{brain}/mcp/discover", {
            params: { path: { brain: brain.id } },
            signal,
            body: {
              profile_id: group!.id,
              environment_id: group!.environment_id ?? null,
              operation_id: null,
              offset,
            },
          }),
        );
        page.unavailable_connections.forEach((c) =>
          unavailable.add(c.connection_id),
        );
        page.tools.forEach((t) =>
          names.set(`${t.connection_id}:${t.tool.name}`, {
            connectionId: t.connection_id,
            name: t.tool.name,
          }),
        );
        if (page.next_offset == null) {
          // Keep one bounded traversal per profile; unavailability is per connection.
          return [
            ...names.values(),
            ...[...unavailable].map((id) => ({
              connectionId: id,
              name: "",
              unavailable: true,
            })),
          ];
        }
        if (page.next_offset <= offset || page.next_offset > 1000)
          throw new Error("Tool metadata exceeds the supported page limit.");
        offset = page.next_offset;
      }
    },
    retry: (failures, error) =>
      failures < 2 &&
      (error instanceof RequestError
        ? [408, 429, 500, 502, 503, 504].includes(error.status)
        : error instanceof TypeError),
    retryDelay: (attempt) => Math.min(500 * 2 ** attempt, 2000),
    refetchInterval: (query) => (query.state.error ? false : pollNormal),
    gcTime: 0,
  });
  const unavailable = tools.data?.some(
    (t) => t.connectionId === connection.id && "unavailable" in t,
  );
  const names =
    enabled && !tools.error && !unavailable
      ? tools.data
          ?.filter(
            (t) => t.connectionId === null || t.connectionId === connection.id,
          )
          .map((t) => t.name)
      : undefined;
  const blocked = brain.archived
    ? "Tools unavailable while this Brain is archived."
    : !connection.enabled
      ? "Connection disabled."
      : connection.availability !== "configured"
        ? "Connection needs attention."
        : "Tool metadata requires Use access.";
  const linkProps = {
    tabIndex: disabled ? -1 : undefined,
    "aria-disabled": disabled || undefined,
    onClick: (event: MouseEvent) => {
      if (disabled) event.preventDefault();
    },
  };
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_minmax(200px,0.45fr)] overflow-hidden rounded-md border border-border">
      <div className="flex min-w-0 gap-3 p-3">
        <McpConnectorIcon definition={definition} size="small" />
        <div className="grid min-w-0 flex-1 gap-1.5">
          <div className="flex min-w-0 flex-wrap items-center gap-1.5">
            <strong className="mr-1 min-w-0 truncate text-label font-semibold text-foreground">
              {connection.name}
            </strong>
            <Badge variant="outline">
              <Layers aria-hidden="true" />
              {scope}
            </Badge>
            {names && (
              <Badge className="mono">
                {names.length} approved tool{names.length === 1 ? "" : "s"}
              </Badge>
            )}
            {!connection.enabled && <Badge>Disabled</Badge>}
          </div>
          {(tools.error || unavailable) && enabled ? (
            <p className={notice} role="alert">
              Tool metadata unavailable.
              <Button
                size="sm"
                variant="ghost"
                disabled={disabled || tools.isFetching}
                onClick={() => void tools.refetch()}
              >
                Retry tools
              </Button>
            </p>
          ) : !enabled ? (
            <p className={notice}>{blocked}</p>
          ) : !names ? (
            <p className={notice} role="status">
              <Spinner size={12} /> Loading approved tools…
            </p>
          ) : !names.length ? (
            <p className={notice}>No approved tools.</p>
          ) : (
            <div className="flex flex-wrap gap-1.5">
              {names.map((name) => (
                <code
                  key={name}
                  className="mono inline-flex h-[22px] items-center rounded-sm border border-border bg-muted px-1.5 text-caption text-foreground-secondary"
                >
                  {name}
                </code>
              ))}
            </div>
          )}
          <Link
            {...linkProps}
            to="/brains/$brainId/connections"
            params={{ brainId: brain.id }}
            search={{ connection: connection.id }}
            className={linkClass}
          >
            Inspect connection
            <ArrowRight aria-hidden="true" />
          </Link>
        </div>
      </div>
      <div className="grid content-start gap-1 border-l border-border bg-muted/50 p-3">
        <h4 className="section-label m-0">Tool groups</h4>
        {groups.length ? (
          groups.map((p) => (
            <Link
              {...linkProps}
              key={p.id}
              to="/brains/$brainId/connections"
              params={{ brainId: brain.id }}
              search={{ tab: "profiles", profile: p.id }}
              className={`${linkClass} min-w-0 py-0.5`}
            >
              <BookOpen aria-hidden="true" />
              <span className="truncate">{p.name}</span>
              {!p.enabled && (
                <small className="text-caption text-muted-foreground">
                  Disabled
                </small>
              )}
              <ArrowRight aria-hidden="true" />
            </Link>
          ))
        ) : (
          <p className={notice}>
            {canConfigure
              ? "No tool groups."
              : "No tool groups visible to you."}
          </p>
        )}
      </div>
    </div>
  );
}
