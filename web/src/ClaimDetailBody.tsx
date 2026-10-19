import { Alert, Button, Card, Select, Text, Title } from "@mantine/core";
import {
  Failure,
  Pages,
  States,
  label,
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
    <>
      <details className="feature-details">
        <summary>Version history</summary>
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
      </details>
      {!view && (
        <Alert>
          {data.selection_state === "not_recorded"
            ? "No claim had been recorded at the selected knowledge time."
            : `This revision was ${data.selection_state}. Its content is unavailable; an older revision has not been substituted.`}
        </Alert>
      )}
    </>
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
  return (
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
        {label(r.origin)} by {r.actor_name} · learned {time(r.recorded_at)}
        {view.knowledge_until
          ? ` · replaced ${time(view.knowledge_until)}`
          : ""}
      </Text>
      <Text size="sm">
        Kind:{" "}
        {memoryKinds.find((kind) => kind.value === r.content.kind)?.label ??
          r.content.kind}
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
          {r.derivation.prompt_label}. Model output does not confer human review
          or operational verification.
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
            ({r.content.validity.precision} precision). This does not establish
            uninterrupted operation.
          </Text>
        ) : (
          <Text size="sm">
            From{" "}
            {r.content.validity.from
              ? time(r.content.validity.from)
              : "unknown"}{" "}
            until{" "}
            {r.content.validity.to ? time(r.content.validity.to) : "unknown"}{" "}
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
                (id) => catalogue?.areas.find((a) => a.id === id)?.name ?? id,
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
          {view.eligibility.strict_operational ? "eligible" : "not eligible"}.
          Operational assessment is the author's recorded assessment.
        </Text>
      </Alert>
    </>
  );
}
