import { useState } from "react";
import "./features/feature-views.css";
import {
  Alert,
  Badge,
  Button,
  Card,
  Divider,
  Group,
  Loader,
  Modal,
  Select,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";

const roles = ["reader", "writer", "admin"];
export function AccessPanel({
  brain,
  actor,
  embedded = false,
}: {
  brain: Brain;
  actor: string;
  embedded?: boolean;
}) {
  const [opened, setOpened] = useState(false);
  const [username, setUsername] = useState("");
  const [role, setRole] = useState("reader");
  const [group, setGroup] = useState("");
  const [groupRole, setGroupRole] = useState("reader");
  const [nextOwner, setNextOwner] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const cache = useQueryClient();
  const id = brain.id;
  const access = useQuery({
    queryKey: ["access", id],
    enabled: opened || embedded,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{id}/access", {
          params: { path: { id } },
        }),
      ),
    refetchInterval: opened || embedded ? 5000 : false,
  });
  const options = useQuery({
    queryKey: ["auth-options"],
    enabled: opened || embedded,
    queryFn: async () => result(await client.GET("/api/auth/options")),
  });
  const action = useMutation({
    mutationFn: async ({
      kind,
      target,
    }: {
      kind: "grant" | "remove" | "group" | "ungroup" | "owner";
      target?: string;
    }) => {
      let remaining: string | null | undefined;
      if (kind === "grant")
        remaining = result(
          await client.POST("/api/brains/{id}/grants", {
            params: { path: { id } },
            body: { username, role },
          }),
        ).effective_role;
      if (kind === "remove")
        remaining = result(
          await client.DELETE("/api/brains/{id}/grants/{account}", {
            params: { path: { id, account: target! } },
          }),
        ).effective_role;
      if (kind === "group")
        result(
          await client.PUT("/api/brains/{id}/group-grants", {
            params: { path: { id } },
            body: { group_name: group, role: groupRole },
          }),
        );
      if (kind === "ungroup")
        result(
          await client.DELETE("/api/brains/{id}/group-grants/{mapping}", {
            params: { path: { id, mapping: target! } },
          }),
        );
      if (kind === "owner")
        remaining = result(
          await client.POST("/api/brains/{id}/owner", {
            params: { path: { id } },
            body: { account_id: nextOwner! },
          }),
        ).effective_role;
      setMessage(
        remaining === undefined
          ? "Group mapping updated."
          : `Effective access ${kind === "owner" ? "for you " : ""}is now ${remaining ?? "none"}.`,
      );
    },
    onSuccess: () => {
      setUsername("");
      setGroup("");
      void cache.invalidateQueries();
    },
  });
  const content = (
    <Stack gap="lg">
      <Text size="sm" c="dimmed">
        Ownership, direct roles and current organization groups each contribute
        access. Removing one can leave another in place.
      </Text>
      {(access.error || action.error) && (
        <Alert color="red">{(access.error || action.error)?.message}</Alert>
      )}
      {message && <Alert color="teal">{message}</Alert>}
      {access.isPending ? (
        <Loader />
      ) : (
        !access.error && (
          <Stack gap="xs">
            {access.data?.members.map((member) => (
              <Card withBorder key={member.account.id}>
                <Group justify="space-between">
                  <Text fw={600}>{member.account.username}</Text>
                  <Badge color={member.effective_role ? "teal" : "gray"}>
                    {member.effective_role ?? "No access"}
                  </Badge>
                </Group>
                <Text size="xs" c="dimmed" mt="xs">
                  {member.owner ? "Owner · " : ""}
                  {member.direct_role
                    ? `Direct ${member.direct_role}`
                    : "No direct grant"}
                  {!member.account.enabled ? " · Account disabled" : ""}
                </Text>
                {member.groups.length > 0 && (
                  <Text size="xs" mt="xs">
                    Groups: {member.groups.join(", ")}
                  </Text>
                )}
                {member.membership_until && (
                  <Text size="xs" c="dimmed">
                    Membership valid until{" "}
                    {new Date(member.membership_until).toLocaleTimeString()}
                  </Text>
                )}
                {member.direct_role && (
                  <Button
                    size="xs"
                    variant="subtle"
                    color="red"
                    mt="xs"
                    disabled={action.isPending}
                    onClick={() =>
                      action.mutate({
                        kind: "remove",
                        target: member.account.id,
                      })
                    }
                  >
                    Remove direct grant
                  </Button>
                )}
              </Card>
            ))}
          </Stack>
        )
      )}
      {!access.error && (
        <>
          <Divider label="Direct access" />
          <form
            onSubmit={(e) => {
              e.preventDefault();
              action.mutate({ kind: "grant" });
            }}
          >
            <Stack>
              <TextInput
                label="Account username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                required
                maxLength={120}
              />
              <Select
                label="Direct role"
                value={role}
                data={roles}
                onChange={(v) => setRole(v ?? "reader")}
              />
              <Button type="submit" loading={action.isPending}>
                Save direct grant
              </Button>
            </Stack>
          </form>
          <Divider label="Organization groups" />
          {access.data?.group_grants.map((mapping) => (
            <Group key={mapping.id} justify="space-between">
              <Text size="sm">
                {mapping.group_name} · {mapping.role}
              </Text>
              <Button
                size="xs"
                variant="subtle"
                color="red"
                disabled={action.isPending}
                onClick={() =>
                  action.mutate({ kind: "ungroup", target: mapping.id })
                }
              >
                Remove group mapping
              </Button>
            </Group>
          ))}
          {options.data?.oidc_configured ? (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                action.mutate({ kind: "group" });
              }}
            >
              <Stack>
                <TextInput
                  label="Group name or ID"
                  value={group}
                  onChange={(e) => setGroup(e.target.value)}
                  required
                  maxLength={200}
                />
                <Select
                  label="Group role"
                  value={groupRole}
                  data={roles}
                  onChange={(v) => setGroupRole(v ?? "reader")}
                />
                <Button
                  type="submit"
                  variant="light"
                  loading={action.isPending}
                >
                  Save group mapping
                </Button>
              </Stack>
            </form>
          ) : (
            <Text size="sm" c="dimmed">
              Organization sign-in is not configured. Direct roles are
              available.
            </Text>
          )}
          {brain.owner_id === actor && (
            <>
              <Divider label="Ownership" />
              <Text size="sm" c="dimmed">
                Transfer this Brain to an enabled member. Your own access will
                then depend on any remaining direct or group grants.
              </Text>
              <Select
                label="New owner"
                value={nextOwner}
                onChange={setNextOwner}
                data={
                  access.data?.members
                    .filter((m) => m.account.enabled && m.account.id !== actor)
                    .map((m) => ({
                      value: m.account.id,
                      label: m.account.username,
                    })) ?? []
                }
                placeholder="Grant access to a teammate first"
              />
              <Button
                variant="default"
                disabled={!nextOwner}
                loading={action.isPending}
                onClick={() => action.mutate({ kind: "owner" })}
              >
                Transfer ownership
              </Button>
            </>
          )}
        </>
      )}
    </Stack>
  );
  return embedded ? (
    <section className="feature-setting" aria-label="Brain access">
      {content}
    </section>
  ) : (
    <>
      <Button mt="md" variant="light" onClick={() => setOpened(true)}>
        Manage access
      </Button>
      <Modal
        opened={opened}
        onClose={() => setOpened(false)}
        title="Brain access"
        size="lg"
        centered
      >
        {content}
      </Modal>
    </>
  );
}
