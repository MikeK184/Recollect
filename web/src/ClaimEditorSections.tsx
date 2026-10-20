import type { ReactNode } from "react";
import { Eye, Plus, X } from "lucide-react";
import type { Brain } from "./api";
import {
  Button,
  MultiSelect,
  NumberField,
  Select,
  TextField,
  TextareaField,
} from "./components/ui";
import {
  Failure,
  Pages,
  freshOptions,
  instant,
  label,
  opOptions,
  time,
  utc,
  type Catalogue,
} from "./claimsShared";
import { MemoryFormFields, kindContent, memoryKinds } from "./MemoryForms";
import type { ClaimEditorState } from "./useClaimEditorState";

/** A hairline group heading inside the editor form. */
export function EditorGroup({
  label: text,
  children,
}: {
  label: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="grid min-w-0 gap-4 border-t border-border pt-4">
      <h3 className="section-label m-0">{text}</h3>
      {children}
    </section>
  );
}

export function ClaimStatementSection({
  brain,
  state,
}: {
  brain: Brain;
  state: ClaimEditorState;
}) {
  const { content, patch, known, setKnown } = state;
  return (
    <>
      <Select
        label="Memory kind"
        data={memoryKinds}
        value={content.kind}
        onChange={(v) => patch(kindContent(v ?? "claim"))}
      />
      <div className="grid grid-cols-2 gap-4">
        <TextField
          label="Subject"
          placeholder="Production Vault"
          required
          maxLength={256}
          value={content.subject}
          onChange={(e) => patch({ subject: e.target.value })}
        />
        <TextField
          label="Property or relationship"
          placeholder="authentication method"
          required
          maxLength={128}
          value={content.predicate}
          onChange={(e) => patch({ predicate: e.target.value })}
        />
      </div>
      <TextareaField
        label="Claim value"
        required
        autosize
        minRows={2}
        maxLength={4000}
        value={content.value}
        onChange={(e) => patch({ value: e.target.value })}
      />
      <TextareaField
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
    </>
  );
}

export function ClaimApplicabilitySection({
  catalogue,
  state,
}: {
  catalogue?: Catalogue;
  state: ClaimEditorState;
}) {
  const {
    content,
    patch,
    repositories,
    manifestOptions,
    manifests,
    manifestOffset,
    setManifestOffset,
  } = state;
  return (
    <EditorGroup label="Applicability">
      <MultiSelect
        inputClassName="relative"
        label="Repositories"
        searchable
        data={repositories}
        value={content.selection.repository_ids ?? []}
        onChange={(repository_ids) =>
          patch({ selection: { ...content.selection, repository_ids } })
        }
      />
      <div className="grid grid-cols-2 gap-4">
        <MultiSelect
          inputClassName="relative"
          label="Areas"
          searchable
          data={
            catalogue?.areas.map((a) => ({ value: a.id, label: a.name })) ?? []
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
      </div>
      <p className="m-0 -mt-2 text-caption text-muted-foreground">
        Empty applicability applies throughout this Brain. Evidence from a
        repository requires selecting that repository.
      </p>
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
    </EditorGroup>
  );
}

export function ClaimValiditySection({ state }: { state: ClaimEditorState }) {
  const { content, patch } = state;
  return (
    <EditorGroup label="Fact validity">
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
        <div className="grid grid-cols-2 gap-4">
          <TextField
            type="datetime-local"
            step="1"
            inputClassName="mono"
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
            <TextField
              type="datetime-local"
              step="1"
              inputClassName="mono"
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
        </div>
      )}
    </EditorGroup>
  );
}

export function ClaimFreshnessSection({ state }: { state: ClaimEditorState }) {
  const { content, patch } = state;
  const observed = ["deployed", "verified"].includes(content.operational);
  return (
    <EditorGroup label="Freshness and outcome">
      <div className="grid grid-cols-2 gap-4">
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
      </div>
      <TextField
        type="datetime-local"
        step="1"
        inputClassName="mono"
        label="Outcome observed at (UTC)"
        required={observed}
        value={utc(content.observed_at)}
        onChange={(e) => patch({ observed_at: instant(e.target.value) })}
      />
      <TextareaField
        label="Observed outcome"
        maxLength={2000}
        required={observed}
        value={content.observation}
        onChange={(e) => patch({ observation: e.target.value })}
      />
    </EditorGroup>
  );
}

export function ClaimSupportsSection({ state }: { state: ClaimEditorState }) {
  const { content, patch, known, setPreview } = state;
  return (
    <EditorGroup
      label={
        <>
          Supporting evidence{" "}
          <span className="mono">({content.supports.length}/20)</span>
        </>
      }
    >
      {!content.supports.length && (
        <p className="m-0 text-label text-muted-foreground">
          Choose at least one exact piece of evidence below.
        </p>
      )}
      {content.supports.map((support, index) => {
        const item = known.find(
          (e) => e.id === support.id && e.kind === support.kind,
        );
        return (
          <div
            key={`${support.kind}:${support.id}`}
            className="grid gap-3 rounded-lg border border-border bg-card p-3"
          >
            <div className="flex items-start justify-between gap-3">
              <span className="min-w-0 text-label font-medium text-foreground [overflow-wrap:anywhere]">
                {item?.label ?? (
                  <span className="mono text-caption">{support.id}</span>
                )}
              </span>
              <div className="flex shrink-0 gap-1">
                {item && (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setPreview(item)}
                  >
                    <Eye aria-hidden="true" />
                    Inspect
                  </Button>
                )}
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() =>
                    patch({
                      supports: content.supports.filter((_, i) => i !== index),
                    })
                  }
                >
                  <X aria-hidden="true" />
                  Remove support
                </Button>
              </div>
            </div>
            {support.kind === "source_version" && (
              <div className="grid grid-cols-2 gap-3">
                {(["line_from", "line_to"] as const).map((key) => (
                  <NumberField
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
              </div>
            )}
          </div>
        );
      })}
    </EditorGroup>
  );
}

export function ClaimEvidencePicker({ state }: { state: ClaimEditorState }) {
  const {
    content,
    patch,
    setKnown,
    kind,
    setKind,
    search,
    setSearch,
    offset,
    setOffset,
    choices,
  } = state;
  return (
    <EditorGroup label="Evidence search">
      <div className="grid grid-cols-2 gap-4">
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
        <TextField
          label="Find evidence"
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            setOffset(0);
          }}
        />
      </div>
      <Failure error={choices.error} />
      {choices.isPending && (
        <p
          role="status"
          className="m-0 flex items-center gap-2 text-label text-muted-foreground"
        >
          <span
            aria-hidden="true"
            className="size-3.5 animate-spin rounded-full border-2 border-border-strong border-t-transparent"
          />
          Loading evidence…
        </p>
      )}
      {choices.data?.items.length === 0 && (
        <p className="m-0 text-label text-muted-foreground">
          No evidence matches. Import a source or publish a repository snapshot
          first.
        </p>
      )}
      {!!choices.data?.items.length && (
        <ul className="m-0 grid list-none overflow-hidden rounded-lg border border-border p-0">
          {choices.data.items.map((item) => (
            <li
              key={`${item.kind}:${item.id}`}
              className="flex items-center justify-between gap-3 border-b border-border px-3 py-2 last:border-b-0"
            >
              <div className="grid min-w-0 gap-0.5">
                <span className="text-label font-medium text-foreground [overflow-wrap:anywhere]">
                  {item.label}
                </span>
                <span className="mono text-caption text-muted-foreground">
                  {time(item.created_at)} · {label(item.availability)}
                  {item.revision ? ` · ${item.revision.slice(0, 12)}` : ""}
                </span>
              </div>
              <Button
                size="sm"
                variant="secondary"
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
                <Plus aria-hidden="true" />
                Use evidence
              </Button>
            </li>
          ))}
        </ul>
      )}
      <Pages
        offset={offset}
        total={choices.data?.total ?? 0}
        onChange={setOffset}
      />
    </EditorGroup>
  );
}
