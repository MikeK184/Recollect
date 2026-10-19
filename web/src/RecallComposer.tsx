import type { ReactNode } from "react";
import { Badge, Button, Group, Stack, Text, Textarea } from "@mantine/core";
import { Search, SlidersHorizontal } from "lucide-react";
import { label, type Filters } from "./recallFilters";

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
      onSubmit={(e) => {
        e.preventDefault();
        if (!disabled && !fetching) onSearch();
      }}
    >
      <Stack gap="sm">
        <Group gap="xs" className="evidence-search-toolbar">
          <Badge color="gray">
            {selectedCount ? `${selectedCount} scope filters` : "Entire Brain"}
          </Badge>
          {(filters.knowledge || filters.fact) && (
            <Badge color="yellow">Time filtered</Badge>
          )}
          {filters.mode !== "investigation" && (
            <Badge color="brand">{label(filters.mode)}</Badge>
          )}
          <Button
            variant="default"
            size="xs"
            leftSection={<SlidersHorizontal size={15} />}
            onClick={toggleOptions}
            aria-expanded={optionsOpen}
          >
            Refine
          </Button>
          {hasResults && (
            <Button variant="subtle" size="xs" onClick={onClear}>
              Clear results
            </Button>
          )}
        </Group>
        <div className="evidence-search-composer">
          <Textarea
            aria-label="Search evidence"
            placeholder="Find the evidence behind a decision, system or past work…"
            value={queryText}
            onChange={(e) => onQueryChange(e.currentTarget.value)}
            maxLength={512}
            autosize
            minRows={2}
            maxRows={6}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                if (!disabled && !fetching) onSearch();
              }
            }}
          />
          <Button
            type="submit"
            disabled={disabled || fetching}
            loading={fetching && hasResults}
            leftSection={<Search size={16} />}
          >
            Find evidence
          </Button>
        </div>
        <Text size="xs" c="dimmed">
          Finds stored evidence without generating an answer. Automatic
          retrieval may use the permitted embedding model.
        </Text>
        {children}
      </Stack>
    </form>
  );
}
