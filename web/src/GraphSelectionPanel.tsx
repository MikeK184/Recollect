import { useEffect, useState, type ReactNode } from "react";
import { ChevronRight, Focus } from "lucide-react";
import { Button } from "./components/ui";
import { InspectorSection } from "./components/Inspector";
import { GraphSwatch, type GraphCanvasProps, type GraphChoice } from "./GraphCanvas";
import { GraphInspector } from "./features/graph/GraphStage";
import { kindGroup } from "./features/graph/graphColors";

const readable = (value: string) => value.replaceAll("_", " ");
const kindNames: Record<string, string> = {
  claim: "Memory",
  source_version: "Source",
  device: "Agent credential",
  contributor: "Person",
  capture: "Input",
};
const kindLabel = (kind: string) => {
  const name = kindNames[kind] ?? readable(kind);
  return name.charAt(0).toUpperCase() + name.slice(1);
};

/**
 * The selected node or relationship: what it records, its direct loaded
 * connections (keyboard navigation through the graph) and the hand-off to its
 * evidence. Docks in the shared inspector on the Graph page.
 */
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
  const opened = !!(node || edge);
  const nodeMap = new Map(nodes.map((item) => [item.key, item]));
  const title = node?.evidence.label ?? (edge ? readable(edge.relation) : "");
  const connections: {
    id: string;
    target: GraphChoice;
    label: string;
    kind?: string;
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
            kind: nodeMap.get(target)?.evidence.kind,
            relation: `${item.from === item.to ? "Loop" : item.from === node.key ? "Outgoing" : "Incoming"} · ${readable(item.relation)}`,
          };
        })
    : edge
      ? [...new Set([edge.from, edge.to])].map((id) => ({
          id,
          target: { kind: "node", id },
          label: nodeMap.get(id)?.evidence.label ?? id,
          kind: nodeMap.get(id)?.evidence.kind,
          relation:
            edge.from === edge.to
              ? "Recorded loop"
              : id === edge.from
                ? "From"
                : "To",
        }))
      : [];
  return (
    <GraphInspector
      opened={opened}
      onClose={() => onChoose(null)}
      label="Selected graph evidence"
      title={
        <span className="flex min-w-0 items-center gap-2">
          {node && <GraphSwatch group={kindGroup(node.evidence.kind)} />}
          {node ? kindLabel(node.evidence.kind) : "Relationship"}
        </span>
      }
    >
      <div className="grid gap-2">
        <h3 className="m-0 text-section font-semibold text-foreground">
          {title}
        </h3>
        {edge && (
          <p className="m-0 text-body text-foreground-secondary">
            {nodeMap.get(edge.from)?.evidence.label ?? edge.from} →{" "}
            {nodeMap.get(edge.to)?.evidence.label ?? edge.to}
          </p>
        )}
        {description}
      </div>
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          variant="secondary"
          onClick={onFocus}
          disabled={!focusEnabled}
          title={
            !focusEnabled
              ? "Keep the qualified shortest path complete"
              : undefined
          }
        >
          <Focus aria-hidden="true" />
          Focus connections
        </Button>
        {actions}
      </div>
      <InspectorSection
        className="border-t pt-4 first:pt-4"
        label={`${connections.length} ${node ? "direct connections" : "endpoints"} in this read`}
      >
        {connections.length ? (
          <ul className="-mx-2 m-0 grid list-none gap-0.5 p-0">
            {connections.slice(0, limit).map((connection) => (
              <li key={connection.id}>
                <button
                  type="button"
                  className="flex w-full min-w-0 items-center gap-2.5 rounded-md px-2 py-1.5 text-left transition-colors duration-fast hover:bg-accent focus-visible:outline-2 focus-visible:outline-ring"
                  onClick={() => onChoose(connection.target)}
                  title={connection.label}
                >
                  {connection.kind && (
                    <GraphSwatch group={kindGroup(connection.kind)} />
                  )}
                  <span className="grid min-w-0 flex-1">
                    <span className="truncate text-body font-medium text-foreground">
                      {connection.label}
                    </span>
                    <small className="text-caption text-muted-foreground">
                      {connection.relation}
                    </small>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="m-0 text-body text-muted-foreground">
            No direct connections in this read.
          </p>
        )}
        {connections.length > limit && (
          <Button
            size="sm"
            variant="ghost"
            className="justify-self-start"
            onClick={() => setLimit((value) => value + 20)}
          >
            Show more ({connections.length - limit})
          </Button>
        )}
      </InspectorSection>
      {details && (
        <details className="group -mx-4 border-t border-border px-4 pt-3">
          <summary className="flex cursor-pointer list-none items-center gap-1.5 text-label font-medium text-foreground-secondary [&::-webkit-details-marker]:hidden">
            <ChevronRight
              aria-hidden="true"
              className="size-3.5 transition-transform duration-fast group-open:rotate-90"
            />
            Details & actions
          </summary>
          <div className="grid gap-3 pt-3">{details}</div>
        </details>
      )}
    </GraphInspector>
  );
}
