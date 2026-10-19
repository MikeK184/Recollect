import type { Dispatch, SetStateAction } from "react";
import type { Brain } from "./api";
import type { ServerDraft } from "./features/connections/mcpConfig";
import { McpConnectionDialog } from "./McpConnectionDialog";
import { ConnectionPowerDialog } from "./McpConnectionPower";
import { McpRunDialog, type McpTool } from "./McpRunDialog";
import type { McpCatalogue } from "./McpPanel";
import { ConnectionTest, DiscoveryDialog } from "./McpPanelParts";

type Setter<T> = Dispatch<SetStateAction<T>>;

export type McpRunRequest = {
  profile: string;
  tool: McpTool;
  environment: string | null;
};

export function McpDialogStack({
  brain,
  data,
  workspaceFailed,
  owner,
  environments,
  testing,
  setTesting,
  power,
  setPower,
  connection,
  setConnection,
  importedDraft,
  setImportedDraft,
  discover,
  setDiscover,
  discoverConnection,
  setDiscoverConnection,
  run,
  setRun,
  session,
  setSetupSaved,
  setSelectedCall,
  setDiagnostics,
  onManageRunners,
  refresh,
  clear,
}: {
  brain: Brain;
  data: McpCatalogue | undefined;
  workspaceFailed: boolean;
  owner: boolean;
  environments: { value: string; label: string }[];
  testing: string | null;
  setTesting: Setter<string | null>;
  power: string | null;
  setPower: Setter<string | null>;
  connection: string | null;
  setConnection: Setter<string | null>;
  importedDraft: ServerDraft | undefined;
  setImportedDraft: Setter<ServerDraft | undefined>;
  discover: string | null;
  setDiscover: Setter<string | null>;
  discoverConnection: string | null;
  setDiscoverConnection: Setter<string | null>;
  run: McpRunRequest | null;
  setRun: Setter<McpRunRequest | null>;
  session: string;
  setSetupSaved: Setter<boolean>;
  setSelectedCall: Setter<string | null>;
  setDiagnostics: Setter<boolean>;
  onManageRunners?: () => void;
  refresh: () => void;
  clear: (message: string) => void;
}) {
  const discoveryProfile = data?.profiles.find(
    (p) => p.id === discover && p.rights.use_profile && p.enabled,
  );
  const runProfile =
    !brain.archived &&
    data?.profiles.find(
      (p) => p.id === run?.profile && p.enabled && p.rights.use_profile,
    );
  return (
    <>
      {data &&
        testing &&
        !workspaceFailed &&
        data.connections.some((c) => c.id === testing) && (
          <ConnectionTest
            key={`${testing}:${data.connections.find((c) => c.id === testing)?.revision}:${data.definitions.find((d) => d.key === data.connections.find((c) => c.id === testing)?.definition_key)?.updated_at}`}
            brain={brain}
            id={testing}
            catalogue={data}
            owner={owner}
            close={() => setTesting(null)}
            tools={(profileId) => {
              setDiscoverConnection(testing);
              setTesting(null);
              setDiscover(profileId);
            }}
          />
        )}
      {data && power && !workspaceFailed && (
        <ConnectionPowerDialog
          key={power}
          brain={brain}
          id={power}
          catalogue={data}
          close={() => setPower(null)}
          saved={() => {
            setPower(null);
            refresh();
          }}
          stale={clear}
        />
      )}
      {data && connection === "new" && !workspaceFailed && (
        <McpConnectionDialog
          initialDraft={importedDraft}
          key={connection}
          brain={brain}
          id={connection}
          catalogue={data}
          environments={environments}
          manageRunners={
            onManageRunners
              ? () => {
                  setConnection(null);
                  setImportedDraft(undefined);
                  onManageRunners();
                }
              : undefined
          }
          close={() => {
            setConnection(null);
            setImportedDraft(undefined);
          }}
          saved={() => {
            setSetupSaved(connection === "new");
            setConnection(null);
            setImportedDraft(undefined);
            refresh();
          }}
          stale={clear}
        />
      )}
      {discoveryProfile && !workspaceFailed && (
        <DiscoveryDialog
          key={`${discover}:${discoverConnection}`}
          connectionId={discoverConnection}
          connectionName={
            data?.connections.find((c) => c.id === discoverConnection)?.name
          }
          brain={brain}
          profile={discoveryProfile}
          environments={environments}
          close={() => setDiscover(null)}
          run={(tool, selectedEnvironment) => {
            setRun({
              profile: discoveryProfile.id,
              tool,
              environment: selectedEnvironment,
            });
            setDiscover(null);
          }}
        />
      )}
      {run && runProfile && !workspaceFailed && (
        <McpRunDialog
          brain={brain}
          profile={runProfile}
          tool={run.tool}
          environment={run.environment}
          session={session}
          close={() => setRun(null)}
          submitted={(id) => {
            setRun(null);
            setSelectedCall(id);
            setDiagnostics(true);
            refresh();
          }}
        />
      )}
    </>
  );
}
