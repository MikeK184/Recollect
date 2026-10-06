import { lazy, Suspense, useEffect, useState } from "react";
import { Tabs } from "@mantine/core";
import { useNavigate } from "@tanstack/react-router";
import { McpPanel } from "../../McpPanel";
import { useBrain, useWorkspace } from "../../app/context";
import { useBrainSearch } from "../../app/useBrainSearch";
import { PageHeader } from "../../components/PageHeader";
import { LoadingState } from "../../components/AsyncState";
const Runners = lazy(() =>
  import("../../McpPrivateRunners").then((m) => ({
    default: m.McpPrivateRunners,
  })),
);
const legacyAgentTabs = ["agents", "agent", "coding-agents", "coding-agent"];
export function ConnectionsPage() {
  const brain = useBrain();
  const session = useWorkspace();
  const navigate = useNavigate();
  const [search, patch] = useBrainSearch();
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
  return (
    <div
      className={
        "connections-page" +
        (search.tab === "profiles" || search.tab === "runners"
          ? " secondary-view"
          : "")
      }
    >
      <PageHeader
        title="Connections"
        description="Tools your agents can use in this Brain."
      />
      <Tabs
        value={
          search.tab === "profiles" || search.tab === "runners"
            ? search.tab
            : "connections"
        }
        onChange={(tab) =>
          patch({ tab: tab === "connections" ? undefined : (tab ?? undefined) })
        }
        mb="md"
      >
        <Tabs.List>
          <Tabs.Tab value="connections">Connections</Tabs.Tab>
          <Tabs.Tab value="profiles">Tool access</Tabs.Tab>
          <Tabs.Tab value="runners">Runners</Tabs.Tab>
        </Tabs.List>
      </Tabs>
      {search.tab === "runners" ? (
        <Suspense fallback={<LoadingState />}>
          <Runners brain={brain} canConfigure={brain.role === "admin"} />
        </Suspense>
      ) : (
        <McpPanel
          key={search.tab === "profiles" ? "profiles" : "connections"}
          brain={brain}
          actor={session.user.id}
          section={search.tab === "profiles" ? "profiles" : "connections"}
          onSectionChange={(tab) => patch({ tab: tab ?? undefined })}
          onManageRunners={() => patch({ tab: "runners" })}
        />
      )}
    </div>
  );
}
