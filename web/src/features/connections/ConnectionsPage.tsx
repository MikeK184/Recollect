import { McpPanel } from "../../McpPanel";
import { useBrain, useWorkspace } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { AgentConnections } from "./AgentConnections";
const tabs = [
  { value: "agents", label: "Coding agents" },
  { value: "connections", label: "MCP servers" },
  { value: "profiles", label: "Tool groups" },
  { value: "runners", label: "Runners" },
] as const;
export function ConnectionsPage() {
  const brain = useBrain();
  const session = useWorkspace();
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "agents",
  );
  return (
    <>
      <PageHeader
        title="Connections"
        description="Give your agents persistent memory and connect the tools they use."
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        {tab === "agents" ? (
          <AgentConnections />
        ) : (
          <McpPanel
            key={`${brain.id}-${tab}`}
            brain={brain}
            actor={session.user.id}
            section={tab as "connections" | "profiles" | "runners"}
            onSectionChange={setTab}
          />
        )}
      </FeatureTabs>
    </>
  );
}
