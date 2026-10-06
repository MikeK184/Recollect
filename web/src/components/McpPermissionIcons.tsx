import { Play, Settings2, Share2 } from "lucide-react";
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
}: {
  rights: Rights;
  subject: string;
  onChange?: (rights: Rights) => void;
  disabled?: boolean;
}) {
  return (
    <span
      className="mcp-permission-icons"
      aria-label={`${subject} permissions`}
    >
      {powers.map(({ key, label, Icon }) => {
        const allowed = rights[key];
        const description = `${label} for ${subject}: ${allowed ? "Allowed" : "Denied"}`;
        return onChange ? (
          <button
            key={key}
            type="button"
            className={`mcp-permission-icon ${allowed ? "allowed" : "denied"}`}
            aria-label={description}
            aria-pressed={allowed}
            title={description}
            disabled={disabled}
            onClick={() => onChange({ ...rights, [key]: !allowed })}
          >
            <Icon size={15} aria-hidden="true" />
          </button>
        ) : (
          <span
            key={key}
            role="img"
            aria-label={description}
            className={`mcp-permission-icon ${allowed ? "allowed" : "denied"}`}
            title={description}
          >
            <Icon size={15} aria-hidden="true" />
          </span>
        );
      })}
    </span>
  );
}
