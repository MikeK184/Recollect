import { useRef } from "react";
import { ChevronDown, Plus, SlidersHorizontal } from "lucide-react";
import { FilterBar } from "./components/FilterBar";
import {
  Alert,
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  SearchInput,
} from "./components/ui";
import { ClaimDialog } from "./ClaimDialog";
import { ClaimEditor } from "./ClaimEditor";
import { EvidenceDialog } from "./ClaimEvidenceDialog";
import { ClaimsFilterDrawer, ClaimsScopeSummary } from "./ClaimsFilterDrawer";
import { ClaimsList } from "./ClaimsList";
import { memoryKindChip } from "./claimsShared";
import { memoryKinds } from "./MemoryForms";
import { MemoryNoteDialog } from "./features/memory/MemoryNoteDialog";
import {
  ExploreHeaderActions,
  useExploreCount,
} from "./features/knowledge/frame";
import { useClaimsRoute, type ClaimsPanelProps } from "./useClaimsRoute";

export { ClaimDialog, ClaimEditor, EvidenceDialog };

export function ClaimsPanel(props: ClaimsPanelProps) {
  const { brain, kind, onKindChange, onOpenHandovers } = props;
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
    data,
    memoryKind,
    setInternalKind,
    offset,
  } = state;
  const writable = brain.role !== "reader" && !brain.archived;
  const addButton = useRef<HTMLButtonElement>(null);
  // Overlays that replace one another (note → structured editor, lineage →
  // record history) lose their opener; give focus back to the control the
  // person started from once the last one closes.
  const refocus = (target: () => HTMLElement | null | undefined) =>
    window.setTimeout(() => {
      const node = target();
      if (node?.isConnected) node.focus({ preventScroll: true });
    }, 60);
  useExploreCount("memory", data ? data.total_candidates : null);
  const shown = data?.items.length ?? 0;
  const paged = !!data && (offset > 0 || data.next_offset != null);
  const countLabel = !data
    ? ""
    : paged
      ? `${shown ? offset + 1 : 0}–${offset + shown} of ${data.total_candidates}`
      : `${shown} ${shown === 1 ? "memory" : "memories"}`;
  const error = catalogue.error ?? claims.error;
  return (
    <section
      className="flex min-w-0 flex-col gap-4"
      aria-label="Memory records"
    >
      {writable && (
        <ExploreHeaderActions>
          <Button ref={addButton} onClick={() => setNoting(true)}>
            <Plus aria-hidden="true" />
            Add memory
          </Button>
        </ExploreHeaderActions>
      )}
      <FilterBar>
        <div className="w-[280px] max-w-full">
          <SearchInput
            size="sm"
            aria-label="Search memory"
            placeholder="Filter memory"
            maxLength={200}
            value={search}
            onChange={(event) => {
              setSearch(event.currentTarget.value);
              setOffset(0);
            }}
          />
        </div>
        {kind === undefined && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className={memoryKind ? "text-foreground" : undefined}
              >
                {memoryKind
                  ? `Kind: ${memoryKindChip(memoryKind).label}`
                  : "Kind"}
                <ChevronDown aria-hidden="true" className="size-3.5" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuLabel>Memory kind</DropdownMenuLabel>
              <DropdownMenuRadioGroup
                value={memoryKind ?? "all"}
                onValueChange={(value) => {
                  const next = value === "all" ? null : value;
                  setInternalKind(next);
                  onKindChange?.(next);
                  setOffset(0);
                }}
              >
                <DropdownMenuRadioItem value="all">
                  All kinds
                </DropdownMenuRadioItem>
                <DropdownMenuSeparator />
                {memoryKinds.map((item) => (
                  <DropdownMenuRadioItem key={item.value} value={item.value}>
                    {item.label}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
        <Button
          variant="ghost"
          size="sm"
          className={filtered ? "text-foreground" : undefined}
          onClick={() => setFiltering(true)}
        >
          <SlidersHorizontal aria-hidden="true" className="size-3.5" />
          Filters{filtered ? " · applied" : ""}
        </Button>
        {countLabel && (
          <span className="mono ml-auto text-caption text-muted-foreground">
            {countLabel}
          </span>
        )}
      </FilterBar>
      {filtered && <ClaimsScopeSummary state={state} />}
      <ClaimsFilterDrawer
        state={state}
        kind={kind}
        onKindChange={onKindChange}
      />
      {error && (
        <Alert
          variant="destructive"
          title="Request failed"
          actions={
            <Button
              variant="destructive-ghost"
              size="sm"
              onClick={() => {
                void catalogue.refetch();
                void claims.refetch();
              }}
            >
              Retry memory
            </Button>
          }
        >
          {error.message}
        </Alert>
      )}
      <ClaimsList state={state} onOpenHandovers={onOpenHandovers} />
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
          onClose={() => {
            setStructuring(false);
            refocus(() => addButton.current);
          }}
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
            onClose={() => {
              patchRoute({
                claim: null,
                detail: null,
                knowledge: null,
                revision: null,
              });
              refocus(() =>
                document.querySelector<HTMLElement>(
                  `tr[data-claim="${selected}"] button`,
                ),
              );
            }}
            onSaved={refresh}
          />
        )}
    </section>
  );
}
