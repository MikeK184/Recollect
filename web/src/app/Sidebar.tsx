import { cloneElement, type ReactElement, type ReactNode } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { motion } from "motion/react";
import {
  ChevronDown,
  ChevronsUpDown,
  LogOut,
  Monitor,
  Moon,
  Search,
  Sun,
  Users,
  type LucideIcon,
} from "lucide-react";
import type { Brain, Session } from "../api";
import { cn } from "../lib/utils";
import { glide } from "../design/motion";
import { useThemeMode, type ThemePreference } from "../design/theme-mode";
import { BrainIcon } from "../components/BrainIcon";
import { BrandMark } from "../components/Brand";
import { Kbds, modKey } from "../components/ui/kbd";
import { Count } from "../components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "../components/ui/dropdown-menu";
import { brainNavigation, brainTiers, globalNavigation } from "./navigation";

const explore = ["memory", "sources", "repositories", "explore"];

/**
 * Sidebar on the canvas (240px, or the 56px rail). Inside a Brain: the
 * Brain switcher, ⌘K search, the Brain tier, the Manage disclosure and the
 * workspace links. Elsewhere: the brand, search, workspace links and recent
 * Brains. The account row keeps the theme choice and a visible Sign out.
 */
export function Sidebar({
  user,
  rail,
  path,
  brainId,
  section,
  navigationBrain,
  navigationBrainId,
  brains,
  brainsPending,
  onSearch,
  onSignOut,
  signingOut,
}: {
  user: Session;
  rail: boolean;
  path: string;
  brainId?: string;
  section?: string;
  navigationBrain?: Brain;
  navigationBrainId?: string;
  brains: Brain[];
  brainsPending: boolean;
  onSearch: () => void;
  onSignOut: () => void;
  signingOut: boolean;
}) {
  const owner = user.user.installation_owner;
  const workspaceLinks = globalNavigation.filter(
    (n) => n.to !== "/team" && (n.to !== "/connectors" || owner),
  );
  const recent = brains.filter((b) => !b.archived).slice(0, 3);
  return (
    <aside
      className={cn(
        "flex shrink-0 flex-col gap-0.5 overflow-y-auto overflow-x-hidden bg-sidebar px-2 py-2.5 text-sidebar-foreground scrollbar-none",
        rail ? "w-14 items-center" : "w-60",
      )}
      data-rail={rail || undefined}
    >
      {navigationBrainId ? (
        <BrainSwitcher
          rail={rail}
          brain={navigationBrain}
          brainId={navigationBrainId}
          brains={brains}
          pending={brainsPending}
        />
      ) : (
        <Link
          to="/"
          aria-label="Recollect home"
          className={cn(
            "flex h-10 items-center gap-2 rounded-md px-1.5 font-serif text-[20px] font-semibold leading-6 tracking-[-0.02em] text-foreground transition-colors duration-fast hover:bg-accent",
            rail && "justify-center px-0",
          )}
        >
          <BrandMark />
          {!rail && (
            <span aria-hidden="true">
              Recollect<span className="text-brand-text">.</span>
            </span>
          )}
        </Link>
      )}

      <button
        type="button"
        onClick={onSearch}
        aria-label="Search and commands"
        aria-keyshortcuts="Meta+K Control+K"
        title={rail ? `Search (${modKey}K)` : undefined}
        className={cn(
          "mb-2.5 mt-1.5 flex h-8 items-center gap-2 rounded-md border border-border bg-card text-body text-muted-foreground shadow-xs transition-colors duration-fast hover:border-border-strong hover:text-foreground",
          rail ? "w-8 justify-center" : "w-full pl-2.5 pr-1.5",
        )}
      >
        <Search className="size-4" aria-hidden="true" />
        {!rail && (
          <>
            <span>Search</span>
            <Kbds keys={[modKey, "K"]} className="ml-auto" />
          </>
        )}
      </button>

      {navigationBrainId && (
        <nav aria-label="Brain navigation" className="flex w-full flex-col gap-0.5">
          {brainTiers.map((tier) => {
            const items = brainNavigation
              .filter((n) => n.tier === tier && !n.hidden)
              .map(({ section: value, label, icon }) => {
                const active =
                  section === value ||
                  (value === "explore" && explore.includes(section ?? ""));
                return (
                  <NavItem
                    key={value}
                    rail={rail}
                    icon={icon}
                    label={label}
                    active={active}
                    group="brain"
                  >
                    <Link
                      to={`/brains/$brainId/${value}`}
                      params={{ brainId: navigationBrainId }}
                      search={{}}
                      aria-current={active ? "page" : undefined}
                    />
                  </NavItem>
                );
              });
            if (tier !== "Manage")
              return (
                <div
                  key={tier}
                  role="group"
                  aria-labelledby={`nav-tier-${tier.toLowerCase()}`}
                  className="flex flex-col gap-0.5"
                >
                  <span id={`nav-tier-${tier.toLowerCase()}`} className="sr-only">
                    {tier}
                  </span>
                  {items}
                </div>
              );
            return (
              <details
                key={tier}
                open
                className="group/manage flex flex-col"
                role="group"
                aria-labelledby={`nav-tier-${tier.toLowerCase()}`}
              >
                <summary
                  id={`nav-tier-${tier.toLowerCase()}`}
                  className={cn(
                    "flex cursor-pointer list-none items-center gap-1 rounded-sm text-caption font-medium text-muted-foreground transition-colors hover:text-foreground [&::-webkit-details-marker]:hidden",
                    rail ? "h-6 justify-center px-0 pt-2" : "px-2.5 pb-1.5 pt-3.5",
                  )}
                >
                  {rail ? <span className="sr-only">{tier}</span> : <span>{tier}</span>}
                  <ChevronDown
                    aria-hidden="true"
                    className={cn(
                      "size-3.5 -rotate-90 transition-transform duration-fast group-open/manage:rotate-0",
                      rail && "hidden",
                    )}
                  />
                </summary>
                <div className="flex flex-col gap-0.5">{items}</div>
              </details>
            );
          })}
        </nav>
      )}

      <nav aria-label="Workspace navigation" className="flex w-full flex-col gap-0.5">
        <SectionLabel rail={rail} first={!navigationBrainId}>
          Workspace
        </SectionLabel>
        {workspaceLinks.map(({ to, label, icon }) => {
          const active = path === to;
          const shown = to === "/" && navigationBrainId ? "All Brains" : label;
          return (
            <NavItem
              key={to}
              rail={rail}
              icon={icon}
              label={shown}
              active={active}
              group="workspace"
              trailing={
                to === "/" && !rail ? (
                  <Count className="ml-auto bg-transparent">{brains.length}</Count>
                ) : undefined
              }
            >
              <Link to={to} search={{}} aria-current={active ? "page" : undefined} />
            </NavItem>
          );
        })}
      </nav>
      {owner && (
        <nav aria-label="Installation navigation" className="flex w-full flex-col gap-0.5">
          <NavItem
            rail={rail}
            icon={Users}
            label="Team"
            active={path === "/team"}
            group="workspace"
          >
            <Link to="/team" aria-current={path === "/team" ? "page" : undefined} />
          </NavItem>
        </nav>
      )}

      {!navigationBrainId && !rail && recent.length > 0 && (
        <nav aria-label="Recent Brains" className="flex w-full flex-col gap-0.5">
          <SectionLabel rail={rail}>Recent Brains</SectionLabel>
          {recent.map((brain) => (
            <Link
              key={brain.id}
              to="/brains/$brainId/dashboard"
              params={{ brainId: brain.id }}
              search={{}}
              className="flex h-8 items-center gap-2.5 rounded-md px-2.5 text-body font-medium text-sidebar-foreground transition-colors duration-fast hover:bg-accent hover:text-foreground"
            >
              <BrainIcon id={brain.id} revision={brain.icon_revision} size={18} />
              <span className="min-w-0 truncate">{brain.name}</span>
            </Link>
          ))}
        </nav>
      )}

      <AccountRow user={user} rail={rail} onSignOut={onSignOut} signingOut={signingOut} />
    </aside>
  );
}

function SectionLabel({
  rail,
  first = false,
  children,
}: {
  rail: boolean;
  first?: boolean;
  children: ReactNode;
}) {
  if (rail)
    return first ? <span className="sr-only">{children}</span> : (
      <span className="mx-auto my-2 h-px w-6 bg-border" role="presentation">
        <span className="sr-only">{children}</span>
      </span>
    );
  return (
    <span
      className={cn(
        "px-2.5 pb-1.5 text-caption font-medium text-muted-foreground",
        first ? "pt-1" : "pt-3.5",
      )}
    >
      {children}
    </span>
  );
}

/**
 * One navigation link. The active item gets a raised chip that glides between
 * items (layoutId) and the jade icon; hover never moves layout.
 */
function NavItem({
  rail,
  icon: Icon,
  label,
  active,
  group,
  trailing,
  children,
}: {
  rail: boolean;
  icon: LucideIcon;
  label: string;
  active: boolean;
  group: string;
  trailing?: ReactNode;
  children: ReactElement<{ className?: string; children?: ReactNode; title?: string }>;
}) {
  const link = children;
  const className = cn(
    "relative flex h-8 items-center gap-2.5 rounded-md text-body font-medium no-underline outline-none transition-colors duration-fast ease-standard focus-visible:outline-2 focus-visible:outline-ring",
    rail ? "w-8 justify-center px-0" : "w-full px-2.5",
    active
      ? "text-sidebar-accent-foreground"
      : "text-sidebar-foreground hover:bg-accent hover:text-foreground",
  );
  const content = (
    <>
      {active && (
        <motion.span
          layoutId={`nav-active-${group}`}
          transition={glide}
          aria-hidden="true"
          className="absolute inset-0 rounded-md bg-sidebar-accent shadow-xs"
        />
      )}
      <Icon
        aria-hidden="true"
        className={cn(
          "relative size-4 shrink-0",
          active ? "text-sidebar-primary" : "text-muted-foreground",
        )}
      />
      <span className={cn("relative min-w-0 truncate", rail && "sr-only")}>{label}</span>
      {trailing && <span className="relative ml-auto flex">{trailing}</span>}
    </>
  );
  return cloneElement(link, {
    className,
    title: rail ? label : undefined,
    children: content,
  });
}

function BrainSwitcher({
  rail,
  brain,
  brainId,
  brains,
  pending,
}: {
  rail: boolean;
  brain?: Brain;
  brainId: string;
  brains: Brain[];
  pending: boolean;
}) {
  const navigate = useNavigate();
  const name = brain?.name ?? (pending ? "Opening Brain…" : "Current Brain");
  return (
    <div className={cn("flex items-center gap-1", rail ? "flex-col" : "w-full")}>
      <Link
        to="/"
        aria-label="Recollect home"
        title="All Brains"
        className="grid size-10 shrink-0 place-items-center rounded-md transition-colors duration-fast hover:bg-accent"
      >
        <BrandMark />
      </Link>
      <div
        className={cn(
          "relative flex h-10 min-w-0 items-center gap-2 rounded-md text-body font-semibold text-foreground transition-colors duration-fast hover:bg-accent has-[select:focus-visible]:outline-2 has-[select:focus-visible]:outline-ring",
          rail ? "w-10 justify-center" : "flex-1 pl-1.5 pr-2",
        )}
        title={rail ? name : undefined}
      >
        {brain?.icon_revision ? (
          <BrainIcon id={brain.id} revision={brain.icon_revision} size={22} />
        ) : rail ? (
          <span className="mono grid size-6 place-items-center rounded-sm bg-secondary text-caption font-medium text-foreground-secondary">
            {name.slice(0, 1).toUpperCase()}
          </span>
        ) : null}
        {!rail && (
          <span className="min-w-0 flex-1 truncate" aria-hidden="true">
            {name}
            {brain?.archived ? " (archived)" : ""}
          </span>
        )}
        {!rail && (
          <ChevronsUpDown className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
        )}
        <label htmlFor="brain-switcher" className="sr-only">
          Switch Brain
        </label>
        <select
          id="brain-switcher"
          value={brainId}
          className="absolute inset-0 cursor-pointer appearance-none opacity-0"
          onChange={(e) => {
            void navigate({
              to: "/brains/$brainId/dashboard",
              params: { brainId: e.currentTarget.value },
              search: {},
            });
          }}
        >
          {!brain && <option value={brainId}>{name}</option>}
          {brains.map((b) => (
            <option key={b.id} value={b.id}>
              {b.name}
              {b.archived ? " (archived)" : ""}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

const themeOptions: { value: ThemePreference; label: string; icon: LucideIcon }[] = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Ink", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
];

function AccountRow({
  user,
  rail,
  onSignOut,
  signingOut,
}: {
  user: Session;
  rail: boolean;
  onSignOut: () => void;
  signingOut: boolean;
}) {
  const theme = useThemeMode();
  const ThemeIcon = theme.resolved === "dark" ? Moon : Sun;
  const initials = user.user.username.slice(0, 2).toUpperCase();
  const iconButton =
    "grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground transition-colors duration-fast hover:bg-accent hover:text-foreground aria-expanded:bg-accent aria-expanded:text-foreground disabled:opacity-50";
  return (
    <div
      className={cn(
        "mt-auto flex w-full items-center gap-2.5 border-t border-sidebar-border pt-2 text-label",
        rail ? "flex-col px-0" : "px-1.5",
      )}
    >
      <span
        className="grid size-6 shrink-0 place-items-center rounded-full bg-kind-claim-soft text-[11px] font-semibold text-kind-claim"
        title={rail ? user.user.username : undefined}
        aria-hidden="true"
      >
        {initials}
      </span>
      {!rail && (
        <span className="min-w-0 flex-1">
          <span className="block truncate font-medium text-foreground">{user.user.username}</span>
          <span className="block truncate text-caption text-muted-foreground">
            {user.user.installation_owner ? "Installation owner" : "Member"}
          </span>
        </span>
      )}
      <DropdownMenu>
        <DropdownMenuTrigger className={iconButton} aria-label="Theme" title="Theme">
          <ThemeIcon className="size-4" aria-hidden="true" />
        </DropdownMenuTrigger>
        <DropdownMenuContent side="top" align={rail ? "start" : "end"} className="min-w-44">
          <DropdownMenuLabel>Theme</DropdownMenuLabel>
          <DropdownMenuRadioGroup
            value={theme.preference}
            onValueChange={(value) => theme.setPreference(value as ThemePreference)}
          >
            {themeOptions.map(({ value, label, icon: Icon }) => (
              <DropdownMenuRadioItem key={value} value={value}>
                <Icon aria-hidden="true" />
                {label}
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuContent>
      </DropdownMenu>
      <button
        type="button"
        className={iconButton}
        aria-label="Sign out"
        title="Sign out"
        onClick={onSignOut}
        disabled={signingOut}
      >
        <LogOut className="size-4" aria-hidden="true" />
      </button>
    </div>
  );
}
