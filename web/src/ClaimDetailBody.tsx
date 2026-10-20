import { ChevronDown, FileSearch } from "lucide-react";
import { InspectorSection, PropList, Quote } from "./components/Inspector";
import { Alert, Button, KindChip, Select } from "./components/ui";
import {
  Failure,
  Pages,
  States,
  label,
  memoryKindChip,
  time,
  type Catalogue,
  type View,
} from "./claimsShared";
import { MemoryFormDetails, memoryKinds } from "./MemoryForms";
import type { ClaimDialogData } from "./useClaimDialogData";

type ClaimData = NonNullable<ClaimDialogData["query"]["data"]>;

// The knowledge-time picker and the notice for a revision that is not shown.
export function ClaimHistorySection({
  data,
  view,
  state,
  onKnowledgeChange,
}: {
  data: ClaimData;
  view?: View | null;
  state: ClaimDialogData;
  onKnowledgeChange?: (at?: string, revision?: string) => void;
}) {
  const { knowledge, setKnowledge, offset, setOffset } = state;
  const history = data.history ?? [];
  const historyOptions = [
    { value: "current", label: "Latest knowledge" },
    ...history.map((r) => ({
      value: r.recorded_at,
      label: `${time(r.recorded_at)} · ${r.content.value.slice(0, 60)}`,
    })),
    ...(data.unavailable_history ?? []).map((r) => ({
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
    <InspectorSection>
      <details className="group/history rounded-lg border border-border bg-card">
        <summary className="flex h-10 cursor-pointer list-none items-center gap-2 px-3 text-label font-medium text-foreground-secondary hover:text-foreground [&::-webkit-details-marker]:hidden">
          Version history
          <ChevronDown
            aria-hidden="true"
            className="ml-auto size-4 text-muted-foreground transition-transform duration-fast group-open/history:rotate-180"
          />
        </summary>
        <div className="grid gap-3 border-t border-border p-3">
          <Select
            label="Knowledge revision"
            data={historyOptions}
            value={knowledge ?? "current"}
            onChange={(value) => {
              const at = value === "current" ? undefined : (value ?? undefined);
              setKnowledge(at);
              const revision = [
                ...history,
                ...(data.unavailable_history ?? []),
              ].find((item) => item.recorded_at === at);
              onKnowledgeChange?.(at, revision?.id);
            }}
          />
          <Pages offset={offset} total={data.total} onChange={setOffset} />
        </div>
      </details>
      {!view && (
        <Alert variant="warning" role="status">
          {data.selection_state === "not_recorded"
            ? "No claim had been recorded at the selected knowledge time."
            : `This revision was ${data.selection_state}. Its content is unavailable; an older revision has not been substituted.`}
        </Alert>
      )}
    </InspectorSection>
  );
}

// The selected revision: statement, provenance, validity, applicability and
// the strict-context eligibility verdict.
export function ClaimDetailBody({
  view,
  state,
  catalogue,
}: {
  view: View;
  state: ClaimDialogData;
  catalogue?: Catalogue;
}) {
  const {
    query,
    manifestId,
    selectedManifest,
    setContribution,
    setSelectedEvidence,
  } = state;
  const r = view.revision;
  const chip = memoryKindChip(r.content.kind);
  const validity = r.content.validity;
  const areas = r.content.selection.area_ids ?? [];
  const repositories = r.content.selection.repository_ids ?? [];
  return (
    <>
      <InspectorSection>
        {r.id !== query.data?.current_revision && (
          <Alert variant="warning" role="status">
            Historical knowledge revision. Later knowledge may differ.
          </Alert>
        )}
        <div className="flex items-center gap-1.5">
          <KindChip kind={chip.kind}>{chip.label}</KindChip>
        </div>
        <h3 className="m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
          {r.content.subject}
        </h3>
        {/* Handover and procedure records repeat their kind as predicate. */}
        {label(r.content.predicate).toLowerCase() !==
          r.content.kind.toLowerCase() && (
          <p className="m-0 text-label font-medium text-foreground-secondary [overflow-wrap:anywhere]">
            {r.content.predicate}
          </p>
        )}
        <div data-testid="claim-value">
          <Quote className="whitespace-pre-wrap [overflow-wrap:anywhere]">
            {r.content.value}
          </Quote>
        </div>
        <States view={view} />
      </InspectorSection>
      <InspectorSection label="Details">
        <PropList
          items={[
            {
              term: "Kind",
              value:
                memoryKinds.find((kind) => kind.value === r.content.kind)
                  ?.label ?? r.content.kind,
            },
            {
              term: "Recorded freshness",
              value: label(r.content.freshness),
            },
            {
              term: "Contributed",
              value: `${label(r.origin)} by ${r.actor_name}`,
            },
            {
              term: "Learned",
              value: (
                <span className="mono text-caption">{time(r.recorded_at)}</span>
              ),
            },
            ...(view.knowledge_until
              ? [
                  {
                    term: "Replaced",
                    value: (
                      <span className="mono text-caption">
                        {time(view.knowledge_until)}
                      </span>
                    ),
                  },
                ]
              : []),
            {
              term: "Environment",
              value:
                catalogue?.environments.find(
                  (e) => e.id === r.content.selection.environment_id,
                )?.name ??
                (r.content.selection.environment_id || "All in this Brain"),
            },
            {
              term: "Areas",
              value: areas.length
                ? areas
                    .map(
                      (id) =>
                        catalogue?.areas.find((a) => a.id === id)?.name ?? id,
                    )
                    .join(", ")
                : "All in this Brain",
            },
          ]}
        />
        {repositories.length > 0 && (
          <ul className="m-0 grid list-none gap-1 p-0">
            {repositories.map((repo) => (
              <li
                key={repo}
                className="mono text-caption text-foreground-secondary [overflow-wrap:anywhere]"
              >
                {catalogue?.repositories.find((p) => p.id === repo)
                  ?.canonical_origin ?? repo}
              </li>
            ))}
          </ul>
        )}
        {r.content.rationale && (
          <p className="m-0 text-body text-foreground-secondary [overflow-wrap:anywhere]">
            {r.content.rationale}
          </p>
        )}
        {r.derivation && (
          <p className="m-0 text-label text-muted-foreground">
            Model derivation: {r.derivation.returned_model} ·{" "}
            {r.derivation.prompt_label}. Model output does not confer human
            review or operational verification.
          </p>
        )}
      </InspectorSection>
      <MemoryFormDetails
        view={view}
        catalogue={catalogue}
        onInspect={(id, at) => setContribution({ id, at })}
      />
      <InspectorSection label="Fact validity">
        {validity.kind === "unknown" ? (
          <p className="m-0 text-body text-foreground-secondary">
            Unknown; no time interval is inferred.
          </p>
        ) : validity.kind === "point" ? (
          <p className="m-0 text-body text-foreground-secondary">
            Point observation:{" "}
            <span className="mono text-caption text-foreground">
              {validity.from ? time(validity.from) : "unknown"}
            </span>{" "}
            ({validity.precision} precision). This does not establish
            uninterrupted operation.
          </p>
        ) : (
          <p className="m-0 text-body text-foreground-secondary">
            From{" "}
            <span className="mono text-caption text-foreground">
              {validity.from ? time(validity.from) : "unknown"}
            </span>{" "}
            until{" "}
            <span className="mono text-caption text-foreground">
              {validity.to ? time(validity.to) : "unknown"}
            </span>{" "}
            (end excluded).
          </p>
        )}
        {r.content.observed_at && (
          <p className="m-0 text-body text-foreground-secondary">
            Outcome observed{" "}
            <span className="mono text-caption text-foreground">
              {time(r.content.observed_at)}
            </span>
            : {r.content.observation}
          </p>
        )}
        {manifestId && (
          <>
            <Failure error={selectedManifest.error} />
            <Button
              variant="secondary"
              size="sm"
              className="w-fit"
              loading={selectedManifest.isPending}
              disabled={!selectedManifest.data || !!selectedManifest.error}
              onClick={() =>
                selectedManifest.data &&
                setSelectedEvidence(selectedManifest.data.evidence)
              }
            >
              <FileSearch aria-hidden="true" />
              Inspect applicability manifest
            </Button>
          </>
        )}
      </InspectorSection>
      <InspectorSection label="Eligibility">
        <Alert
          variant={view.eligibility.strict_accepted ? "brand" : "warning"}
          role="status"
          title={
            view.eligibility.strict_accepted
              ? "Eligible for strict accepted context"
              : "Does not qualify for strict accepted context"
          }
        >
          <p>
            {view.eligibility.reasons.map(label).join(" · ") ||
              "Current accepted evidence"}
          </p>
          <p className="!mt-1.5 text-caption text-muted-foreground">
            Strict operational context:{" "}
            {view.eligibility.strict_operational ? "eligible" : "not eligible"}.
            Operational assessment is the author's recorded assessment.
          </p>
        </Alert>
      </InspectorSection>
    </>
  );
}
