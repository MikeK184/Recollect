import { useEffect, useState } from "react";
import { Button, Group, Modal, Stack, Text } from "@mantine/core";
import { useQuery, useMutation } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpCatalogue } from "./McpPanel";
import { ErrorState, LoadingState } from "./components/AsyncState";
type Detail = components["schemas"]["McpConnectionDetail"];
type Props = {
  brain: Brain;
  id: string;
  catalogue: McpCatalogue;
  close: () => void;
  saved: () => void;
  stale: (message: string) => void;
};
export function ConnectionPowerDialog(props: Props) {
  const [pending, setPending] = useState(false);
  const query = useQuery({
    queryKey: ["mcp", props.brain.id, "connection", props.id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/connections/{id}", {
          params: { path: { brain: props.brain.id, id: props.id } },
          signal,
        }),
      ),
    retry: false,
    gcTime: 0,
    refetchInterval: 5000,
  });
  const summary = props.catalogue.connections.find((c) => c.id === props.id);
  return (
    <Modal
      opened
      onClose={() => !pending && props.close()}
      closeOnEscape={!pending}
      closeOnClickOutside={!pending}
      title={
        summary?.enabled ? "Pause connection use" : "Enable connection use"
      }
      size="md"
      centered
    >
      {query.error ? (
        <ErrorState error={query.error} />
      ) : !query.data ? (
        <LoadingState />
      ) : (
        <PowerForm
          {...props}
          detail={query.data}
          revision={summary?.revision}
          onPending={setPending}
        />
      )}
    </Modal>
  );
}
function PowerForm({
  detail,
  revision,
  onPending,
  ...props
}: Props & {
  detail: Detail;
  revision?: string;
  onPending: (pending: boolean) => void;
}) {
  // Freeze the prepared operation, including every field and its original revision.
  const [original] = useState(detail);
  const [enabled] = useState(!detail.summary.enabled);
  const stale =
    original.summary.revision !== detail.summary.revision ||
    original.summary.revision !== revision;
  useEffect(() => {
    if (stale)
      props.stale("This connection changed. Reopen it before changing use.");
  }, [stale, props.stale]);
  const save = useMutation({
    mutationFn: async () =>
      result(
        await client.PUT("/api/brains/{brain}/mcp/connections/{id}", {
          params: { path: { brain: props.brain.id, id: props.id } },
          body: {
            name: original.summary.name,
            description: original.summary.description,
            definition_key: original.summary.definition_key,
            target: original.target,
            placement: original.summary.placement,
            runner_reference: original.runner_reference,
            credential_alias: original.credential_alias,
            environment_id: original.summary.environment_id,
            configuration: original.configuration,
            enabled,
            base_revision: original.summary.revision,
          },
        }),
      ),
    onSuccess: props.saved,
  });
  useEffect(() => {
    onPending(save.isPending);
    return () => onPending(false);
  }, [save.isPending, onPending]);
  return (
    <Stack gap="lg">
      <Text fw={600}>{original.summary.name}</Text>
      <Text size="sm">
        {enabled
          ? "Allows future authorized calls. The session opens on demand; this does not start or test the server."
          : "Blocks future calls through this connection. Already dispatched calls may finish. A remote server stays running independently."}
      </Text>
      <Text size="xs" c="dimmed">
        Applies to all tool groups using this connection in this Brain.
      </Text>
      <ErrorState error={save.error} />
      <Group justify="flex-end">
        <Button
          variant="default"
          disabled={save.isPending}
          onClick={props.close}
        >
          Cancel
        </Button>
        <Button
          disabled={
            stale || props.brain.archived || !props.catalogue.can_configure
          }
          loading={save.isPending}
          onClick={() => save.mutate()}
        >
          {enabled ? "Enable use" : "Pause use"}
        </Button>
      </Group>
    </Stack>
  );
}
