import { BookOpen, Layers, Pencil, Plus } from "lucide-react";
import {
  Badge,
  Button,
  Input,
  Select,
  StatusBadge,
  Switch,
  Tile,
} from "./components/ui";
import type { McpProfileCardProps, ProfileState } from "./McpProfileState";

// The card heading (name, environment, enabled state, edit entry) and the
// description line beneath it. The environment editor replaces its badge in
// the same metadata row; it never moves into the Enabled/action area.
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
      <header className="flex items-start gap-3 border-b border-border px-4 py-3.5">
        <Tile tone="muted" className="mt-0.5">
          <BookOpen aria-hidden="true" />
        </Tile>
        <div className="min-w-0 flex-1">
          {editing && canManage ? (
            <Input
              size="sm"
              aria-label="Tool group name"
              placeholder="Tool group name"
              required
              maxLength={120}
              className="max-w-[320px] font-medium"
              value={name}
              onChange={(e) =>
                setInput({ ...input, name: e.currentTarget.value })
              }
            />
          ) : (
            <h2 className="m-0 truncate text-title font-semibold text-foreground">
              {name}
            </h2>
          )}
          <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
            {editing && canManage && environments.length > 0 ? (
              <div className="w-[220px]">
                <Select
                  aria-label="Tool group environment"
                  size="sm"
                  clearable
                  disabled={save.isPending || brain.archived}
                  placeholder="Brain-wide"
                  value={input.environment_id}
                  data={environments}
                  onChange={(value) =>
                    setInput({ ...input, environment_id: value })
                  }
                />
              </div>
            ) : (
              <Badge variant="outline" className="max-w-[220px]">
                <Layers aria-hidden="true" />
                <span className="truncate">
                  {environments.find(
                    (e) =>
                      e.value ===
                      (editing
                        ? input.environment_id
                        : profile?.environment_id),
                  )?.label ?? "Brain-wide"}
                </span>
              </Badge>
            )}
            <Badge variant="default" className="mono">
              {connectionIds.length} MCP
              {connectionIds.length === 1 ? "" : "s"}
            </Badge>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2 pt-0.5">
          {editing && canManage ? (
            <Switch
              label="Enabled"
              checked={input.enabled}
              disabled={brain.archived || save.isPending}
              onChange={(e) =>
                setInput({ ...input, enabled: e.currentTarget.checked })
              }
            />
          ) : (
            <StatusBadge state={profile?.enabled ? "positive" : "neutral"}>
              {profile?.enabled ? "Enabled" : "Disabled"}
            </StatusBadge>
          )}
          {!editing && (canManage || canShare) && (
            <Button
              variant="ghost"
              size="sm"
              aria-label={`Edit ${profile?.name ?? "tool group"}`}
              disabled={brain.archived || !detail.data || !!detail.error}
              onClick={edit}
            >
              <Pencil aria-hidden="true" />
              Edit
            </Button>
          )}
        </div>
      </header>
      {editing && canManage && (showDescription || description) ? (
        <div className="px-4 pt-3">
          <Input
            size="sm"
            aria-label="Tool group description"
            placeholder="Description (optional)"
            value={input.description}
            maxLength={2000}
            onChange={(e) =>
              setInput({ ...input, description: e.currentTarget.value })
            }
          />
        </div>
      ) : editing && canManage ? (
        <div className="px-3 pt-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setShowDescription(true)}
          >
            <Plus aria-hidden="true" />
            Add description
          </Button>
        </div>
      ) : description ? (
        <p className="m-0 px-4 pt-3 text-label text-muted-foreground">
          {description}
        </p>
      ) : null}
    </>
  );
}
