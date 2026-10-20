import { X } from "lucide-react";
import { ScopeSummary } from "./components/ScopeSummary";
import { Badge, Button, Drawer, Select, TextField } from "./components/ui";
import { instant, label, time } from "./claimsShared";
import { memoryKinds } from "./MemoryForms";
import type { ClaimsRouteState } from "./useClaimsRoute";

// The applied-filter badges with the clear action.
export function ClaimsScopeSummary({ state }: { state: ClaimsRouteState }) {
  const { memoryKind, mode, environment, repository, catalogue, times, clear } =
    state;
  return (
    <ScopeSummary>
      {memoryKind && (
        <Badge>
          {memoryKinds.find((item) => item.value === memoryKind)?.label ??
            label(memoryKind)}
        </Badge>
      )}
      {mode !== "investigation" && (
        <Badge>{label(mode ?? "investigation")}</Badge>
      )}
      {environment && (
        <Badge>
          {catalogue.data?.environments.find((e) => e.id === environment)
            ?.name ?? "Selected environment"}
        </Badge>
      )}
      {repository && (
        <Badge className="max-w-[360px]">
          <span className="truncate">
            {catalogue.data?.repositories.find((r) => r.id === repository)
              ?.canonical_origin ?? "Selected repository"}
          </span>
        </Badge>
      )}
      {times.fact_at && (
        <Badge>
          Fact time <span className="mono">{time(times.fact_at)}</span>
        </Badge>
      )}
      {times.knowledge_at && (
        <Badge>
          Knowledge time{" "}
          <span className="mono">{time(times.knowledge_at)}</span>
        </Badge>
      )}
      <Button variant="ghost" size="sm" onClick={clear}>
        <X aria-hidden="true" className="size-3.5" />
        Clear filters
      </Button>
    </ScopeSummary>
  );
}

export function ClaimsFilterDrawer({
  state,
  kind,
  onKindChange,
}: {
  state: ClaimsRouteState;
  kind?: string | null;
  onKindChange?: (value: string | null) => void;
}) {
  const {
    filtering,
    setFiltering,
    memoryKind,
    setInternalKind,
    mode,
    setMode,
    environment,
    setEnvironment,
    repository,
    setRepository,
    catalogue,
    factDraft,
    setFactDraft,
    knowledgeDraft,
    setKnowledgeDraft,
    setTimes,
    setOffset,
    clear,
  } = state;
  return (
    <Drawer
      opened={filtering}
      onClose={() => setFiltering(false)}
      position="right"
      title="Filter memory"
      description="Search matches subjects, predicates, values and rationale. These filters keep applicability and knowledge history explicit."
      footer={
        <>
          <Button variant="secondary" onClick={clear}>
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
        </>
      }
    >
      <div className="grid gap-4">
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
        <TextField
          type="datetime-local"
          step="1"
          label="Fact time (UTC)"
          inputClassName="mono"
          value={factDraft}
          onChange={(event) => setFactDraft(event.currentTarget.value)}
        />
        <TextField
          type="datetime-local"
          step="1"
          label="Knowledge time (UTC)"
          inputClassName="mono"
          value={knowledgeDraft}
          onChange={(event) => setKnowledgeDraft(event.currentTarget.value)}
        />
      </div>
    </Drawer>
  );
}
