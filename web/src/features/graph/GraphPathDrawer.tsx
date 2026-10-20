import type { ReactNode } from "react";
import {
  Button,
  Drawer,
  NumberField,
  Select,
  TextField,
} from "../../components/ui";
import type { components } from "../../api-schema";
import { label, meanings, type GraphNodeView } from "./graphScope";
import { Failure } from "./GraphPanelParts";

export function GraphPathDrawer({
  opened,
  onClose,
  readable,
  start,
  end,
  direction,
  hops,
  onStartChange,
  onEndChange,
  onDirectionChange,
  onHopsChange,
  onFind,
  isFetching,
  error,
  data,
  submitted,
  renderNode,
}: {
  opened: boolean;
  onClose: () => void;
  readable: boolean;
  start: string;
  end: string;
  direction: string;
  hops: number;
  onStartChange: (value: string) => void;
  onEndChange: (value: string) => void;
  onDirectionChange: (value: string) => void;
  onHopsChange: (value: number) => void;
  onFind: () => void;
  isFetching: boolean;
  error: Error | null;
  data: components["schemas"]["GraphPath"] | undefined;
  submitted: boolean;
  renderNode: (n: GraphNodeView, choose: boolean) => ReactNode;
}) {
  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title="Find an evidence path"
      size={560}
      bodyClassName="grid content-start gap-4"
    >
      {readable ? (
        <>
          <div className="grid gap-4 sm:grid-cols-2">
            <TextField
              label="Path start"
              value={start}
              onChange={(e) => onStartChange(e.currentTarget.value)}
            />
            <TextField
              label="Path end"
              value={end}
              onChange={(e) => onEndChange(e.currentTarget.value)}
            />
            <Select
              label="Path direction"
              value={direction}
              data={["outgoing", "incoming", "both"]}
              onChange={(value) => onDirectionChange(value ?? "outgoing")}
            />
            <NumberField
              label="Maximum path hops"
              min={1}
              max={8}
              value={hops}
              onChange={(n) => onHopsChange(Number(n) || 1)}
            />
          </div>
          <Button
            className="justify-self-start"
            onClick={onFind}
            loading={isFetching}
            disabled={!start || !end}
          >
            Find shortest eligible path
          </Button>
          <Failure error={error} />
          {submitted && data && !error && (
            <section className="grid gap-3 border-t border-border pt-4">
              <h3 className="m-0 text-title font-semibold">
                {data.status === "path"
                  ? `${data.edges.length} hops in the eligible graph`
                  : "No path within the selected hop bound"}
              </h3>
              <ol className="m-0 grid list-none gap-2 p-0">
                {data.nodes.map((n, i) => (
                  <li key={`${n.key}-${i}`} className="grid gap-2">
                    {renderNode(n, false)}
                    {data.edges[i] && (
                      <div className="ml-4 grid gap-1 border-l-2 border-brand/50 py-1 pl-3 [overflow-wrap:anywhere]">
                        <p className="m-0 text-label font-semibold text-foreground">
                          {label(data.edges[i].relation)} ·{" "}
                          {label(data.edges[i].family)}
                        </p>
                        <p className="m-0 text-label text-foreground-secondary">
                          Recorded direction:{" "}
                          {
                            data.nodes.find(
                              (node) => node.key === data.edges[i].from,
                            )?.evidence.label
                          }
                          {" → "}
                          {
                            data.nodes.find(
                              (node) => node.key === data.edges[i].to,
                            )?.evidence.label
                          }
                        </p>
                        {data.edges[i].from !== n.key && (
                          <p className="m-0 text-label text-warning-text">
                            Followed in reverse for this path.
                          </p>
                        )}
                        <p className="m-0 text-label text-muted-foreground">
                          {meanings[data.edges[i].relation] ??
                            `A static ${label(data.edges[i].relation)} relationship in the selected direction.`}
                        </p>
                      </div>
                    )}
                  </li>
                ))}
              </ol>
            </section>
          )}
        </>
      ) : (
        <p className="m-0 text-body text-muted-foreground">
          Load a graph view first, then choose its path endpoints.
        </p>
      )}
    </Drawer>
  );
}
