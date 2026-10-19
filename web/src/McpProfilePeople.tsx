import { useEffect, useState } from "react";
import { Button, Group, Popover, Select, TextInput } from "@mantine/core";
import { Plus, Users } from "lucide-react";
import type { components } from "./api-schema";
import type { McpRights } from "./McpPanel";
import { McpPermissionIcons } from "./components/McpPermissionIcons";
import {
  inheritedIndicators,
  projectMemberPermissions,
} from "./features/connections/member-permissions";
import {
  denied,
  grantKey,
  type GrantInput,
  type McpProfileCardProps,
  type ProfileState,
} from "./McpProfileState";

// The people and groups with access to a tool group, the staged grant edits and
// the add-person form.
export function McpProfilePeople({
  state,
  props,
}: {
  state: ProfileState;
  props: McpProfileCardProps;
}) {
  const { brain, actor } = props;
  const {
    editing,
    canShare,
    savedId,
    adding,
    setAdding,
    save,
    detail,
    effective,
    pendingGrants,
    stage,
    profile,
    groups,
    ambiguousGroup,
    groupRights,
    addedPeople,
    options,
    subjectKind,
    setSubjectKind,
    subject,
    setSubject,
    newRights,
    setNewRights,
    canManage,
  } = state;
  return (
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
        <div className="mcp-card-person" data-testid="mcp-own-permissions">
          <span className="mcp-person-avatar" aria-hidden="true">
            Y
          </span>
          <strong>You</strong>
          <McpPermissionIcons subject="you" rights={profile.rights} />
        </div>
      )}
      {groups.map((grant) => (
        <div className="mcp-card-person" key={grant.id} data-testid="mcp-grant">
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
              editing && canShare && !ambiguousGroup(grant.group_name!)
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
            label={subjectKind === "account" ? "Username" : "Group name"}
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
                (subjectKind === "group" && ambiguousGroup(subject.trim()))
              }
              onClick={() => {
                stage({
                  username: subjectKind === "account" ? subject.trim() : null,
                  group_name: subjectKind === "group" ? subject.trim() : null,
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
