import { useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Divider,
  Group,
  Loader,
  MultiSelect,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
} from "@mantine/core";
import { useMutation, useQuery } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
type Content = components["schemas"]["ClaimContent"];
type View = components["schemas"]["ClaimView"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];
export const memoryKinds = [
  { value: "claim", label: "Claim" },
  { value: "decision", label: "Decision / intent" },
  { value: "procedure", label: "Procedure / runbook" },
  { value: "handover", label: "Handover" },
];
export function kindContent(kind: string): Partial<Content> {
  return {
    kind,
    procedure:
      kind === "procedure"
        ? { conditions: "", steps: [], expected_outcome: "", observations: [] }
        : null,
    handover:
      kind === "handover"
        ? { completed: [], next_steps: [], risks: [], contributions: [] }
        : null,
    ...(kind === "handover"
      ? {
          predicate: "handover",
          operational: "declared",
          observation: "",
          observed_at: null,
        }
      : {}),
  };
}
function Lines({
  label,
  values,
  onChange,
  required = false,
}: {
  label: string;
  values: string[];
  onChange: (v: string[]) => void;
  required?: boolean;
}) {
  const [draft, setDraft] = useState(values.join("\n"));
  return (
    <Textarea
      label={label}
      description="One entry per line."
      autosize
      minRows={2}
      maxLength={20000}
      required={required}
      value={draft}
      onChange={(e) => {
        setDraft(e.target.value);
        onChange(
          e.target.value
            .split("\n")
            .map((s) => s.trim())
            .filter(Boolean),
        );
      }}
    />
  );
}
export function combineContributions(rows: View[]) {
  const environment =
    rows[0]?.revision.content.selection.environment_id ?? null;
  if (
    rows.some(
      (r) =>
        (r.revision.content.selection.environment_id ?? null) !== environment,
    )
  )
    throw new Error("Choose contributions from the same environment.");
  const supports: Content["supports"] = [];
  const evidence: Evidence[] = [];
  for (const row of rows)
    for (const support of row.revision.content.supports) {
      const old = supports.find(
        (s) => s.kind === support.kind && s.id === support.id,
      );
      if (
        old &&
        (old.line_from !== support.line_from || old.line_to !== support.line_to)
      )
        throw new Error(
          "The selected records use different spans of the same evidence.",
        );
      if (!old) supports.push(support);
      const choice = row.evidence.find(
        (e) => e.id === support.id && e.kind === support.kind,
      );
      if (
        choice &&
        !evidence.some((e) => e.id === choice.id && e.kind === choice.kind)
      )
        evidence.push(choice);
    }
  if (supports.length > 20)
    throw new Error(
      "Choose fewer records; the combined evidence exceeds 20 supports.",
    );
  return {
    selection: {
      environment_id: environment,
      repository_ids: [
        ...new Set(
          rows.flatMap(
            (r) => r.revision.content.selection.repository_ids ?? [],
          ),
        ),
      ].sort(),
      area_ids: [
        ...new Set(
          rows.flatMap((r) => r.revision.content.selection.area_ids ?? []),
        ),
      ].sort(),
    },
    supports,
    evidence,
  };
}
export function ContributionPicker({
  brain,
  ids,
  onChange,
}: {
  brain: Brain;
  ids: string[];
  onChange: (rows: View[]) => void;
}) {
  const [offset, setOffset] = useState(0);
  const choices = useQuery({
    queryKey: ["handover-choices", brain.id, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/claims", {
          params: {
            path: { brain: brain.id },
            query: { mode: "investigation", offset },
          },
        }),
      ),
  });
  const selected = useQuery({
    queryKey: ["handover-selected", brain.id, ids],
    enabled: ids.length > 0,
    queryFn: async () =>
      Promise.all(
        ids.map((revision) =>
          client
            .GET("/api/brains/{brain}/handover-inputs/{revision}", {
              params: { path: { brain: brain.id, revision } },
            })
            .then(result),
        ),
      ),
  });
  const select = useMutation({
    mutationFn: async (next: string[]) => {
      const rows = await Promise.all(
        next.map(async (revision) => {
          const known = [
            ...(selected.data ?? []),
            ...(choices.data?.items ?? []),
          ].find((r) => r.revision.id === revision);
          return (
            known ??
            result(
              await client.GET(
                "/api/brains/{brain}/handover-inputs/{revision}",
                { params: { path: { brain: brain.id, revision } } },
              ),
            )
          );
        }),
      );
      combineContributions(rows);
      onChange(rows);
    },
  });
  const error = select.error ?? selected.error ?? choices.error;
  return (
    <Stack gap="sm">
      <Text fw={600}>Contributing memory</Text>
      <Text size="sm" c="dimmed">
        Choose up to 12 current claims, decisions or procedures from one
        environment. Their exact scope and evidence are preserved.
      </Text>
      {error && <Alert color="red">{error.message}</Alert>}
      {choices.isPending && <Loader size="sm" />}
      {ids.map((id) => (
        <Group key={id} justify="space-between">
          <Text size="sm" className="claim-wrap">
            {selected.data?.find((r) => r.revision.id === id)?.revision.content
              .subject ?? "Selected revision"}
          </Text>
          <Button
            size="compact-xs"
            variant="subtle"
            disabled={select.isPending}
            onClick={() => select.mutate(ids.filter((value) => value !== id))}
          >
            Remove contribution
          </Button>
        </Group>
      ))}
      {choices.data?.items
        .filter((r) => r.revision.content.kind !== "handover")
        .map((row) => (
          <Checkbox
            key={row.revision.id}
            label={
              row.revision.content.subject + " · " + row.revision.content.kind
            }
            description={row.revision.content.value.slice(0, 140)}
            checked={ids.includes(row.revision.id)}
            disabled={
              select.isPending ||
              (!ids.includes(row.revision.id) && ids.length >= 12)
            }
            onChange={(e) =>
              select.mutate(
                e.currentTarget.checked
                  ? [...ids, row.revision.id]
                  : ids.filter((id) => id !== row.revision.id),
              )
            }
          />
        ))}
      {choices.data &&
        !choices.data.items.some(
          (r) => r.revision.content.kind !== "handover",
        ) && <Text c="dimmed">No eligible contributions on this page.</Text>}
      <Group>
        <Button
          size="compact-sm"
          variant="subtle"
          disabled={!offset}
          onClick={() => setOffset(Math.max(0, offset - 20))}
        >
          Previous contributions
        </Button>
        <Button
          size="compact-sm"
          variant="subtle"
          disabled={choices.data?.next_offset == null}
          onClick={() => setOffset(choices.data?.next_offset ?? offset)}
        >
          More contributions
        </Button>
      </Group>
    </Stack>
  );
}
export function MemoryFormFields({
  brain,
  content,
  evidence,
  patch,
  onEvidence,
}: {
  brain: Brain;
  content: Content;
  evidence: Evidence[];
  patch: (v: Partial<Content>) => void;
  onEvidence: (v: Evidence[]) => void;
}) {
  const p = content.procedure;
  const h = content.handover;
  return (
    <>
      {p && (
        <Stack>
          <Divider label="Procedure and observed outcomes" />
          <Textarea
            label="Procedure conditions"
            required
            maxLength={2000}
            value={p.conditions}
            onChange={(e) =>
              patch({ procedure: { ...p, conditions: e.target.value } })
            }
          />
          <Lines
            label="Ordered procedure steps"
            required
            values={p.steps}
            onChange={(steps) => patch({ procedure: { ...p, steps } })}
          />
          <Textarea
            label="Expected outcome"
            required
            maxLength={2000}
            value={p.expected_outcome}
            onChange={(e) =>
              patch({ procedure: { ...p, expected_outcome: e.target.value } })
            }
          />
          {!p.observations.length && (
            <Text c="dimmed">
              Untested: no procedure observations recorded.
            </Text>
          )}
          {p.observations.map((o, index) => {
            const update = (fields: Partial<typeof o>) =>
              patch({
                procedure: {
                  ...p,
                  observations: p.observations.map((value, i) =>
                    i === index ? { ...value, ...fields } : value,
                  ),
                },
              });
            return (
              <Card withBorder key={index}>
                <Stack gap="sm">
                  <Select
                    label={"Observation result " + (index + 1)}
                    data={["success", "failure"]}
                    value={o.result}
                    onChange={(value) => update({ result: value ?? "success" })}
                  />
                  <TextInput
                    type="datetime-local"
                    step={1}
                    required
                    label={"Observation time (UTC) " + (index + 1)}
                    value={o.observed_at.replace(/Z$/, "").slice(0, 19)}
                    onChange={(e) =>
                      update({
                        observed_at: e.target.value
                          ? new Date(e.target.value + "Z").toISOString()
                          : "",
                      })
                    }
                  />
                  <Textarea
                    required
                    label={"Tested conditions " + (index + 1)}
                    maxLength={2000}
                    value={o.conditions}
                    onChange={(e) => update({ conditions: e.target.value })}
                  />
                  <Textarea
                    required
                    label={"Observed result " + (index + 1)}
                    maxLength={2000}
                    value={o.summary}
                    onChange={(e) => update({ summary: e.target.value })}
                  />
                  <MultiSelect
                    searchable
                    label={"Observation evidence " + (index + 1)}
                    data={content.supports.map((s) => ({
                      value: s.id,
                      label:
                        evidence.find((e) => e.id === s.id)?.label ??
                        s.kind.replaceAll("_", " "),
                    }))}
                    value={o.support_ids}
                    onChange={(support_ids) => update({ support_ids })}
                  />
                  <Button
                    variant="subtle"
                    size="xs"
                    color="red"
                    onClick={() =>
                      patch({
                        procedure: {
                          ...p,
                          observations: p.observations.filter(
                            (_, i) => i !== index,
                          ),
                        },
                      })
                    }
                  >
                    Remove observation
                  </Button>
                </Stack>
              </Card>
            );
          })}
          <Button
            variant="light"
            disabled={p.observations.length >= 20}
            onClick={() =>
              patch({
                procedure: {
                  ...p,
                  observations: [
                    ...p.observations,
                    {
                      result: "success",
                      observed_at: new Date().toISOString(),
                      conditions: "",
                      summary: "",
                      support_ids: [],
                    },
                  ],
                },
              })
            }
          >
            Add procedure observation
          </Button>
          <Text size="sm" c="dimmed">
            Observations use this record's environment and revision
            applicability. Recording a procedure does not execute it.
          </Text>
        </Stack>
      )}
      {h && (
        <Stack>
          <Divider label="Handover" />
          <ContributionPicker
            brain={brain}
            ids={h.contributions}
            onChange={(rows) => {
              const combined = combineContributions(rows);
              patch({
                selection: combined.selection,
                supports: combined.supports,
                manifest_revision_id: null,
                handover: {
                  ...h,
                  contributions: rows.map((r) => r.revision.id),
                },
              });
              onEvidence(combined.evidence);
            }}
          />
          <Text size="sm" c="dimmed">
            Contributions set scope and supporting evidence. Add any extra
            evidence after choosing them.
          </Text>
          <Lines
            label="Completed work"
            values={h.completed}
            onChange={(completed) => patch({ handover: { ...h, completed } })}
          />
          <Lines
            label="Next steps"
            values={h.next_steps}
            onChange={(next_steps) => patch({ handover: { ...h, next_steps } })}
          />
          <Lines
            label="Risks and open questions"
            values={h.risks}
            onChange={(risks) => patch({ handover: { ...h, risks } })}
          />
        </Stack>
      )}
    </>
  );
}
export function MemoryFormDetails({
  view,
  catalogue,
  onInspect,
}: {
  view: View;
  catalogue?: Catalogue;
  onInspect: (id: string, at: string) => void;
}) {
  const p = view.revision.content.procedure;
  const h = view.revision.content.handover;
  return (
    <>
      {p && (
        <Stack>
          <Divider label="Procedure" />
          <Text fw={600}>Conditions</Text>
          <Text className="claim-wrap">{p.conditions}</Text>
          <ol>
            {p.steps.map((step, i) => (
              <li key={i} className="claim-wrap">
                {step}
              </li>
            ))}
          </ol>
          <Text fw={600}>Expected outcome</Text>
          <Text className="claim-wrap">{p.expected_outcome}</Text>
          {!p.observations.length && <Badge color="gray">Untested</Badge>}
          {p.observations.map((o, i) => (
            <Card withBorder key={i}>
              <Stack gap="xs">
                <Badge
                  color={o.result === "success" ? "teal" : "orange"}
                  w="fit-content"
                >
                  Observed {o.result}
                </Badge>
                <Text size="sm">
                  {new Date(o.observed_at).toLocaleString()}
                </Text>
                <Text size="sm" className="claim-wrap">
                  {o.conditions}
                </Text>
                <Text className="claim-wrap">{o.summary}</Text>
                <Text size="xs" c="dimmed">
                  {o.support_ids.length} linked evidence supports
                </Text>
              </Stack>
            </Card>
          ))}
          <Text size="sm" c="dimmed">
            These are attributed observations for the applicability below. This
            record does not grant execution permission.
          </Text>
        </Stack>
      )}
      {h && (
        <Stack>
          <Divider label="Handover" />
          {(
            [
              ["Completed work", h.completed],
              ["Next steps", h.next_steps],
              ["Risks and open questions", h.risks],
            ] as const
          ).map(([title, values]) => (
            <div key={title}>
              <Text fw={600}>{title}</Text>
              {values.length ? (
                <ul>
                  {values.map((value, i) => (
                    <li key={i} className="claim-wrap">
                      {value}
                    </li>
                  ))}
                </ul>
              ) : (
                <Text c="dimmed">None recorded.</Text>
              )}
            </div>
          ))}
          <Text fw={600}>Exact contributions</Text>
          {view.contributions?.map((c) => (
            <Card withBorder key={c.revision_id}>
              <Stack gap="xs">
                <Badge
                  color={c.state === "current" ? "gray" : "yellow"}
                  w="fit-content"
                >
                  {c.state}
                </Badge>
                {c.revision ? (
                  <>
                    <Button
                      variant="subtle"
                      justify="start"
                      h="auto"
                      className="claim-evidence-button"
                      onClick={() =>
                        onInspect(c.revision!.claim_id, c.revision!.recorded_at)
                      }
                    >
                      {c.revision.content.subject}
                    </Button>
                    <Text size="sm">
                      Review: {c.revision.review} · Freshness:{" "}
                      {c.eligibility?.effective_freshness} · Assessment:{" "}
                      {c.revision.content.operational}
                    </Text>
                    <Text size="sm" className="claim-wrap">
                      Repositories:{" "}
                      {(c.revision.content.selection.repository_ids ?? [])
                        .map(
                          (id) =>
                            catalogue?.repositories.find((r) => r.id === id)
                              ?.canonical_origin ?? id,
                        )
                        .join(", ") || "All in this Brain"}
                    </Text>
                    <Text size="sm">
                      Environment:{" "}
                      {catalogue?.environments.find(
                        (e) =>
                          e.id === c.revision!.content.selection.environment_id,
                      )?.name ??
                        (c.revision.content.selection.environment_id
                          ? "Selected environment"
                          : "All in this Brain")}
                    </Text>
                    {!!c.revision.content.manifest_revision_id && (
                      <Text size="xs" className="claim-wrap">
                        Exact manifest:{" "}
                        {c.revision.content.manifest_revision_id}
                      </Text>
                    )}
                  </>
                ) : (
                  <Text>Contributing content is unavailable.</Text>
                )}
              </Stack>
            </Card>
          ))}
        </Stack>
      )}
    </>
  );
}
