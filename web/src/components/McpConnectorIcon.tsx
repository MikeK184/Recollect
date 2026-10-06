import { useState } from "react";
import { Box } from "lucide-react";
import type { components } from "../api-schema";

export function McpConnectorIcon({
  definition,
  size,
}: {
  definition?: components["schemas"]["McpDefinitionSummary"];
  size?: "small";
}) {
  const [failed, setFailed] = useState<string | null>(null);
  const icon = (
    definition as
      | (components["schemas"]["McpDefinitionSummary"] & {
          icon_png?: string | null;
        })
      | undefined
  )?.icon_png;
  return (
    <div
      className={`management-icon connection-icon${size ? " mcp-icon-small" : ""}`}
    >
      {icon?.startsWith("data:image/png;base64,") && failed !== icon ? (
        <img src={icon} alt="" onError={() => setFailed(icon)} />
      ) : (
        <Box aria-hidden="true" />
      )}
    </div>
  );
}
