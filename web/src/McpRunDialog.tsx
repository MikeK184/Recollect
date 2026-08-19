import { useState } from "react";
import {
  Alert,
  Button,
  Code,
  Modal,
  NumberInput,
  Stack,
  Text,
  Textarea,
} from "@mantine/core";
import { client, result, RequestError, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpProfile } from "./McpPanel";

export type McpTool = components["schemas"]["McpDiscoveredTool"];
type Input = components["schemas"]["McpCallInput"];
export function McpRunDialog({
  brain,
  profile,
  tool,
  environment,
  session,
  close,
  submitted,
}: {
  brain: Brain;
  profile: McpProfile;
  tool: McpTool;
  environment: string | null;
  session: string;
  close: () => void;
  submitted: (id: string) => void;
}) {
  const [argumentsText, setArguments] = useState("{}");
  const [timeout, setTimeout] = useState<string | number>(300);
  const [pending, setPending] = useState<Input | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      const args: unknown = JSON.parse(argumentsText);
      if (
        !args ||
        typeof args !== "object" ||
        Array.isArray(args) ||
        new TextEncoder().encode(argumentsText).length > 32768
      )
        throw new Error(
          "Arguments must be a JSON object no larger than 32 KiB.",
        );
      if (
        !Number.isInteger(Number(timeout)) ||
        Number(timeout) < 1 ||
        Number(timeout) > 3600
      )
        throw new Error("Choose a timeout from 1 to 3600 seconds.");
      const input: Input = pending ?? {
        request_id: crypto.randomUUID(),
        profile_id: profile.id,
        connection_id: tool.connection_id,
        tool_name: tool.tool.name,
        arguments: args,
        environment_id: environment,
        operation_id: null,
        client_session_id: session,
        timeout_seconds: Number(timeout),
      };
      setPending(input);
      const call = result(
        await client.POST("/api/brains/{brain}/mcp/calls", {
          params: { path: { brain: brain.id } },
          body: input,
        }),
      );
      submitted(call.id);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not submit this call.");
      // A validation rejection did not admit work. Uncertain delivery retains the
      // frozen request identity so retry cannot create a second provider attempt.
      if (e instanceof RequestError && e.status === 400) setPending(null);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      opened
      onClose={close}
      title={`Run tool · ${tool.tool.name}`}
      size="xl"
      closeOnClickOutside={!busy}
    >
      <Stack>
        <Text>
          {profile.name} · {tool.connection_name}
        </Text>
        <Text size="sm" c="dimmed">
          The selected profile and environment remain fixed for this call. Tools
          may change their target system.
        </Text>
        <details>
          <summary>Input schema</summary>
          <Code block>{JSON.stringify(tool.tool.inputSchema, null, 2)}</Code>
        </details>
        <Textarea
          label="Tool arguments"
          description="Non-secret JSON. Connector credentials are supplied by the selected runner."
          autosize
          minRows={6}
          maxRows={18}
          value={argumentsText}
          onChange={(e) => setArguments(e.currentTarget.value)}
          disabled={busy || !!pending}
        />
        <NumberInput
          label="Call timeout (seconds)"
          min={1}
          max={3600}
          allowDecimal={false}
          value={timeout}
          onChange={setTimeout}
          disabled={busy || !!pending}
        />
        {error && (
          <Alert color="red" title="Call not confirmed">
            {error}
          </Alert>
        )}
        {pending && (
          <Text size="sm">
            Request {pending.request_id}. Retrying checks this same submission;
            it does not repeat a completed tool call.
          </Text>
        )}
        <Button onClick={() => void run()} loading={busy}>
          {pending ? "Check submission" : "Run tool"}
        </Button>
      </Stack>
    </Modal>
  );
}
