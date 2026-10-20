import type { Dispatch, ReactNode, SetStateAction } from "react";
import { Button, Drawer, Spinner } from "../../components/ui";
import type { components } from "../../api-schema";
import type { GraphNodeView } from "./graphScope";
import { Failure } from "./GraphPanelParts";

export function GraphEntitiesDrawer({
  opened,
  onClose,
  readable,
  expired,
  isError,
  isPending,
  error,
  data,
  memoryEpoch,
  setEntityOffset,
  renderNode,
}: {
  opened: boolean;
  onClose: () => void;
  readable: boolean;
  expired: boolean;
  isError: boolean;
  isPending: boolean;
  error: Error | null;
  data: components["schemas"]["GraphView"] | undefined;
  memoryEpoch: number | undefined;
  setEntityOffset: Dispatch<SetStateAction<number>>;
  renderNode: (n: GraphNodeView, choose: boolean) => ReactNode;
}) {
  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title="Entities"
      size={640}
      bodyClassName="grid content-start gap-4"
    >
      {isError && <Failure error={error} />}
      {readable && isPending && <Spinner label="Loading entities" />}
      {(!readable || expired) && (
        <p className="m-0 text-body text-muted-foreground">
          Reload the graph to inspect current eligible entities.
        </p>
      )}
      {readable &&
        !expired &&
        !isError &&
        data &&
        data.memory_epoch === memoryEpoch && (
          <div className="grid gap-3">
            <p className="m-0 text-caption text-muted-foreground">
              All eligible entities in the selected scope. Paging does not
              replace the displayed canvas.
            </p>
            <div className="grid gap-2 md:grid-cols-2">
              {data.nodes.map((n) => renderNode(n, true))}
            </div>
            {(data.next_offset != null || data.offset > 0) && (
              <div className="flex flex-wrap items-center justify-between gap-2">
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={!data.offset}
                  onClick={() => setEntityOffset(data.offset - 100)}
                >
                  Previous entities
                </Button>
                <p className="mono m-0 text-caption text-muted-foreground">
                  {data.nodes.length} eligible entities in this page
                </p>
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={data.next_offset == null}
                  onClick={() => setEntityOffset(data.next_offset!)}
                >
                  More entities
                </Button>
              </div>
            )}
          </div>
        )}
    </Drawer>
  );
}
