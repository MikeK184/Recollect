import type { components } from "../../api-schema";

type Rights = components["schemas"]["McpRights"];
type Member = components["schemas"]["McpEffectiveMember"];
type Grant = components["schemas"]["McpGrant"];
type Draft = components["schemas"]["McpGrantInput"];

export const noPermissions = (): Rights => ({
  use_profile: false,
  manage: false,
  share: false,
});

export function inheritedIndicators(member: Member, known: Rights): Rights {
  return {
    use_profile:
      member.rights.use_profile &&
      (known.use_profile || !member.direct_rights.use_profile),
    manage:
      member.rights.manage && (known.manage || !member.direct_rights.manage),
    share: member.rights.share && (known.share || !member.direct_rights.share),
  };
}

export function projectMemberPermissions(
  member: Member,
  grants: Grant[],
  drafts: Record<string, Draft> = {},
) {
  const direct =
    drafts[`account:${member.username}`]?.rights ?? member.direct_rights;
  const available = member.enabled && !!member.brain_role;
  const inherited = noPermissions();
  const ambiguousGroups = member.groups.filter(
    (name) => grants.filter((grant) => grant.group_name === name).length > 1,
  );
  if (available) {
    inherited.manage = member.brain_role === "admin";
    inherited.share = member.brain_role === "admin";
    for (const group of member.groups) {
      if (ambiguousGroups.includes(group)) continue;
      const rights =
        drafts[`group:${group}`]?.rights ??
        grants.find((grant) => grant.group_name === group)?.rights;
      if (rights) {
        inherited.use_profile ||= rights.use_profile;
        inherited.manage ||= rights.manage;
        inherited.share ||= rights.share;
      }
    }
  }
  return {
    direct,
    inherited,
    available,
    ambiguousGroups,
    effective: ambiguousGroups.length
      ? member.rights
      : available
        ? {
            use_profile: direct.use_profile || inherited.use_profile,
            manage: direct.manage || inherited.manage,
            share: direct.share || inherited.share,
          }
        : noPermissions(),
  };
}
