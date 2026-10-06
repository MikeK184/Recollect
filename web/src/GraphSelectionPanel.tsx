import { useEffect, useState, type ReactNode } from "react";
import {
  ActionIcon,
  Badge,
  Button,
  Group,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { Focus, X } from "lucide-react";
import type { GraphCanvasProps, GraphChoice } from "./GraphCanvas";

const readable = (value: string) => value.replaceAll("_", " ");
const kindNames: Record<string, string> = {
  claim: "Memory",
  source_version: "Source",
  device: "Agent credential",
  contributor: "Person",
  capture: "Input",
};
const kindLabel = (kind: string) => kindNames[kind] ?? readable(kind);

export function GraphSelectionPanel({
  nodes,
  edges,
  choice,
  onChoose,
  onFocus,
  description,
  actions,
  details,
  focusEnabled = true,
}: Pick<GraphCanvasProps, "nodes" | "edges" | "choice" | "onChoose"> & {
  onFocus: () => void;
  description?: ReactNode;
  actions?: ReactNode;
  details?: ReactNode;
  focusEnabled?: boolean;
}) {
  const [limit, setLimit] = useState(10);
  useEffect(() => setLimit(10), [choice?.kind, choice?.id]);
  const node =
    choice?.kind === "node"
      ? nodes.find((item) => item.key === choice.id)
      : undefined;
  const edge =
    choice?.kind === "edge"
      ? edges.find((item) => item.id === choice.id)
      : undefined;
  if (!node && !edge) return null;
  const title = node?.evidence.label ?? readable(edge!.relation);
  const nodeMap = new Map(nodes.map((item) => [item.key, item]));
  const connections: {
    id: string;
    target: GraphChoice;
    label: string;
    relation: string;
  }[] = node
    ? edges
        .filter((item) => item.from === node.key || item.to === node.key)
        .map((item) => {
          const target = item.from === node.key ? item.to : item.from;
          return {
            id: item.id,
            target: { kind: "node", id: target },
            label: nodeMap.get(target)?.evidence.label ?? target,
            relation: `${item.from === item.to ? "Loop" : item.from === node.key ? "Outgoing" : "Incoming"} · ${readable(item.relation)}`,
          };
        })
    : [...new Set([edge!.from, edge!.to])].map((id) => ({
        id,
        target: { kind: "node", id },
        label: nodeMap.get(id)?.evidence.label ?? id,
        relation:
          edge!.from === edge!.to
            ? "Recorded loop"
            : id === edge!.from
              ? "From"
              : "To",
      }));
  return (
    <aside
      className="graph-selection-panel"
      aria-label="Selected graph evidence"
    >
      <Group justify="space-between" wrap="nowrap">
        <Badge variant="light">
          {node ? kindLabel(node.evidence.kind) : "Relationship"}
        </Badge>
        <ActionIcon
          aria-label="Close graph summary"
          variant="subtle"
          onClick={() => onChoose(null)}
        >
          <X size={16} />
        </ActionIcon>
      </Group>
      <Title order={3}>{title}</Title>
      {edge && (
        <Text size="sm">
          {nodeMap.get(edge.from)?.evidence.label ?? edge.from} →{" "}
          {nodeMap.get(edge.to)?.evidence.label ?? edge.to}
        </Text>
      )}
      {description}
      <Group gap="xs">
        <Button
          size="xs"
          variant="default"
          leftSection={<Focus size={14} />}
          onClick={onFocus}
          disabled={!focusEnabled}
          title={
            !focusEnabled
              ? "Keep the qualified shortest path complete"
              : undefined
          }
        >
          Focus connections
        </Button>
        {actions}
      </Group>
      <div className="graph-selection-connections">
        <Text size="xs" c="dimmed">
          {connections.length} {node ? "direct connections" : "endpoints"} in
          this read
        </Text>
        <Stack gap={2} mt="xs">
          {connections.slice(0, limit).map((connection) => (
            <button
              key={connection.id}
              type="button"
              className="graph-connection-link"
              onClick={() => onChoose(connection.target)}
              title={connection.label}
            >
              <span>{connection.label}</span>
              <small>{connection.relation}</small>
            </button>
          ))}
        </Stack>
        {connections.length > limit && (
          <Button
            size="xs"
            variant="subtle"
            onClick={() => setLimit((value) => value + 20)}
          >
            Show more ({connections.length - limit})
          </Button>
        )}
        {!connections.length && (
          <Text size="sm" c="dimmed">
            No direct connections in this read.
          </Text>
        )}
      </div>
      {details && (
        <details className="graph-selection-details">
          <summary>Details & actions</summary>
          {details}
        </details>
      )}
    </aside>
  );
}
