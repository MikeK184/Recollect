import {
  Badge,
  Button,
  Drawer,
  Select,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { ScopeSummary } from "./components/ScopeSummary";
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
        <Badge variant="light">
          {memoryKinds.find((item) => item.value === memoryKind)?.label ??
            label(memoryKind)}
        </Badge>
      )}
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
        <Badge variant="light">Knowledge time {time(times.knowledge_at)}</Badge>
      )}
      <Button variant="subtle" size="compact-sm" onClick={clear}>
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
  );
}
