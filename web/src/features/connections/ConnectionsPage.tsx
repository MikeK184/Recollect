import { useEffect } from "react";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { McpPanel } from "../../McpPanel";
import { useBrain, useWorkspace } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { LoadingState } from "../../components/AsyncState";
import { ToolGroupGrants, WhereItRuns } from "./ConnectionGuidance";

type ConnectionSection = "connections" | "profiles" | "runners";
const tabs = [
  { value: "connections", label: "MCP servers" },
  { value: "profiles", label: "Tool groups" },
  { value: "runners", label: "Runners" },
] as const;
const tabValues = tabs.map((tab) => tab.value);
// Outbound configuration is the first actionable step, so MCP servers is the
// documented default. Agents owns coding-agent wiring entirely.
const defaultSection: ConnectionSection = "connections";
// Legacy Connections coding-agent state. ADR 0017 removed the second setup path,
// so every spelling resolves to the Agents destination instead of a dead tab.
const legacyAgentTabs = [
  "agents",
  "agent",
  "coding-agents",
  "coding-agent",
] as const;

/** Rewrites legacy agent state onto the Agents destination. A pure view change:
 * it issues no request, creates no credential, token or connection record. */
function LegacyAgentTab({ brainId }: { brainId: string }) {
  const navigate = useNavigate();
  useEffect(() => {
    void navigate({
      to: "/brains/$brainId/agents",
      params: { brainId },
      search: { tab: "setup" },
      replace: true,
    });
  }, [navigate, brainId]);
  return <LoadingState label="Opening Agents…" />;
}

export function ConnectionsPage() {
  const brain = useBrain();
  const session = useWorkspace();
  const requested = useRouterState({
    select: (state) => (state.location.search as { tab?: unknown }).tab,
  });
  const [tab, setTab] = useFeatureTab(tabValues, defaultSection);
  // Checked before any MCP content renders, so no catalogue read accompanies a
  // legacy agent URL and no second setup path is offered.
  if (typeof requested === "string" && legacyAgentTabs.includes(requested as never))
    return <LegacyAgentTab brainId={brain.id} />;
  const section: ConnectionSection =
    tab === "profiles" ? "profiles" : tab === "runners" ? "runners" : "connections";
  return (
    <>
      <PageHeader
        title="Connections"
        description="Outbound tools your Brain can use. Connect a coding agent in Agents."
      />
      <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
        <>
          {section === "profiles" ? <ToolGroupGrants /> : <WhereItRuns />}
          <McpPanel
            key={`${brain.id}-${section}`}
            brain={brain}
            actor={session.user.id}
            section={section}
            onSectionChange={setTab}
          />
        </>
      </FeatureTabs>
    </>
  );
}