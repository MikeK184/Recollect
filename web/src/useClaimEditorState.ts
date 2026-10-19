import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { useIdempotency } from "./useIdempotency";
import {
  emptyContent,
  label,
  type Catalogue,
  type Content,
  type Evidence,
  type Revision,
} from "./claimsShared";

export type ClaimEditorProps = {
  brain: Brain;
  catalogue?: Catalogue;
  revision?: Revision;
  evidence?: Evidence[];
  onClose: () => void;
  onSaved: (id: string) => void;
  draft?: Content;
  onCommit?: (content: Content) => Promise<string>;
  heading?: string;
  submitLabel?: string;
  notice?: string;
};

// The draft content, the evidence and manifest choice queries and the save
// mutation for one claim editor. The parent calls this unconditionally, so the
// draft survives the evidence preview that replaces the form.
export function useClaimEditorState({
  brain,
  catalogue,
  revision,
  evidence = [],
  onSaved,
  draft,
  onCommit,
}: ClaimEditorProps) {
  const [baseRevision] = useState(revision);
  const [content, setContent] = useState<Content>(() =>
    draft
      ? structuredClone(draft)
      : revision
        ? structuredClone(revision.content)
        : emptyContent(),
  );
  const [known, setKnown] = useState<Evidence[]>(evidence);
  const [kind, setKind] = useState<string | null>("source_version");
  const [search, setSearch] = useState("");
  const [offset, setOffset] = useState(0);
  const [manifestOffset, setManifestOffset] = useState(0);
  const [preview, setPreview] = useState<Evidence | null>(null);
  const command = useIdempotency();
  const choices = useQuery({
    queryKey: ["claim-evidence", brain.id, kind, search, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/claim-evidence", {
          params: {
            path: { brain: brain.id },
            query: {
              kind: kind ?? undefined,
              search: search || undefined,
              offset,
            },
          },
        }),
      ),
  });
  const manifests = useQuery({
    queryKey: [
      "claim-manifest-choices",
      brain.id,
      content.selection.environment_id,
      manifestOffset,
    ],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/revision-manifests", {
          params: {
            path: { brain: brain.id },
            query: {
              environment_id: content.selection.environment_id ?? undefined,
              offset: manifestOffset,
            },
          },
        }),
      ),
  });
  const save = useMutation({
    mutationFn: async () => {
      if (onCommit) return { claim_id: await onCommit(content) };
      const body = {
        base_revision: baseRevision?.id ?? null,
        operation_id: null,
        content,
      };
      const headers = { "Idempotency-Key": command.forInput(body) };
      return baseRevision
        ? result(
            await client.PUT("/api/brains/{brain}/claims/{claim}", {
              params: {
                path: { brain: brain.id, claim: baseRevision.claim_id },
              },
              body,
              headers,
            }),
          )
        : result(
            await client.POST("/api/brains/{brain}/claims", {
              params: { path: { brain: brain.id } },
              body,
              headers,
            }),
          );
    },
    onSuccess: (r) => onSaved(r.claim_id),
  });
  const patch = (value: Partial<Content>) =>
    setContent((c) => ({ ...c, ...value }));
  const repositories =
    catalogue?.repositories.map((r) => ({
      value: r.id,
      label: r.canonical_origin,
    })) ?? [];
  const manifestOptions =
    manifests.data?.items.map((m) => ({
      value: m.id,
      label: `${m.name} · ${label(m.kind)}`,
    })) ?? [];
  if (
    content.manifest_revision_id &&
    !manifestOptions.some((m) => m.value === content.manifest_revision_id)
  )
    manifestOptions.push({
      value: content.manifest_revision_id,
      label: "Previously selected exact manifest revision",
    });
  return {
    content,
    patch,
    known,
    setKnown,
    kind,
    setKind,
    search,
    setSearch,
    offset,
    setOffset,
    manifestOffset,
    setManifestOffset,
    preview,
    setPreview,
    choices,
    manifests,
    save,
    repositories,
    manifestOptions,
  };
}

export type ClaimEditorState = ReturnType<typeof useClaimEditorState>;
