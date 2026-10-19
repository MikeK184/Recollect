import type { Dispatch, ReactNode, SetStateAction } from "react";
import {
  Button,
  Drawer,
  Group,
  Loader,
  SimpleGrid,
  Stack,
  Text,
} from "@mantine/core";
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
      className="feature-drawer"
      opened={opened}
      onClose={onClose}
      position="right"
      title="Entities"
      size="lg"
    >
      {isError && <Failure error={error} />}
      {readable && isPending && <Loader size="sm" />}
      {(!readable || expired) && (
        <Text size="sm" c="dimmed">
          Reload the graph to inspect current eligible entities.
        </Text>
      )}
      {readable &&
        !expired &&
        !isError &&
        data &&
        data.memory_epoch === memoryEpoch && (
          <Stack>
            <Text size="xs" c="dimmed">
              All eligible entities in the selected scope. Paging does not
              replace the displayed canvas.
            </Text>
            <SimpleGrid cols={{ base: 1, md: 2 }}>
              {data.nodes.map((n) => renderNode(n, true))}
            </SimpleGrid>
            {(data.next_offset != null || data.offset > 0) && (
              <Group>
                <Button
                  disabled={!data.offset}
                  onClick={() => setEntityOffset(data.offset - 100)}
                >
                  Previous entities
                </Button>
                <Text size="sm">
                  {data.nodes.length} eligible entities in this page
                </Text>
                <Button
                  disabled={data.next_offset == null}
                  onClick={() => setEntityOffset(data.next_offset!)}
                >
                  More entities
                </Button>
              </Group>
            )}
          </Stack>
        )}
    </Drawer>
  );
}
