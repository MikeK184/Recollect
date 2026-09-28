import {
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "../../api";
import { useIdempotency } from "../../useIdempotency";
import { ErrorState } from "../../components/AsyncState";

export function ManagedMemoryPanel({
  brain,
  compact = false,
}: {
  brain: Brain;
  compact?: boolean;
}) {
  const cache = useQueryClient();
  const command = useIdempotency();
  const settings = useQuery({
    queryKey: ["automation", brain.id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/automation", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
  });
  const activate = useMutation({
    mutationFn: async () => {
      if (!settings.data || settings.isError)
        throw new Error("Reload Brain settings before continuing.");
      const body = {
        model_change: settings.data.models.current.change_id,
        capture_change: settings.data.capture.change_id,
      };
      return result(
        await client.PUT("/api/brains/{brain}/automation", {
          params: { path: { brain: brain.id } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: async () => {
      command.reset();
      await cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain.id),
      });
    },
  });
  const data = settings.isError ? undefined : settings.data;
  const policy = data?.models.current.policy;
  const ready =
    !!policy?.enabled &&
    policy.autonomous_memory &&
    policy.automatic_embedding &&
    ["extraction", "synthesis", "embedding", "answering"].every((p) =>
      policy.purposes.includes(p),
    ) &&
    [
      "document",
      "raw_session",
      "tool_output",
      "support_excerpt",
      "repository",
      "claim",
      "query",
    ].every((c) => policy.content_classes.includes(c)) &&
    data?.capture.policy.enabled &&
    data.capture.policy.managed_tools;
  return (
    <section className="feature-setting" aria-label="Autonomous memory setup">
      <Stack gap="md">
        <Group justify="space-between">
          <Title order={3}>
            {compact ? "Set up autonomous memory" : "Autonomous memory"}
          </Title>
          {data && (
            <Badge color={ready ? "brand" : "gray"}>
              {ready ? "Configured" : "Setup needed"}
            </Badge>
          )}
        </Group>
        <Text size="sm">
          Your agents contribute evidence. Recollect learns from it, updates
          memory when evidence changes, builds semantic search and removes
          expired material automatically.
        </Text>
        {settings.isPending && <Loader size="sm" />}
        <ErrorState error={settings.error ?? activate.error} />
        {data && !data.models.installed.credentials_present && (
          <Alert color="yellow" title="Model provider is not connected">
            The installation needs a provider credential before learning or Ask
            can run. Capture and evidence search remain available.
          </Alert>
        )}
        {data && !ready && (
          <>
            <Text size="sm">
              One setup enables Ask, learning and supported agent capture for
              this Brain. Its retained documents, repository evidence, memories,
              questions and sanitized sessions/tool results may be processed by
              the installed {data.models.installed.provider} models. Resource
              limits are managed for you.
            </Text>
            {brain.role === "admin" && !brain.archived ? (
              <Button
                w="fit-content"
                loading={activate.isPending}
                onClick={() => activate.mutate()}
              >
                Enable autonomous memory
              </Button>
            ) : (
              <Text size="sm" c="dimmed">
                {brain.archived
                  ? "Reopen this Brain to resume setup."
                  : "A Brain administrator can enable autonomous memory once."}
              </Text>
            )}
          </>
        )}
        {ready && (
          <Text size="sm">
            Ask, source learning, semantic indexing and supported capture are
            enabled. Background work follows this Brain’s evidence and retention
            rules; individual memories do not need approval.
          </Text>
        )}
        {!compact && data && (
          <>
            <Text size="sm" c="dimmed">
              Raw sessions expire after {data.retention.policy.raw_session_days}{" "}
              days and tool output after{" "}
              {data.retention.policy.tool_output_days} days. Useful supported
              knowledge follows its own lifecycle. Existing exclusions and
              retention choices are preserved.
            </Text>
            <Group>
              <Button
                component="a"
                href={`/brains/${brain.id}/connections`}
                variant="light"
              >
                Connect your agent
              </Button>
              <Button
                component="a"
                href={`/brains/${brain.id}/activity`}
                variant="subtle"
              >
                View processing activity
              </Button>
            </Group>
            <Text size="xs" c="dimmed">
              Configured permissions are shown here. Agent coverage and
              successful processing appear in Agents and Activity.
            </Text>
          </>
        )}
      </Stack>
    </section>
  );
}
