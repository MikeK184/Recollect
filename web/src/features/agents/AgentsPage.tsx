import {
  Alert,
  Button,
  Card,
  Code,
  CopyButton,
  Group,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { Link } from "@tanstack/react-router";
import { Bot, Check, Copy, ShieldCheck } from "lucide-react";
import { useBrain } from "../../app/context";
import { McpAgentSetup } from "../../McpAgentSetup";
import { WorkspacePanel } from "../../WorkspacePanel";
import { CapturePanel } from "../../CapturePanel";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
const tabs = [
  { value: "setup", label: "Connect an agent" },
  { value: "sessions", label: "Captured sessions" },
  { value: "contexts", label: "Your working contexts" },
] as const;
export function AgentsPage() {
  const brain = useBrain();
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "setup",
  );
  const check = `Use Recollect workspace.list to verify access to Brain ${brain.id}. Then start a task for this workspace and use memory.recall to find a piece of knowledge from it. Report the actual result and its citations. Do not change permissions or call connected tools.`;
  return (
    <>
      <PageHeader
        title="Agents"
        description="Give your coding agent a memory. Connect once, then let it handle the routine work."
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        {tab === "setup" && (
          <Stack gap="xl">
            <div className="onboarding-grid">
              <Card withBorder p="lg">
                <span className="onboarding-step">1</span>
                <Title order={3}>Connect over MCP</Title>
                <Text size="sm" c="dimmed">
                  Add Recollect to Codex or Claude Code using a server URL and
                  access token. No companion is needed for memory tools.
                </Text>
                <Group mt="lg">
                  <McpAgentSetup brain={brain} />
                </Group>
              </Card>
              <Card withBorder p="lg">
                <span className="onboarding-step">2</span>
                <Title order={3}>Verify a memory read</Title>
                <Text size="sm" c="dimmed">
                  Ask your host to retrieve real evidence. A saved configuration
                  alone is not a connection check.
                </Text>
                <CopyButton value={check}>
                  {({ copied, copy }) => (
                    <Button
                      variant="default"
                      mt="lg"
                      onClick={copy}
                      leftSection={
                        copied ? <Check size={16} /> : <Copy size={16} />
                      }
                    >
                      {copied ? "Copied" : "Copy verification prompt"}
                    </Button>
                  )}
                </CopyButton>
              </Card>
            </div>
            <Alert
              color="brand"
              icon={<Bot size={20} />}
              title="Your agent manages its working context"
            >
              Once connected, your agent can list published repositories, manage
              its own task scope, recall and contribute memory, and use
              explicitly granted tools. You do not need to create tasks in the
              browser for it.
            </Alert>
            <Card withBorder p="lg">
              <Group gap="sm" mb="sm">
                <ShieldCheck size={20} />
                <Title order={3}>Capture and tools are your choice</Title>
              </Group>
              <Text size="sm" c="dimmed">
                Capturing sessions, sending content to a model, and calling
                external tools have separate permissions.
              </Text>
              <Group mt="lg">
                <Link
                  to="/brains/$brainId/settings"
                  params={{ brainId: brain.id }}
                  search={{ tab: "capture" }}
                >
                  Capture settings
                </Link>
                <Link
                  to="/brains/$brainId/connections"
                  params={{ brainId: brain.id }}
                  search={{}}
                >
                  Tool connections
                </Link>
              </Group>
            </Card>
            <details className="setup-details">
              <summary>What the verification checks</summary>
              <Code block mt="md">
                {check}
              </Code>
            </details>
          </Stack>
        )}
        {tab === "sessions" && (
          <CapturePanel brain={brain} section="sessions" />
        )}
        {tab === "contexts" && (
          <>
            <Alert mb="lg" color="gray">
              These are your private working scopes, not a to-do list. Other
              Brain members cannot see them.
            </Alert>
            <WorkspacePanel brain={brain} section="tasks" />
          </>
        )}
      </FeatureTabs>
    </>
  );
}
