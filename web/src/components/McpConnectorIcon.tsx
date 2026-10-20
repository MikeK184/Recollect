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
      className={`grid shrink-0 place-items-center overflow-hidden rounded-md border border-border bg-muted text-muted-foreground ${size ? "size-7 [&_svg]:size-3.5" : "size-9 [&_svg]:size-4"}`}
    >
      {icon?.startsWith("data:image/png;base64,") && failed !== icon ? (
        <img
          src={icon}
          alt=""
          className="size-full object-contain p-1"
          onError={() => setFailed(icon)}
        />
      ) : (
        <Box aria-hidden="true" />
      )}
    </div>
  );
}
