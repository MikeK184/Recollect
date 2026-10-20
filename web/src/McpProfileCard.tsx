import { Alert, Button } from "./components/ui";
import { cn } from "./lib/utils";
import { McpProfileConnections } from "./McpProfileConnections";
import { McpProfileFooter, McpProfileNotices } from "./McpProfileFooter";
import { McpProfileHeader } from "./McpProfileHeader";
import { McpProfilePeople } from "./McpProfilePeople";
import { useProfileState, type McpProfileCardProps } from "./McpProfileState";

// One card per real tool group: identity header, MCPs and people in two thin
// columns, edited in place with Cancel/Save in the card footer.
export function McpProfileCard(props: McpProfileCardProps) {
  const state = useProfileState(props);
  const { savedId, editing, profile, save, detail } = state;
  return (
    <article
      className={cn(
        "mcp-tool-group-card animate-enter scroll-mt-6 overflow-hidden rounded-lg border bg-card transition-[border-color,box-shadow] duration-base ease-standard",
        editing
          ? "is-editing border-border-strong shadow-sm"
          : "border-border",
      )}
      data-testid="mcp-profile"
      id={`tool-group-${savedId}`}
      aria-label={`Tool group ${profile?.name ?? "new"}`}
    >
      <fieldset
        disabled={save.isPending}
        className="m-0 min-w-0 border-0 p-0"
      >
        <McpProfileHeader state={state} props={props} />
        {detail.error ? (
          <div className="px-4 py-3">
            <Alert
              variant="destructive"
              title="Tool group unavailable"
              actions={
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => void detail.refetch()}
                >
                  Reload tool group
                </Button>
              }
            >
              {detail.error.message}
            </Alert>
          </div>
        ) : (
          <>
            <div className="grid grid-cols-[minmax(0,1fr)_minmax(300px,1fr)] divide-x divide-border">
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
