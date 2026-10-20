import { lazy, Suspense, useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { McpPanel } from "../../McpPanel";
import { useBrain, useWorkspace } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { FeatureTabs } from "../../components/FeatureTabs";
import { Page } from "../../components/Page";
import { PageHeader } from "../../components/PageHeader";
import { LoadingState } from "../../components/AsyncState";
const Runners = lazy(() =>
  import("../../McpPrivateRunners").then((m) => ({
    default: m.McpPrivateRunners,
  })),
);
const legacyAgentTabs = ["agents", "agent", "coding-agents", "coding-agent"];
const tabs = [
  { value: "connections", label: "Connections" },
  { value: "profiles", label: "Tool access" },
  { value: "runners", label: "Private Runners" },
] as const;

export function ConnectionsPage() {
  const brain = useBrain();
  const session = useWorkspace();
  const navigate = useNavigate();
  const [search, patch] = useBrainSearch();
  // The page header's action slot: the panel renders its primary action
  // (Add connection, Create tool group) there, inside the same region.
  const [actions, setActions] = useState<HTMLDivElement | null>(null);
  const legacy = !!search.tab && legacyAgentTabs.includes(search.tab);
  useEffect(() => {
    if (legacy)
      void navigate({
        to: "/brains/$brainId/agents",
        params: { brainId: brain.id },
        search: { ...search, tab: undefined },
        replace: true,
      });
  }, [legacy, navigate, brain.id, search]);
  if (legacy) return <LoadingState label="Opening Agents…" />;
  const tab =
    search.tab === "profiles" || search.tab === "runners"
      ? search.tab
      : "connections";
  return (
    <Page>
      <section
        className="flex flex-col gap-6"
        aria-label="MCP connections and tool groups"
      >
        <PageHeader
          title="Connections"
          description="Tools your agents can use in this Brain."
          actions={<div ref={setActions} className="flex items-center gap-2" />}
        />
        <FeatureTabs
          tabs={tabs}
          value={tab}
          onChange={(next) =>
            patch({
              tab: next === "connections" ? undefined : (next ?? undefined),
            })
          }
        >
          {tab === "runners" ? (
            <Suspense fallback={<LoadingState />}>
              <Runners brain={brain} canConfigure={brain.role === "admin"} />
            </Suspense>
          ) : (
            <McpPanel
              key={tab}
              brain={brain}
              actor={session.user.id}
              section={tab}
              region={false}
              actionsMount={actions}
              onSectionChange={(next) => patch({ tab: next ?? undefined })}
              onManageRunners={() => patch({ tab: "runners" })}
              initialConnection={search.connection}
              initialProfile={search.profile}
            />
          )}
        </FeatureTabs>
      </section>
    </Page>
  );
}
