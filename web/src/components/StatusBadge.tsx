import { Badge } from "@mantine/core";
import { Check, Clock3, CircleAlert, CircleDashed } from "lucide-react";

export function StatusBadge({
  children,
  state = "neutral",
}: {
  children: string;
  state?: "positive" | "attention" | "negative" | "neutral";
}) {
  const Icon =
    state === "positive"
      ? Check
      : state === "attention"
        ? Clock3
        : state === "negative"
          ? CircleAlert
          : CircleDashed;
  return (
    <Badge
      color={
        state === "positive"
          ? "brand"
          : state === "attention"
            ? "yellow"
            : state === "negative"
              ? "red"
              : "gray"
      }
      leftSection={<Icon size={12} />}
    >
      {children}
    </Badge>
  );
}
