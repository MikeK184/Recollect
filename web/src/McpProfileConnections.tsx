import { Alert, Button, Select } from "@mantine/core";
import { X } from "lucide-react";
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
    <section aria-label="MCP connections">
      {editing && (
        <div className="mcp-card-section-heading">
          <h3>MCPs</h3>
          {editing && canManage && (
            <span className="management-muted">{connectionIds.length}/20</span>
          )}
        </div>
      )}
      {connectionIds.map((connectionId) => {
        const connection = catalogue.connections.find(
          (c) => c.id === connectionId,
        );
        const definition = catalogue.definitions.find(
          (d) => d.key === connection?.definition_key,
        );
        return (
          <div className="mcp-card-connection" key={connectionId}>
            <McpConnectorIcon definition={definition} size="small" />
            <div>
              <strong>{connection?.name ?? "Unavailable connection"}</strong>
              <small>{definition?.tool_count ?? "—"} tools</small>
            </div>
            {editing && canManage && (
              <button
                type="button"
                className="mcp-inline-remove"
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
                <X size={16} />
              </button>
            )}
          </div>
        );
      })}
      {!connectionIds.length && (
        <p className="management-muted">No MCPs added.</p>
      )}
      {editing && canManage && (
        <Select
          size="xs"
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
          <Alert color="yellow">
            Remove MCPs from other environments before saving.
          </Alert>
        )}
      {!editing && (
        <Button
          variant="subtle"
          className="mcp-card-tools-action"
          size="compact-sm"
          disabled={
            !profile?.rights.use_profile || !profile.enabled || brain.archived
          }
          onClick={() => profile && tools(profile.id)}
          aria-label="Tools & testing"
        >
          Tools & testing →
        </Button>
      )}
    </section>
  );
}
