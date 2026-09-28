import { Card, SimpleGrid, Stack, Text, Title } from "@mantine/core";
import { useBrain } from "../../app/context";
import { McpAgentSetup } from "../../McpAgentSetup";

export function AgentConnections() {
  const brain = useBrain();
  return (
    <Stack gap="lg">
      <Text c="dimmed" size="sm">
        Connect your coding agent over MCP. It can recall useful context and
        contribute evidence as you work.
      </Text>
      <SimpleGrid cols={{ base: 1, sm: 2 }}>
        {[
          { host: "codex", name: "Codex" },
          { host: "claude", name: "Claude Code" },
        ].map(({ host, name }) => (
          <Card withBorder p="xl" key={host}>
            <Stack gap="md" align="flex-start">
              <Title order={3}>{name}</Title>
              <Text size="sm" c="dimmed">
                Persistent memory through a standard MCP connection, with
                evidence-backed recall.
              </Text>
              <McpAgentSetup
                brain={brain}
                initialHost={host}
                buttonLabel={`Connect ${name}`}
              />
            </Stack>
          </Card>
        ))}
      </SimpleGrid>
      <Text size="xs" c="dimmed">
        Direct MCP needs no companion. Automatic session capture is an optional
        local integration, available in the setup details.
      </Text>
    </Stack>
  );
}
