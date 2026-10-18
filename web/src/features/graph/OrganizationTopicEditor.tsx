import { useState } from "react";
import {
  Button,
  Checkbox,
  Group,
  Select,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
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
    <Stack gap="md">
      <Text size="sm">
        Changes apply to this{" "}
        {topic.topic_kind === "manual_overlay"
          ? "manual group"
          : "automatic topic"}
        . Source evidence and manual collections keep their own identities.
      </Text>
      <TextInput
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
      <Group>
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
          variant="default"
          disabled={busy}
          onClick={() => onChange({ action: "pin", pinned: !topic.pinned })}
        >
          {topic.pinned ? "Unpin topic" : "Pin topic"}
        </Button>
      </Group>
      <Text fw={600} size="sm">
        Source memberships in this read
      </Text>
      {inputs.length === 0 && (
        <Text size="sm" c="dimmed">
          This page has no editable membership rows. Explore the topic and read
          the next organization page to load its eligible source memberships.
        </Text>
      )}
      <Checkbox.Group value={selected} onChange={setSelected}>
        <Stack gap="xs">
          {inputs.map((item) => (
            <Checkbox
              key={item.id}
              value={item.id}
              label={inputLabel(item)}
              disabled={busy}
            />
          ))}
        </Stack>
      </Checkbox.Group>
      {view.coverage.counts_are_lower_bounds && (
        <Text size="xs" c="dimmed">
          This read is partial. Only the selected, currently eligible source
          memberships change.
        </Text>
      )}
      <Group>
        <Button
          variant="default"
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
          variant="light"
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
      </Group>
      <Text size="xs" c="dimmed">
        Exclusion removes membership from this group. Splitting moves the
        selected memberships into a separate topic; neither action erases source
        evidence.
      </Text>
    </Stack>
  );
}
