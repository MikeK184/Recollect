import type { ReactNode } from "react";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";

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

/** Intra-page tabs held in validated URL state (`?tab=`). */
export function FeatureTabs({
  tabs,
  value,
  onChange,
  children,
  actions,
}: {
  tabs: readonly { value: string; label: string; count?: ReactNode }[];
  value: string;
  onChange: (value: string | null) => void;
  children: ReactNode;
  /** Controls aligned with the tab row, at its right. */
  actions?: ReactNode;
}) {
  return (
    <Tabs value={value} onValueChange={onChange}>
      <div className="flex items-end gap-3 border-b border-border">
        <TabsList className="min-w-0 flex-1 border-b-0">
          {tabs.map((tab) => (
            <TabsTrigger key={tab.value} value={tab.value}>
              {tab.label}
              {tab.count}
            </TabsTrigger>
          ))}
        </TabsList>
        {actions && <div className="flex shrink-0 items-center gap-2 pb-1.5">{actions}</div>}
      </div>
      {tabs.map((tab) => (
        <TabsContent key={tab.value} value={tab.value}>
          {tab.value === value ? children : null}
        </TabsContent>
      ))}
    </Tabs>
  );
}
