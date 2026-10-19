import { useState } from "react";
import { Alert, Button, Loader, Modal } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { client, result } from "./api";
import "./features/feature-views.css";
import "./features/connections/inline-management.css";
import { usePollInterval } from "./usePollInterval";
import {
  placementText,
  useConnectionState,
  type Detail,
  type Props,
} from "./McpConnectionState";
import { InlineConnectionForm } from "./McpConnectionInline";
import { StandardConnectionForm } from "./McpConnectionStandard";

export { placementText };

export function McpConnectionDialog(props: Props) {
  const pollNormal = usePollInterval("normal");
  const [formRevision, setFormRevision] = useState(0);
  const query = useQuery({
    queryKey: ["mcp", props.brain.id, "connection", props.id],
    enabled: props.id !== "new",
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/connections/{id}", {
          params: { path: { brain: props.brain.id, id: props.id } },
          signal,
        }),
      ),
    refetchInterval: pollNormal,
    retry: false,
    gcTime: 0,
  });
  const summary = props.catalogue.connections.find((c) => c.id === props.id);
  const contents = query.error ? (
    <Alert color="red">
      {query.error.message}
      <Button size="xs" onClick={() => void query.refetch()}>
        Reload connection
      </Button>
    </Alert>
  ) : props.id !== "new" && !query.data ? (
    <Loader />
  ) : (
    <ConnectionForm
      {...props}
      key={formRevision}
      detail={query.data}
      currentRevision={summary?.revision}
      reload={() =>
        void query.refetch().then((response) => {
          if (!response.error) setFormRevision((v) => v + 1);
        })
      }
    />
  );
  return props.inline ? (
    contents
  ) : (
    <Modal
      opened
      onClose={props.close}
      title={props.id === "new" ? "Add connection" : "Edit connection"}
      size={800}
      className="connection-setup-modal"
    >
      {contents}
    </Modal>
  );
}

function ConnectionForm({
  detail,
  currentRevision,
  reload,
  ...props
}: Props & { detail?: Detail; currentRevision?: string; reload: () => void }) {
  const form = useConnectionState(props, detail, currentRevision);
  if (props.inline)
    return <InlineConnectionForm form={form} props={props} reload={reload} />;
  return <StandardConnectionForm form={form} props={props} detail={detail} />;
}
