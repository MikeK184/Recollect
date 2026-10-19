import {
  Button,
  Card,
  Divider,
  Group,
  Loader,
  MultiSelect,
  NumberInput,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
} from "@mantine/core";
import type { Brain } from "./api";
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
    <>
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
    </>
  );
}

export function ClaimValiditySection({ state }: { state: ClaimEditorState }) {
  const { content, patch } = state;
  return (
    <>
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
    </>
  );
}

export function ClaimFreshnessSection({ state }: { state: ClaimEditorState }) {
  const { content, patch } = state;
  return (
    <>
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
    </>
  );
}

export function ClaimSupportsSection({ state }: { state: ClaimEditorState }) {
  const { content, patch, known, setPreview } = state;
  return (
    <>
      <Divider label={`Supporting evidence (${content.supports.length}/20)`} />
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
    </>
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
    <>
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
          No evidence matches. Import a source or publish a repository snapshot
          first.
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
                    item.repository_id && !repos.includes(item.repository_id)
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
    </>
  );
}
