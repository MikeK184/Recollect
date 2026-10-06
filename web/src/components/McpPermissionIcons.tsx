import { LockKeyhole, Play, Settings2, Share2 } from "lucide-react";
import type { components } from "../api-schema";

type Rights = components["schemas"]["McpRights"];
const powers = [
  { key: "use_profile", label: "Use", Icon: Play },
  { key: "manage", label: "Manage", Icon: Settings2 },
  { key: "share", label: "Share", Icon: Share2 },
] as const;

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
      className="mcp-permission-icons"
      aria-label={`${subject} permissions`}
    >
      {powers.map(({ key, label, Icon }) => {
        const allowed = rights[key];
        const locked = !!inherited?.[key];
        const description = `${label} for ${subject}: ${allowed ? "Allowed" : "Denied"}${locked ? " (inherited)" : ""}`;
        const icon = (
          <>
            <Icon size={15} aria-hidden="true" />
            {locked && (
              <LockKeyhole
                className="mcp-permission-lock"
                size={9}
                aria-hidden="true"
              />
            )}
          </>
        );
        return onChange ? (
          <button
            key={key}
            type="button"
            className={`mcp-permission-icon ${allowed ? "allowed" : "denied"}${locked ? " inherited" : ""}`}
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
            className={`mcp-permission-icon ${allowed ? "allowed" : "denied"}${locked ? " inherited" : ""}`}
            title={description}
          >
            {icon}
          </span>
        );
      })}
    </span>
  );
}
