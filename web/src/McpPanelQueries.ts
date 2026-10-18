import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { ServerDraft } from "./features/connections/mcpConfig";
import type { McpCatalogue } from "./McpPanel";
import type { McpRunRequest } from "./McpPanelDialogs";
import {
  metadataRetryDelay,
  retryMetadataRead,
  type MetadataRead,
} from "./McpPanelParts";
import { usePollInterval } from "./usePollInterval";
import { useWorkspaceCatalogue } from "./catalogueQueries";

type ConnectionSummary = components["schemas"]["McpConnectionSummary"];

export function useMcpCatalogueQueries(
  brain: Brain,
  section: "connections" | "profiles" | "runners" | "activity",
) {
  const pollNormal = usePollInterval("normal");
  const catalogue = useQuery({
    queryKey: ["mcp", brain.id, "catalogue"],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
    refetchInterval: pollNormal,
    retry: false,
    gcTime: 0,
  });
  const runtime = useQuery({
    queryKey: ["mcp", brain.id, "runtime"],
    enabled: section === "connections" && !!catalogue.data && !catalogue.error,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/runtime", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
    refetchInterval: pollNormal,
    retry: false,
    gcTime: 0,
  });
  const workspace = useWorkspaceCatalogue(brain.id, undefined, {
    poll: "normal",
  });
  return { catalogue, runtime, workspace };
}

export function useInspectorSelection(
  brainId: string,
  initialConnection: string | undefined,
) {
  const [inspectedId, setInspectedId] = useState<string | null>(
    initialConnection ?? null,
  );
  const [inspectorDismissed, setInspectorDismissed] = useState(false);
  const [inspectorTab, setInspectorTab] = useState<string | null>(
    initialConnection ? "tools" : "overview",
  );
  useEffect(() => {
    setInspectedId(initialConnection ?? null);
    setInspectorDismissed(false);
    setInspectorTab(initialConnection ? "tools" : "overview");
  }, [brainId, initialConnection]);
  return {
    inspectedId,
    setInspectedId,
    inspectorDismissed,
    setInspectorDismissed,
    inspectorTab,
    setInspectorTab,
  };
}

export function useInspectedMetadata({
  brain,
  section,
  inspected,
  data,
  workspaceFailed,
}: {
  brain: Brain;
  section: "connections" | "profiles" | "runners" | "activity";
  inspected: ConnectionSummary | undefined;
  data: McpCatalogue | undefined;
  workspaceFailed: boolean;
}) {
  const enabled =
    section === "connections" &&
    !!inspected &&
    !!data?.can_configure &&
    !workspaceFailed;
  const inspectedDetail = useQuery({
    queryKey: ["mcp", brain.id, "connection", inspected?.id],
    enabled,
    gcTime: 0,
    retry: retryMetadataRead,
    retryDelay: metadataRetryDelay,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/connections/{id}", {
          params: { path: { brain: brain.id, id: inspected!.id } },
          signal,
        }),
      ),
  });
  const inspectedDefinition = useQuery({
    queryKey: ["mcp", brain.id, "definition", inspected?.definition_key],
    enabled,
    gcTime: 0,
    retry: retryMetadataRead,
    retryDelay: metadataRetryDelay,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/definitions/{key}", {
          params: { path: { brain: brain.id, key: inspected!.definition_key } },
          signal,
        }),
      ),
  });
  const visibleDetail =
    data?.can_configure && !inspectedDetail.error
      ? inspectedDetail.data
      : undefined;
  const visibleDefinition =
    data?.can_configure && !inspectedDefinition.error
      ? inspectedDefinition.data
      : undefined;
  const approvedTools: MetadataRead = {
    tools: visibleDefinition?.tools,
    error: inspectedDefinition.error,
    pending: inspectedDefinition.isPending,
    refreshing: inspectedDefinition.isFetching,
    reload: () => void inspectedDefinition.refetch(),
  };
  return {
    detailError: inspectedDetail.error,
    visibleDetail,
    visibleDefinition,
    approvedTools,
  };
}

type Setter<T> = Dispatch<SetStateAction<T>>;

// Closes drawers and dialogs whose backing catalogue entry or right vanished.
export function useCatalogueGuards({
  catalogueData,
  catalogueError,
  workspaceError,
  archived,
  testing,
  power,
  connectorSetup,
  connection,
  profile,
  discover,
  setTesting,
  setPower,
  setConnectorSetup,
  setConnection,
  setImportedDraft,
  setProfile,
  setDiscover,
  setRun,
  setNotice,
}: {
  catalogueData: McpCatalogue | undefined;
  catalogueError: Error | null;
  workspaceError: Error | null;
  archived: boolean;
  testing: string | null;
  power: string | null;
  connectorSetup: boolean;
  connection: string | null;
  profile: string | null;
  discover: string | null;
  setTesting: Setter<string | null>;
  setPower: Setter<string | null>;
  setConnectorSetup: Setter<boolean>;
  setConnection: Setter<string | null>;
  setImportedDraft: Setter<ServerDraft | undefined>;
  setProfile: Setter<string | null>;
  setDiscover: Setter<string | null>;
  setRun: Setter<McpRunRequest | null>;
  setNotice: Setter<string | null>;
}) {
  useEffect(() => {
    if (catalogueError || workspaceError) {
      setTesting(null);
      setConnection(null);
      setImportedDraft(undefined);
      setPower(null);
      setConnectorSetup(false);
      setProfile(null);
      setDiscover(null);
      setRun(null);
      setNotice(
        "Catalogue access could not be refreshed. Reload it to continue.",
      );
      return;
    }
    if (!catalogueData) return;
    if (
      testing &&
      (archived ||
        !catalogueData.can_configure ||
        !catalogueData.connections.some((c) => c.id === testing))
    )
      setTesting(null);
    if (
      power &&
      (!catalogueData.can_configure ||
        archived ||
        !catalogueData.connections.some((c) => c.id === power))
    )
      setPower(null);
    if (connectorSetup && (!catalogueData.can_configure || archived))
      setConnectorSetup(false);
    if (connection && !catalogueData.can_configure) {
      setConnection(null);
      setImportedDraft(undefined);
      setNotice("Connection administration is no longer available.");
    }
    if (
      profile &&
      profile !== "new" &&
      !catalogueData.profiles.some((p) => p.id === profile)
    ) {
      setProfile(null);
      setNotice("Tool-group access changed. The inspector was closed.");
    }
    if (profile === "new" && (!catalogueData.can_configure || archived))
      setProfile(null);
    if (
      discover &&
      (archived ||
        !catalogueData.profiles.some(
          (p) => p.id === discover && p.rights.use_profile && p.enabled,
        ))
    ) {
      setDiscover(null);
      setNotice("Tool-group use changed. Cached tool results were cleared.");
    }
  }, [
    catalogueData,
    catalogueError,
    workspaceError,
    archived,
    connection,
    profile,
    discover,
    connectorSetup,
    power,
    testing,
  ]);
}
