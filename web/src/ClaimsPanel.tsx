import { MemoryNoteDialog } from "./features/memory/MemoryNoteDialog";
import { FilterBar } from "./components/FilterBar";
import { ScopeSummary } from "./components/ScopeSummary";
import { DetailInspector } from "./components/DetailInspector";
import { ActionMenu } from "./components/ActionMenu";
import { StatusBadge } from "./components/StatusBadge";
import { iconSize } from "./design/tokens";
import { useEffect, useState } from "react";
import { useBrainSearch } from "./app/useBrainSearch";
import { useDebouncedValue } from "@mantine/hooks";
import { EmptyState } from "./components/AsyncState";
import "./features/feature-views.css";
import {
  Alert,
  Badge,
  Button,
  Card,
  Divider,
  Drawer,
  Group,
  Loader,
  Modal,
  MultiSelect,
  NumberInput,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BookMarked, Plus, Search, SlidersHorizontal } from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import { ClaimReviewDialog } from "./ClaimReviewDialog";
import { EraseAction } from "./RetentionPanel";
import { useContentDeadline } from "./useContentDeadline";
import {
  MemoryFormDetails,
  MemoryFormFields,
  kindContent,
  memoryKinds,
} from "./MemoryForms";
type Content = components["schemas"]["ClaimContent"];
type Revision = components["schemas"]["ClaimRevision"];
type View = components["schemas"]["ClaimView"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];
const label = (s: string) => s.replaceAll("_", " ");
const time = (s: string) => new Date(s).toLocaleString();
const utc = (s?: string | null) => (s ? s.replace(/Z$/, "").slice(0, 19) : "");
const instant = (s: string) => (s ? new Date(`${s}Z`).toISOString() : null);
const freshOptions = ["current", "needs_verification", "superseded"].map(
  (value) => ({ value, label: label(value) }),
);
const opOptions = ["declared", "implemented", "deployed", "verified"].map(
  (value) => ({ value, label: label(value) }),
);
function Failure({ error }: { error: Error | null }) {
  return error ? (
    <Alert color="red" title="Request failed">
      {error.message}
    </Alert>
  ) : null;
}
function Pages({
  offset,
  total,
  onChange,
}: {
  offset: number;
  total: number;
  onChange: (value: number) => void;
}) {
  return total > 20 ? (
    <Group justify="space-between">
      <Button
        variant="subtle"
        disabled={!offset}
        onClick={() => onChange(Math.max(0, offset - 20))}
      >
        Previous page
      </Button>
      <Text size="xs">
        {offset + 1}–{Math.min(offset + 20, total)} of {total}
      </Text>
      <Button
        variant="subtle"
        disabled={offset + 20 >= total}
        onClick={() => onChange(offset + 20)}
      >
        Next page
      </Button>
    </Group>
  ) : null;
}
function States({ view }: { view: View }) {
  return (
    <Group gap="xs">
      <Badge color={view.revision.review === "accepted" ? "teal" : "gray"}>
        Review: {label(view.revision.review)}
      </Badge>
      <Badge
        color={
          view.eligibility.effective_freshness === "current" ? "blue" : "yellow"
        }
      >
        Freshness: {label(view.eligibility.effective_freshness)}
      </Badge>
      <Badge variant="outline">
        Assessment: {label(view.revision.content.operational)}
      </Badge>
      {view.revision.lifecycle === "withdrawn" && (
        <Badge color="orange">Withdrawn</Badge>
      )}
      {!!view.eligibility.rule_ids?.length && (
        <Badge color="red">Blocked by review rule</Badge>
      )}
      {!!view.eligibility.conflicting_claim_ids?.length && (
        <Badge color="yellow">Unresolved conflict</Badge>
      )}
    </Group>
  );
}
/** One composite status mark for dense list rows. The three dimensions keep
 * their separate order and meaning inside the text, so no distinction relies
 * on color alone; the full dimension set stays in the inspector. */
function CompositeStatus({ view }: { view: View }) {
  const flags: string[] = [];
  if (view.revision.lifecycle === "withdrawn") flags.push("Withdrawn");
  if (view.eligibility.rule_ids?.length) flags.push("Blocked by review rule");
  if (view.eligibility.conflicting_claim_ids?.length)
    flags.push("Unresolved conflict");
  const text = [
    `Review: ${label(view.revision.review)}`,
    `Freshness: ${label(view.eligibility.effective_freshness)}`,
    `Operational: ${label(view.revision.content.operational)}`,
    ...flags,
  ].join(" · ");
  const state = flags.length
    ? "attention"
    : view.revision.review === "accepted" &&
        view.eligibility.effective_freshness === "current"
      ? "positive"
      : "neutral";
  return (
    <div className="memory-record-status">
      <StatusBadge state={state}>{text}</StatusBadge>
    </div>
  );
}
export function EvidenceDialog({
  brain,
  evidence,
  onClose,
  returnLabel = "Back to claim",
  eraseSource = false,
}: {
  brain: Brain;
  evidence: Evidence;
  onClose: () => void;
  returnLabel?: string;
  eraseSource?: boolean;
}) {
  const query = useQuery({
    queryKey: ["claim-evidence-detail", brain.id, evidence.kind, evidence.id],
    gcTime: 0,
    retry: false,
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET(
          "/api/brains/{brain}/claim-evidence/{kind}/{evidence}",
          {
            signal,
            params: {
              path: {
                brain: brain.id,
                kind: evidence.kind,
                evidence: evidence.id,
              },
            },
          },
        ),
      ),
  });
  const expired = useContentDeadline(query.data?.expires_at);
  const data = !query.isError && !expired ? query.data : undefined;
  return (
    <Modal
      opened
      onClose={onClose}
      size="xl"
      title="Supporting evidence"
      closeButtonProps={{ "aria-label": "Close evidence" }}
    >
      <Stack>
        <Text fw={600}>{data?.evidence.label ?? "Evidence"}</Text>
        <Failure error={query.error} />
        {query.isPending && <Loader />}
        {expired && (
          <Alert color="yellow">
            This evidence reached its retention deadline. Its content has been
            cleared.
          </Alert>
        )}
        {(query.isError || expired) && (
          <Button variant="light" onClick={() => void query.refetch()}>
            Refresh evidence
          </Button>
        )}
        {data && (
          <>
            <Badge w="fit-content">{label(data.evidence.availability)}</Badge>
            <Text size="sm">
              {label(data.evidence.kind)} · recorded{" "}
              {time(data.evidence.created_at)}
            </Text>
            {data.evidence.revision && (
              <Text size="xs" className="claim-wrap">
                Exact revision: {data.evidence.revision}
              </Text>
            )}
            {data.evidence.reference && (
              <Text size="sm" className="claim-wrap">
                Reference: {data.evidence.reference}
              </Text>
            )}
            {data.evidence.observed_at && (
              <Text size="sm">Observed: {time(data.evidence.observed_at)}</Text>
            )}
            {data.text !== null && data.text !== undefined ? (
              <pre className="source-content" data-testid="claim-evidence-text">
                {data.text}
              </pre>
            ) : evidence.kind === "source_version" ? (
              <Alert color="yellow">
                Source text is unavailable here. A reference does not confirm
                remote availability.
              </Alert>
            ) : null}
            <details className="feature-details">
              <summary>Technical record</summary>
              <pre className="source-content">
                {JSON.stringify(data.data, null, 2)}
              </pre>
            </details>
          </>
        )}
        {eraseSource && evidence.source_id && (
          <EraseAction
            brain={brain.id}
            permitted={brain.role === "admin"}
            target={{ kind: "source", id: evidence.source_id }}
            name="Source evidence"
          />
        )}
        <Button variant="subtle" onClick={onClose}>
          {returnLabel}
        </Button>
      </Stack>
    </Modal>
  );
}
const emptyContent = (): Content => ({
  kind: "claim",
  subject: "",
  predicate: "",
  value: "",
  rationale: "",
  selection: { repository_ids: [], area_ids: [], environment_id: null },
  manifest_revision_id: null,
  validity: { kind: "unknown", from: null, to: null, precision: "unknown" },
  freshness: "current",
  operational: "declared",
  observed_at: null,
  observation: "",
  supports: [],
});

export function ClaimEditor({
  brain,
  catalogue,
  revision,
  evidence = [],
  onClose,
  onSaved,
  draft,
  onCommit,
  heading,
  submitLabel,
  notice,
}: {
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
}) {
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
  if (preview)
    return (
      <EvidenceDialog
        brain={brain}
        evidence={preview}
        onClose={() => setPreview(null)}
      />
    );
  return (
    <Modal
      opened
      onClose={onClose}
      size="xl"
      title={heading ?? (revision ? "Revise proposal" : "New memory")}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate();
        }}
      >
        <Stack className="claims-panel">
          <Alert color="blue">
            {notice ??
              "Saved as a proposal. Authorship and operational assessment do not count as human review."}
          </Alert>
          <Select
            label="Memory kind"
            data={memoryKinds}
            value={content.kind}
            onChange={(v) => patch(kindContent(v ?? "claim"))}
          />
          <TextInput
            label="Subject"
            placeholder="Production Vault"
            required
            maxLength={256}
            value={content.subject}
            onChange={(e) => patch({ subject: e.target.value })}
          />
          <TextInput
            label="Property or relationship"
            placeholder="authentication method"
            required
            maxLength={128}
            value={content.predicate}
            onChange={(e) => patch({ predicate: e.target.value })}
          />
          <Textarea
            label="Claim value"
            required
            autosize
            minRows={2}
            maxLength={4000}
            value={content.value}
            onChange={(e) => patch({ value: e.target.value })}
          />
          <Textarea
            label="Rationale"
            autosize
            minRows={2}
            maxLength={4000}
            value={content.rationale}
            onChange={(e) => patch({ rationale: e.target.value })}
          />
          <MemoryFormFields
            brain={brain}
            content={content}
            evidence={known}
            patch={patch}
            onEvidence={setKnown}
          />
          <Divider label="Applicability" />
          <MultiSelect
            label="Repositories"
            searchable
            data={repositories}
            value={content.selection.repository_ids ?? []}
            onChange={(repository_ids) =>
              patch({ selection: { ...content.selection, repository_ids } })
            }
          />
          <MultiSelect
            label="Areas"
            searchable
            data={
              catalogue?.areas.map((a) => ({ value: a.id, label: a.name })) ??
              []
            }
            value={content.selection.area_ids ?? []}
            onChange={(area_ids) =>
              patch({ selection: { ...content.selection, area_ids } })
            }
          />
          <Select
            label="Claim environment"
            clearable
            data={
              catalogue?.environments.map((a) => ({
                value: a.id,
                label: a.name,
              })) ?? []
            }
            value={content.selection.environment_id}
            onChange={(environment_id) => {
              patch({
                selection: { ...content.selection, environment_id },
                manifest_revision_id: null,
              });
              setManifestOffset(0);
            }}
          />
          <Text size="xs" c="dimmed">
            Empty applicability applies throughout this Brain. Evidence from a
            repository requires selecting that repository.
          </Text>
          <Select
            label="Exact manifest selection"
            clearable
            searchable
            data={manifestOptions}
            value={content.manifest_revision_id}
            onChange={(id) => {
              const manifest = manifests.data?.items.find((m) => m.id === id);
              patch({
                manifest_revision_id: id,
                ...(manifest
                  ? {
                      selection: {
                        ...content.selection,
                        environment_id: manifest.environment_id,
                      },
                    }
                  : {}),
              });
            }}
          />
          <Failure error={manifests.error} />
          <Pages
            offset={manifestOffset}
            total={manifests.data?.total ?? 0}
            onChange={setManifestOffset}
          />
          <Divider label="Fact validity" />
          <Select
            label="Validity kind"
            data={[
              { value: "unknown", label: "Unknown" },
              { value: "point", label: "Point observation" },
              { value: "interval", label: "Interval [from, to)" },
            ]}
            value={content.validity.kind}
            onChange={(v) =>
              patch({
                validity: {
                  kind: v ?? "unknown",
                  from: null,
                  to: null,
                  precision: v === "unknown" ? "unknown" : "second",
                },
              })
            }
          />
          {content.validity.kind !== "unknown" && (
            <>
              <TextInput
                type="datetime-local"
                step="1"
                label={
                  content.validity.kind === "point"
                    ? "Observed fact time (UTC)"
                    : "Valid from (UTC; blank means unknown)"
                }
                required={content.validity.kind === "point"}
                value={utc(content.validity.from)}
                onChange={(e) =>
                  patch({
                    validity: {
                      ...content.validity,
                      from: instant(e.target.value),
                    },
                  })
                }
              />
              {content.validity.kind === "interval" ? (
                <TextInput
                  type="datetime-local"
                  step="1"
                  label="Valid until (UTC; exclusive, blank means unknown)"
                  value={utc(content.validity.to)}
                  onChange={(e) =>
                    patch({
                      validity: {
                        ...content.validity,
                        to: instant(e.target.value),
                      },
                    })
                  }
                />
              ) : (
                <Select
                  label="Observation precision"
                  data={["second", "minute", "hour", "day"]}
                  value={content.validity.precision}
                  onChange={(v) =>
                    patch({
                      validity: {
                        ...content.validity,
                        precision: v ?? "second",
                      },
                    })
                  }
                />
              )}
            </>
          )}
          <Group grow align="start">
            <Select
              label="Recorded freshness"
              data={freshOptions}
              value={content.freshness}
              onChange={(v) => patch({ freshness: v ?? "current" })}
            />
            <Select
              label="Operational assessment"
              data={opOptions}
              value={content.operational}
              onChange={(v) => patch({ operational: v ?? "declared" })}
            />
          </Group>
          <TextInput
            type="datetime-local"
            step="1"
            label="Outcome observed at (UTC)"
            required={["deployed", "verified"].includes(content.operational)}
            value={utc(content.observed_at)}
            onChange={(e) => patch({ observed_at: instant(e.target.value) })}
          />
          <Textarea
            label="Observed outcome"
            maxLength={2000}
            required={["deployed", "verified"].includes(content.operational)}
            value={content.observation}
            onChange={(e) => patch({ observation: e.target.value })}
          />
          <Divider
            label={`Supporting evidence (${content.supports.length}/20)`}
          />
          {content.supports.map((support, index) => {
            const item = known.find(
              (e) => e.id === support.id && e.kind === support.kind,
            );
            return (
              <Card key={`${support.kind}:${support.id}`} withBorder>
                <Stack gap="xs">
                  <Group justify="space-between">
                    <Text size="sm" className="claim-wrap">
                      {item?.label ?? support.id}
                    </Text>
                    <Group gap="xs">
                      {item && (
                        <Button
                          size="compact-xs"
                          variant="subtle"
                          onClick={() => setPreview(item)}
                        >
                          Inspect
                        </Button>
                      )}
                      <Button
                        size="compact-xs"
                        variant="subtle"
                        color="gray"
                        onClick={() =>
                          patch({
                            supports: content.supports.filter(
                              (_, i) => i !== index,
                            ),
                          })
                        }
                      >
                        Remove support
                      </Button>
                    </Group>
                  </Group>
                  {support.kind === "source_version" && (
                    <Group grow>
                      {(["line_from", "line_to"] as const).map((key) => (
                        <NumberInput
                          key={key}
                          label={
                            key === "line_from"
                              ? "First supporting line (optional)"
                              : "Last supporting line (inclusive)"
                          }
                          min={1}
                          max={1000000}
                          allowDecimal={false}
                          value={support[key] ?? ""}
                          onChange={(v) =>
                            patch({
                              supports: content.supports.map((s, i) =>
                                i === index
                                  ? {
                                      ...s,
                                      [key]: typeof v === "number" ? v : null,
                                    }
                                  : s,
                              ),
                            })
                          }
                        />
                      ))}
                    </Group>
                  )}
                </Stack>
              </Card>
            );
          })}
          <Group grow>
            <Select
              label="Evidence kind"
              data={[
                { value: "source_version", label: "Document versions" },
                { value: "repository_fact", label: "Repository facts" },
                { value: "manifest_revision", label: "Manifest revisions" },
              ]}
              value={kind}
              onChange={(v) => {
                setKind(v);
                setOffset(0);
              }}
            />
            <TextInput
              label="Find evidence"
              value={search}
              onChange={(e) => {
                setSearch(e.target.value);
                setOffset(0);
              }}
            />
          </Group>
          <Failure error={choices.error} />
          {choices.isPending && <Loader size="sm" />}
          {choices.data?.items.length === 0 && (
            <Text c="dimmed" size="sm">
              No evidence matches. Import a source or publish a repository
              snapshot first.
            </Text>
          )}
          {choices.data?.items.map((item) => (
            <Group
              key={`${item.kind}:${item.id}`}
              justify="space-between"
              wrap="nowrap"
            >
              <div className="claim-wrap">
                <Text size="sm" fw={500}>
                  {item.label}
                </Text>
                <Text size="xs" c="dimmed">
                  {time(item.created_at)} · {label(item.availability)}
                  {item.revision ? ` · ${item.revision.slice(0, 12)}` : ""}
                </Text>
              </div>
              <Button
                size="compact-xs"
                variant="light"
                disabled={
                  ["expired", "erased"].includes(item.availability) ||
                  content.supports.length >= 20 ||
                  content.supports.some(
                    (s) => s.id === item.id && s.kind === item.kind,
                  )
                }
                onClick={() => {
                  setKnown((k) => [...k.filter((e) => e.id !== item.id), item]);
                  const repos = content.selection.repository_ids ?? [];
                  patch({
                    supports: [
                      ...content.supports,
                      {
                        kind: item.kind,
                        id: item.id,
                        line_from: null,
                        line_to: null,
                      },
                    ],
                    selection: {
                      ...content.selection,
                      repository_ids:
                        item.repository_id &&
                        !repos.includes(item.repository_id)
                          ? [...repos, item.repository_id]
                          : repos,
                    },
                  });
                }}
              >
                Use evidence
              </Button>
            </Group>
          ))}
          <Pages
            offset={offset}
            total={choices.data?.total ?? 0}
            onChange={setOffset}
          />
          <Failure error={save.error} />
          <Group justify="flex-end">
            <Button variant="default" onClick={onClose}>
              Cancel
            </Button>
            <Button
              type="submit"
              loading={save.isPending}
              disabled={!catalogue || !content.supports.length}
            >
              {submitLabel ??
                (revision ? "Save new proposal" : "Save proposal")}
            </Button>
          </Group>
        </Stack>
      </form>
    </Modal>
  );
}

export function ClaimDialog({
  brain,
  id,
  catalogue,
  knowledgeAt,
  factAt,
  revisionId,
  onKnowledgeChange,
  onClose,
  onSaved,
}: {
  brain: Brain;
  id: string;
  catalogue?: Catalogue;
  knowledgeAt?: string;
  factAt?: string;
  revisionId?: string;
  onKnowledgeChange?: (at?: string, revision?: string) => void;
  onClose: () => void;
  onSaved: () => void;
}) {
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
    refetchInterval: 4000,
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
  if (query.isError || exactError)
    return (
      <DetailInspector
        className="feature-drawer"
        position="right"
        opened
        onClose={onClose}
        size="min(44rem, 90vw)"
        title="Claim and knowledge history"
      >
        <Stack>
          <Failure error={query.error ?? exactError} />
          <Button variant="light" onClick={() => void query.refetch()}>
            Try again
          </Button>
        </Stack>
      </DetailInspector>
    );
  if (view && contribution)
    return (
      <ClaimDialog
        key={contribution.id + contribution.at}
        brain={brain}
        id={contribution.id}
        knowledgeAt={contribution.at}
        catalogue={catalogue}
        onClose={() => setContribution(null)}
        onSaved={onSaved}
      />
    );
  if (view && editing)
    return (
      <ClaimEditor
        brain={brain}
        catalogue={catalogue}
        revision={view.revision}
        evidence={view.evidence}
        onClose={() => setEditing(false)}
        onSaved={() => {
          setEditing(false);
          void query.refetch();
          onSaved();
        }}
      />
    );
  if (view && selectedEvidence)
    return (
      <EvidenceDialog
        brain={brain}
        evidence={selectedEvidence}
        onClose={() => setSelectedEvidence(null)}
      />
    );
  if (view && reviewing)
    return (
      <ClaimReviewDialog
        brain={brain}
        view={reviewing}
        catalogue={catalogue}
        current={reviewing.revision.id === query.data?.current_revision}
        Editor={ClaimEditor}
        EvidenceViewer={EvidenceDialog}
        onClose={() => setReviewing(null)}
        onSaved={() => {
          void query.refetch();
          onSaved();
        }}
      />
    );
  const r = view?.revision;
  const history = query.data?.history ?? [];
  const historyOptions = [
    { value: "current", label: "Latest knowledge" },
    ...history.map((r) => ({
      value: r.recorded_at,
      label: `${time(r.recorded_at)} · ${r.content.value.slice(0, 60)}`,
    })),
    ...(query.data?.unavailable_history ?? []).map((r) => ({
      value: r.recorded_at,
      label: `${time(r.recorded_at)} · ${label(r.state)}`,
    })),
  ];
  if (knowledge && !historyOptions.some((o) => o.value === knowledge))
    historyOptions.push({
      value: knowledge,
      label: `Knowledge at ${time(knowledge)}`,
    });
  return (
    <DetailInspector
      className="feature-drawer"
      position="right"
      opened
      onClose={onClose}
      size="min(44rem, 90vw)"
      title="Claim and knowledge history"
    >
      <Stack className="claims-panel">
        {query.isPending && <Loader />}
        {query.data && (
          <>
            <details className="feature-details">
              <summary>Version history</summary>
              <Select
                label="Knowledge revision"
                data={historyOptions}
                value={knowledge ?? "current"}
                onChange={(value) => {
                  const at =
                    value === "current" ? undefined : (value ?? undefined);
                  setKnowledge(at);
                  const revision = [
                    ...history,
                    ...(query.data?.unavailable_history ?? []),
                  ].find((item) => item.recorded_at === at);
                  onKnowledgeChange?.(at, revision?.id);
                }}
              />
              <Pages
                offset={offset}
                total={query.data.total}
                onChange={setOffset}
              />
            </details>
            {!view && (
              <Alert>
                {query.data?.selection_state === "not_recorded"
                  ? "No claim had been recorded at the selected knowledge time."
                  : `This revision was ${query.data?.selection_state}. Its content is unavailable; an older revision has not been substituted.`}
              </Alert>
            )}
          </>
        )}
        {view && r && (
          <>
            {r.id !== query.data?.current_revision && (
              <Alert color="yellow">
                Historical knowledge revision. Later knowledge may differ.
              </Alert>
            )}
            <Title order={3}>{r.content.subject}</Title>
            <Text fw={600}>{r.content.predicate}</Text>
            <Text
              data-testid="claim-value"
              className="claim-wrap"
              style={{ whiteSpace: "pre-wrap" }}
            >
              {r.content.value}
            </Text>
            <States view={view} />
            <Text size="sm">
              {label(r.origin)} by {r.actor_name} · learned{" "}
              {time(r.recorded_at)}
              {view.knowledge_until
                ? ` · replaced ${time(view.knowledge_until)}`
                : ""}
            </Text>
            <Text size="sm">
              Kind:{" "}
              {memoryKinds.find((kind) => kind.value === r.content.kind)
                ?.label ?? r.content.kind}
              . Recorded freshness: {label(r.content.freshness)}.
            </Text>
            {r.content.rationale && (
              <Text size="sm" className="claim-wrap">
                {r.content.rationale}
              </Text>
            )}
            {r.derivation && (
              <Text size="sm">
                Model derivation: {r.derivation.returned_model} ·{" "}
                {r.derivation.prompt_label}. Model output does not confer human
                review or operational verification.
              </Text>
            )}
            <MemoryFormDetails
              view={view}
              catalogue={catalogue}
              onInspect={(id, at) => setContribution({ id, at })}
            />
            <Card withBorder>
              <Text fw={600} size="sm">
                Fact validity
              </Text>
              {r.content.validity.kind === "unknown" ? (
                <Text size="sm">Unknown; no time interval is inferred.</Text>
              ) : r.content.validity.kind === "point" ? (
                <Text size="sm">
                  Point observation:{" "}
                  {r.content.validity.from
                    ? time(r.content.validity.from)
                    : "unknown"}{" "}
                  ({r.content.validity.precision} precision). This does not
                  establish uninterrupted operation.
                </Text>
              ) : (
                <Text size="sm">
                  From{" "}
                  {r.content.validity.from
                    ? time(r.content.validity.from)
                    : "unknown"}{" "}
                  until{" "}
                  {r.content.validity.to
                    ? time(r.content.validity.to)
                    : "unknown"}{" "}
                  (end excluded).
                </Text>
              )}
              {r.content.observed_at && (
                <Text size="sm" mt="xs">
                  Outcome observed {time(r.content.observed_at)}:{" "}
                  {r.content.observation}
                </Text>
              )}
            </Card>
            <Text size="sm">
              Environment:{" "}
              {catalogue?.environments.find(
                (e) => e.id === r.content.selection.environment_id,
              )?.name ??
                (r.content.selection.environment_id || "All in this Brain")}
            </Text>
            <Text size="sm">
              Areas:{" "}
              {(r.content.selection.area_ids ?? []).length
                ? r.content.selection
                    .area_ids!.map(
                      (id) =>
                        catalogue?.areas.find((a) => a.id === id)?.name ?? id,
                    )
                    .join(", ")
                : "All in this Brain"}
            </Text>
            {(r.content.selection.repository_ids ?? []).map((repo) => (
              <Text size="xs" className="claim-wrap" key={repo}>
                {catalogue?.repositories.find((p) => p.id === repo)
                  ?.canonical_origin ?? repo}
              </Text>
            ))}
            {manifestId && (
              <>
                <Failure error={selectedManifest.error} />
                <Button
                  variant="default"
                  loading={selectedManifest.isPending}
                  disabled={!selectedManifest.data || !!selectedManifest.error}
                  onClick={() =>
                    selectedManifest.data &&
                    setSelectedEvidence(selectedManifest.data.evidence)
                  }
                >
                  Inspect applicability manifest
                </Button>
              </>
            )}
            <Alert
              color={view.eligibility.strict_accepted ? "teal" : "yellow"}
              title={
                view.eligibility.strict_accepted
                  ? "Eligible for strict accepted context"
                  : "Does not qualify for strict accepted context"
              }
            >
              <Text size="sm">
                {view.eligibility.reasons.map(label).join(" · ") ||
                  "Current accepted evidence"}
              </Text>
              <Text size="xs" mt="xs">
                Strict operational context:{" "}
                {view.eligibility.strict_operational
                  ? "eligible"
                  : "not eligible"}
                . Operational assessment is the author's recorded assessment.
              </Text>
            </Alert>
            <Divider label="Exact supporting evidence" />
            {view.evidence.map((e, index) => (
              <Button
                key={`${e.kind}:${e.id}`}
                variant="default"
                h="auto"
                py="sm"
                className="claim-evidence-button"
                onClick={() => setSelectedEvidence(e)}
              >
                {e.label} · {label(e.kind)} · {label(e.availability)}
                {r.content.supports[index]?.line_from
                  ? ` · lines ${r.content.supports[index].line_from}–${r.content.supports[index].line_to}`
                  : ""}
              </Button>
            ))}
            <Alert color="blue">
              New input is handled by autonomous learning. See Activity → Model
              usage and learning for recent learning.
            </Alert>
            <Button variant="light" onClick={() => setReviewing(view)}>
              Review and corrections
            </Button>
            {!knowledge &&
              !view.knowledge_until &&
              r.review === "proposed" &&
              brain.role !== "reader" &&
              !brain.archived && (
                <Button variant="default" onClick={() => setEditing(true)}>
                  Revise proposal
                </Button>
              )}
          </>
        )}
        {query.data && (
          <>
            <ActionMenu label="More memory actions">
              <EraseAction
                brain={brain.id}
                permitted={brain.role === "admin"}
                target={{ kind: "claim", id }}
                name={view?.revision.content.subject ?? "Claim history"}
              />
            </ActionMenu>
          </>
        )}
      </Stack>
    </DetailInspector>
  );
}

export function ClaimsPanel({
  brain,
  kind,
  onKindChange,
  selectedId,
  onSelectedIdChange,
}: {
  brain: Brain;
  kind?: string | null;
  onKindChange?: (value: string | null) => void;
  selectedId?: string | null;
  onSelectedIdChange?: (value: string | null) => void;
}) {
  const cache = useQueryClient();
  const [route, patchRoute] = useBrainSearch();
  const [offset, setOffset] = useState(0);
  const mode = route.mode ?? "investigation";
  const setMode = (mode: string | null) => patchRoute({ mode });
  const internalKind = ["claim", "decision", "procedure", "handover"].includes(
    route.kind ?? "",
  )
    ? route.kind!
    : null;
  const setInternalKind = (kind: string | null) => patchRoute({ kind });
  const memoryKind = kind === undefined ? internalKind : kind;
  const environment = route.environment ?? null;
  const setEnvironment = (environment: string | null) =>
    patchRoute({ environment });
  const repository = route.repository ?? null;
  const setRepository = (repository: string | null) =>
    patchRoute({ repository });
  const [search, setSearch] = useState("");
  const [query] = useDebouncedValue(search.trim(), 300);
  const [filtering, setFiltering] = useState(false);
  const [factDraft, setFactDraft] = useState(utc(route.fact));
  const [knowledgeDraft, setKnowledgeDraft] = useState(utc(route.knowledge));
  const times = { fact_at: route.fact, knowledge_at: route.knowledge };
  const setTimes = (times: { fact_at?: string; knowledge_at?: string }) =>
    patchRoute({
      fact: times.fact_at ?? null,
      knowledge: times.knowledge_at ?? null,
      revision: null,
    });
  useEffect(() => {
    setFactDraft(utc(route.fact));
    setKnowledgeDraft(utc(route.knowledge));
  }, [route.fact, route.knowledge]);
  useEffect(
    () => setOffset(0),
    [mode, memoryKind, environment, repository, route.fact, route.knowledge],
  );
  const [noting, setNoting] = useState(false);
  const [structuring, setStructuring] = useState(false);
  const selected =
    selectedId === undefined ? (route.claim ?? null) : selectedId;
  const setSelected = (value: string | null) => {
    patchRoute({ claim: value, revision: null, detail: null });
    onSelectedIdChange?.(value);
  };
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          signal,
          params: { path: { brain: brain.id } },
        }),
      ),
    refetchInterval: 4000,
  });
  const claims = useQuery({
    queryKey: [
      "claims",
      brain.id,
      memoryKind,
      mode,
      environment,
      repository,
      times,
      query,
      offset,
    ],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/claims", {
          signal,
          params: {
            path: { brain: brain.id },
            query: {
              q: query || undefined,
              mode: mode ?? undefined,
              kind: memoryKind ?? undefined,
              environment_id: environment ?? undefined,
              repository_id: repository ?? undefined,
              ...times,
              offset,
            },
          },
        }),
      ),
    refetchInterval: 4000,
  });
  const refresh = () => {
    for (const key of ["claims", "claim", "audit", "jobs", "processing"])
      void cache.invalidateQueries({ queryKey: [key, brain.id] });
  };
  const clear = () => {
    patchRoute({
      mode: null,
      kind: null,
      environment: null,
      repository: null,
      fact: null,
      knowledge: null,
      revision: null,
    });
    onKindChange?.(null);
    setSearch("");
    setFactDraft("");
    setKnowledgeDraft("");
    setOffset(0);
  };
  const filtered =
    mode !== "investigation" ||
    !!environment ||
    !!repository ||
    !!times.fact_at ||
    !!times.knowledge_at;
  const data = claims.error || catalogue.error ? undefined : claims.data;
  return (
    <section className="feature-view claims-panel" aria-label="Memory records">
      <FilterBar>
        <TextInput
          className="feature-search"
          aria-label="Search memory"
          placeholder="Find a memory…"
          leftSection={<Search size={iconSize.navigation} />}
          maxLength={200}
          value={search}
          onChange={(event) => {
            setSearch(event.currentTarget.value);
            setOffset(0);
          }}
        />
        <Button
          variant="default"
          leftSection={<SlidersHorizontal size={iconSize.small} />}
          onClick={() => setFiltering(true)}
        >
          Filters{filtered ? " · applied" : ""}
        </Button>
        {brain.role !== "reader" && !brain.archived && (
          <Button
            leftSection={<Plus size={iconSize.small} />}
            onClick={() => setNoting(true)}
            disabled={!catalogue.data || !!catalogue.error}
          >
            Add memory
          </Button>
        )}
      </FilterBar>
      {filtered && (
        <ScopeSummary>
          {mode !== "investigation" && (
            <Badge variant="light">{label(mode ?? "investigation")}</Badge>
          )}
          {environment && (
            <Badge variant="light">
              {catalogue.data?.environments.find((e) => e.id === environment)
                ?.name ?? "Selected environment"}
            </Badge>
          )}
          {repository && (
            <Badge variant="light">
              {catalogue.data?.repositories.find((r) => r.id === repository)
                ?.canonical_origin ?? "Selected repository"}
            </Badge>
          )}
          {times.fact_at && (
            <Badge variant="light">Fact time {time(times.fact_at)}</Badge>
          )}
          {times.knowledge_at && (
            <Badge variant="light">
              Knowledge time {time(times.knowledge_at)}
            </Badge>
          )}
          <Button variant="subtle" size="compact-sm" onClick={clear}>
            Clear filters
          </Button>
        </ScopeSummary>
      )}
      <Drawer
        className="feature-drawer"
        opened={filtering}
        onClose={() => setFiltering(false)}
        position="right"
        title="Filter memory"
        size="md"
      >
        <Stack>
          <Text size="sm" c="dimmed">
            Search matches subjects, predicates, values and rationale. These
            filters keep applicability and knowledge history explicit.
          </Text>
          {kind === undefined && (
            <Select
              label="Filter memory kind"
              data={memoryKinds}
              clearable
              value={memoryKind}
              onChange={(value) => {
                setInternalKind(value);
                onKindChange?.(value);
                setOffset(0);
              }}
            />
          )}
          <Select
            label="Claim view"
            data={[
              { value: "investigation", label: "Investigation" },
              { value: "strict_accepted", label: "Strict accepted" },
              { value: "strict_operational", label: "Strict operational" },
              { value: "history", label: "Include historical states" },
            ]}
            value={mode}
            onChange={(value) => {
              setMode(value);
              setOffset(0);
            }}
          />
          <Select
            label="Filter claim environment"
            clearable
            data={
              catalogue.data?.environments.map((e) => ({
                value: e.id,
                label: e.name,
              })) ?? []
            }
            value={environment}
            onChange={(value) => {
              setEnvironment(value);
              setOffset(0);
            }}
          />
          <Select
            label="Filter claim repository"
            clearable
            searchable
            data={
              catalogue.data?.repositories.map((r) => ({
                value: r.id,
                label: r.canonical_origin,
              })) ?? []
            }
            value={repository}
            onChange={(value) => {
              setRepository(value);
              setOffset(0);
            }}
          />
          <TextInput
            type="datetime-local"
            step="1"
            label="Fact time (UTC)"
            value={factDraft}
            onChange={(event) => setFactDraft(event.currentTarget.value)}
          />
          <TextInput
            type="datetime-local"
            step="1"
            label="Knowledge time (UTC)"
            value={knowledgeDraft}
            onChange={(event) => setKnowledgeDraft(event.currentTarget.value)}
          />
          <Button variant="default" onClick={clear}>
            Clear memory filters
          </Button>
          <Button
            onClick={() => {
              setTimes({
                fact_at: instant(factDraft) ?? undefined,
                knowledge_at: instant(knowledgeDraft) ?? undefined,
              });
              setOffset(0);
              setFiltering(false);
            }}
          >
            Apply filters
          </Button>
        </Stack>
      </Drawer>
      <Failure error={catalogue.error ?? claims.error} />
      {(catalogue.error || claims.error) && (
        <Button
          variant="subtle"
          onClick={() => {
            void catalogue.refetch();
            void claims.refetch();
          }}
        >
          Retry memory
        </Button>
      )}
      {claims.isPending && <Loader />}
      {data && (
        <>
          {data.items.length === 0 && !data.unavailable?.length && (
            <EmptyState
              icon={BookMarked}
              title={
                query || filtered
                  ? "No matching memory"
                  : "Your knowledge will gather here"
              }
              description={
                query || filtered
                  ? "Try another phrase or adjust the filters. Historical and qualified records can be inspected separately."
                  : "Recollect learns from permitted sources and agent activity. You can also add an evidence-backed memory yourself."
              }
            />
          )}
          {!!data.items.length && (
            <div className="feature-list">
              {data.items.map((view) => (
                <article className="memory-record" key={view.revision.claim_id}>
                  {/* Dense row: one primary identifier line plus one composite
                      status line. Value, evidence and knowledge time move to
                      the shared lineage inspector. */}
                  <Stack gap="xs">
                    <Group justify="space-between" align="start" gap="xs">
                      <Button
                        variant="subtle"
                        justify="start"
                        px={0}
                        h="auto"
                        className="memory-record-title"
                        onClick={() => setSelected(view.revision.claim_id)}
                      >
                        {view.revision.content.subject} ·{" "}
                        {view.revision.content.predicate}
                      </Button>
                      <Badge variant="light" color="gray">
                        {memoryKinds.find(
                          (item) => item.value === view.revision.content.kind,
                        )?.label ?? label(view.revision.content.kind)}
                      </Badge>
                    </Group>
                    <CompositeStatus view={view} />
                  </Stack>
                </article>
              ))}
            </div>
          )}
          {data.unavailable?.map((item) => (
            <Card withBorder key={item.claim_id} mt="sm">
              <Button
                variant="subtle"
                onClick={() => setSelected(item.claim_id)}
              >
                Unavailable memory · {item.claim_id.slice(0, 8)}
              </Button>
              <Text size="sm" c="dimmed">
                Content is unavailable. Inspect its remaining history and erased
                intervals.
              </Text>
            </Card>
          ))}
          <Pages
            offset={offset}
            total={data.total_candidates}
            onChange={setOffset}
          />
        </>
      )}
      <Text size="xs" c="dimmed" mt="md">
        Memory search matches the recorded assertion. Evidence, review,
        freshness and operational assessment remain separate.
      </Text>
      {noting && (
        <MemoryNoteDialog
          brain={brain}
          close={() => setNoting(false)}
          structured={() => {
            setNoting(false);
            setStructuring(true);
          }}
        />
      )}
      {structuring && !catalogue.error && (
        <ClaimEditor
          brain={brain}
          catalogue={catalogue.data}
          onClose={() => setStructuring(false)}
          onSaved={(id) => {
            setStructuring(false);
            patchRoute({ claim: id, revision: null, detail: "record" });
            refresh();
          }}
        />
      )}
      {selected &&
        !catalogue.error &&
        (route.detail === "record" ||
          !!route.knowledge ||
          !!route.revision) && (
          <ClaimDialog
            key={selected}
            brain={brain}
            id={selected}
            catalogue={catalogue.data}
            knowledgeAt={times.knowledge_at}
            factAt={times.fact_at}
            revisionId={route.revision}
            onKnowledgeChange={(knowledge, revision) =>
              patchRoute({
                detail: "record",
                knowledge: knowledge ?? null,
                revision: revision ?? null,
              })
            }
            onClose={() =>
              patchRoute({
                claim: null,
                detail: null,
                knowledge: null,
                revision: null,
              })
            }
            onSaved={refresh}
          />
        )}
    </section>
  );
}
