import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Divider,
  Group,
  Loader,
  Drawer,
  Tabs,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Users } from "lucide-react";
import { client, result } from "./api";
import "./features/workspace/control-panel.css";
import { ErrorState } from "./components/AsyncState";

export function TeamPanel() {
  const cache = useQueryClient();
  const [opened, setOpened] = useState(false);
  const [username, setUsername] = useState("");
  const [kind, setKind] = useState("local");
  const [subject, setSubject] = useState("");
  const [link, setLink] = useState("");
  const team = useQuery({
    queryKey: ["team"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/team", { signal })),
    gcTime: 0,
    refetchInterval: 5000,
  });
  const options = useQuery({
    queryKey: ["auth-options"],
    queryFn: async () => result(await client.GET("/api/auth/options")),
  });
  const history = useQuery({
    queryKey: ["team-audit"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/team/audit", { signal })),
    gcTime: 0,
  });
  const refresh = () => {
    void cache.invalidateQueries({ queryKey: ["team"] });
    void cache.invalidateQueries({ queryKey: ["team-audit"] });
  };
  const invite = useMutation({
    mutationFn: async () => {
      if (kind === "oidc") {
        result(
          await client.POST("/api/team/oidc-accounts", {
            body: { username, subject },
          }),
        );
        return null;
      }
      return result(
        await client.POST("/api/team/invitations", { body: { username } }),
      );
    },
    onSuccess: (value) => {
      setLink(value ? `${window.location.origin}/#invite=${value.token}` : "");
      setOpened(false);
      setUsername("");
      setSubject("");
      refresh();
    },
  });
  const action = useMutation({
    mutationFn: async ({
      id,
      kind,
      enabled,
    }: {
      id: string;
      kind: "status" | "reset" | "revoke";
      enabled?: boolean;
    }) => {
      if (kind === "status")
        result(
          await client.PATCH("/api/team/accounts/{id}", {
            params: { path: { id } },
            body: { enabled: !!enabled },
          }),
        );
      if (kind === "revoke")
        result(
          await client.DELETE("/api/team/invitations/{id}", {
            params: { path: { id } },
          }),
        );
      if (kind === "reset") {
        const value = result(
          await client.POST("/api/team/accounts/{id}/reset", {
            params: { path: { id } },
          }),
        );
        setLink(`${window.location.origin}/#invite=${value.token}`);
      }
    },
    onSuccess: refresh,
  });
  useEffect(() => {
    if (team.isError) {
      setOpened(false);
      setLink("");
    }
  }, [team.isError]);
  if (team.isError)
    return <ErrorState error={team.error} retry={() => void team.refetch()} />;
  return (
    <Stack gap="xl">
      <div className="page-heading">
        <div>
          <span className="eyebrow">PEOPLE AND PERMISSIONS</span>
          <Title order={1}>Team</Title>
          <Text c="dimmed" mt="xs">
            Invite people, manage their sign-in and keep access current.
          </Text>
        </div>
        <Button
          leftSection={<Plus size={17} />}
          onClick={() => {
            invite.reset();
            setOpened(true);
          }}
        >
          Add teammate
        </Button>
      </div>
      <Alert icon={<Users size={18} />} color="teal">
        Accounts sign in to this installation. Share each Brain separately from
        its Access panel.
      </Alert>
      {(team.error || action.error) && (
        <Alert color="red" title="Team action failed">
          {(team.error || action.error)?.message}
          <Button variant="subtle" onClick={() => void team.refetch()}>
            Refresh
          </Button>
        </Alert>
      )}
      <Tabs defaultValue="accounts">
        <Tabs.List mb="lg">
          <Tabs.Tab value="accounts">Accounts</Tabs.Tab>
          <Tabs.Tab value="invitations">Invitations</Tabs.Tab>
          <Tabs.Tab value="activity">Account activity</Tabs.Tab>
        </Tabs.List>
        <Tabs.Panel value="accounts">
          {team.isPending ? (
            <Loader />
          ) : (
            !team.error && (
              <Stack gap="sm">
                {team.data?.accounts.map((account) => (
                  <Card
                    withBorder
                    key={account.id}
                    className="control-account-row"
                  >
                    <Group justify="space-between" wrap="wrap">
                      <div>
                        <Group gap="xs">
                          <span className="control-initial">
                            {account.username.slice(0, 2).toUpperCase()}
                          </span>
                          <Text fw={600}>{account.username}</Text>
                          <Badge color={account.enabled ? "teal" : "gray"}>
                            {account.enabled ? "Enabled" : "Disabled"}
                          </Badge>
                        </Group>
                        <Text size="xs" c="dimmed" mt={4}>
                          {account.installation_owner
                            ? "Installation owner"
                            : account.auth_kind === "oidc"
                              ? "Organization identity"
                              : "Local account"}
                        </Text>
                      </div>
                      {!account.installation_owner && (
                        <Group gap="xs">
                          <Button
                            variant="default"
                            size="xs"
                            loading={
                              action.isPending &&
                              action.variables?.id === account.id
                            }
                            onClick={() =>
                              action.mutate({
                                id: account.id,
                                kind: "status",
                                enabled: !account.enabled,
                              })
                            }
                          >
                            {account.enabled ? "Disable" : "Enable"}
                          </Button>
                          {account.auth_kind === "local" && (
                            <Button
                              variant="light"
                              size="xs"
                              disabled={action.isPending}
                              onClick={() =>
                                action.mutate({ id: account.id, kind: "reset" })
                              }
                            >
                              Reset sign-in
                            </Button>
                          )}
                        </Group>
                      )}
                    </Group>
                  </Card>
                ))}
              </Stack>
            )
          )}
        </Tabs.Panel>
        <Tabs.Panel value="invitations">
          <section>
            <Title order={3} mb="md">
              Invitations
            </Title>
            {team.data?.invitations.length === 0 ? (
              <Text c="dimmed" size="sm">
                No invitations yet.
              </Text>
            ) : (
              <Stack gap="xs">
                {team.data?.invitations.map((invite) => (
                  <Card key={invite.id} withBorder padding="md">
                    <Group justify="space-between">
                      <div>
                        <Text fw={500} size="sm">
                          {invite.username}
                        </Text>
                        <Text c="dimmed" size="xs">
                          Expires {new Date(invite.expires_at).toLocaleString()}
                        </Text>
                      </div>
                      <Group>
                        <Badge
                          variant="light"
                          color={invite.state === "pending" ? "teal" : "gray"}
                        >
                          {invite.state}
                        </Badge>
                        {invite.state === "pending" && (
                          <Button
                            size="xs"
                            variant="subtle"
                            color="red"
                            disabled={action.isPending}
                            onClick={() =>
                              action.mutate({ id: invite.id, kind: "revoke" })
                            }
                          >
                            Revoke invitation
                          </Button>
                        )}
                      </Group>
                    </Group>
                  </Card>
                ))}
              </Stack>
            )}
          </section>
        </Tabs.Panel>
        <Tabs.Panel value="activity">
          <section>
            <Title order={3} mb="md">
              Account activity
            </Title>
            {history.error ? (
              <Alert color="red">{history.error.message}</Alert>
            ) : (
              <Card withBorder>
                {history.data?.slice(0, 12).map((event) => (
                  <Group key={event.id} justify="space-between" py="xs">
                    <Text size="sm">
                      {event.action.replaceAll(".", " ").replaceAll("_", " ")} ·{" "}
                      {event.disposition.replaceAll("_", " ")}
                    </Text>
                    <Text size="xs" c="dimmed">
                      {new Date(event.created_at).toLocaleString()}
                    </Text>
                  </Group>
                ))}
              </Card>
            )}
          </section>
        </Tabs.Panel>
      </Tabs>
      <Drawer
        opened={opened}
        onClose={() => setOpened(false)}
        title="Add a teammate"
        position="right"
        size="lg"
        className="control-drawer"
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            invite.mutate();
          }}
        >
          <Stack>
            <TextInput
              label="Teammate username"
              value={username}
              required
              maxLength={120}
              onChange={(e) => setUsername(e.target.value)}
            />
            <Select
              label="Sign-in method"
              value={kind}
              onChange={(v) => setKind(v ?? "local")}
              data={[
                { value: "local", label: "Local invitation" },
                ...(options.data?.oidc_configured
                  ? [{ value: "oidc", label: "Organization identity" }]
                  : []),
              ]}
            />
            {kind === "oidc" ? (
              <TextInput
                label="Provider subject"
                description="Use the exact stable subject from your configured identity provider."
                value={subject}
                required
                maxLength={255}
                onChange={(e) => setSubject(e.target.value)}
              />
            ) : (
              <Text size="sm" c="dimmed">
                You will receive a private invitation link to share. It expires
                after 24 hours and can be used once.
              </Text>
            )}
            {invite.error && <Alert color="red">{invite.error.message}</Alert>}
            <Button type="submit" loading={invite.isPending}>
              {kind === "oidc" ? "Enroll identity" : "Create invitation"}
            </Button>
          </Stack>
        </form>
      </Drawer>
      <Drawer
        opened={!!link}
        onClose={() => setLink("")}
        title="Your invitation is ready"
        position="right"
        size="lg"
        className="control-drawer"
      >
        <Stack>
          <Text size="sm">
            Share this link privately with your teammate. It is shown here once.
          </Text>
          <Textarea
            label="Invitation link"
            value={link}
            readOnly
            autosize
            minRows={3}
          />
          <Divider />
          <Text c="dimmed" size="xs">
            Resetting sign-in revokes existing sessions and requires a new
            invitation acceptance.
          </Text>
          <Button onClick={() => setLink("")}>Done</Button>
        </Stack>
      </Drawer>
    </Stack>
  );
}
