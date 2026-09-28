import { Tabs } from "@mantine/core";
import type { ReactNode } from "react";
import { useNavigate, useRouterState } from "@tanstack/react-router";

export function useFeatureTab(allowed: readonly string[], fallback: string) {
  const selected = useRouterState({
    select: (s) => s.location.search as Record<string, unknown>,
  });
  const navigate = useNavigate();
  const tab =
    typeof selected.tab === "string" && allowed.includes(selected.tab)
      ? selected.tab
      : fallback;
  const setTab = (next: string | null) => {
    if (next && allowed.includes(next))
      void navigate({
        to: ".",
        search: (previous) => ({ ...previous, tab: next }),
        resetScroll: false,
      });
  };
  return [tab, setTab] as const;
}
export function FeatureTabs({
  tabs,
  value,
  onChange,
  children,
}: {
  tabs: readonly { value: string; label: string }[];
  value: string;
  onChange: (value: string | null) => void;
  children: ReactNode;
}) {
  return (
    <Tabs value={value} onChange={onChange} className="feature-tabs">
      <Tabs.List mb="lg">
        {tabs.map((tab) => (
          <Tabs.Tab key={tab.value} value={tab.value}>
            {tab.label}
          </Tabs.Tab>
        ))}
      </Tabs.List>
      {tabs.map((tab) => (
        <Tabs.Panel key={tab.value} value={tab.value}>
          {tab.value === value ? children : null}
        </Tabs.Panel>
      ))}
    </Tabs>
  );
}
