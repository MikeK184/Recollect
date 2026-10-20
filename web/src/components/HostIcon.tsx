import { Sparkle, Terminal, Bot } from "lucide-react";
/** Compact host glyphs use the shared line-icon system without inventing logos. */
export function HostIcon({
  host,
  size = 30,
}: {
  host?: string | null;
  size?: number;
}) {
  if (host === "codex" || host === "openai")
    return (
      <span
        className="grid shrink-0 place-items-center overflow-hidden dark:invert"
        style={{ width: size, height: size }}
        aria-hidden="true"
      >
        <img
          src="/assets/providers/openai-blossom.svg"
          width={size * 1.5}
          height={size * 1.5}
          alt=""
        />
      </span>
    );
  const Icon =
    host === "claude" || host === "claude_code"
      ? Sparkle
      : host === "opencode"
        ? Terminal
        : Bot;
  return <Icon size={size} strokeWidth={1.75} aria-hidden="true" />;
}
