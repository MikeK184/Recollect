import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Group,
  Popover,
  Select,
  Switch,
  TextInput,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BookOpen, Pencil, Plus, Users, X } from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpCatalogue, McpRights } from "./McpPanel";
import { McpPermissionIcons } from "./components/McpPermissionIcons";
import { McpConnectorIcon } from "./components/McpConnectorIcon";
import { useIdempotency } from "./useIdempotency";
import {
  inheritedIndicators,
  projectMemberPermissions,
} from "./features/connections/member-permissions";
import "./features/connections/inline-management.css";

type Detail = components["schemas"]["McpProfileDetail"];
type Input = components["schemas"]["McpProfileInput"];
type GrantInput = components["schemas"]["McpGrantInput"];
const denied = (): McpRights => ({
  use_profile: false,
  manage: false,
  share: false,
});
const grantKey = (grant: GrantInput) =>
  `${grant.username ? "account" : "group"}:${grant.username ?? grant.group_name}`;
const profileInput = (profile?: Detail["profile"]): Input => ({
  name: profile?.name ?? "",
  description: profile?.description ?? "",
  enabled: profile?.enabled ?? true,
  environment_id: profile?.environment_id ?? null,
  connection_ids: profile?.connection_ids ?? [],
  base_revision: profile?.revision ?? null,
});
const configuration = (input: Input) =>
  JSON.stringify({ ...input, base_revision: null });

export function McpProfileCard({
  brain,
  actor,
  id,
  catalogue,
  environments,
  saved,
  close,
  tools,
}: {
  brain: Brain;
  actor: string;
  id: string;
  catalogue: McpCatalogue;
  environments: { value: string; label: string }[];
  saved: () => void;
  close?: () => void;
  tools: (id: string) => void;
}) {
  const cache = useQueryClient();
  const [savedId, setSavedId] = useState(id);
  const [editing, setEditing] = useState(id === "new");
  const [input, setInput] = useState<Input>(profileInput);
  const baseline = useRef<Input>(profileInput());
  const [pendingGrants, setPendingGrants] = useState<
    Record<string, GrantInput>
  >({});
  const [progress, setProgress] = useState<string[]>([]);
  const [adding, setAdding] = useState(false);
  const [showDescription, setShowDescription] = useState(false);
  const [subjectKind, setSubjectKind] = useState("account");
  const [subject, setSubject] = useState("");
  const [newRights, setNewRights] = useState<McpRights>({
    ...denied(),
    use_profile: true,
  });
  const command = useIdempotency();
  const queryKey = ["mcp", brain.id, "profile", savedId];
  const detail = useQuery({
    queryKey,
    enabled: savedId !== "new",
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/mcp/profiles/{id}", {
          params: { path: { brain: brain.id, id: savedId } },
          signal,
        }),
      ),
    refetchInterval: 5000,
    retry: false,
    gcTime: 0,
  });
  const options = useQuery({
    queryKey: ["auth-options"],
    enabled: editing,
    queryFn: async () => result(await client.GET("/api/auth/options")),
  });
  const current =
    catalogue.profiles.find((p) => p.id === savedId) ?? detail.data?.profile;
  const profile = detail.data?.profile ?? current;
  const canManage =
    savedId === "new"
      ? catalogue.can_configure
      : !!profile?.rights.manage && !!current?.rights.manage;
  const canShare = !!profile?.rights.share && !!current?.rights.share;
  const stale =
    editing &&
    !!baseline.current.base_revision &&
    !!detail.data &&
    baseline.current.base_revision !== detail.data.profile.revision;
  useEffect(() => {
    if (editing && savedId !== "new" && !canManage) {
      setInput(baseline.current);
    }
    if (savedId !== "new" && (!canShare || brain.archived)) {
      setPendingGrants({});
      setAdding(false);
      setSubject("");
    }
    if (
      editing &&
      (brain.archived || (savedId !== "new" && !canManage && !canShare))
    ) {
      setEditing(false);
      setInput(baseline.current);
    }
  }, [editing, savedId, canManage, canShare, brain.archived]);
  const stage = (grant: GrantInput) =>
    setPendingGrants((values) => ({ ...values, [grantKey(grant)]: grant }));
  const effective = detail.data?.effective_members ?? [];
  const groups = detail.data?.grants?.filter((grant) => grant.group_name) ?? [];
  const ambiguousGroup = (name: string) =>
    groups.filter((g) => g.group_name === name).length > 1;
  const groupRights = (name: string) =>
    pendingGrants[`group:${name}`]?.rights ??
    groups.find((g) => g.group_name === name)?.rights ??
    denied();
  const save = useMutation({
    mutationFn: async () => {
      if (brain.archived || detail.error)
        throw new Error("Reload current access before saving.");
      let target = savedId;
      let active = detail.data;
      const outcomes = [...progress];
      const recordSaved = (outcome: string) => {
        if (!outcomes.includes(outcome)) outcomes.push(outcome);
        setProgress([...outcomes]);
      };
      const configChanged =
        configuration(input) !== configuration(baseline.current);
      if (configChanged || target === "new") {
        if (!canManage)
          throw new Error(
            "Manage permission is required for tool-group settings.",
          );
        if (stale)
          throw new Error(
            "This group changed elsewhere. Your draft is retained; reload before saving configuration.",
          );
        active =
          target === "new"
            ? result(
                await client.POST("/api/brains/{brain}/mcp/profiles", {
                  params: { path: { brain: brain.id } },
                  body: input,
                  headers: { "Idempotency-Key": command.forInput(input) },
                }),
              )
            : result(
                await client.PUT("/api/brains/{brain}/mcp/profiles/{id}", {
                  params: { path: { brain: brain.id, id: target } },
                  body: {
                    ...input,
                    base_revision: baseline.current.base_revision,
                  },
                }),
              );
        target = active.profile.id;
        setSavedId(target);
        baseline.current = profileInput(active.profile);
        setInput(baseline.current);
        cache.setQueryData(["mcp", brain.id, "profile", target], active);
        recordSaved("Group settings saved");
        saved();
      }
      // Apply self-revocation last so it cannot prevent earlier prepared grants.
      const ownUsername = active?.effective_members.find(
        (m) => m.account_id === actor,
      )?.username;
      const grants = Object.entries(pendingGrants).sort(
        ([, a], [, b]) =>
          Number(a.username === ownUsername) -
          Number(b.username === ownUsername),
      );
      for (const [key, grant] of grants) {
        if (!active?.profile.rights.share)
          throw new Error(
            "Share permission is required for the remaining people changes.",
          );
        const response = result(
          await client.PUT("/api/brains/{brain}/mcp/profiles/{id}/grants", {
            params: { path: { brain: brain.id, id: target } },
            body: grant,
          }),
        );
        setPendingGrants((values) => {
          const remaining = { ...values };
          delete remaining[key];
          return remaining;
        });
        recordSaved(`${grant.username ?? grant.group_name} permissions saved`);
        active = response.profile ?? undefined;
        if (active)
          cache.setQueryData(["mcp", brain.id, "profile", target], active);
        saved();
        if (!active && key !== grants.at(-1)?.[0])
          throw new Error(
            "Your access was removed. Remaining changes were not saved.",
          );
      }
    },
    onSuccess: () => {
      setEditing(false);
      setAdding(false);
      setPendingGrants({});
      setProgress([]);
      if (id === "new") close?.();
      saved();
    },
  });
  function edit() {
    if (!profile) return;
    baseline.current = profileInput(profile);
    setInput(baseline.current);
    setPendingGrants({});
    setProgress([]);
    save.reset();
    setShowDescription(!!profile.description);
    setEditing(true);
  }
  function cancel() {
    setEditing(false);
    setPendingGrants({});
    setProgress([]);
    setAdding(false);
    save.reset();
    if (id === "new") close?.();
  }
  const compatible = catalogue.connections.filter(
    (c) => !c.environment_id || c.environment_id === input.environment_id,
  );
  const connectionIds = editing
    ? input.connection_ids
    : (profile?.connection_ids ?? []);
  const description = editing ? input.description : profile?.description;
  const name = editing ? input.name : (profile?.name ?? "Tool group");
  const addedPeople = Object.values(pendingGrants).filter(
    (g) =>
      !effective.some((m) => m.username === g.username) &&
      !groups.some((m) => m.group_name === g.group_name),
  );
  return (
    <article
      className={`mcp-tool-group-card rc-enter${editing ? " is-editing" : ""}`}
      data-testid="mcp-profile"
      id={`tool-group-${savedId}`}
      aria-label={`Tool group ${profile?.name ?? "new"}`}
    >
      <fieldset disabled={save.isPending} className="mcp-inline-fields">
        <header className="mcp-tool-group-heading">
          <div className="management-icon connection-icon">
            <BookOpen aria-hidden="true" />
          </div>
          <div className="mcp-tool-group-identity">
            {editing && canManage ? (
              <TextInput
                size="xs"
                aria-label="Tool group name"
                placeholder="Tool group name"
                required
                maxLength={120}
                className="mcp-inline-title"
                value={name}
                onChange={(e) =>
                  setInput({ ...input, name: e.currentTarget.value })
                }
              />
            ) : (
              <h2>{name}</h2>
            )}
            <div className="mcp-tool-group-meta">
              {editing && canManage && environments.length > 0 ? (
                <Select
                  className="mcp-inline-environment"
                  aria-label="Tool group environment"
                  size="xs"
                  clearable
                  disabled={save.isPending || brain.archived}
                  placeholder="Brain-wide"
                  value={input.environment_id}
                  data={environments}
                  onChange={(value) =>
                    setInput({ ...input, environment_id: value })
                  }
                />
              ) : (
                <Badge
                  className="mcp-environment-badge"
                  variant="light"
                  color="brand"
                >
                  {environments.find(
                    (e) =>
                      e.value ===
                      (editing
                        ? input.environment_id
                        : profile?.environment_id),
                  )?.label ?? "Brain-wide"}
                </Badge>
              )}
              <Badge
                className="mcp-connection-count"
                variant="light"
                color="gray"
              >
                {connectionIds.length} MCP
                {connectionIds.length === 1 ? "" : "s"}
              </Badge>
            </div>
          </div>
          {editing && canManage ? (
            <Switch
              size="xs"
              label="Enabled"
              checked={input.enabled}
              disabled={brain.archived || save.isPending}
              onChange={(e) =>
                setInput({ ...input, enabled: e.currentTarget.checked })
              }
            />
          ) : (
            <Badge variant="light" color={profile?.enabled ? "brand" : "gray"}>
              {profile?.enabled ? "Enabled" : "Disabled"}
            </Badge>
          )}
          {!editing && (canManage || canShare) && (
            <Button
              variant="subtle"
              size="compact-sm"
              leftSection={<Pencil size={15} />}
              aria-label={`Edit ${profile?.name ?? "tool group"}`}
              disabled={brain.archived || !detail.data || !!detail.error}
              onClick={edit}
            >
              Edit
            </Button>
          )}
        </header>
        {editing && canManage && (showDescription || description) ? (
          <TextInput
            size="xs"
            className="mcp-tool-group-description"
            aria-label="Tool group description"
            placeholder="Description (optional)"
            value={input.description}
            maxLength={2000}
            onChange={(e) =>
              setInput({ ...input, description: e.currentTarget.value })
            }
          />
        ) : editing && canManage ? (
          <Button
            className="mcp-add-description"
            variant="subtle"
            size="compact-xs"
            leftSection={<Plus size={13} />}
            onClick={() => setShowDescription(true)}
          >
            Add description
          </Button>
        ) : description ? (
          <p className="mcp-tool-group-description">{description}</p>
        ) : null}
        {detail.error ? (
          <Alert color="red" title="Tool group unavailable">
            {detail.error.message}
            <Button
              variant="subtle"
              size="xs"
              onClick={() => void detail.refetch()}
            >
              Reload tool group
            </Button>
          </Alert>
        ) : (
          <>
            <div className="mcp-tool-group-sections">
              <section aria-label="MCP connections">
                {editing && (
                  <div className="mcp-card-section-heading">
                    <h3>MCPs</h3>
                    {editing && canManage && (
                      <span className="management-muted">
                        {connectionIds.length}/20
                      </span>
                    )}
                  </div>
                )}
                {connectionIds.map((connectionId) => {
                  const connection = catalogue.connections.find(
                    (c) => c.id === connectionId,
                  );
                  const definition = catalogue.definitions.find(
                    (d) => d.key === connection?.definition_key,
                  );
                  return (
                    <div className="mcp-card-connection" key={connectionId}>
                      <McpConnectorIcon definition={definition} size="small" />
                      <div>
                        <strong>
                          {connection?.name ?? "Unavailable connection"}
                        </strong>
                        <small>{definition?.tool_count ?? "—"} tools</small>
                      </div>
                      {editing && canManage && (
                        <button
                          type="button"
                          className="mcp-inline-remove"
                          title={`Remove ${connection?.name ?? "connection"}`}
                          aria-label={`Remove ${connection?.name ?? "connection"}`}
                          disabled={save.isPending}
                          onClick={() =>
                            setInput({
                              ...input,
                              connection_ids: connectionIds.filter(
                                (c) => c !== connectionId,
                              ),
                            })
                          }
                        >
                          <X size={16} />
                        </button>
                      )}
                    </div>
                  );
                })}
                {!connectionIds.length && (
                  <p className="management-muted">No MCPs added.</p>
                )}
                {editing && canManage && (
                  <Select
                    size="xs"
                    label="Add MCP"
                    searchable
                    clearable
                    placeholder="Choose a connection"
                    value={null}
                    disabled={
                      input.connection_ids.length >= 20 ||
                      save.isPending ||
                      brain.archived
                    }
                    data={catalogue.connections
                      .filter((c) => !input.connection_ids.includes(c.id))
                      .map((c) => ({
                        value: c.id,
                        label: c.name,
                        disabled: !compatible.some((p) => p.id === c.id),
                      }))}
                    onChange={(value) =>
                      value &&
                      setInput({
                        ...input,
                        connection_ids: [...input.connection_ids, value],
                      })
                    }
                  />
                )}
                {editing &&
                  input.connection_ids.some(
                    (c) => !compatible.some((p) => p.id === c),
                  ) && (
                    <Alert color="yellow">
                      Remove MCPs from other environments before saving.
                    </Alert>
                  )}
                {!editing && (
                  <Button
                    variant="subtle"
                    className="mcp-card-tools-action"
                    size="compact-sm"
                    disabled={
                      !profile?.rights.use_profile ||
                      !profile.enabled ||
                      brain.archived
                    }
                    onClick={() => profile && tools(profile.id)}
                    aria-label="Tools & testing"
                  >
                    Tools & testing →
                  </Button>
                )}
              </section>
              <section aria-label="People and groups">
                {editing && (
                  <div className="mcp-card-section-heading">
                    <h3>People</h3>
                    {editing && (canShare || savedId === "new") && (
                      <Button
                        variant="subtle"
                        size="compact-xs"
                        leftSection={<Plus size={14} />}
                        disabled={save.isPending}
                        onClick={() => setAdding(!adding)}
                      >
                        Add person
                      </Button>
                    )}
                  </div>
                )}
                {editing && (
                  <div className="mcp-permission-legend" aria-hidden="true">
                    <span>Use</span>
                    <span>Manage</span>
                    <span>Share</span>
                  </div>
                )}
                {detail.isPending && savedId !== "new" && (
                  <p className="management-muted" role="status">
                    Loading permissions…
                  </p>
                )}
                {effective.map((member) => (
                  <MemberPermissions
                    key={member.account_id}
                    member={member}
                    actor={actor}
                    grants={detail.data?.grants ?? []}
                    drafts={pendingGrants}
                    editing={editing && canShare && !brain.archived}
                    disabled={save.isPending || brain.archived}
                    change={(rights) =>
                      stage({
                        username: member.username,
                        group_name: null,
                        rights,
                      })
                    }
                  />
                ))}
                {!effective.length && profile && !canShare && (
                  <div
                    className="mcp-card-person"
                    data-testid="mcp-own-permissions"
                  >
                    <span className="mcp-person-avatar" aria-hidden="true">
                      Y
                    </span>
                    <strong>You</strong>
                    <McpPermissionIcons subject="you" rights={profile.rights} />
                  </div>
                )}
                {groups.map((grant) => (
                  <div
                    className="mcp-card-person"
                    key={grant.id}
                    data-testid="mcp-grant"
                  >
                    <span className="mcp-person-avatar" aria-hidden="true">
                      <Users size={15} />
                    </span>
                    <strong>{grant.group_name}</strong>
                    <small
                      className="mcp-access-source"
                      title={grant.issuer ?? undefined}
                    >
                      Group
                    </small>
                    <McpPermissionIcons
                      subject={grant.group_name!}
                      rights={
                        editing && !ambiguousGroup(grant.group_name!)
                          ? groupRights(grant.group_name!)
                          : grant.rights
                      }
                      disabled={brain.archived || save.isPending}
                      onChange={
                        editing &&
                        canShare &&
                        !ambiguousGroup(grant.group_name!)
                          ? (rights) =>
                              stage({
                                username: null,
                                group_name: grant.group_name!,
                                rights,
                              })
                          : undefined
                      }
                    />
                  </div>
                ))}
                {editing &&
                  !brain.archived &&
                  (canShare || (savedId === "new" && canManage)) &&
                  addedPeople.map((grant) => (
                    <div className="mcp-card-person" key={grantKey(grant)}>
                      <span className="mcp-person-avatar" aria-hidden="true">
                        {grant.username ? (
                          grant.username.slice(0, 1).toUpperCase()
                        ) : (
                          <Users size={15} />
                        )}
                      </span>
                      <strong>{grant.username ?? grant.group_name}</strong>
                      <small className="mcp-pending-grant">Pending</small>
                      <McpPermissionIcons
                        subject={grant.username ?? grant.group_name!}
                        rights={grant.rights}
                        disabled={save.isPending}
                        onChange={(rights) => stage({ ...grant, rights })}
                      />
                    </div>
                  ))}
                {!effective.length &&
                  !groups.length &&
                  !addedPeople.length &&
                  (!profile || canShare) &&
                  !detail.isPending && (
                    <p className="management-muted">No people visible.</p>
                  )}
                {adding && editing && (canShare || savedId === "new") && (
                  <div className="mcp-add-person">
                    <Select
                      size="xs"
                      aria-label="Recipient type"
                      disabled={save.isPending || brain.archived}
                      value={subjectKind}
                      data={[
                        { value: "account", label: "Account" },
                        {
                          value: "group",
                          label: "Organization group",
                          disabled: !options.data?.oidc_configured,
                        },
                      ]}
                      onChange={(value) => {
                        setSubjectKind(value ?? "account");
                        setSubject("");
                      }}
                    />
                    <TextInput
                      size="xs"
                      label={
                        subjectKind === "account" ? "Username" : "Group name"
                      }
                      value={subject}
                      maxLength={subjectKind === "account" ? 120 : 200}
                      onChange={(e) => setSubject(e.currentTarget.value)}
                    />
                    <Group justify="space-between">
                      <McpPermissionIcons
                        subject={subject || "new person"}
                        rights={newRights}
                        onChange={setNewRights}
                      />
                      <Button
                        size="compact-sm"
                        disabled={
                          !subject.trim() ||
                          save.isPending ||
                          (subjectKind === "group" &&
                            ambiguousGroup(subject.trim()))
                        }
                        onClick={() => {
                          stage({
                            username:
                              subjectKind === "account" ? subject.trim() : null,
                            group_name:
                              subjectKind === "group" ? subject.trim() : null,
                            rights: newRights,
                          });
                          setSubject("");
                          setAdding(false);
                        }}
                      >
                        Add
                      </Button>
                    </Group>
                  </div>
                )}
              </section>
            </div>
            {stale && (
              <Alert color="yellow" title="Configuration changed">
                Your draft is retained. Reload replaces the draft with current
                settings.
                <Button
                  variant="subtle"
                  size="xs"
                  disabled={save.isPending}
                  onClick={() => {
                    if (detail.data) {
                      baseline.current = profileInput(detail.data.profile);
                      setInput(baseline.current);
                      save.reset();
                    }
                  }}
                >
                  Reload settings
                </Button>
              </Alert>
            )}
            {save.error && (
              <Alert
                color="red"
                title={
                  progress.length ? "Some changes saved" : "Changes not saved"
                }
              >
                {progress.length > 0 && (
                  <ul>
                    {progress.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                )}
                {save.error.message} Remaining edits are retained.
              </Alert>
            )}
          </>
        )}
        {editing && (
          <footer className="mcp-tool-group-footer">
            {editing ? (
              <Group gap="xs">
                <Button
                  variant="default"
                  size="xs"
                  disabled={save.isPending}
                  onClick={cancel}
                >
                  Cancel
                </Button>
                <Button
                  size="xs"
                  loading={save.isPending}
                  disabled={
                    brain.archived ||
                    !!detail.error ||
                    (!canManage && !canShare) ||
                    (canManage &&
                      (!input.name.trim() ||
                        input.connection_ids.some(
                          (c) => !compatible.some((p) => p.id === c),
                        )))
                  }
                  onClick={() => save.mutate()}
                >
                  {save.error && progress.length
                    ? "Retry remaining changes"
                    : "Save changes"}
                </Button>
              </Group>
            ) : null}
          </footer>
        )}
      </fieldset>
    </article>
  );
}

function MemberPermissions({
  member,
  actor,
  grants,
  drafts,
  editing,
  disabled,
  change,
}: {
  member: components["schemas"]["McpEffectiveMember"];
  actor: string;
  grants: components["schemas"]["McpGrant"][];
  drafts: Record<string, GrantInput>;
  editing: boolean;
  disabled: boolean;
  change: (rights: McpRights) => void;
}) {
  const [opened, setOpened] = useState(false);
  const projected = projectMemberPermissions(
    member,
    grants,
    editing ? drafts : {},
  );
  const rights = editing ? projected.effective : member.rights;
  // Read-only readers may not receive all grants; authoritative effective rights
  // remain the source of truth outside a Share editor's draft projection.
  const inherited = editing
    ? projected.inherited
    : inheritedIndicators(member, projected.inherited);
  const source = !member.enabled
    ? "Disabled"
    : !member.brain_role
      ? "No Brain access"
      : member.brain_role === "admin"
        ? "Admin"
        : member.groups.length
          ? "Group"
          : "Direct";
  useEffect(() => setOpened(false), [editing, disabled]);
  return (
    <div className="mcp-card-person" data-testid="mcp-effective-member">
      <span className="mcp-person-avatar" aria-hidden="true">
        {member.username.slice(0, 1).toUpperCase()}
      </span>
      <strong>
        {member.username}
        {member.account_id === actor && <small> (you)</small>}
      </strong>
      <Popover
        opened={opened}
        onDismiss={() => setOpened(false)}
        width={270}
        position="bottom-end"
        withArrow
        trapFocus
        returnFocus
        classNames={{ dropdown: "mcp-access-popover" }}
      >
        <Popover.Target>
          <button
            type="button"
            className="mcp-access-source"
            aria-label={`Access sources for ${member.username}`}
            onClick={() => setOpened(!opened)}
            disabled={disabled}
          >
            {source}
          </button>
        </Popover.Target>
        <Popover.Dropdown>
          <strong>{member.username}</strong>
          <dl>
            <div>
              <dt>Brain access</dt>
              <dd>{member.brain_role ?? "None"}</dd>
            </div>
            {!member.enabled && (
              <div>
                <dt>Account</dt>
                <dd>Disabled</dd>
              </div>
            )}
            {member.brain_role === "admin" && (
              <div>
                <dt>Admin grants</dt>
                <dd>Manage · Share</dd>
              </div>
            )}
            {member.groups.length > 0 && (
              <div>
                <dt>Groups</dt>
                <dd>{member.groups.join(", ")}</dd>
              </div>
            )}
            {member.membership_until && (
              <div>
                <dt>Membership until</dt>
                <dd>{new Date(member.membership_until).toLocaleString()}</dd>
              </div>
            )}
          </dl>
          {projected.ambiguousGroups.length > 0 && (
            <p className="management-muted">
              Multiple providers for {projected.ambiguousGroups.join(", ")}.
              Current access is shown; edit direct grants below.
            </p>
          )}
          <div className="mcp-source-direct">
            <span>Direct grant</span>
            <McpPermissionIcons
              subject={`${member.username} direct`}
              rights={projected.direct}
              disabled={disabled || !projected.available}
              onChange={editing ? change : undefined}
            />
          </div>
          {editing && Object.values(projected.direct).some(Boolean) && (
            <Button
              variant="subtle"
              size="compact-xs"
              color="red"
              disabled={disabled}
              onClick={() => change(denied())}
            >
              Clear direct grant
            </Button>
          )}
        </Popover.Dropdown>
      </Popover>
      <McpPermissionIcons
        subject={member.username}
        rights={rights}
        inherited={inherited}
        disabled={
          disabled ||
          !projected.available ||
          projected.ambiguousGroups.length > 0
        }
        onChange={
          editing
            ? (next) =>
                change({
                  use_profile: inherited.use_profile
                    ? projected.direct.use_profile
                    : next.use_profile,
                  manage: inherited.manage
                    ? projected.direct.manage
                    : next.manage,
                  share: inherited.share ? projected.direct.share : next.share,
                })
            : undefined
        }
      />
    </div>
  );
}
