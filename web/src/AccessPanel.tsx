import { useEffect, useState } from "react";
import "./features/feature-views.css";
import "./features/guided-controls.css";
import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Menu,
  Modal,
  Select,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { MoreHorizontal, Plus, Users } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
const roles = ["reader", "writer", "admin"];
type Action = {
  kind: "grant" | "remove" | "group" | "ungroup" | "owner";
  target?: string;
};
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
  const [editor, setEditor] = useState<Action | null>(null);
  const [username, setUsername] = useState("");
  const [role, setRole] = useState("reader");
  const [group, setGroup] = useState("");
  const [groupRole, setGroupRole] = useState("reader");
  const [nextOwner, setNextOwner] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const cache = useQueryClient();
  const id = brain.id;
  const canEdit = brain.role === "admin" && !brain.archived;
  const access = useQuery({
    queryKey: ["access", id],
    enabled: opened || embedded,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{id}/access", {
          params: { path: { id } },
          signal,
        }),
      ),
    refetchInterval: opened || embedded ? 5000 : false,
    retry: false,
    gcTime: 0,
  });
  const options = useQuery({
    queryKey: ["auth-options"],
    enabled: opened || embedded,
    queryFn: async () => result(await client.GET("/api/auth/options")),
  });
  const action = useMutation({
    mutationFn: async ({ kind, target }: Action) => {
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
      return remaining === undefined
        ? "Group mapping updated."
        : `Effective access ${kind === "owner" ? "for you " : ""}is now ${remaining ?? "none"}.`;
    },
    onSuccess: (notice) => {
      setMessage(notice);
      setEditor(null);
      setUsername("");
      setGroup("");
      void cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(id) || q.queryKey[0] === "brains",
      });
    },
  });
  useEffect(() => {
    if (access.error || !canEdit) setEditor(null);
  }, [access.error, canEdit]);
  const edit = (value: Action) => {
    action.reset();
    setEditor(value);
  };
  const member = access.data?.members.find(
    (m) => m.account.id === editor?.target,
  );
  const editingSelf = editor?.kind === "remove" && editor.target === actor;
  const content = (
    <Stack gap="lg">
      <Group justify="space-between">
        <Text fw={600}>
          Members{" "}
          {access.data && !access.error && (
            <Text component="span" c="dimmed" size="sm">
              · {access.data.members.length}
            </Text>
          )}
        </Text>
        <Group gap="sm">
          {canEdit && options.data?.oidc_configured && (
            <Button
              variant="default"
              size="sm"
              leftSection={<Users size={16} />}
              disabled={!access.data || !!access.error}
              onClick={() => edit({ kind: "group" })}
            >
              Add group
            </Button>
          )}
          {canEdit && (
            <Button
              size="sm"
              leftSection={<Plus size={16} />}
              disabled={!access.data || !!access.error}
              onClick={() => {
                setUsername("");
                setRole("reader");
                edit({ kind: "grant" });
              }}
            >
              Add member
            </Button>
          )}
        </Group>
      </Group>
      {access.error && (
        <Alert color="red">
          {access.error.message}
          <Button
            variant="subtle"
            size="xs"
            onClick={() => void access.refetch()}
          >
            Retry members
          </Button>
        </Alert>
      )}
      {message && (
        <Alert color="brand" withCloseButton onClose={() => setMessage("")}>
          {message}
        </Alert>
      )}
      {access.isPending ? (
        <Loader />
      ) : (
        !access.error && (
          <div className="member-list">
            {access.data?.members.map((m) => (
              <div
                className="member-row"
                key={m.account.id}
                data-testid="brain-member"
              >
                <div className="member-identity">
                  <span className="member-avatar" aria-hidden>
                    {m.account.username.slice(0, 2).toUpperCase()}
                  </span>
                  <div>
                    <Text fw={600}>
                      {m.account.username}
                      {m.account.id === actor ? " (you)" : ""}
                    </Text>
                    <Text size="xs" c="dimmed">
                      {m.owner
                        ? "Owner"
                        : m.direct_role
                          ? "Direct access"
                          : "Group access"}
                      {!m.account.enabled ? " · Account disabled" : ""}
                    </Text>
                  </div>
                </div>
                <Badge color={m.effective_role ? "brand" : "gray"}>
                  {m.effective_role ?? "No access"}
                </Badge>
                <Menu position="bottom-end">
                  <Menu.Target>
                    <ActionIcon
                      variant="subtle"
                      aria-label={`Manage ${m.account.username}`}
                    >
                      <MoreHorizontal size={18} />
                    </ActionIcon>
                  </Menu.Target>
                  <Menu.Dropdown>
                    <Menu.Label>
                      {m.owner ? "Owner" : "Member"}
                      {m.direct_role
                        ? ` · Direct ${m.direct_role}`
                        : " · No direct grant"}
                    </Menu.Label>
                    {m.groups.map((g) => (
                      <Menu.Label key={g}>Group: {g}</Menu.Label>
                    ))}
                    {m.membership_until && (
                      <Menu.Label>
                        Valid until{" "}
                        {new Date(m.membership_until).toLocaleString()}
                      </Menu.Label>
                    )}
                    {canEdit && (
                      <Menu.Item
                        onClick={() => {
                          setUsername(m.account.username);
                          setRole(m.direct_role ?? "reader");
                          edit({ kind: "grant", target: m.account.id });
                        }}
                      >
                        Edit direct role
                      </Menu.Item>
                    )}
                    {canEdit && m.direct_role && (
                      <Menu.Item
                        color="red"
                        onClick={() =>
                          edit({ kind: "remove", target: m.account.id })
                        }
                      >
                        Remove direct grant
                      </Menu.Item>
                    )}
                    {canEdit &&
                      brain.owner_id === actor &&
                      m.account.enabled &&
                      m.account.id !== actor && (
                        <Menu.Item
                          onClick={() => {
                            setNextOwner(m.account.id);
                            edit({ kind: "owner" });
                          }}
                        >
                          Transfer ownership
                        </Menu.Item>
                      )}
                  </Menu.Dropdown>
                </Menu>
              </div>
            ))}
          </div>
        )
      )}
      {!access.error && !!access.data?.group_grants.length && (
        <Stack gap="xs">
          <Text fw={600} size="sm">
            Organization groups
          </Text>
          {access.data.group_grants.map((g) => (
            <Group key={g.id} justify="space-between">
              <Text size="sm">
                {g.group_name} · {g.role}
              </Text>
              {canEdit && (
                <Button
                  variant="subtle"
                  size="xs"
                  color="red"
                  onClick={() => edit({ kind: "ungroup", target: g.id })}
                >
                  Remove group mapping
                </Button>
              )}
            </Group>
          ))}
        </Stack>
      )}
      <Text size="xs" c="dimmed">
        Tool-use permissions are managed in Connections → Tool access.
      </Text>
    </Stack>
  );
  return (
    <>
      {embedded ? (
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
            onClose={() => !editor && setOpened(false)}
            trapFocus={!editor}
            closeOnEscape={!editor}
            closeOnClickOutside={!editor}
            title="Brain access"
            size="lg"
            centered
          >
            {content}
          </Modal>
        </>
      )}
      {editor && canEdit && !access.error && (
        <Modal
          opened
          onClose={() => !action.isPending && setEditor(null)}
          title={
            editor.kind === "grant"
              ? editor.target
                ? "Edit member"
                : "Add member"
              : editor.kind === "remove"
                ? "Remove direct access"
                : editor.kind === "owner"
                  ? "Transfer ownership"
                  : editor.kind === "group"
                    ? "Add organization group"
                    : "Remove organization group"
          }
          size="md"
          closeOnEscape={!action.isPending}
          closeOnClickOutside={!action.isPending}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              action.mutate(editor);
            }}
          >
            <Stack>
              {editor.kind === "grant" && (
                <>
                  <TextInput
                    label="Account username"
                    value={username}
                    disabled={!!editor.target || action.isPending}
                    onChange={(e) => setUsername(e.currentTarget.value)}
                    required
                    maxLength={120}
                  />
                  <Select
                    label="Direct role"
                    value={role}
                    data={roles}
                    onChange={(v) => setRole(v ?? "reader")}
                    disabled={action.isPending}
                  />
                  {member?.groups.length ? (
                    <Text size="xs" c="dimmed">
                      Group access remains independent of this role.
                    </Text>
                  ) : null}
                </>
              )}
              {editor.kind === "group" && (
                <>
                  <TextInput
                    label="Group name or ID"
                    value={group}
                    onChange={(e) => setGroup(e.currentTarget.value)}
                    required
                    maxLength={200}
                  />
                  <Select
                    label="Group role"
                    value={groupRole}
                    data={roles}
                    onChange={(v) => setGroupRole(v ?? "reader")}
                  />
                </>
              )}
              {editor.kind === "remove" && (
                <Text>
                  Remove the direct grant for {member?.account.username}?{" "}
                  {member?.owner || member?.groups.length
                    ? "Owner or group permissions will still apply."
                    : "This removes their direct Brain access."}
                  {editingSelf && " You may lose your own access."}
                </Text>
              )}
              {editor.kind === "ungroup" && (
                <Text>
                  Remove this organization's grant to the Brain? Direct grants
                  remain.
                </Text>
              )}
              {editor.kind === "owner" && (
                <>
                  <Select
                    label="New owner"
                    value={nextOwner}
                    onChange={setNextOwner}
                    data={
                      access.data?.members
                        .filter(
                          (m) => m.account.enabled && m.account.id !== actor,
                        )
                        .map((m) => ({
                          value: m.account.id,
                          label: m.account.username,
                        })) ?? []
                    }
                  />
                  <Text size="sm">
                    Your access will depend on your remaining direct or group
                    grants.
                  </Text>
                </>
              )}
              {action.error && (
                <Alert color="red">{action.error.message}</Alert>
              )}
              <Group justify="flex-end">
                <Button
                  variant="default"
                  disabled={action.isPending}
                  onClick={() => setEditor(null)}
                >
                  Cancel
                </Button>
                <Button
                  type="submit"
                  loading={action.isPending}
                  color={
                    editor.kind === "remove" || editor.kind === "ungroup"
                      ? "red"
                      : undefined
                  }
                  disabled={editor.kind === "owner" && !nextOwner}
                >
                  {editor.kind === "grant"
                    ? "Save direct grant"
                    : editor.kind === "group"
                      ? "Save group mapping"
                      : editor.kind === "owner"
                        ? "Transfer ownership"
                        : "Remove grant"}
                </Button>
              </Group>
            </Stack>
          </form>
        </Modal>
      )}
    </>
  );
}
