import { useState } from "react";
import { createRoot } from "react-dom/client";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Drawer,
  Group,
  MantineProvider,
  Select,
  Stack,
  Table,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { ArrowRight, FileText, SlidersHorizontal } from "lucide-react";
import { theme, cssVariablesResolver } from "./theme";
import { tokens } from "./tokens";
import { Brand } from "../components/Brand";
import { PageHeader } from "../components/PageHeader";
import { StatusBadge } from "../components/StatusBadge";
import { EmptyState } from "../components/AsyncState";
import { SettingsSection } from "../components/SettingsSection";
import "@mantine/core/styles.css";
import "./typography.css";
import "../styles.css";
import "./preview.css";

// Vite serves this entry only during development; it is not a production input.
// All content is static specimen data. No API/session/model request is made.
function Preview() {
  const [opened, setOpened] = useState(false);
  return (
    <main className="design-preview">
      <Brand />
      <PageHeader
        title="The Recollect interface"
        eyebrow="Component reference"
        description="Static examples of the shared light theme. Check changes here before changing feature views."
      />
      <div className="token-grid">
        {Object.entries(tokens).map(([name, value]) => (
          <div className="token-swatch" key={name}>
            <span style={{ background: value }} />
            <Text size="xs">{name}</Text>
            <Text className="meta">{value}</Text>
          </div>
        ))}
      </div>
      <SettingsSection
        title="Typography"
        description="Newsreader for page and section titles. Manrope for controls and reading. DM Mono for compact metadata."
      >
        <Stack>
          <Title order={1}>What would you like to remember?</Title>
          <Title order={2}>Knowledge, with its evidence.</Title>
          <Title order={3}>A consistent set of controls</Title>
          <Text>
            Use clear labels and a comfortable reading width. Decisions, sources
            and recorded context share the same visual language.
          </Text>
          <Text className="meta">RECORDED · 26 SEPTEMBER · EXAMPLE</Text>
        </Stack>
      </SettingsSection>
      <SettingsSection
        title="Actions & status"
        description="One primary action; explicit labels and icons for meaning beyond color."
      >
        <Stack>
          <Group>
            <Button rightSection={<ArrowRight size={16} />}>
              Primary action
            </Button>
            <Button variant="default">Secondary</Button>
            <Button variant="subtle">Details</Button>
            <Button color="red">Remove example</Button>
            <Button disabled>Unavailable</Button>
          </Group>
          <Group>
            <StatusBadge state="positive">Ready</StatusBadge>
            <StatusBadge state="attention">Needs attention</StatusBadge>
            <StatusBadge state="negative">Failed</StatusBadge>
            <StatusBadge>Not configured</StatusBadge>
            <Badge>Example</Badge>
          </Group>
        </Stack>
      </SettingsSection>
      <SettingsSection
        title="Forms"
        description="Visible labels, helpful descriptions and a clear error state."
      >
        <Stack>
          <TextInput
            label="Name"
            placeholder="A memorable name"
            description="Shared labels come from the theme."
          />
          <Select
            label="Scope"
            data={["Entire Brain", "Selected collection"]}
            defaultValue="Entire Brain"
          />
          <TextInput
            label="Example validation"
            error="Enter a name before continuing."
          />
          <Checkbox
            label="An explicit permission"
            description="Explain the consequence before enabling it."
          />
          <Button
            variant="default"
            leftSection={<SlidersHorizontal size={16} />}
            onClick={() => setOpened(true)}
          >
            Open example inspector
          </Button>
        </Stack>
      </SettingsSection>
      <SettingsSection
        title="Content & states"
        description="Lists and empty/error states use the same surfaces and spacing."
      >
        <Stack>
          <Card withBorder>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Source</Table.Th>
                  <Table.Th>Status</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                <Table.Tr>
                  <Table.Td>Architecture notes</Table.Td>
                  <Table.Td>
                    <StatusBadge state="positive">Ready</StatusBadge>
                  </Table.Td>
                </Table.Tr>
              </Table.Tbody>
            </Table>
          </Card>
          <Alert color="yellow" title="This example needs attention">
            Tell the person what changed and the next useful step.
          </Alert>
          <EmptyState
            icon={FileText}
            title="No sources yet"
            description="Add a document to give this Brain something to remember."
            action={<Button>Import source</Button>}
          />
        </Stack>
      </SettingsSection>
      <Drawer
        opened={opened}
        onClose={() => setOpened(false)}
        title="Source inspector"
      >
        <Stack>
          <Text>
            A focused detail view keeps the list available when you return.
          </Text>
          <Button variant="default" onClick={() => setOpened(false)}>
            Return to reference
          </Button>
        </Stack>
      </Drawer>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <MantineProvider
    theme={theme}
    cssVariablesResolver={cssVariablesResolver}
    forceColorScheme="light"
  >
    <Preview />
  </MantineProvider>,
);
