import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { ShieldCheck } from "lucide-react";
import { client, result, type Brain } from "../../api";
import { ErrorState } from "../../components/AsyncState";
import { Skeleton } from "../../components/Skeleton";
import { Alert, Badge, Button, Tile } from "../../components/ui";
import { usePollInterval } from "../../usePollInterval";
import { ChooseModelNotice, needsModel } from "./ChooseModel";

export function ManagedMemoryPanel({
  brain,
  compact = false,
}: {
  brain: Brain;
  compact?: boolean;
}) {
  const pollSlow = usePollInterval("slow");
  const settings = useQuery({
    queryKey: ["automation", brain.id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/automation", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
    refetchInterval: pollSlow,
    retry: false,
    gcTime: 0,
  });
  const data = settings.isError ? undefined : settings.data;
  const policy = data?.models.current.policy;
  const ready =
    !!policy?.enabled &&
    policy.autonomous_memory &&
    policy.automatic_embedding &&
    ["extraction", "synthesis", "embedding", "answering"].every((p) =>
      policy.purposes.includes(p),
    ) &&
    [
      "document",
      "raw_session",
      "tool_output",
      "support_excerpt",
      "repository",
      "claim",
      "query",
    ].every((c) => policy.content_classes.includes(c)) &&
    data?.capture.policy.enabled &&
    data.capture.policy.managed_tools;
  return (
    <section
      className="grid gap-3 rounded-lg border border-border bg-card p-4"
      aria-label="AI permissions summary"
    >
      <div className="flex items-center gap-3">
        <Tile size="sm" tone="muted">
          <ShieldCheck aria-hidden="true" />
        </Tile>
        <h3 className="m-0 text-title font-semibold text-foreground">
          {compact ? "AI permissions" : "Memory permissions"}
        </h3>
        {policy && (
          <Badge
            className="ml-auto"
            variant={
              needsModel(policy) ? "warning" : policy.enabled ? "brand" : "default"
            }
          >
            {needsModel(policy)
              ? "Choose a model"
              : !policy.enabled
                ? "Off"
                : ready
                  ? "Configured"
                  : "Custom permissions"}
          </Badge>
        )}
      </div>
      {settings.isPending && (
        <div className="grid gap-2" role="status" aria-label="Loading AI permissions">
          <Skeleton width="60%" />
          <Skeleton width="40%" height={10} />
        </div>
      )}
      <ErrorState error={settings.error} />
      {needsModel(policy) && <ChooseModelNotice brain={brain} />}
      {policy?.enabled &&
        data &&
        !data.models.installed.credentials_present && (
          <Alert variant="warning" title="Provider credential missing">
            An administrator must connect the installed provider before
            permitted model processing can run.
          </Alert>
        )}
      <Button asChild variant="secondary" className="w-fit">
        <Link
          to="/brains/$brainId/settings"
          params={{ brainId: brain.id }}
          search={{ tab: "ai" }}
        >
          Review AI permissions
        </Link>
      </Button>
    </section>
  );
}
