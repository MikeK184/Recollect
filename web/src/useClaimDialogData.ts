import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { Catalogue, Evidence, View } from "./claimsShared";
import { usePollInterval } from "./usePollInterval";

export type ClaimDialogProps = {
  brain: Brain;
  id: string;
  catalogue?: Catalogue;
  knowledgeAt?: string;
  factAt?: string;
  revisionId?: string;
  onKnowledgeChange?: (at?: string, revision?: string) => void;
  onClose: () => void;
  onSaved: () => void;
};

// The selected knowledge time, the dialog's drill-in states and the claim and
// manifest queries. The dialog calls this unconditionally before it decides
// which drill-in (editor, evidence, review, contribution) replaces the detail.
export function useClaimDialogData({
  brain,
  id,
  knowledgeAt,
  factAt,
  revisionId,
}: ClaimDialogProps) {
  const pollNormal = usePollInterval("normal");
  const [knowledge, setKnowledge] = useState<string | undefined>(knowledgeAt);
  useEffect(() => setKnowledge(knowledgeAt), [knowledgeAt]);
  const [offset, setOffset] = useState(0);
  const [selectedEvidence, setSelectedEvidence] = useState<Evidence | null>(
    null,
  );
  const [reviewing, setReviewing] = useState<View | null>(null);
  const [editing, setEditing] = useState(false);
  const [contribution, setContribution] = useState<{
    id: string;
    at: string;
  } | null>(null);
  const query = useQuery({
    queryKey: ["claim", brain.id, id, knowledge, factAt, offset],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/claims/{claim}", {
          signal,
          params: {
            path: { brain: brain.id, claim: id },
            query: { knowledge_at: knowledge, fact_at: factAt, offset },
          },
        }),
      ),
    refetchInterval: pollNormal,
    retry: false,
    gcTime: 0,
  });
  const mismatch =
    revisionId &&
    query.data?.selected &&
    query.data.selected.revision.id !== revisionId;
  const exactError =
    revisionId && !knowledgeAt
      ? new Error(
          "An exact revision link also needs its recorded knowledge time. No different revision is shown.",
        )
      : mismatch
        ? new Error(
            "This revision does not match the selected claim and knowledge time. No different revision is shown.",
          )
        : null;
  const view = !query.isError && !exactError ? query.data?.selected : undefined;
  const manifestId = view?.revision.content.manifest_revision_id;
  const selectedManifest = useQuery({
    queryKey: ["claim-manifest", brain.id, manifestId],
    enabled: !!manifestId,
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/claim-evidence/{kind}/{evidence}",
          {
            params: {
              path: {
                brain: brain.id,
                kind: "manifest_revision",
                evidence: manifestId!,
              },
            },
          },
        ),
      ),
  });
  return {
    knowledge,
    setKnowledge,
    offset,
    setOffset,
    selectedEvidence,
    setSelectedEvidence,
    reviewing,
    setReviewing,
    editing,
    setEditing,
    contribution,
    setContribution,
    query,
    exactError,
    view,
    manifestId,
    selectedManifest,
  };
}

export type ClaimDialogData = ReturnType<typeof useClaimDialogData>;
