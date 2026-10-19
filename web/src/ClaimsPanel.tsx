import { Button, TextInput } from "@mantine/core";
import { Plus, Search, SlidersHorizontal } from "lucide-react";
import { FilterBar } from "./components/FilterBar";
import { iconSize } from "./design/tokens";
import { ClaimDialog } from "./ClaimDialog";
import { ClaimEditor } from "./ClaimEditor";
import { EvidenceDialog } from "./ClaimEvidenceDialog";
import { ClaimsFilterDrawer, ClaimsScopeSummary } from "./ClaimsFilterDrawer";
import { ClaimsList } from "./ClaimsList";
import { Failure } from "./claimsShared";
import { MemoryNoteDialog } from "./features/memory/MemoryNoteDialog";
import { useClaimsRoute, type ClaimsPanelProps } from "./useClaimsRoute";
import "./features/feature-views.css";

export { ClaimDialog, ClaimEditor, EvidenceDialog };

export function ClaimsPanel(props: ClaimsPanelProps) {
  const { brain, kind, onKindChange } = props;
  const state = useClaimsRoute(props);
  const {
    route,
    patchRoute,
    setOffset,
    search,
    setSearch,
    setFiltering,
    noting,
    setNoting,
    structuring,
    setStructuring,
    selected,
    catalogue,
    claims,
    times,
    refresh,
    filtered,
  } = state;
  return (
    <section
      className="feature-view claims-panel rc-enter"
      aria-label="Memory records"
    >
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
          >
            Add memory
          </Button>
        )}
      </FilterBar>
      {filtered && <ClaimsScopeSummary state={state} />}
      <ClaimsFilterDrawer
        state={state}
        kind={kind}
        onKindChange={onKindChange}
      />
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
      <ClaimsList state={state} />
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
