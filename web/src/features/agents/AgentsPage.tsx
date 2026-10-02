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
import { Check, Copy, ShieldCheck } from "lucide-react";
import { useBrain } from "../../app/context";
import { McpAgentSetup, agentMemoryReadCheck } from "../../McpAgentSetup";
import { WorkspacePanel } from "../../WorkspacePanel";
import { CapturePanel } from "../../CapturePanel";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { AgentsRoster } from "./AgentsRoster";
import { useBrainSearch } from "../../app/useBrainSearch";
const tabs = [
  { value: "setup", label: "Connect an agent" },
  { value: "sessions", label: "Captured sessions" },
  { value: "contexts", label: "Your working contexts" },
] as const;
const tabValues = tabs.map((tab) => tab.value);
export function AgentsPage() {
  const brain = useBrain();
  const [search] = useBrainSearch();
  const [tab, setTab] = useFeatureTab(tabValues, "setup");
  const check = agentMemoryReadCheck(brain);
  return (
    <>
      <PageHeader title="Agents" />
      <Stack gap="lg">
        <AgentsRoster brain={brain} />
        <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
          {tab === "setup" && (
            <Stack gap="xl">
              <div className="onboarding-grid">
                <Card withBorder p="lg">
                  <span className="onboarding-step">1</span>
                  <Title order={3}>Choose a host and connect</Title>
                  <Text size="sm" c="dimmed">
                    Recollect plugin, or a direct MCP access token.
                  </Text>
                  <Group mt="lg">
                    <McpAgentSetup brain={brain} />
                  </Group>
                </Card>
                <Card withBorder p="lg">
                  <span className="onboarding-step">2</span>
                  <Title order={3}>Set its working context</Title>
                  <Text size="sm" c="dimmed">
                    Your agent picks this Brain&apos;s repositories, areas and
                    environments as its private scope.
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
                    A created token is configured; this call proves connected.
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
              <Card withBorder p="lg">
                <Group gap="sm" mb="sm">
                  <ShieldCheck size={20} />
                  <Title order={3}>Capture and tools are your choice</Title>
                </Group>
                <Text size="sm" c="dimmed">
                  Capturing sessions and calling external tools have separate
                  permissions.
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
            <CapturePanel
              key={search.device}
              brain={brain}
              section="sessions"
              initialCoverage={!!search.device}
            />
          )}
          {tab === "contexts" && (
            <>
              <Alert mb="lg" color="gray">
                Private to your account. Other Brain members cannot see these
                scopes.
              </Alert>
              <WorkspacePanel brain={brain} section="tasks" />
            </>
          )}
        </FeatureTabs>
      </Stack>
    </>
  );
}
