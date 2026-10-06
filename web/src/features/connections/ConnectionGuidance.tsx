import { Card, Group, SimpleGrid, Stack, Text, Title } from "@mantine/core";
import { Link } from "@tanstack/react-router";
import { Globe, Laptop, Network } from "lucide-react";
import { iconSize } from "../../design/tokens";

// Plain-language explanation of the concept the catalogue stores as `placement`
// (docs/contracts/mcp-catalogue-and-profiles.md) and the private-runner
// registration (docs/contracts/mcp-vault-and-private-runners.md). Wording only:
// no grant, trust or runtime rule is restated as an affordance here.

function Heading({ title, children }: { title: string; children: string }) {
  return (
    <Stack gap="xs">
      <Title order={4}>{title}</Title>
      <Text size="sm" c="dimmed">
        {children}
      </Text>
    </Stack>
  );
}

/** Where an approved outbound connection executes, and what that decides. */
export function WhereItRuns({ brainId }: { brainId: string }) {
  const places = [
    {
      key: "central",
      icon: Globe,
      name: "Central service",
      body: "The Recollect service runs the tool in its own environment. Only the networks that service can reach are available to this connection. No device of yours is involved.",
    },
    {
      key: "local",
      icon: Laptop,
      name: "Paired device",
      body: "One of your paired devices runs the tool through its plugin runner, so the target can be reachable from that machine's network. The device must be online and approved; an offline device applies nothing.",
    },
    {
      key: "private",
      icon: Network,
      name: "Private-network runner",
      body: "A runner registered for this Brain on one paired device. Use it for targets on a network the service cannot reach. Register it on the Private Runners tab; a registration is metadata, not proof of a live connection.",
    },
  ];
  return (
    <details className="feature-details" aria-label="Where a connection runs">
      <summary>Where a connection runs</summary>
      <Stack gap="md" mb="lg">
        <Heading title="Where a connection runs">
          Every outbound connection you approve executes in exactly one of these
          places. The choice decides which network can reach the target, and
          Recollect never falls back to another place when the chosen one is
          unreachable.
        </Heading>
        <SimpleGrid cols={{ base: 1, sm: 3 }} spacing="md">
          {places.map((place) => (
            <Card
              withBorder
              p="md"
              key={place.key}
              data-testid={`placement-${place.key}`}
            >
              <Stack gap="xs">
                <Group gap="xs">
                  <place.icon size={iconSize.small} aria-hidden />
                  <Text fw={600}>{place.name}</Text>
                </Group>
                <Text size="sm" c="dimmed">
                  {place.body}
                </Text>
              </Stack>
            </Card>
          ))}
        </SimpleGrid>
        <Text size="xs" c="dimmed">
          Device identity stays global: your agents are listed and managed in{" "}
          <Link to="/brains/$brainId/agents" params={{ brainId }}>
            this Brain&apos;s Agents list
          </Link>
          . A saved connection reads as configured until a real tool call
          succeeds under an explicit Use grant.
        </Text>
      </Stack>
    </details>
  );
}

/** Tool groups and the three independent grants, stated once. */
export function ToolGroupGrants() {
  const grants = [
    {
      name: "Use",
      body: "Run this group's tools, under your own current Brain access. Creating a group and Brain administration never grant it.",
    },
    {
      name: "Manage",
      body: "Change the group's name, environment, enabled state and which connections it holds — never a connection's target, executable, credential reference or runner.",
    },
    {
      name: "Share",
      body: "Assign or remove these three rights for enrolled accounts and organization groups, including an explicit self-use grant.",
    },
  ];
  return (
    <section aria-label="Tool group grants">
      <Stack gap="md" mb="lg">
        <Heading title="Tool groups and their three grants">
          A tool group is the named set of MCP connections an agent may be
          allowed to use together. Use, Manage and Share are independent rights:
          none implies another, and current Brain access is always a
          prerequisite.
        </Heading>
        <SimpleGrid cols={{ base: 1, sm: 3 }} spacing="md">
          {grants.map((grant) => (
            <Card
              withBorder
              p="md"
              key={grant.name}
              data-testid={`grant-${grant.name}`}
            >
              <Stack gap="xs">
                <Text fw={600}>{grant.name}</Text>
                <Text size="sm" c="dimmed">
                  {grant.body}
                </Text>
              </Stack>
            </Card>
          ))}
        </SimpleGrid>
      </Stack>
    </section>
  );
}
