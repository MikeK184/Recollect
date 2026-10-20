import type { ReactNode } from "react";
import { Search, SlidersHorizontal, X } from "lucide-react";
import { Badge, Button, Kbds, Textarea, modKey } from "./components/ui";
import { cn } from "./lib/utils";
import { label, type Filters } from "./recallFilters";

// The Find evidence composer: the same rounded-xl surface as the Ask composer,
// with scope badges and the explicit submit in its bar. Submitting is the only
// way a search runs.
export function RecallComposer({
  filters,
  queryText,
  selectedCount,
  optionsOpen,
  toggleOptions,
  hasResults,
  disabled,
  fetching,
  onQueryChange,
  onSearch,
  onClear,
  children,
}: {
  filters: Filters;
  queryText: string;
  selectedCount: number;
  optionsOpen: boolean;
  toggleOptions: () => void;
  hasResults: boolean;
  disabled: boolean;
  fetching: boolean;
  onQueryChange: (value: string) => void;
  onSearch: () => void;
  onClear: () => void;
  children: ReactNode;
}) {
  return (
    <form
      className="grid gap-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (!disabled && !fetching) onSearch();
      }}
    >
      <div
        className={cn(
          "w-full rounded-xl border border-border bg-card shadow-sm transition-[border-color,box-shadow] duration-fast ease-standard focus-within:border-ring focus-within:shadow-focus",
          fetching && "border-brand shadow-glow focus-within:border-brand focus-within:shadow-glow",
        )}
      >
        <Textarea
          aria-label="Search evidence"
          placeholder="Find the evidence behind a decision, system or past work…"
          value={queryText}
          onChange={(e) => onQueryChange(e.currentTarget.value)}
          maxLength={512}
          autosize
          minRows={2}
          maxRows={6}
          className="min-h-[60px] rounded-none border-0 bg-transparent px-4 pb-1 pt-3.5 text-[16px] leading-[26px] shadow-none hover:border-0 focus-visible:shadow-none"
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              if (!disabled && !fetching) onSearch();
            }
          }}
        />
        <div className="flex flex-wrap items-center gap-1.5 px-2.5 pb-2 pt-2">
          <Badge variant="outline">
            {selectedCount ? `${selectedCount} scope filters` : "Entire Brain"}
          </Badge>
          {(filters.knowledge || filters.fact) && (
            <Badge variant="warning">Time filtered</Badge>
          )}
          {filters.mode !== "investigation" && (
            <Badge variant="brand">{label(filters.mode)}</Badge>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={toggleOptions}
            aria-expanded={optionsOpen}
          >
            <SlidersHorizontal aria-hidden="true" />
            Refine
          </Button>
          {hasResults && (
            <Button variant="ghost" size="sm" onClick={onClear}>
              <X aria-hidden="true" />
              Clear results
            </Button>
          )}
          <Kbds keys={[modKey, "↵"]} className="ml-auto mr-1" />
          <Button
            type="submit"
            disabled={disabled || fetching}
            loading={fetching && hasResults}
          >
            {!(fetching && hasResults) && <Search aria-hidden="true" />}
            Find evidence
          </Button>
        </div>
      </div>
      <p className="m-0 px-1 text-caption text-muted-foreground">
        Finds stored evidence without generating an answer. Automatic retrieval
        may use the permitted embedding model.
      </p>
      {children}
    </form>
  );
}
