import { useId, useState, type ReactNode } from "react";
import { Plus, Trash2, X } from "lucide-react";
import { useMutation, useQuery } from "@tanstack/react-query";
import {
  Alert,
  Badge,
  Button,
  Checkbox,
  MultiSelect,
  Select,
  StatusBadge,
  TextField,
  TextareaField,
} from "./components/ui";
import { InspectorSection } from "./components/Inspector";
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
/** A hairline group heading inside the memory editor form. */
function FormGroup({
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
const muted = "m-0 text-label text-muted-foreground";
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
    <TextareaField
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
  const eligible =
    choices.data?.items.filter((r) => r.revision.content.kind !== "handover") ??
    [];
  return (
    <div className="grid gap-3">
      <div className="grid gap-1">
        <p className="m-0 text-body font-medium text-foreground">
          Contributing memory
        </p>
        <p className={muted}>
          Choose up to 12 current claims, decisions or procedures from one
          environment. Their exact scope and evidence are preserved.
        </p>
      </div>
      {error && <Alert variant="destructive">{error.message}</Alert>}
      {choices.isPending && (
        <p role="status" className={muted}>
          Loading contributions…
        </p>
      )}
      {ids.length > 0 && (
        <ul className="m-0 grid list-none gap-1.5 p-0">
          {ids.map((id) => (
            <li
              key={id}
              className="flex items-center justify-between gap-3 rounded-md bg-muted px-3 py-1.5"
            >
              <span className="min-w-0 text-label text-foreground [overflow-wrap:anywhere]">
                {selected.data?.find((r) => r.revision.id === id)?.revision
                  .content.subject ?? "Selected revision"}
              </span>
              <Button
                size="sm"
                variant="ghost"
                disabled={select.isPending}
                onClick={() =>
                  select.mutate(ids.filter((value) => value !== id))
                }
              >
                <X aria-hidden="true" />
                Remove contribution
              </Button>
            </li>
          ))}
        </ul>
      )}
      {eligible.length > 0 && (
        <ul className="m-0 grid max-h-[280px] list-none overflow-auto rounded-lg border border-border p-0 scrollbar-thin">
          {eligible.map((row) => (
            <ContributionChoice
              key={row.revision.id}
              row={row}
              checked={ids.includes(row.revision.id)}
              disabled={
                select.isPending ||
                (!ids.includes(row.revision.id) && ids.length >= 12)
              }
              onChange={(checked) =>
                select.mutate(
                  checked
                    ? [...ids, row.revision.id]
                    : ids.filter((id) => id !== row.revision.id),
                )
              }
            />
          ))}
        </ul>
      )}
      {choices.data && !eligible.length && (
        <p className={muted}>No eligible contributions on this page.</p>
      )}
      <div className="flex gap-2">
        <Button
          size="sm"
          variant="ghost"
          disabled={!offset}
          onClick={() => setOffset(Math.max(0, offset - 20))}
        >
          Previous contributions
        </Button>
        <Button
          size="sm"
          variant="ghost"
          disabled={choices.data?.next_offset == null}
          onClick={() => setOffset(choices.data?.next_offset ?? offset)}
        >
          More contributions
        </Button>
      </div>
    </div>
  );
}
/** One selectable contribution; the value preview describes the checkbox. */
function ContributionChoice({
  row,
  checked,
  disabled,
  onChange,
}: {
  row: View;
  checked: boolean;
  disabled: boolean;
  onChange: (checked: boolean) => void;
}) {
  const id = useId();
  return (
    <li className="grid gap-0.5 border-b border-border px-3 py-2 last:border-b-0">
      <Checkbox
        label={row.revision.content.subject + " · " + row.revision.content.kind}
        aria-describedby={`${id}-value`}
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.currentTarget.checked)}
      />
      <span
        id={`${id}-value`}
        className="pl-[26px] text-caption text-muted-foreground [overflow-wrap:anywhere]"
      >
        {row.revision.content.value.slice(0, 140)}
      </span>
    </li>
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
        <FormGroup label="Procedure and observed outcomes">
          <TextareaField
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
          <TextareaField
            label="Expected outcome"
            required
            maxLength={2000}
            value={p.expected_outcome}
            onChange={(e) =>
              patch({ procedure: { ...p, expected_outcome: e.target.value } })
            }
          />
          {!p.observations.length && (
            <p className={muted}>
              Untested: no procedure observations recorded.
            </p>
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
              <div
                key={index}
                className="grid gap-3 rounded-lg border border-border bg-card p-3"
              >
                <div className="grid grid-cols-2 gap-3">
                  <Select
                    label={"Observation result " + (index + 1)}
                    data={["success", "failure"]}
                    value={o.result}
                    onChange={(value) => update({ result: value ?? "success" })}
                  />
                  <TextField
                    type="datetime-local"
                    step={1}
                    required
                    inputClassName="mono"
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
                </div>
                <TextareaField
                  required
                  label={"Tested conditions " + (index + 1)}
                  maxLength={2000}
                  value={o.conditions}
                  onChange={(e) => update({ conditions: e.target.value })}
                />
                <TextareaField
                  required
                  label={"Observed result " + (index + 1)}
                  maxLength={2000}
                  value={o.summary}
                  onChange={(e) => update({ summary: e.target.value })}
                />
                <MultiSelect
                  inputClassName="relative"
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
                  variant="destructive-ghost"
                  size="sm"
                  className="w-fit"
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
                  <Trash2 aria-hidden="true" />
                  Remove observation
                </Button>
              </div>
            );
          })}
          <Button
            variant="secondary"
            size="sm"
            className="w-fit"
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
            <Plus aria-hidden="true" />
            Add procedure observation
          </Button>
          <p className={muted}>
            Observations use this record's environment and revision
            applicability. Recording a procedure does not execute it.
          </p>
        </FormGroup>
      )}
      {h && (
        <FormGroup label="Handover">
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
          <p className={muted}>
            Contributions set scope and supporting evidence. Add any extra
            evidence after choosing them.
          </p>
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
        </FormGroup>
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
        <InspectorSection label="Procedure">
          <div className="grid gap-1">
            <p className="m-0 text-label font-medium text-foreground">
              Conditions
            </p>
            <p className="m-0 text-body text-foreground-secondary [overflow-wrap:anywhere]">
              {p.conditions}
            </p>
          </div>
          <ol className="m-0 grid gap-1 pl-5 text-body text-foreground-secondary">
            {p.steps.map((step, i) => (
              <li key={i} className="[overflow-wrap:anywhere]">
                {step}
              </li>
            ))}
          </ol>
          <div className="grid gap-1">
            <p className="m-0 text-label font-medium text-foreground">
              Expected outcome
            </p>
            <p className="m-0 text-body text-foreground-secondary [overflow-wrap:anywhere]">
              {p.expected_outcome}
            </p>
          </div>
          {!p.observations.length && (
            <div>
              <Badge>Untested</Badge>
            </div>
          )}
          {p.observations.map((o, i) => (
            <div
              key={i}
              className="grid gap-1.5 rounded-lg border border-border bg-card p-3"
            >
              <div className="flex flex-wrap items-center gap-2">
                <StatusBadge
                  state={o.result === "success" ? "positive" : "attention"}
                >
                  Observed {o.result}
                </StatusBadge>
                <span className="mono text-caption text-muted-foreground">
                  {new Date(o.observed_at).toLocaleString()}
                </span>
              </div>
              <p className="m-0 text-label text-foreground-secondary [overflow-wrap:anywhere]">
                {o.conditions}
              </p>
              <p className="m-0 text-body text-foreground [overflow-wrap:anywhere]">
                {o.summary}
              </p>
              <p className="m-0 text-caption text-muted-foreground">
                <span className="mono">{o.support_ids.length}</span> linked
                evidence supports
              </p>
            </div>
          ))}
          <p className={muted}>
            These are attributed observations for the applicability below. This
            record does not grant execution permission.
          </p>
        </InspectorSection>
      )}
      {h && (
        <InspectorSection label="Handover">
          {(
            [
              ["Completed work", h.completed],
              ["Next steps", h.next_steps],
              ["Risks and open questions", h.risks],
            ] as const
          ).map(([title, values]) => (
            <div key={title} className="grid gap-1">
              <p className="m-0 text-label font-medium text-foreground">
                {title}
              </p>
              {values.length ? (
                <ul className="m-0 grid gap-1 pl-5 text-body text-foreground-secondary">
                  {values.map((value, i) => (
                    <li key={i} className="[overflow-wrap:anywhere]">
                      {value}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className={muted}>None recorded.</p>
              )}
            </div>
          ))}
          <p className="m-0 text-label font-medium text-foreground">
            Exact contributions
          </p>
          {view.contributions?.map((c) => (
            <div
              key={c.revision_id}
              className="grid gap-1.5 rounded-lg border border-border bg-card p-3"
            >
              <div>
                {c.state === "current" ? (
                  <Badge>{c.state}</Badge>
                ) : (
                  <StatusBadge state="attention">{c.state}</StatusBadge>
                )}
              </div>
              {c.revision ? (
                <>
                  <Button
                    variant="link"
                    className="claim-evidence-button w-fit justify-start text-left [overflow-wrap:anywhere] whitespace-normal"
                    onClick={() =>
                      onInspect(c.revision!.claim_id, c.revision!.recorded_at)
                    }
                  >
                    {c.revision.content.subject}
                  </Button>
                  <p className="m-0 text-label text-foreground-secondary">
                    Review: {c.revision.review} · Freshness:{" "}
                    {c.eligibility?.effective_freshness} · Assessment:{" "}
                    {c.revision.content.operational}
                  </p>
                  <p className="m-0 text-label text-foreground-secondary [overflow-wrap:anywhere]">
                    Repositories:{" "}
                    {(c.revision.content.selection.repository_ids ?? [])
                      .map(
                        (id) =>
                          catalogue?.repositories.find((r) => r.id === id)
                            ?.canonical_origin ?? id,
                      )
                      .join(", ") || "All in this Brain"}
                  </p>
                  <p className="m-0 text-label text-foreground-secondary">
                    Environment:{" "}
                    {catalogue?.environments.find(
                      (e) =>
                        e.id === c.revision!.content.selection.environment_id,
                    )?.name ??
                      (c.revision.content.selection.environment_id
                        ? "Selected environment"
                        : "All in this Brain")}
                  </p>
                  {!!c.revision.content.manifest_revision_id && (
                    <p className="m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
                      Exact manifest:{" "}
                      <span className="mono">
                        {c.revision.content.manifest_revision_id}
                      </span>
                    </p>
                  )}
                </>
              ) : (
                <p className={muted}>Contributing content is unavailable.</p>
              )}
            </div>
          ))}
        </InspectorSection>
      )}
    </>
  );
}
