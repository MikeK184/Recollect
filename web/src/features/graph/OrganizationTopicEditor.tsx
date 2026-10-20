import { useState } from "react";
import { Button, Checkbox, Select, TextField } from "../../components/ui";
import type { SemanticCard, TopicChange, View } from "./organizationModel";

export function TopicEditor({
  topic,
  view,
  busy,
  onChange,
}: {
  topic: SemanticCard;
  view: View;
  busy: boolean;
  onChange: (change: TopicChange) => void;
}) {
  const [name, setName] = useState(topic.label),
    [selected, setSelected] = useState<string[]>([]);
  const sources = new Map(view.evidence.map((node) => [node.key, node]));
  const native = [
    ...(view.source_memberships ?? []),
    ...(view.representative_sources ?? []),
  ].filter((member) => member.topic_id === topic.id);
  const sourceTopic = native.length > 0;
  const inputs = [
    ...new Map(
      (sourceTopic ? native : view.memberships)
        .filter((member) => member.topic_id === topic.id)
        .map((member) => [
          member.input_id,
          { id: member.input_id, node: sources.get(member.source_key) },
        ]),
    ).values(),
  ].filter((item) => item.node);
  const titleCounts = new Map<string, number>();
  for (const item of inputs) {
    const title = item.node!.evidence.label;
    titleCounts.set(title, (titleCounts.get(title) ?? 0) + 1);
  }
  const inputLabel = (item: (typeof inputs)[number]) => {
    const node = item.node!,
      title = node.evidence.label;
    return titleCounts.get(title)! > 1
      ? `${title} · version ${node.key.replace(/^source_version:/, "").slice(0, 8)}`
      : title;
  };
  const [basis, setBasis] = useState<string | null>(inputs[0]?.id ?? null);
  const maximumNameBytes = sourceTopic ? 240 : 256;
  const validName =
    name.trim().length > 0 &&
    new TextEncoder().encode(name.trim()).length <= maximumNameBytes &&
    !/[\u0000-\u001f\u007f]/.test(name);
  return (
    <div className="grid gap-4">
      <p className="m-0 text-body text-foreground-secondary">
        Changes apply to this{" "}
        {topic.topic_kind === "manual_overlay"
          ? "manual group"
          : "automatic topic"}
        . Source evidence and manual collections keep their own identities.
      </p>
      <TextField
        label="Topic name"
        value={name}
        onChange={(event) => setName(event.currentTarget.value)}
        maxLength={maximumNameBytes}
        disabled={busy}
      />
      <Select
        label="Name supported by"
        value={basis}
        onChange={setBasis}
        allowDeselect={false}
        data={inputs.map((item) => ({
          value: item.id,
          label: inputLabel(item),
        }))}
        disabled={busy}
      />
      <div className="flex flex-wrap gap-2">
        <Button
          disabled={busy || !basis || !validName}
          onClick={() =>
            basis &&
            onChange({
              action: sourceTopic ? "rename_source" : "rename",
              label: name.trim(),
              basis_input: basis,
            })
          }
        >
          Save topic name
        </Button>
        <Button
          variant="secondary"
          disabled={busy}
          onClick={() => onChange({ action: "pin", pinned: !topic.pinned })}
        >
          {topic.pinned ? "Unpin topic" : "Pin topic"}
        </Button>
      </div>
      <fieldset className="m-0 grid gap-2 border-0 border-t border-border p-0 pt-4">
        <legend className="mb-2 p-0 text-label font-semibold">
          Source memberships in this read
        </legend>
        {inputs.length === 0 && (
          <p className="m-0 text-body text-muted-foreground">
            This page has no editable membership rows. Explore the topic and
            read the next organization page to load its eligible source
            memberships.
          </p>
        )}
        {inputs.map((item) => (
          <Checkbox
            key={item.id}
            value={item.id}
            label={inputLabel(item)}
            disabled={busy}
            checked={selected.includes(item.id)}
            onChange={(event) => {
              const checked = event.currentTarget.checked;
              setSelected((old) =>
                checked
                  ? [...old.filter((id) => id !== item.id), item.id]
                  : old.filter((id) => id !== item.id),
              );
            }}
          />
        ))}
      </fieldset>
      {view.coverage.counts_are_lower_bounds && (
        <p className="m-0 text-caption text-muted-foreground">
          This read is partial. Only the selected, currently eligible source
          memberships change.
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        <Button
          variant="secondary"
          disabled={busy || !selected.length}
          onClick={() =>
            onChange({
              action: sourceTopic ? "exclude_sources" : "exclude",
              input_ids: selected,
              excluded: true,
            })
          }
        >
          Exclude selected sources
        </Button>
        <Button
          variant="secondary"
          disabled={busy || !selected.length}
          onClick={() =>
            onChange({
              action: sourceTopic ? "split_sources" : "split",
              input_ids: selected,
            })
          }
        >
          Split selected into topic
        </Button>
      </div>
      <p className="m-0 text-caption text-muted-foreground">
        Exclusion removes membership from this group. Splitting moves the
        selected memberships into a separate topic; neither action erases source
        evidence.
      </p>
    </div>
  );
}
