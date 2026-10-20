import { LockKeyhole, Play, Settings2, Share2 } from "lucide-react";
import type { components } from "../api-schema";
import { cn } from "../lib/utils";

type Rights = components["schemas"]["McpRights"];
const powers = [
  { key: "use_profile", label: "Use", Icon: Play },
  { key: "manage", label: "Manage", Icon: Settings2 },
  { key: "share", label: "Share", Icon: Share2 },
] as const;

// Granted powers are checked controls (jade soft fill); denied powers are a
// soft red. Each icon keeps a distinct glyph, an accessible name stating the
// word, and a lock for inherited rights, so colour never carries meaning alone.
const iconClass = (allowed: boolean, locked: boolean) =>
  cn(
    "relative inline-grid size-[26px] shrink-0 place-items-center rounded-md border border-transparent transition-[background-color,color,transform] duration-fast ease-standard [&>svg:first-child]:size-[15px]",
    allowed
      ? "allowed bg-brand-soft text-brand-text"
      : "denied bg-destructive-soft text-destructive-text",
    locked && "inherited",
  );

export function McpPermissionIcons({
  rights,
  subject,
  onChange,
  disabled = false,
  inherited,
}: {
  rights: Rights;
  subject: string;
  onChange?: (rights: Rights) => void;
  disabled?: boolean;
  inherited?: Partial<Rights>;
}) {
  return (
    <span
      className="mcp-permission-icons inline-flex shrink-0 items-center gap-1 align-middle"
      aria-label={`${subject} permissions`}
    >
      {powers.map(({ key, label, Icon }) => {
        const allowed = rights[key];
        const locked = !!inherited?.[key];
        const description = `${label} for ${subject}: ${allowed ? "Allowed" : "Denied"}${locked ? " (inherited)" : ""}`;
        const icon = (
          <>
            <Icon aria-hidden="true" strokeWidth={1.75} />
            {locked && (
              <LockKeyhole
                className="absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full bg-card p-px text-muted-foreground"
                aria-hidden="true"
              />
            )}
          </>
        );
        return onChange ? (
          <button
            key={key}
            type="button"
            className={cn(
              iconClass(allowed, locked),
              "cursor-pointer hover:border-border-strong active:scale-95 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:cursor-default disabled:opacity-70 disabled:hover:border-transparent",
            )}
            aria-label={description}
            aria-pressed={allowed}
            title={description}
            disabled={disabled || locked}
            onClick={() => onChange({ ...rights, [key]: !allowed })}
          >
            {icon}
          </button>
        ) : (
          <span
            key={key}
            role="img"
            aria-label={description}
            className={iconClass(allowed, locked)}
            title={description}
          >
            {icon}
          </span>
        );
      })}
    </span>
  );
}
