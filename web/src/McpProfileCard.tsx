import { Alert, Button } from "@mantine/core";
import "./features/connections/inline-management.css";
import { McpProfileConnections } from "./McpProfileConnections";
import { McpProfileFooter, McpProfileNotices } from "./McpProfileFooter";
import { McpProfileHeader } from "./McpProfileHeader";
import { McpProfilePeople } from "./McpProfilePeople";
import { useProfileState, type McpProfileCardProps } from "./McpProfileState";

export function McpProfileCard(props: McpProfileCardProps) {
  const state = useProfileState(props);
  const { savedId, editing, profile, save, detail } = state;
  return (
    <article
      className={`mcp-tool-group-card rc-enter${editing ? " is-editing" : ""}`}
      data-testid="mcp-profile"
      id={`tool-group-${savedId}`}
      aria-label={`Tool group ${profile?.name ?? "new"}`}
    >
      <fieldset disabled={save.isPending} className="mcp-inline-fields">
        <McpProfileHeader state={state} props={props} />
        {detail.error ? (
          <Alert color="red" title="Tool group unavailable">
            {detail.error.message}
            <Button
              variant="subtle"
              size="xs"
              onClick={() => void detail.refetch()}
            >
              Reload tool group
            </Button>
          </Alert>
        ) : (
          <>
            <div className="mcp-tool-group-sections">
              <McpProfileConnections state={state} props={props} />
              <McpProfilePeople state={state} props={props} />
            </div>
            <McpProfileNotices state={state} />
          </>
        )}
        {editing && <McpProfileFooter state={state} props={props} />}
      </fieldset>
    </article>
  );
}
