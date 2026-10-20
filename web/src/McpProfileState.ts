import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import type { McpCatalogue, McpRights } from "./McpPanel";
import { useIdempotency } from "./useIdempotency";
import { usePollInterval } from "./usePollInterval";

export type Detail = components["schemas"]["McpProfileDetail"];
export type Input = components["schemas"]["McpProfileInput"];
export type GrantInput = components["schemas"]["McpGrantInput"];
export const denied = (): McpRights => ({
  use_profile: false,
  manage: false,
  share: false,
});
export const grantKey = (grant: GrantInput) =>
  `${grant.username ? "account" : "group"}:${grant.username ?? grant.group_name}`;
export const profileInput = (profile?: Detail["profile"]): Input => ({
  name: profile?.name ?? "",
  description: profile?.description ?? "",
  enabled: profile?.enabled ?? true,
  environment_id: profile?.environment_id ?? null,
  connection_ids: profile?.connection_ids ?? [],
  base_revision: profile?.revision ?? null,
});
const configuration = (input: Input) =>
  JSON.stringify({ ...input, base_revision: null });

export type McpProfileCardProps = {
  brain: Brain;
  actor: string;
  id: string;
  catalogue: McpCatalogue;
  environments: { value: string; label: string }[];
  saved: () => void;
  close?: () => void;
  tools: (id: string) => void;
};

// The draft, the loaded profile and its rights, the staged grants and the save
// mutation for one tool-group card. The card calls this unconditionally, so a
// draft survives the sections it collapses.
export function useProfileState({
  brain,
  actor,
  id,
  catalogue,
  saved,
  close,
}: McpProfileCardProps) {
  const pollNormal = usePollInterval("normal");
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
    refetchInterval: pollNormal,
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
  // The tools dialog opens from the catalogue entry, so a fresher detail read
  // alone must not enable it: the dialog would close again at once.
  const canUse =
    !!profile?.rights.use_profile &&
    !!profile.enabled &&
    !!current?.rights.use_profile &&
    current.enabled;
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
  return {
    savedId,
    editing,
    input,
    setInput,
    baseline,
    pendingGrants,
    progress,
    adding,
    setAdding,
    showDescription,
    setShowDescription,
    subjectKind,
    setSubjectKind,
    subject,
    setSubject,
    newRights,
    setNewRights,
    detail,
    options,
    profile,
    canManage,
    canShare,
    canUse,
    stale,
    stage,
    effective,
    groups,
    ambiguousGroup,
    groupRights,
    save,
    edit,
    cancel,
    compatible,
    connectionIds,
    description,
    name,
    addedPeople,
  };
}

export type ProfileState = ReturnType<typeof useProfileState>;
