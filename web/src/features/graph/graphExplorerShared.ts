import type { components } from "../../api-schema";

export type ExplorerNode = components["schemas"]["GraphNode"];
export type ExplorerView = components["schemas"]["GraphView"];
export type ExplorerPath = components["schemas"]["GraphPath"];
export const label = (s: string) => s.replaceAll("_", " ");
