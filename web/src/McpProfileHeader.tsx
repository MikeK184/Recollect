import { Badge, Button, Select, Switch, TextInput } from "@mantine/core";
import { BookOpen, Pencil, Plus } from "lucide-react";
import type { McpProfileCardProps, ProfileState } from "./McpProfileState";

// The card heading (name, environment, enabled state, edit entry) and the
// description line beneath it.
export function McpProfileHeader({
  state,
  props,
}: {
  state: ProfileState;
  props: McpProfileCardProps;
}) {
  const { brain, environments } = props;
  const {
    editing,
    canManage,
    canShare,
    input,
    setInput,
    name,
    save,
    profile,
    detail,
    edit,
    connectionIds,
    description,
    showDescription,
    setShowDescription,
  } = state;
  return (
    <>
      <header className="mcp-tool-group-heading">
        <div className="management-icon connection-icon">
          <BookOpen aria-hidden="true" />
        </div>
        <div className="mcp-tool-group-identity">
          {editing && canManage ? (
            <TextInput
              size="xs"
              aria-label="Tool group name"
              placeholder="Tool group name"
              required
              maxLength={120}
              className="mcp-inline-title"
              value={name}
              onChange={(e) =>
                setInput({ ...input, name: e.currentTarget.value })
              }
            />
          ) : (
            <h2>{name}</h2>
          )}
          <div className="mcp-tool-group-meta">
            {editing && canManage && environments.length > 0 ? (
              <Select
                className="mcp-inline-environment"
                aria-label="Tool group environment"
                size="xs"
                clearable
                disabled={save.isPending || brain.archived}
                placeholder="Brain-wide"
                value={input.environment_id}
                data={environments}
                onChange={(value) =>
                  setInput({ ...input, environment_id: value })
                }
              />
            ) : (
              <Badge
                className="mcp-environment-badge"
                variant="light"
                color="brand"
              >
                {environments.find(
                  (e) =>
                    e.value ===
                    (editing ? input.environment_id : profile?.environment_id),
                )?.label ?? "Brain-wide"}
              </Badge>
            )}
            <Badge
              className="mcp-connection-count"
              variant="light"
              color="gray"
            >
              {connectionIds.length} MCP
              {connectionIds.length === 1 ? "" : "s"}
            </Badge>
          </div>
        </div>
        {editing && canManage ? (
          <Switch
            size="xs"
            label="Enabled"
            checked={input.enabled}
            disabled={brain.archived || save.isPending}
            onChange={(e) =>
              setInput({ ...input, enabled: e.currentTarget.checked })
            }
          />
        ) : (
          <Badge variant="light" color={profile?.enabled ? "brand" : "gray"}>
            {profile?.enabled ? "Enabled" : "Disabled"}
          </Badge>
        )}
        {!editing && (canManage || canShare) && (
          <Button
            variant="subtle"
            size="compact-sm"
            leftSection={<Pencil size={15} />}
            aria-label={`Edit ${profile?.name ?? "tool group"}`}
            disabled={brain.archived || !detail.data || !!detail.error}
            onClick={edit}
          >
            Edit
          </Button>
        )}
      </header>
      {editing && canManage && (showDescription || description) ? (
        <TextInput
          size="xs"
          className="mcp-tool-group-description"
          aria-label="Tool group description"
          placeholder="Description (optional)"
          value={input.description}
          maxLength={2000}
          onChange={(e) =>
            setInput({ ...input, description: e.currentTarget.value })
          }
        />
      ) : editing && canManage ? (
        <Button
          className="mcp-add-description"
          variant="subtle"
          size="compact-xs"
          leftSection={<Plus size={13} />}
          onClick={() => setShowDescription(true)}
        >
          Add description
        </Button>
      ) : description ? (
        <p className="mcp-tool-group-description">{description}</p>
      ) : null}
    </>
  );
}
