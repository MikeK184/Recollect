import { Alert, Button, Group } from "@mantine/core";
import {
  profileInput,
  type McpProfileCardProps,
  type ProfileState,
} from "./McpProfileState";

// The stale-configuration and partial-save notices under the sections.
export function McpProfileNotices({ state }: { state: ProfileState }) {
  const { stale, save, detail, baseline, setInput, progress } = state;
  return (
    <>
      {stale && (
        <Alert color="yellow" title="Configuration changed">
          Your draft is retained. Reload replaces the draft with current
          settings.
          <Button
            variant="subtle"
            size="xs"
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
        </Alert>
      )}
      {save.error && (
        <Alert
          color="red"
          title={progress.length ? "Some changes saved" : "Changes not saved"}
        >
          {progress.length > 0 && (
            <ul>
              {progress.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          )}
          {save.error.message} Remaining edits are retained.
        </Alert>
      )}
    </>
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
  return (
    <footer className="mcp-tool-group-footer">
      {editing ? (
        <Group gap="xs">
          <Button
            variant="default"
            size="xs"
            disabled={save.isPending}
            onClick={cancel}
          >
            Cancel
          </Button>
          <Button
            size="xs"
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
        </Group>
      ) : null}
    </footer>
  );
}
