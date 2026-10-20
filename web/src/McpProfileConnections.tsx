import { ArrowRight, X } from "lucide-react";
import { Alert, Button, Select } from "./components/ui";
import { McpConnectorIcon } from "./components/McpConnectorIcon";
import type { McpProfileCardProps, ProfileState } from "./McpProfileState";

// The MCPs a tool group contains, with the picker and the environment check.
export function McpProfileConnections({
  state,
  props,
}: {
  state: ProfileState;
  props: McpProfileCardProps;
}) {
  const { brain, catalogue, tools } = props;
  const {
    editing,
    canManage,
    connectionIds,
    input,
    setInput,
    save,
    compatible,
    profile,
  } = state;
  return (
    <section aria-label="MCP connections" className="min-w-0 px-4 py-3">
      {editing && (
        <div className="flex min-h-7 items-center justify-between gap-2">
          <h3 className="section-label m-0">MCPs</h3>
          {canManage && (
            <span className="mono text-caption text-muted-foreground">
              {connectionIds.length}/20
            </span>
          )}
        </div>
      )}
      <ul className="m-0 list-none p-0">
        {connectionIds.map((connectionId) => {
          const connection = catalogue.connections.find(
            (c) => c.id === connectionId,
          );
          const definition = catalogue.definitions.find(
            (d) => d.key === connection?.definition_key,
          );
          return (
            <li
              className="flex min-h-12 items-center gap-3 border-b border-border py-2 last:border-b-0"
              key={connectionId}
            >
              <McpConnectorIcon definition={definition} size="small" />
              <div className="min-w-0 flex-1">
                <strong className="block truncate text-label font-medium text-foreground">
                  {connection?.name ?? "Unavailable connection"}
                </strong>
                <small className="mono block text-caption text-muted-foreground">
                  {definition?.tool_count ?? "—"} tools
                </small>
              </div>
              {editing && canManage && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="text-muted-foreground hover:bg-destructive-soft hover:text-destructive-text"
                  title={`Remove ${connection?.name ?? "connection"}`}
                  aria-label={`Remove ${connection?.name ?? "connection"}`}
                  disabled={save.isPending}
                  onClick={() =>
                    setInput({
                      ...input,
                      connection_ids: connectionIds.filter(
                        (c) => c !== connectionId,
                      ),
                    })
                  }
                >
                  <X aria-hidden="true" />
                </Button>
              )}
            </li>
          );
        })}
      </ul>
      {!connectionIds.length && (
        <p className="m-0 py-2 text-label text-muted-foreground">
          No MCPs added.
        </p>
      )}
      {editing && canManage && (
        <Select
          className="mt-3"
          size="sm"
          label="Add MCP"
          searchable
          clearable
          placeholder="Choose a connection"
          value={null}
          disabled={
            input.connection_ids.length >= 20 ||
            save.isPending ||
            brain.archived
          }
          data={catalogue.connections
            .filter((c) => !input.connection_ids.includes(c.id))
            .map((c) => ({
              value: c.id,
              label: c.name,
              disabled: !compatible.some((p) => p.id === c.id),
            }))}
          onChange={(value) =>
            value &&
            setInput({
              ...input,
              connection_ids: [...input.connection_ids, value],
            })
          }
        />
      )}
      {editing &&
        input.connection_ids.some(
          (c) => !compatible.some((p) => p.id === c),
        ) && (
          <Alert variant="warning" className="mt-3">
            Remove MCPs from other environments before saving.
          </Alert>
        )}
      {!editing && (
        <Button
          variant="ghost"
          size="sm"
          className="-ml-2.5 mt-2"
          disabled={
            !profile?.rights.use_profile || !profile.enabled || brain.archived
          }
          onClick={() => profile && tools(profile.id)}
          aria-label="Tools & testing"
        >
          Tools & testing
          <ArrowRight aria-hidden="true" />
        </Button>
      )}
    </section>
  );
}
