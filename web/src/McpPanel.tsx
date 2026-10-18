import type { ServerDraft } from "./features/connections/mcpConfig";
import { McpConnectorSetup } from "./McpConnectorSetup";
import { useEffect, useState } from "react";
import { Alert, Button, Stack } from "@mantine/core";
import { SkeletonRows } from "./components/Skeleton";
import "./features/feature-views.css";
import "./features/guided-controls.css";
import "./features/connections/connections-inspection.css";
import { useQueryClient } from "@tanstack/react-query";
import { useWorkspace } from "./app/context";
import { type Brain } from "./api";
import type { components } from "./api-schema";
import { McpPrivateRunners } from "./McpPrivateRunners";
import { workspaceKey } from "./catalogueQueries";
import { ConnectionsSection } from "./McpConnectionsSection";
import { McpDialogStack, type McpRunRequest } from "./McpPanelDialogs";
import {
  DiagnosticsDrawer,
  McpRuntimeView,
  McpToolbar,
  ProfilesSection,
} from "./McpPanelSections";
import {
  useInspectedMetadata,
  useCatalogueGuards,
  useInspectorSelection,
  useMcpCatalogueQueries,
} from "./McpPanelQueries";

export type McpProfile = components["schemas"]["McpProfile"];
export type McpCatalogue = components["schemas"]["McpCatalogue"];
export type McpRights = components["schemas"]["McpRights"];
export const rightsLabel = (rights: McpRights) =>
  [
    rights.use_profile && "Use",
    rights.manage && "Manage",
    rights.share && "Share",
  ]
    .filter(Boolean)
    .join(" · ") || "No tool-group rights";
export { availabilityLabel } from "./McpPanelParts";

export function McpPanel({
  brain,
  actor,
  section = "connections",
  onSectionChange,
  initialCall,
  initialConnection,
  initialProfile,
  onManageRunners,
  onInspectorChange,
}: {
  brain: Brain;
  actor: string;
  section?: "connections" | "profiles" | "runners" | "activity";
  onSectionChange?: (section: string | null) => void;
  initialCall?: string;
  initialConnection?: string;
  initialProfile?: string;
  onManageRunners?: () => void;
  onInspectorChange?: (open: boolean) => void;
}) {
  const cache = useQueryClient();
  const account = useWorkspace();
  const [testing, setTesting] = useState<string | null>(null);
  const {
    inspectedId,
    setInspectedId,
    inspectorDismissed,
    setInspectorDismissed,
    inspectorTab,
    setInspectorTab,
  } = useInspectorSelection(brain.id, initialConnection);
  const [connectionSearch, setConnectionSearch] = useState("");
  const [importedDraft, setImportedDraft] = useState<ServerDraft | undefined>();
  const [connection, setConnection] = useState<string | null>(null);
  const [connectionNameMount, setConnectionNameMount] =
    useState<HTMLDivElement | null>(null);
  const [power, setPower] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState(false);
  const [connectorSetup, setConnectorSetup] = useState(false);
  const [profile, setProfile] = useState<string | null>(null);
  const [discover, setDiscover] = useState<string | null>(null);
  const [discoverConnection, setDiscoverConnection] = useState<string | null>(
    null,
  );
  const [environment, setEnvironment] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [setupSaved, setSetupSaved] = useState(false);
  const [session, setSession] = useState(() => crypto.randomUUID());
  const [selectedCall, setSelectedCall] = useState<string | null>(
    initialCall ?? null,
  );
  const [run, setRun] = useState<McpRunRequest | null>(null);
  useEffect(() => {
    onInspectorChange?.(
      !!(
        connection ||
        power ||
        profile ||
        discover ||
        run ||
        diagnostics ||
        connectorSetup ||
        testing
      ),
    );
    return () => onInspectorChange?.(false);
  }, [
    connection,
    power,
    profile,
    discover,
    run,
    diagnostics,
    connectorSetup,
    onInspectorChange,
    testing,
  ]);
  const { catalogue, runtime, workspace } = useMcpCatalogueQueries(
    brain,
    section,
  );
  const refresh = () => {
    void cache.invalidateQueries({ queryKey: ["mcp", brain.id] });
    void cache.invalidateQueries({ queryKey: workspaceKey(brain.id) });
    void cache.invalidateQueries({ queryKey: ["audit", brain.id] });
  };
  const clear = (message: string) => {
    setConnection(null);
    setImportedDraft(undefined);
    setPower(null);
    setProfile(null);
    setDiscover(null);
    setRun(null);
    setNotice(message);
    refresh();
  };
  useCatalogueGuards({
    catalogueData: catalogue.data,
    catalogueError: catalogue.error,
    workspaceError: workspace.error,
    archived: brain.archived,
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
  });
  const data = catalogue.error ? undefined : catalogue.data;
  const matchingConnections =
    data?.connections.filter(
      (c) =>
        (!environment ||
          !c.environment_id ||
          c.environment_id === environment) &&
        c.name
          .toLocaleLowerCase()
          .includes(connectionSearch.trim().toLocaleLowerCase()),
    ) ?? [];

  const inspected =
    connection && connection !== "new"
      ? data?.connections.find((item) => item.id === connection)
      : inspectorDismissed
        ? undefined
        : inspectedId
          ? matchingConnections.find((c) => c.id === inspectedId)
          : matchingConnections[0];
  const profileIds = data?.profiles.map((p) => p.id).join(",");
  useEffect(() => {
    if (section === "profiles" && initialProfile)
      document
        .getElementById(`tool-group-${initialProfile}`)
        ?.scrollIntoView({ block: "center" });
  }, [brain.id, section, initialProfile, profileIds]);
  const { detailError, visibleDetail, visibleDefinition, approvedTools } =
    useInspectedMetadata({
      brain,
      section,
      inspected,
      data,
      workspaceFailed: !!workspace.error,
    });
  const environments =
    workspace.data?.environments.map((e) => ({ value: e.id, label: e.name })) ??
    [];
  const environmentName = (id: string | null | undefined) =>
    id
      ? (environments.find((e) => e.value === id)?.label ??
        "Unavailable environment")
      : "Brain-wide";
  const released = () => {
    setRun(null);
    setSession(crypto.randomUUID());
  };
  return (
    <section
      className={`feature-view mcp-management-view${section === "profiles" ? " mcp-tool-access-view" : ""}`}
      aria-label="MCP connections and tool groups"
    >
      <Stack gap="md">
        {connectorSetup &&
          data?.can_configure &&
          !brain.archived &&
          !workspace.error && (
            <McpConnectorSetup
              brain={brain}
              close={() => setConnectorSetup(false)}
              approvedConnectors={data.definitions.filter((d) => d.enabled)}
              reuse={(draft) => {
                setImportedDraft(draft);
                setConnectorSetup(false);
                setConnection("new");
              }}
              saved={(connected) => {
                setConnectorSetup(false);
                setNotice(
                  connected
                    ? "MCP server added. Include it in a tool group to make it available to your agents."
                    : "Connector registered. Add a connection to choose its target and execution location.",
                );
                refresh();
              }}
            />
          )}
        {notice && (
          <Alert
            title="Catalogue updated"
            withCloseButton
            onClose={() => setNotice(null)}
          >
            {notice}
          </Alert>
        )}
        {(catalogue.error || workspace.error) && (
          <Alert color="red" title="Catalogue unavailable">
            {(catalogue.error || workspace.error)?.message}
            <Button size="xs" variant="light" onClick={refresh}>
              Reload catalogue
            </Button>
          </Alert>
        )}
        {catalogue.isPending && (
          <SkeletonRows label="Loading catalogue…" rows={4} />
        )}
        {data && !workspace.error && (
          <>
            {section !== "activity" && (
              <McpToolbar
                section={section}
                canConfigure={data.can_configure}
                archived={brain.archived}
                connection={connection}
                connectionSearch={connectionSearch}
                setConnectionSearch={setConnectionSearch}
                environments={environments}
                environment={environment}
                setEnvironment={setEnvironment}
                setDiscover={setDiscover}
                setConnectorSetup={setConnectorSetup}
                setProfile={setProfile}
              />
            )}
            {brain.archived && (
              <Alert color="gray">
                This Brain is archived. Configuration and tool use are paused.
              </Alert>
            )}
            {section === "connections" && (
              <ConnectionsSection
                brain={brain}
                data={data}
                inspected={inspected}
                matchingConnections={matchingConnections}
                initialConnection={initialConnection}
                connectionSearch={connectionSearch}
                environment={environment}
                setupSaved={setupSaved}
                setSetupSaved={setSetupSaved}
                controls={{
                  connection,
                  setConnection,
                  setImportedDraft,
                  setPower,
                  setTesting,
                  setDiagnostics,
                  setInspectedId,
                  setInspectorDismissed,
                  inspectorTab,
                  setInspectorTab,
                }}
                runtime={runtime}
                detailError={detailError}
                visibleDetail={visibleDetail}
                visibleDefinition={visibleDefinition}
                approvedTools={approvedTools}
                environments={environments}
                environmentName={environmentName}
                connectionNameMount={connectionNameMount}
                setConnectionNameMount={setConnectionNameMount}
                onSectionChange={onSectionChange}
                onManageRunners={onManageRunners}
                refresh={refresh}
                clear={clear}
              />
            )}
            {section === "profiles" && (
              <ProfilesSection
                brain={brain}
                actor={actor}
                data={data}
                initialProfile={initialProfile}
                profile={profile}
                setProfile={setProfile}
                environments={environments}
                environment={environment}
                refresh={refresh}
                openTools={(id) => {
                  setDiscoverConnection(null);
                  setDiscover(id);
                }}
              />
            )}
            {section === "runners" && (
              <McpPrivateRunners
                brain={brain}
                canConfigure={data.can_configure}
              />
            )}
            {section === "activity" && (
              <McpRuntimeView
                brain={brain}
                actor={actor}
                catalogue={data}
                selected={selectedCall}
                select={setSelectedCall}
                session={session}
                released={released}
              />
            )}
            <DiagnosticsDrawer
              open={diagnostics && section !== "activity"}
              selectedCall={selectedCall}
              setSelectedCall={setSelectedCall}
              setDiagnostics={setDiagnostics}
              brain={brain}
              actor={actor}
              catalogue={data}
              session={session}
              released={released}
            />
          </>
        )}

        <McpDialogStack
          brain={brain}
          data={data}
          workspaceFailed={!!workspace.error}
          owner={account.user.installation_owner}
          environments={environments}
          testing={testing}
          setTesting={setTesting}
          power={power}
          setPower={setPower}
          connection={connection}
          setConnection={setConnection}
          importedDraft={importedDraft}
          setImportedDraft={setImportedDraft}
          discover={discover}
          setDiscover={setDiscover}
          discoverConnection={discoverConnection}
          setDiscoverConnection={setDiscoverConnection}
          run={run}
          setRun={setRun}
          session={session}
          setSetupSaved={setSetupSaved}
          setSelectedCall={setSelectedCall}
          setDiagnostics={setDiagnostics}
          onManageRunners={onManageRunners}
          refresh={refresh}
          clear={clear}
        />
      </Stack>
    </section>
  );
}
