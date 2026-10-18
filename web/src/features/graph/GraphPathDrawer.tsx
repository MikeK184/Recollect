import type { ReactNode } from "react";
import {
  Alert,
  Button,
  Card,
  Drawer,
  NumberInput,
  Select,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
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
      className="feature-drawer"
      opened={opened}
      onClose={onClose}
      position="right"
      title="Find an evidence path"
      size="lg"
    >
      {readable ? (
        <Stack>
          <SimpleGrid cols={{ base: 1, sm: 2 }}>
            <TextInput
              label="Path start"
              value={start}
              onChange={(e) => onStartChange(e.currentTarget.value)}
            />
            <TextInput
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
            <NumberInput
              label="Maximum path hops"
              min={1}
              max={8}
              value={hops}
              onChange={(n) => onHopsChange(Number(n) || 1)}
            />
          </SimpleGrid>
          <Button
            onClick={onFind}
            loading={isFetching}
            disabled={!start || !end}
          >
            Find shortest eligible path
          </Button>
          <Failure error={error} />
          {submitted && data && !error && (
            <Card withBorder p="md">
              <Stack>
                <Title order={4}>
                  {data.status === "path"
                    ? `${data.edges.length} hops in the eligible graph`
                    : "No path within the selected hop bound"}
                </Title>
                {data.nodes.map((n, i) => (
                  <Stack key={`${n.key}-${i}`} gap="xs">
                    {renderNode(n, false)}
                    {data.edges[i] && (
                      <Alert color="blue">
                        <Text fw={600}>
                          {label(data.edges[i].relation)} ·{" "}
                          {label(data.edges[i].family)}
                        </Text>
                        <Text size="sm" style={{ overflowWrap: "anywhere" }}>
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
                        </Text>
                        {data.edges[i].from !== n.key && (
                          <Text size="sm">
                            Followed in reverse for this path.
                          </Text>
                        )}
                        <Text size="sm">
                          {meanings[data.edges[i].relation] ??
                            `A static ${label(data.edges[i].relation)} relationship in the selected direction.`}
                        </Text>
                      </Alert>
                    )}
                  </Stack>
                ))}
              </Stack>
            </Card>
          )}
        </Stack>
      ) : (
        <Text>Load a graph view first, then choose its path endpoints.</Text>
      )}
    </Drawer>
  );
}
