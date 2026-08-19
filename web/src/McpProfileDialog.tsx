import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Divider,
  Group,
  Loader,
  Modal,
  MultiSelect,
  Select,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { type McpCatalogue, type McpRights, rightsLabel } from "./McpPanel";
import { useIdempotency } from "./useIdempotency";

type Detail = components["schemas"]["McpProfileDetail"];
type Input = components["schemas"]["McpProfileInput"];
type Grant = components["schemas"]["McpGrant"];
type Props = {
  brain: Brain;
  actor: string;
  id: string;
  catalogue: McpCatalogue;
  environments: { value: string; label: string }[];
  close: () => void;
  saved: () => void;
  stale: (message: string) => void;
};
const noRights = (): McpRights => ({
  use_profile: false,
  manage: false,
  share: false,
});

export function McpProfileDialog(props: Props) {
  const detail = useQuery({
    queryKey: ["mcp", props.brain.id, "profile", props.id],
    enabled: props.id !== "new",
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/profiles/{id}", {
          params: { path: { brain: props.brain.id, id: props.id } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  return (
    <Modal
      opened
      onClose={props.close}
      title={
        props.id === "new" ? "Create execution profile" : "Execution profile"
      }
      size="xl"
    >
      {detail.error ? (
        <Alert color="red">
          {detail.error.message}
          <Button
            size="xs"
            variant="light"
            onClick={() => void detail.refetch()}
          >
            Reload profile
          </Button>
        </Alert>
      ) : props.id !== "new" && !detail.data ? (
        <Loader />
      ) : (
        <ProfileContents {...props} detail={detail.data} />
      )}
    </Modal>
  );
}

function ProfileContents({ detail, ...props }: Props & { detail?: Detail }) {
  const profile = detail?.profile;
  const current = props.catalogue.profiles.find((p) => p.id === props.id);
  const canManage =
    props.id === "new"
      ? props.catalogue.can_configure
      : !!profile?.rights.manage && !!current?.rights.manage;
  const canShare = !!profile?.rights.share && !!current?.rights.share;
  const [input, setInput] = useState<Input>(() => ({
    name: profile?.name ?? "",
    description: profile?.description ?? "",
    environment_id: profile?.environment_id ?? null,
    connection_ids: profile?.connection_ids ?? [],
    enabled: profile?.enabled ?? true,
    base_revision: profile?.revision ?? null,
  }));
  const command = useIdempotency();
  useEffect(() => {
    if (
      input.base_revision &&
      ((current && current.revision !== input.base_revision) ||
        (profile && profile.revision !== input.base_revision))
    )
      props.stale(
        "This profile configuration changed. Reopen it before editing.",
      );
  }, [input.base_revision, current, profile, props.stale]);
  const save = useMutation({
    mutationFn: async () => {
      if (props.id === "new")
        return result(
          await client.POST("/api/brains/{brain}/mcp/profiles", {
            params: { path: { brain: props.brain.id } },
            body: input,
            headers: { "Idempotency-Key": command.forInput(input) },
          }),
        );
      return result(
        await client.PUT("/api/brains/{brain}/mcp/profiles/{id}", {
          params: { path: { brain: props.brain.id, id: props.id } },
          body: input,
        }),
      );
    },
    onSuccess: () => {
      props.close();
      props.saved();
    },
  });
  const patch = (value: Partial<Input>) => {
    setInput((v) => ({ ...v, ...value }));
    save.reset();
  };
  const compatible = props.catalogue.connections.filter(
    (c) => !c.environment_id || c.environment_id === input.environment_id,
  );
  return (
    <Stack>
      <Text size="sm">
        Use, Manage and Share are independent. Creation and Brain administration
        do not grant Use.
      </Text>
      {profile && (
        <>
          <Title order={4}>{profile.name}</Title>
          <Text>
            Your rights: {rightsLabel(current?.rights ?? profile.rights)}
          </Text>
          <Text size="sm">
            {props.environments.find((e) => e.value === profile.environment_id)
              ?.label ?? "Brain-wide"}{" "}
            · {profile.enabled ? "Enabled" : "Disabled"}
          </Text>
        </>
      )}
      {canManage ? (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            save.mutate();
          }}
        >
          <Stack>
            <TextInput
              label="Profile name"
              required
              value={input.name}
              maxLength={120}
              onChange={(e) => patch({ name: e.currentTarget.value })}
            />
            <TextInput
              label="Profile description"
              value={input.description}
              maxLength={2000}
              onChange={(e) => patch({ description: e.currentTarget.value })}
            />
            <Select
              label="Profile environment"
              clearable
              placeholder="Brain-wide"
              data={props.environments}
              value={input.environment_id}
              onChange={(v) => patch({ environment_id: v })}
            />
            <MultiSelect
              label="Profile connections"
              searchable
              clearable
              value={input.connection_ids}
              maxValues={20}
              data={props.catalogue.connections.map((c) => ({
                value: c.id,
                label: c.name,
                disabled: !compatible.some((p) => p.id === c.id),
              }))}
              onChange={(v) => patch({ connection_ids: v })}
            />
            {input.connection_ids.some(
              (id) => !compatible.some((c) => c.id === id),
            ) && (
              <Alert color="yellow">
                Remove connections that do not match this environment before
                saving.
              </Alert>
            )}
            <Checkbox
              label="Profile enabled"
              checked={input.enabled}
              onChange={(e) => patch({ enabled: e.currentTarget.checked })}
            />
            {save.error && (
              <Alert color="red" title="Profile not saved">
                {save.error.message}
              </Alert>
            )}
            <Group justify="end">
              <Button
                type="submit"
                disabled={props.brain.archived}
                loading={save.isPending}
              >
                Save profile
              </Button>
            </Group>
          </Stack>
        </form>
      ) : (
        profile && (
          <Stack gap="xs">
            <Text size="sm">{profile.description}</Text>
            {profile.connection_ids.map((id) => (
              <Text size="sm" key={id}>
                {props.catalogue.connections.find((c) => c.id === id)?.name ??
                  "Unavailable connection"}
              </Text>
            ))}
          </Stack>
        )
      )}
      {detail && canShare && (
        <>
          <Divider />
          <GrantControls {...props} detail={detail} />
        </>
      )}
      <Group justify="end">
        <Button variant="default" onClick={props.close}>
          Close
        </Button>
      </Group>
    </Stack>
  );
}

function GrantControls({ detail, ...props }: Props & { detail: Detail }) {
  const cache = useQueryClient();
  const [subjectKind, setSubjectKind] = useState("account");
  const [subject, setSubject] = useState("");
  const [rights, setRights] = useState<McpRights>(noRights);
  const [notice, setNotice] = useState<string | null>(null);
  const options = useQuery({
    queryKey: ["auth-options"],
    queryFn: async () => result(await client.GET("/api/auth/options")),
  });
  const save = useMutation({
    mutationFn: async (remove?: string) => {
      if (remove)
        return result(
          await client.DELETE(
            "/api/brains/{brain}/mcp/profiles/{id}/grants/{grant}",
            {
              params: {
                path: { brain: props.brain.id, id: props.id, grant: remove },
              },
            },
          ),
        );
      return result(
        await client.PUT("/api/brains/{brain}/mcp/profiles/{id}/grants", {
          params: { path: { brain: props.brain.id, id: props.id } },
          body: {
            username: subjectKind === "account" ? subject : null,
            group_name: subjectKind === "group" ? subject : null,
            rights,
          },
        }),
      );
    },
    onSuccess: (response) => {
      if (!response.profile) {
        props.stale(
          "Your last profile right was removed. The inspector was closed.",
        );
        return;
      }
      cache.setQueryData(
        ["mcp", props.brain.id, "profile", props.id],
        response.profile,
      );
      setNotice(
        "Grant updated. Effective rights below include remaining direct, group and administrator authority.",
      );
      setSubject("");
      setRights(noRights());
      props.saved();
    },
  });
  const chooseGrant = (grant: Grant) => {
    setSubjectKind(grant.account_id ? "account" : "group");
    setSubject(grant.username ?? grant.group_name ?? "");
    setRights(grant.rights);
    save.reset();
  };
  const me = detail.effective_members.find((m) => m.account_id === props.actor);
  const mine = detail.grants?.find((g) => g.account_id === props.actor);
  return (
    <Stack>
      <Title order={4}>Profile permissions</Title>
      <Text size="sm" c="dimmed">
        Every recipient also needs current Brain access. Group access uses the
        existing organization membership expiry; removing one grant can leave
        another effective.
      </Text>
      {notice && <Alert color="teal">{notice}</Alert>}
      {detail.grants?.length === 0 && (
        <Text size="sm">
          No explicit profile grants yet. Administrators can manage and share,
          but cannot use this profile.
        </Text>
      )}
      {detail.grants?.map((g) => (
        <Card withBorder key={g.id} data-testid="mcp-grant">
          <Group justify="space-between">
            <Text fw={600}>{g.username ?? g.group_name}</Text>
            <Badge>{g.account_id ? "Account" : "Organization group"}</Badge>
          </Group>
          <Text size="sm">{rightsLabel(g.rights)}</Text>
          {g.issuer && (
            <Text size="xs" c="dimmed">
              Issuer: {g.issuer}
            </Text>
          )}
          <Group mt="xs">
            <Button
              size="xs"
              variant="light"
              disabled={props.brain.archived || save.isPending}
              onClick={() => chooseGrant(g)}
            >
              Edit grant
            </Button>
            <Button
              size="xs"
              variant="subtle"
              color="red"
              disabled={props.brain.archived || save.isPending}
              onClick={() => save.mutate(g.id)}
            >
              Remove grant
            </Button>
          </Group>
        </Card>
      ))}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate(undefined);
        }}
      >
        <Stack gap="sm">
          <Select
            label="Grant recipient type"
            value={subjectKind}
            data={[
              { value: "account", label: "Enrolled account" },
              {
                value: "group",
                label: "Organization group",
                disabled: !options.data?.oidc_configured,
              },
            ]}
            onChange={(v) => {
              setSubjectKind(v ?? "account");
              setSubject("");
              setRights(noRights());
            }}
          />
          <TextInput
            label={subjectKind === "account" ? "Grant username" : "Grant group"}
            value={subject}
            required
            maxLength={subjectKind === "account" ? 120 : 200}
            onChange={(e) => setSubject(e.currentTarget.value)}
          />
          <Group>
            <Checkbox
              label="Use profile"
              checked={rights.use_profile}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setRights((v) => ({ ...v, use_profile: checked }));
              }}
            />
            <Checkbox
              label="Manage profile"
              checked={rights.manage}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setRights((v) => ({ ...v, manage: checked }));
              }}
            />
            <Checkbox
              label="Share profile"
              checked={rights.share}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setRights((v) => ({ ...v, share: checked }));
              }}
            />
          </Group>
          <Text size="xs" c="dimmed">
            Saving with all rights cleared removes that recipient's grant.
          </Text>
          {save.error && (
            <Alert color="red" title="Grant not saved">
              {save.error.message}
            </Alert>
          )}
          <Group justify="end">
            {me && (
              <Button
                variant="light"
                disabled={props.brain.archived}
                onClick={() => {
                  setSubjectKind("account");
                  setSubject(me.username);
                  setRights({
                    ...(mine?.rights ?? noRights()),
                    use_profile: true,
                  });
                }}
              >
                Select myself for Use
              </Button>
            )}
            <Button
              type="submit"
              loading={save.isPending}
              disabled={props.brain.archived}
            >
              Save grant
            </Button>
          </Group>
        </Stack>
      </form>
      {detail.effective_members.length > 0 && (
        <>
          <Title order={5}>Effective account rights</Title>
          {detail.effective_members.map((m) => (
            <Card
              withBorder
              key={m.account_id}
              data-testid="mcp-effective-member"
            >
              <Text fw={600}>{m.username}</Text>
              <Text size="sm">
                {rightsLabel(m.rights)} ·{" "}
                {m.enabled ? "Account enabled" : "Account disabled"}
              </Text>
              <Text size="xs">
                Brain role: {m.brain_role ?? "No current access"} · Direct:{" "}
                {rightsLabel(m.direct_rights)}
              </Text>
              <Text size="xs">
                Current groups: {m.groups.join(", ") || "None"}
              </Text>
              {m.membership_until && (
                <Text size="xs">
                  Membership valid until{" "}
                  {new Date(m.membership_until).toLocaleString()}
                </Text>
              )}
            </Card>
          ))}
        </>
      )}
    </Stack>
  );
}
