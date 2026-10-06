import {
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Stack,
  Title,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "../../api";
import { ErrorState } from "../../components/AsyncState";

export function ManagedMemoryPanel({
  brain,
  compact = false,
}: {
  brain: Brain;
  compact?: boolean;
}) {
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
    retry: false,
    gcTime: 0,
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
    <section className="feature-setting" aria-label="AI permissions summary">
      <Stack gap="md">
        <Group justify="space-between">
          <Title order={3}>
            {compact ? "AI permissions" : "Memory permissions"}
          </Title>
          {policy && (
            <Badge color={policy.enabled ? "brand" : "gray"}>
              {!policy.enabled
                ? "Off"
                : ready
                  ? "Configured"
                  : "Custom permissions"}
            </Badge>
          )}
        </Group>
        {settings.isPending && <Loader size="sm" />}
        <ErrorState error={settings.error} />
        {policy?.enabled &&
          data &&
          !data.models.installed.credentials_present && (
            <Alert color="yellow" title="Provider credential missing">
              An administrator must connect the installed provider before
              permitted model processing can run.
            </Alert>
          )}
        <Button
          component="a"
          href={`/brains/${brain.id}/settings?tab=ai`}
          variant="default"
          w="fit-content"
        >
          Review AI permissions
        </Button>
      </Stack>
    </section>
  );
}
