import { test, expect } from "@playwright/test";
import type { components } from "../src/api-schema";
import {
  inheritedIndicators,
  noPermissions,
  projectMemberPermissions,
} from "../src/features/connections/member-permissions";

type Member = components["schemas"]["McpEffectiveMember"];
const member = (overrides: Partial<Member> = {}): Member => ({
  account_id: "fixture",
  username: "reader",
  brain_role: "reader",
  enabled: true,
  direct_rights: noPermissions(),
  rights: noPermissions(),
  groups: [],
  ...overrides,
});
const group: components["schemas"]["McpGrant"] = {
  id: "fixture-group",
  group_name: "research",
  rights: { use_profile: true, manage: false, share: true },
};

test("administrator inheritance never supplies Use and survives direct removal", () => {
  const m = member({
    brain_role: "admin",
    direct_rights: { use_profile: true, manage: true, share: true },
  });
  const result = projectMemberPermissions(m, [], {
    "account:reader": { username: "reader", rights: noPermissions() },
  });
  expect(result.effective).toEqual({
    use_profile: false,
    manage: true,
    share: true,
  });
  expect(result.inherited).toEqual({
    use_profile: false,
    manage: true,
    share: true,
  });
});

test("overlapping direct and active group access stays inherited after direct removal", () => {
  const m = member({ groups: ["research"], direct_rights: group.rights });
  const result = projectMemberPermissions(m, [group], {
    "account:reader": { username: "reader", rights: noPermissions() },
  });
  expect(result.direct).toEqual(noPermissions());
  expect(result.effective).toEqual(group.rights);
  expect(result.inherited.use_profile).toBe(true);
});

test("read-mode overlap keeps the group Use lock, and authoritative denial stays denied", () => {
  const m = member({
    groups: ["research"],
    direct_rights: group.rights,
    rights: group.rights,
  });
  const projected = projectMemberPermissions(m, [group]);
  expect(inheritedIndicators(m, projected.inherited)).toEqual(group.rights);
  expect(
    inheritedIndicators({ ...m, rights: noPermissions() }, projected.inherited),
  ).toEqual(noPermissions());
});

test("staged group removal unlocks powers while retaining independent direct access", () => {
  const m = member({
    groups: ["research"],
    direct_rights: { use_profile: true, manage: false, share: false },
  });
  const result = projectMemberPermissions(m, [group], {
    "group:research": { group_name: "research", rights: noPermissions() },
  });
  expect(result.inherited).toEqual(noPermissions());
  expect(result.effective).toEqual(m.direct_rights);
});

test("independent staged powers do not infer use from manage or share", () => {
  const result = projectMemberPermissions(member(), [], {
    "account:reader": {
      username: "reader",
      rights: { use_profile: false, manage: true, share: false },
    },
  });
  expect(result.effective).toEqual({
    use_profile: false,
    manage: true,
    share: false,
  });
});

test("same-name groups from different providers retain authoritative access instead of guessing", () => {
  const m = member({ groups: ["research"], rights: noPermissions() });
  const other = {
    ...group,
    id: "foreign",
    issuer: "https://other.example",
    rights: { use_profile: true, manage: true, share: true },
  };
  const result = projectMemberPermissions(
    m,
    [other, { ...group, issuer: "https://current.example" }],
    { "group:research": { group_name: "research", rights: other.rights } },
  );
  expect(result.ambiguousGroups).toEqual(["research"]);
  expect(result.effective).toEqual(m.rights);
  expect(result.inherited).toEqual(noPermissions());
});

for (const [name, overrides] of [
  ["disabled account", { enabled: false }],
  ["absent Brain access", { brain_role: null }],
] as const) {
  test(`${name} denies effective powers while retaining stored provenance`, () => {
    const m = member({
      direct_rights: group.rights,
      groups: ["research"],
      ...overrides,
    });
    const result = projectMemberPermissions(m, [group]);
    expect(result.available).toBe(false);
    expect(result.effective).toEqual(noPermissions());
    expect(result.direct).toEqual(group.rights);
    expect(result.inherited).toEqual(noPermissions());
  });
}
