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
import { McpAgentSetup, agentMemoryReadCheck } from "../../McpAgentSetup";
import { WorkspacePanel } from "../../WorkspacePanel";
import { CapturePanel } from "../../CapturePanel";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
const tabs = [
  { value: "setup", label: "Connect an agent" },
  { value: "sessions", label: "Captured sessions" },
  { value: "contexts", label: "Your working contexts" },
] as const;
const tabValues = tabs.map((tab) => tab.value);
export function AgentsPage() {
  const brain = useBrain();
  const [tab, setTab] = useFeatureTab(tabValues, "setup");
  const check = agentMemoryReadCheck(brain);
  return (
    <>
      <PageHeader
        title="Agents"
        description="The only place to connect a coding tool that works as you. Connect once, then let it handle the routine work."
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        {tab === "setup" && (
          <Stack gap="xl">
            <div className="onboarding-grid">
              <Card withBorder p="lg">
                <span className="onboarding-step">1</span>
                <Title order={3}>Choose a host and connect</Title>
                <Text size="sm" c="dimmed">
                  Pick Codex, Claude Code or OpenCode, bind this Brain, create
                  your access token and add it to your host. Memory tools need no
                  companion; automatic session capture pairs one.
                </Text>
                <Group mt="lg">
                  <McpAgentSetup brain={brain} />
                </Group>
              </Card>
              <Card withBorder p="lg">
                <span className="onboarding-step">2</span>
                <Title order={3}>Set its working context</Title>
                <Text size="sm" c="dimmed">
                  Your restarted agent lists this Brain&apos;s repositories,
                  areas and environments, then starts a task with explicit scope.
                  Those scopes stay private to your account, including from Brain
                  admins.
                </Text>
                <Group mt="lg">
                  <Link
                    to="/brains/$brainId/agents"
                    params={{ brainId: brain.id }}
                    search={{ tab: "contexts" }}
                  >
                    Your working contexts
                  </Link>
                </Group>
              </Card>
              <Card withBorder p="lg">
                <span className="onboarding-step">3</span>
                <Title order={3}>Verify a real memory read</Title>
                <Text size="sm" c="dimmed">
                  Run this in your host and read its actual answer. A setup card
                  on screen, a saved configuration or a created token is
                  configured only. It is connected once this call returns, and
                  healthy only once repeated calls succeed.
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
                external tools have separate permissions. Captured sessions and
                coverage gaps are evidence of delivery, never of configuration:
                a hook that has published nothing shows as a gap. The capture
                policy has one editor, in Settings.
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
            <details className="feature-advanced">
              <summary>What the verification checks</summary>
              <Code block data-testid="agent-verification-summary">
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