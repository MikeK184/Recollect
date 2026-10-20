import * as React from "react";
import { Popover as PopoverPrimitive } from "radix-ui";
import { Check, ChevronsUpDown, X } from "lucide-react";
import { cn } from "../../lib/utils";
import { Field, controlClass, useFieldControl } from "./field";

// Select and MultiSelect: an input-based combobox with a listbox popover.
// The control stays an <input>, so getByLabel()/getByRole("textbox") and
// toHaveValue(label) keep working; options are role="option" in a listbox.
// The textbox role does not allow aria-expanded: aria-controls (set while
// open) and aria-activedescendant carry the popup state instead.

export type SelectItem = { value: string; label: string; disabled?: boolean };
export type SelectGroup = { group: string; items: (string | SelectItem)[] };
export type SelectData = readonly (string | SelectItem | SelectGroup)[];

function normalize(data: SelectData) {
  const groups: { label?: string; items: SelectItem[] }[] = [];
  let loose: { items: SelectItem[] } | null = null;
  const item = (value: string | SelectItem): SelectItem =>
    typeof value === "string" ? { value, label: value } : value;
  for (const entry of data) {
    if (typeof entry === "object" && "group" in entry) {
      loose = null;
      groups.push({ label: entry.group, items: entry.items.map(item) });
    } else {
      if (!loose) {
        loose = { items: [] };
        groups.push(loose);
      }
      loose.items.push(item(entry));
    }
  }
  return groups;
}

type FieldProps = {
  label?: React.ReactNode;
  description?: React.ReactNode;
  error?: React.ReactNode;
  required?: boolean;
  className?: string;
  /** Class for the control itself (the field wrapper takes className). */
  inputClassName?: string;
};

type ListboxProps = {
  id: string;
  groups: { label?: string; items: SelectItem[] }[];
  active: string | null;
  isSelected: (value: string) => boolean;
  onPick: (item: SelectItem) => void;
  onHover: (value: string) => void;
  nothingFound: React.ReactNode;
  maxHeight?: number;
};

function Listbox({
  id,
  groups,
  active,
  isSelected,
  onPick,
  onHover,
  nothingFound,
  maxHeight = 280,
}: ListboxProps) {
  const list = React.useRef<HTMLDivElement>(null);
  React.useEffect(() => {
    if (!active) return;
    list.current
      ?.querySelector(`[data-value="${CSS.escape(active)}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [active]);
  const empty = groups.every((group) => group.items.length === 0);
  return (
    <div
      ref={list}
      id={id}
      role="listbox"
      className="overflow-auto p-1 scrollbar-thin"
      style={{ maxHeight }}
    >
      {empty ? (
        <div className="px-2 py-2 text-label text-muted-foreground">
          {nothingFound}
        </div>
      ) : (
        groups.map((group, index) =>
          group.items.length ? (
            <div
              key={group.label ?? `group-${index}`}
              role={group.label ? "group" : undefined}
              aria-label={group.label}
            >
              {group.label && (
                <div className="px-2 pb-1 pt-2 text-caption font-medium text-muted-foreground">
                  {group.label}
                </div>
              )}
              {group.items.map((item) => {
                const selected = isSelected(item.value);
                return (
                  <div
                    key={item.value}
                    id={`${id}-${item.value}`}
                    data-value={item.value}
                    role="option"
                    aria-selected={selected}
                    aria-disabled={item.disabled || undefined}
                    data-active={active === item.value || undefined}
                    className={cn(
                      "flex h-8 cursor-pointer select-none items-center gap-2 rounded-sm px-2 text-body text-foreground",
                      "data-[active]:bg-accent",
                      item.disabled && "cursor-not-allowed opacity-50",
                    )}
                    onMouseDown={(event) => event.preventDefault()}
                    onMouseMove={() => onHover(item.value)}
                    onClick={() => !item.disabled && onPick(item)}
                  >
                    <span className="min-w-0 flex-1 truncate">{item.label}</span>
                    {selected && (
                      <Check
                        className="size-4 shrink-0 text-brand-text"
                        aria-hidden="true"
                      />
                    )}
                  </div>
                );
              })}
            </div>
          ) : null,
        )
      )}
    </div>
  );
}

function useCombobox(
  flat: SelectItem[],
  onPick: (item: SelectItem) => void,
  current: string | null,
) {
  const [open, setOpen] = React.useState(false);
  const [active, setActive] = React.useState<string | null>(null);
  const enabled = flat.filter((item) => !item.disabled);
  const move = (step: 1 | -1) => {
    if (!enabled.length) return;
    const index = enabled.findIndex((item) => item.value === active);
    const next =
      index < 0
        ? step === 1
          ? 0
          : enabled.length - 1
        : (index + step + enabled.length) % enabled.length;
    setActive(enabled[next].value);
  };
  const show = () => {
    setOpen(true);
    setActive(current ?? enabled[0]?.value ?? null);
  };
  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!open) show();
      else move(event.key === "ArrowDown" ? 1 : -1);
    } else if (event.key === "Enter") {
      if (!open) return;
      event.preventDefault();
      const item = enabled.find((entry) => entry.value === active);
      if (item) onPick(item);
    } else if (event.key === "Escape") {
      if (open) {
        event.preventDefault();
        event.stopPropagation();
        setOpen(false);
      }
    } else if (event.key === "Tab") {
      setOpen(false);
    }
  };
  return { open, setOpen, active, setActive, show, onKeyDown };
}

const contentClass =
  "z-50 w-(--radix-popover-trigger-width) min-w-40 overflow-hidden rounded-lg border border-border bg-popover text-popover-foreground shadow-md outline-none data-[state=open]:animate-pop";

export function Select({
  data,
  value,
  onChange,
  label,
  description,
  error,
  required,
  className,
  inputClassName,
  placeholder,
  searchable = false,
  clearable = false,
  allowDeselect = false,
  nothingFoundMessage = "No matches",
  size = "default",
  disabled,
  readOnly,
  id,
  name,
  maxDropdownHeight,
  leftSection,
  limit,
  comboboxProps: _comboboxProps,
  ...aria
}: FieldProps & {
  data: SelectData;
  value: string | null | undefined;
  onChange: (value: string | null) => void;
  placeholder?: string;
  searchable?: boolean;
  clearable?: boolean;
  allowDeselect?: boolean;
  nothingFoundMessage?: React.ReactNode;
  size?: "sm" | "default" | "lg";
  disabled?: boolean;
  readOnly?: boolean;
  id?: string;
  name?: string;
  maxDropdownHeight?: number;
  leftSection?: React.ReactNode;
  /** Render at most this many matching options. */
  limit?: number;
  /** Accepted for call-site compatibility; positioning is automatic. */
  comboboxProps?: unknown;
  "aria-label"?: string;
  "aria-describedby"?: string;
}) {
  const control = (
    <SelectControl
      {...{
        data,
        value,
        onChange,
        placeholder,
        searchable,
        clearable,
        allowDeselect,
        nothingFoundMessage,
        size,
        disabled,
        readOnly,
        id,
        name,
        maxDropdownHeight,
        leftSection,
        limit,
        inputClassName,
        required,
        ...aria,
      }}
    />
  );
  if (!label && !description && !error)
    return className ? <div className={className}>{control}</div> : control;
  return (
    <Field
      label={label}
      description={description}
      error={error}
      required={required}
      id={id}
      className={className}
    >
      {control}
    </Field>
  );
}

function SelectControl({
  data,
  value,
  onChange,
  placeholder,
  searchable,
  clearable,
  allowDeselect,
  nothingFoundMessage,
  size,
  maxDropdownHeight,
  leftSection,
  limit,
  inputClassName,
  ...rest
}: {
  data: SelectData;
  value: string | null | undefined;
  onChange: (value: string | null) => void;
  placeholder?: string;
  searchable: boolean;
  clearable: boolean;
  allowDeselect: boolean;
  nothingFoundMessage: React.ReactNode;
  size: "sm" | "default" | "lg";
  maxDropdownHeight?: number;
  leftSection?: React.ReactNode;
  limit?: number;
  inputClassName?: string;
  disabled?: boolean;
  readOnly?: boolean;
  required?: boolean;
  id?: string;
  name?: string;
  "aria-label"?: string;
  "aria-describedby"?: string;
}) {
  const props = useFieldControl(rest);
  const generated = React.useId();
  const listId = `${props.id ?? generated}-listbox`;
  const groups = React.useMemo(() => normalize(data), [data]);
  const flat = React.useMemo(() => groups.flatMap((g) => g.items), [groups]);
  const selected = flat.find((item) => item.value === value) ?? null;
  const [search, setSearch] = React.useState<string | null>(null);
  const pick = (item: SelectItem) => {
    const next = allowDeselect && item.value === value ? null : item.value;
    onChange(next);
    setSearch(null);
    combo.setOpen(false);
  };
  const query = search?.trim().toLowerCase() ?? "";
  const filtered = query
    ? groups.map((group) => ({
        ...group,
        items: group.items.filter((item) =>
          item.label.toLowerCase().includes(query),
        ),
      }))
    : groups;
  let budget = limit ?? Infinity;
  const visible = filtered.map((group) => {
    const items = group.items.slice(0, Math.max(0, budget));
    budget -= items.length;
    return { ...group, items };
  });
  // Arrow keys move through what is shown: after typing, ArrowDown goes to
  // the first match.
  const combo = useCombobox(
    visible.flatMap((group) => group.items),
    pick,
    search === null ? (value ?? null) : null,
  );
  const interactive = !props.disabled && !props.readOnly;
  const anchor = React.useRef<HTMLDivElement>(null);
  return (
    <PopoverPrimitive.Root
      open={combo.open && interactive}
      onOpenChange={(open) => {
        if (!open) {
          combo.setOpen(false);
          setSearch(null);
        }
      }}
    >
      <PopoverPrimitive.Anchor asChild>
        <div ref={anchor} className="relative flex w-full items-center">
          {leftSection && (
            <span className="pointer-events-none absolute left-2.5 flex text-muted-foreground [&_svg]:size-4">
              {leftSection}
            </span>
          )}
          <input
            {...props}
            type="text"
            autoComplete="off"
            aria-haspopup="listbox"
            aria-controls={combo.open ? listId : undefined}
            aria-activedescendant={
              combo.open && combo.active ? `${listId}-${combo.active}` : undefined
            }
            readOnly={!searchable || props.readOnly}
            placeholder={placeholder}
            value={search ?? selected?.label ?? ""}
            data-slot="select"
            className={cn(
              controlClass,
              "pl-2.5 pr-8 text-left",
              !searchable && "cursor-pointer caret-transparent",
              size === "sm" && "h-7 text-label",
              size === "default" && "h-8",
              size === "lg" && "h-10",
              leftSection && "pl-8",
              inputClassName,
            )}
            onClick={() => {
              if (!interactive) return;
              if (combo.open && !searchable) combo.setOpen(false);
              else combo.show();
            }}
            onChange={(event) => {
              if (!searchable) return;
              setSearch(event.currentTarget.value);
              if (!combo.open) combo.setOpen(true);
              combo.setActive(null);
            }}
            onKeyDown={(event) => {
              if (!interactive) return;
              combo.onKeyDown(event);
            }}
            onBlur={() => {
              setSearch(null);
            }}
          />
          {clearable && value && interactive ? (
            <button
              type="button"
              aria-label="Clear selection"
              className="absolute right-1.5 grid size-6 place-items-center rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground"
              onClick={() => onChange(null)}
            >
              <X className="size-3.5" aria-hidden="true" />
            </button>
          ) : (
            <ChevronsUpDown
              className="pointer-events-none absolute right-2.5 size-3.5 text-muted-foreground"
              aria-hidden="true"
            />
          )}
        </div>
      </PopoverPrimitive.Anchor>
      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Content
          align="start"
          sideOffset={4}
          className={contentClass}
          onOpenAutoFocus={(event) => event.preventDefault()}
          onCloseAutoFocus={(event) => event.preventDefault()}
          onInteractOutside={(event) => {
            if (anchor.current?.contains(event.target as Node))
              event.preventDefault();
          }}
        >
          <Listbox
            id={listId}
            groups={visible}
            active={combo.active}
            isSelected={(item) => item === value}
            onPick={pick}
            onHover={combo.setActive}
            nothingFound={nothingFoundMessage}
            maxHeight={maxDropdownHeight}
          />
        </PopoverPrimitive.Content>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  );
}

export function MultiSelect({
  data,
  value,
  onChange,
  label,
  description,
  error,
  required,
  className,
  inputClassName,
  placeholder,
  searchable = true,
  clearable = false,
  nothingFoundMessage = "No matches",
  maxValues,
  disabled,
  id,
  maxDropdownHeight,
  hidePickedOptions = false,
  ...aria
}: FieldProps & {
  data: SelectData;
  value: string[];
  onChange: (value: string[]) => void;
  placeholder?: string;
  searchable?: boolean;
  clearable?: boolean;
  nothingFoundMessage?: React.ReactNode;
  maxValues?: number;
  disabled?: boolean;
  id?: string;
  maxDropdownHeight?: number;
  hidePickedOptions?: boolean;
  "aria-label"?: string;
}) {
  const control = (
    <MultiSelectControl
      {...{
        data,
        value,
        onChange,
        placeholder,
        searchable,
        clearable,
        nothingFoundMessage,
        maxValues,
        disabled,
        id,
        maxDropdownHeight,
        hidePickedOptions,
        inputClassName,
        required,
        ...aria,
      }}
    />
  );
  if (!label && !description && !error)
    return className ? <div className={className}>{control}</div> : control;
  return (
    <Field
      label={label}
      description={description}
      error={error}
      required={required}
      id={id}
      className={className}
    >
      {control}
    </Field>
  );
}

function MultiSelectControl({
  data,
  value,
  onChange,
  placeholder,
  searchable,
  clearable,
  nothingFoundMessage,
  maxValues,
  maxDropdownHeight,
  hidePickedOptions,
  inputClassName,
  ...rest
}: {
  data: SelectData;
  value: string[];
  onChange: (value: string[]) => void;
  placeholder?: string;
  searchable: boolean;
  clearable: boolean;
  nothingFoundMessage: React.ReactNode;
  maxValues?: number;
  maxDropdownHeight?: number;
  hidePickedOptions: boolean;
  inputClassName?: string;
  disabled?: boolean;
  required?: boolean;
  id?: string;
  "aria-label"?: string;
}) {
  const props = useFieldControl(rest);
  const generated = React.useId();
  const listId = `${props.id ?? generated}-listbox`;
  const groups = React.useMemo(() => normalize(data), [data]);
  const flat = React.useMemo(() => groups.flatMap((g) => g.items), [groups]);
  const [search, setSearch] = React.useState("");
  const toggle = (item: SelectItem) => {
    if (value.includes(item.value))
      onChange(value.filter((entry) => entry !== item.value));
    else if (maxValues === undefined || value.length < maxValues)
      onChange([...value, item.value]);
    setSearch("");
  };
  const query = search.trim().toLowerCase();
  const visible = groups.map((group) => ({
    ...group,
    items: group.items.filter(
      (item) =>
        (!hidePickedOptions || !value.includes(item.value)) &&
        (!query || item.label.toLowerCase().includes(query)),
    ),
  }));
  const combo = useCombobox(
    visible.flatMap((group) => group.items),
    toggle,
    null,
  );
  const labelOf = (entry: string) =>
    flat.find((item) => item.value === entry)?.label ?? entry;
  const wrapper = React.useRef<HTMLDivElement>(null);
  return (
    <PopoverPrimitive.Root
      open={combo.open && !props.disabled}
      onOpenChange={(open) => {
        if (!open) combo.setOpen(false);
      }}
    >
      <PopoverPrimitive.Anchor asChild>
        <div
          ref={wrapper}
          data-slot="multi-select"
          aria-disabled={props.disabled || undefined}
          className={cn(
            controlClass,
            "relative flex min-h-8 cursor-text flex-wrap items-center gap-1 px-1.5 py-1 pr-8 focus-within:border-ring focus-within:shadow-focus",
            (props as { "aria-invalid"?: unknown })["aria-invalid"] ? "border-destructive" : undefined,
            inputClassName,
          )}
          onClick={() => {
            if (props.disabled) return;
            wrapper.current?.querySelector("input")?.focus();
            combo.show();
          }}
        >
          {value.map((entry) => (
            <span
              key={entry}
              className="inline-flex h-[22px] max-w-full items-center gap-1 rounded-sm bg-secondary pl-2 pr-0.5 text-caption font-medium text-foreground-secondary"
            >
              <span className="truncate">{labelOf(entry)}</span>
              <button
                type="button"
                aria-label={`Remove ${labelOf(entry)}`}
                disabled={props.disabled}
                className="grid size-4 place-items-center rounded-xs text-muted-foreground hover:bg-accent hover:text-foreground"
                onClick={(event) => {
                  event.stopPropagation();
                  onChange(value.filter((item) => item !== entry));
                }}
              >
                <X className="size-3" aria-hidden="true" />
              </button>
            </span>
          ))}
          <input
            {...props}
            type="text"
            autoComplete="off"
            aria-haspopup="listbox"
            aria-controls={combo.open ? listId : undefined}
            aria-activedescendant={
              combo.open && combo.active ? `${listId}-${combo.active}` : undefined
            }
            readOnly={!searchable}
            placeholder={value.length ? undefined : placeholder}
            value={search}
            className="h-6 min-w-16 flex-1 border-0 bg-transparent px-1 text-body text-foreground outline-none placeholder:text-muted-foreground"
            onChange={(event) => {
              setSearch(event.currentTarget.value);
              if (!combo.open) combo.setOpen(true);
              combo.setActive(null);
            }}
            onKeyDown={(event) => {
              if (event.key === "Backspace" && !search && value.length) {
                onChange(value.slice(0, -1));
                return;
              }
              combo.onKeyDown(event);
            }}
          />
          {clearable && value.length > 0 && !props.disabled ? (
            <button
              type="button"
              aria-label="Clear selection"
              className="absolute right-1.5 grid size-6 place-items-center rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground"
              onClick={(event) => {
                event.stopPropagation();
                onChange([]);
              }}
            >
              <X className="size-3.5" aria-hidden="true" />
            </button>
          ) : (
            <ChevronsUpDown
              className="pointer-events-none absolute right-2.5 size-3.5 text-muted-foreground"
              aria-hidden="true"
            />
          )}
        </div>
      </PopoverPrimitive.Anchor>
      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Content
          align="start"
          sideOffset={4}
          className={contentClass}
          onOpenAutoFocus={(event) => event.preventDefault()}
          onCloseAutoFocus={(event) => event.preventDefault()}
          onInteractOutside={(event) => {
            if (wrapper.current?.contains(event.target as Node))
              event.preventDefault();
          }}
        >
          <Listbox
            id={listId}
            groups={visible}
            active={combo.active}
            isSelected={(item) => value.includes(item)}
            onPick={toggle}
            onHover={combo.setActive}
            nothingFound={nothingFoundMessage}
            maxHeight={maxDropdownHeight}
          />
        </PopoverPrimitive.Content>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  );
}

/** Styled native select for compact toolbars where a native menu is fine. */
export function NativeSelect({
  className,
  children,
  size = "default",
  ...rest
}: Omit<React.ComponentProps<"select">, "size"> & {
  size?: "sm" | "default";
}) {
  const props = useFieldControl(rest);
  return (
    <div className="relative flex w-full items-center">
      <select
        data-slot="native-select"
        className={cn(
          controlClass,
          "cursor-pointer appearance-none pl-2.5 pr-8",
          size === "sm" ? "h-7 text-label" : "h-8",
          className,
        )}
        {...props}
      >
        {children}
      </select>
      <ChevronsUpDown
        className="pointer-events-none absolute right-2.5 size-3.5 text-muted-foreground"
        aria-hidden="true"
      />
    </div>
  );
}
