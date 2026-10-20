import { Alert, Button } from "./components/ui";
import {
  profileInput,
  type McpProfileCardProps,
  type ProfileState,
} from "./McpProfileState";

// The stale-configuration and partial-save notices under the sections.
export function McpProfileNotices({ state }: { state: ProfileState }) {
  const { stale, save, detail, baseline, setInput, progress } = state;
  if (!stale && !save.error) return null;
  return (
    <div className="grid gap-2 px-4 pb-3">
      {stale && (
        <Alert
          variant="warning"
          title="Configuration changed"
          actions={
            <Button
              variant="secondary"
              size="sm"
              disabled={save.isPending}
              onClick={() => {
                if (detail.data) {
                  baseline.current = profileInput(detail.data.profile);
                  setInput(baseline.current);
                  save.reset();
                }
              }}
            >
              Reload settings
            </Button>
          }
        >
          Your draft is retained. Reload replaces the draft with current
          settings.
        </Alert>
      )}
      {save.error && (
        <Alert
          variant="destructive"
          title={progress.length ? "Some changes saved" : "Changes not saved"}
        >
          {progress.length > 0 && (
            <ul className="m-0 mb-1 grid gap-0.5 pl-4">
              {progress.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          )}
          {save.error.message} Remaining edits are retained.
        </Alert>
      )}
    </div>
  );
}

// The cancel and save controls shown while editing.
export function McpProfileFooter({
  state,
  props,
}: {
  state: ProfileState;
  props: McpProfileCardProps;
}) {
  const { brain } = props;
  const {
    editing,
    save,
    cancel,
    detail,
    canManage,
    canShare,
    input,
    compatible,
    progress,
  } = state;
  if (!editing) return null;
  return (
    <footer className="flex items-center justify-end gap-2 border-t border-border bg-muted px-4 py-2.5">
      <Button
        variant="secondary"
        size="sm"
        disabled={save.isPending}
        onClick={cancel}
      >
        Cancel
      </Button>
      <Button
        size="sm"
        loading={save.isPending}
        disabled={
          brain.archived ||
          !!detail.error ||
          (!canManage && !canShare) ||
          (canManage &&
            (!input.name.trim() ||
              input.connection_ids.some(
                (c) => !compatible.some((p) => p.id === c),
              )))
        }
        onClick={() => save.mutate()}
      >
        {save.error && progress.length
          ? "Retry remaining changes"
          : "Save changes"}
      </Button>
    </footer>
  );
}
