import { Alert } from "@mantine/core";
import { TeamPanel } from "../../TeamPanel";
import { useWorkspace } from "../../app/context";

export function TeamPage() {
  const session = useWorkspace();
  return session.user.installation_owner ? (
    <TeamPanel />
  ) : (
    <Alert title="Installation owner access required">
      Your Brain access is managed in each Brain’s Settings.
    </Alert>
  );
}
