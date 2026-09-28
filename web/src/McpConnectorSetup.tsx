import { useState } from "react";
import {
  Alert,
  Button,
  FileInput,
  JsonInput,
  Modal,
  Stack,
  Text,
  TextInput,
  SegmentedControl,
} from "@mantine/core";
import { useMutation } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { useIdempotency } from "./useIdempotency";
import { useWorkspace } from "./app/context";
import { ErrorState } from "./components/AsyncState";

export function McpConnectorSetup({
  close,
  saved,
  brain,
}: {
  close: () => void;
  saved: (connected?: boolean) => void;
  brain: Brain;
}) {
  const session = useWorkspace();
  const [manifest, setManifest] = useState("");
  const [fileError, setFileError] = useState<Error | null>(null);
  const [reading, setReading] = useState(false);
  const [mode, setMode] = useState("form");
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const command = useIdempotency();
  const inspection = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/mcp/definitions/inspect-http", {
          body: { name, url },
        }),
      ),
    onSuccess: (value) => setManifest(JSON.stringify(value, null, 2)),
  });
  const edit = () => {
    setManifest("");
    setFileError(null);
    inspection.reset();
    save.reset();
  };
  const save = useMutation({
    mutationFn: async () => {
      let body;
      try {
        body = JSON.parse(manifest);
      } catch {
        throw new Error("Choose a valid JSON connector manifest.");
      }
      const definition = result(
        await client.POST("/api/mcp/definitions", { body }),
      );
      if (mode === "form") {
        const connection = {
          name: name.trim(),
          description: "",
          definition_key: definition.key,
          target: url.trim(),
          placement: "central",
          runner_reference: null,
          credential_alias: null,
          environment_id: null,
          configuration: {},
          enabled: true,
          base_revision: null,
        };
        result(
          await client.POST("/api/brains/{brain}/mcp/connections", {
            params: { path: { brain: brain.id } },
            body: connection,
            headers: { "Idempotency-Key": command.forInput(connection) },
          }),
        );
      }
      return mode === "form";
    },
    onSuccess: saved,
  });
  return (
    <Modal
      opened
      onClose={() => !save.isPending && close()}
      title="Add an MCP connector"
      size="lg"
    >
      <Stack>
        <Text size="sm">
          Add an MCP server by its address. Recollect reads its available tools
          and prepares the connection for this Brain.
        </Text>
        {session.user.installation_owner ? (
          <>
            <SegmentedControl
              value={mode}
              onChange={(v) => {
                edit();
                setMode(v);
              }}
              disabled={save.isPending || inspection.isPending}
              data={[
                { value: "form", label: "Server address" },
                { value: "manifest", label: "Import manifest" },
              ]}
            />
            {mode === "form" ? (
              <>
                <TextInput
                  label="Server name"
                  value={name}
                  onChange={(e) => {
                    edit();
                    setName(e.currentTarget.value);
                  }}
                  disabled={inspection.isPending || save.isPending}
                  required
                />
                <TextInput
                  label="MCP server URL"
                  placeholder="https://example.com/mcp"
                  value={url}
                  onChange={(e) => {
                    edit();
                    setUrl(e.currentTarget.value);
                  }}
                  disabled={inspection.isPending || save.isPending}
                  required
                />
                <Button
                  variant="subtle"
                  disabled={inspection.isPending || save.isPending}
                  onClick={() => {
                    edit();
                    setName("Context7");
                    setUrl("https://mcp.context7.com/mcp");
                  }}
                >
                  Use Context7 · no API key
                </Button>
                <Text size="sm" c="dimmed">
                  This form connects without credentials. Use manifest import
                  for servers that need a configured credential or local runner.
                </Text>
                <Button
                  variant="light"
                  disabled={!name.trim() || !url.trim() || save.isPending}
                  loading={inspection.isPending}
                  onClick={() => inspection.mutate()}
                >
                  Find tools
                </Button>
                {inspection.data && manifest && (
                  <Alert
                    title={`${inspection.data.tools.length} tools found`}
                    color="teal"
                  >
                    {inspection.data.tools.map((tool) => tool.name).join(" · ")}
                    <Text size="sm">Ready to add. No tools have been run.</Text>
                  </Alert>
                )}
              </>
            ) : (
              <>
                <FileInput
                  label="Connector manifest"
                  placeholder="Choose a Recollect connector JSON file"
                  accept="application/json,.json"
                  disabled={save.isPending || reading}
                  onChange={async (file) => {
                    if (!file) return;
                    setFileError(null);
                    if (file.size > 524288) {
                      setFileError(
                        new Error(
                          "Connector manifests must be 512 KiB or smaller.",
                        ),
                      );
                      return;
                    }
                    setReading(true);
                    try {
                      setManifest(await file.text());
                    } catch {
                      setFileError(
                        new Error("The connector file could not be read."),
                      );
                    } finally {
                      setReading(false);
                    }
                  }}
                />
                <details className="feature-advanced">
                  <summary>Paste or inspect connector JSON</summary>
                  <JsonInput
                    label="Connector JSON"
                    value={manifest}
                    onChange={setManifest}
                    minRows={8}
                    maxRows={18}
                    autosize
                    formatOnBlur
                    validationError="Invalid JSON"
                  />
                </details>
              </>
            )}
            <Text size="sm">
              As installation owner, you are approving this connector for
              configuration. Registration does not download or start software.
              Existing connector keys cannot be replaced here.
            </Text>
            <ErrorState error={fileError ?? inspection.error ?? save.error} />
            <Button
              disabled={!manifest.trim() || reading || !!fileError}
              loading={save.isPending}
              onClick={() => save.mutate()}
            >
              {mode === "form" ? "Add server" : "Register connector"}
            </Button>
          </>
        ) : (
          <Alert color="gray" title="Installation setup required">
            Ask the installation owner to register this MCP connector here.
            Operators can also use{" "}
            <code>recollect-server mcp-definition-import PATH</code> with its
            approved manifest. You can configure its connection once it is
            registered.
          </Alert>
        )}
      </Stack>
    </Modal>
  );
}
