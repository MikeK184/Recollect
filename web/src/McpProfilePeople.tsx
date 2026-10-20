import { useEffect, useState, type ReactNode } from "react";
import { Plus, Users } from "lucide-react";
import type { components } from "./api-schema";
import type { McpRights } from "./McpPanel";
import {
  Badge,
  Button,
  Popover,
  PopoverContent,
  PopoverTrigger,
  Select,
  TextField,
} from "./components/ui";
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

const rowClass =
  "flex min-h-11 items-center gap-2.5 border-b border-border py-1.5 last:border-b-0";
const nameClass =
  "min-w-0 flex-1 text-label font-medium text-foreground [overflow-wrap:anywhere]";
const sourceClass =
  "inline-flex h-[22px] shrink-0 items-center rounded-full border border-border px-2 text-caption text-muted-foreground";

function Avatar({ children }: { children: ReactNode }) {
  return (
    <span
      className="grid size-[26px] shrink-0 place-items-center rounded-full bg-secondary text-caption font-medium text-foreground-secondary [&_svg]:size-3.5"
      aria-hidden="true"
    >
      {children}
    </span>
  );
}

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
    <section aria-label="People and groups" className="min-w-0 px-4 py-3">
      {editing && (
        <div className="flex min-h-7 items-center justify-between gap-2">
          <h3 className="section-label m-0">People</h3>
          {(canShare || savedId === "new") && (
            <Button
              variant="ghost"
              size="sm"
              disabled={save.isPending}
              aria-expanded={adding}
              onClick={() => setAdding(!adding)}
            >
              <Plus aria-hidden="true" />
              Add person
            </Button>
          )}
        </div>
      )}
      {editing && (
        <p
          className="m-0 pb-1 text-right text-caption text-muted-foreground"
          aria-hidden="true"
        >
          Use · Manage · Share
        </p>
      )}
      {detail.isPending && savedId !== "new" && (
        <p className="m-0 py-2 text-label text-muted-foreground" role="status">
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
        <div className={rowClass} data-testid="mcp-own-permissions">
          <Avatar>Y</Avatar>
          <strong className={nameClass}>You</strong>
          <McpPermissionIcons subject="you" rights={profile.rights} />
        </div>
      )}
      {groups.map((grant) => (
        <div className={rowClass} key={grant.id} data-testid="mcp-grant">
          <Avatar>
            <Users />
          </Avatar>
          <strong className={nameClass}>{grant.group_name}</strong>
          <small className={sourceClass} title={grant.issuer ?? undefined}>
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
          <div className={`${rowClass} animate-enter`} key={grantKey(grant)}>
            <Avatar>
              {grant.username ? (
                grant.username.slice(0, 1).toUpperCase()
              ) : (
                <Users />
              )}
            </Avatar>
            <strong className={nameClass}>
              {grant.username ?? grant.group_name}
            </strong>
            <Badge variant="warning">Pending</Badge>
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
          <p className="m-0 py-2 text-label text-muted-foreground">
            No people visible.
          </p>
        )}
      {adding && editing && (canShare || savedId === "new") && (
        <div className="mt-3 grid animate-enter gap-2.5 rounded-md border border-border bg-muted p-3">
          <Select
            size="sm"
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
          <TextField
            size="sm"
            label={subjectKind === "account" ? "Username" : "Group name"}
            value={subject}
            maxLength={subjectKind === "account" ? 120 : 200}
            onChange={(e) => setSubject(e.currentTarget.value)}
          />
          <div className="flex items-center justify-between gap-2">
            <McpPermissionIcons
              subject={subject || "new person"}
              rights={newRights}
              onChange={setNewRights}
            />
            <Button
              size="sm"
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
          </div>
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
  const facts: { term: string; value: ReactNode }[] = [
    { term: "Brain access", value: member.brain_role ?? "None" },
    ...(!member.enabled ? [{ term: "Account", value: "Disabled" }] : []),
    ...(member.brain_role === "admin"
      ? [{ term: "Admin grants", value: "Manage · Share" }]
      : []),
    ...(member.groups.length > 0
      ? [{ term: "Groups", value: member.groups.join(", ") }]
      : []),
    ...(member.membership_until
      ? [
          {
            term: "Membership until",
            value: (
              <span className="mono">
                {new Date(member.membership_until).toLocaleString()}
              </span>
            ),
          },
        ]
      : []),
  ];
  return (
    <div className={rowClass} data-testid="mcp-effective-member">
      <Avatar>{member.username.slice(0, 1).toUpperCase()}</Avatar>
      <strong className={nameClass}>
        {member.username}
        {member.account_id === actor && (
          <small className="font-normal text-muted-foreground"> (you)</small>
        )}
      </strong>
      <Popover open={opened} onOpenChange={setOpened}>
        <PopoverTrigger asChild>
          <button
            type="button"
            className={`${sourceClass} cursor-pointer transition-colors duration-fast hover:border-border-strong hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:cursor-default disabled:opacity-60`}
            aria-label={`Access sources for ${member.username}`}
            disabled={disabled}
          >
            {source}
          </button>
        </PopoverTrigger>
        <PopoverContent align="end" className="grid w-[280px] gap-3">
          <strong className="text-label font-semibold text-foreground">
            {member.username}
          </strong>
          <dl className="m-0 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-label">
            {facts.map((fact) => (
              <div key={fact.term} className="contents">
                <dt className="text-muted-foreground">{fact.term}</dt>
                <dd className="m-0 text-right text-foreground [overflow-wrap:anywhere]">
                  {fact.value}
                </dd>
              </div>
            ))}
          </dl>
          {projected.ambiguousGroups.length > 0 && (
            <p className="m-0 text-caption text-muted-foreground">
              Multiple providers for {projected.ambiguousGroups.join(", ")}.
              Current access is shown; edit direct grants below.
            </p>
          )}
          <div className="flex items-center justify-between gap-2 border-t border-border pt-3 text-label">
            <span className="text-foreground-secondary">Direct grant</span>
            <McpPermissionIcons
              subject={`${member.username} direct`}
              rights={projected.direct}
              disabled={disabled || !projected.available}
              onChange={editing ? change : undefined}
            />
          </div>
          {editing && Object.values(projected.direct).some(Boolean) && (
            <Button
              variant="destructive-ghost"
              size="sm"
              className="justify-self-start"
              disabled={disabled}
              onClick={() => change(denied())}
            >
              Clear direct grant
            </Button>
          )}
        </PopoverContent>
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
