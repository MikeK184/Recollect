import { useEffect, useState } from "react";
import { MoreHorizontal, Plus, Users, X } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "./api";
import { usePollInterval } from "./usePollInterval";
import { SkeletonRows } from "./components/Skeleton";
import {
  Alert,
  Badge,
  Button,
  CardBar,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  IconButton,
  Modal,
  Select,
  TextField,
} from "./components/ui";
import { cn } from "./lib/utils";
import { useFirstLoadStagger } from "./features/activity/firstLoad";
const roles = ["reader", "writer", "admin"];
type Action = {
  kind: "grant" | "remove" | "group" | "ungroup" | "owner";
  target?: string;
};
export function AccessPanel({
  brain,
  actor,
  embedded = false,
}: {
  brain: Brain;
  actor: string;
  embedded?: boolean;
}) {
  const [opened, setOpened] = useState(false);
  const pollNormalWhen0 = usePollInterval("normal", opened || embedded);
  const [editor, setEditor] = useState<Action | null>(null);
  const [username, setUsername] = useState("");
  const [role, setRole] = useState("reader");
  const [group, setGroup] = useState("");
  const [groupRole, setGroupRole] = useState("reader");
  const [nextOwner, setNextOwner] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const cache = useQueryClient();
  const id = brain.id;
  const canEdit = brain.role === "admin" && !brain.archived;
  const access = useQuery({
    queryKey: ["access", id],
    enabled: opened || embedded,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{id}/access", {
          params: { path: { id } },
          signal,
        }),
      ),
    refetchInterval: pollNormalWhen0,
    retry: false,
    gcTime: 0,
  });
  const options = useQuery({
    queryKey: ["auth-options"],
    enabled: opened || embedded,
    queryFn: async () => result(await client.GET("/api/auth/options")),
  });
  const action = useMutation({
    mutationFn: async ({ kind, target }: Action) => {
      let remaining: string | null | undefined;
      if (kind === "grant")
        remaining = result(
          await client.POST("/api/brains/{id}/grants", {
            params: { path: { id } },
            body: { username, role },
          }),
        ).effective_role;
      if (kind === "remove")
        remaining = result(
          await client.DELETE("/api/brains/{id}/grants/{account}", {
            params: { path: { id, account: target! } },
          }),
        ).effective_role;
      if (kind === "group")
        result(
          await client.PUT("/api/brains/{id}/group-grants", {
            params: { path: { id } },
            body: { group_name: group, role: groupRole },
          }),
        );
      if (kind === "ungroup")
        result(
          await client.DELETE("/api/brains/{id}/group-grants/{mapping}", {
            params: { path: { id, mapping: target! } },
          }),
        );
      if (kind === "owner")
        remaining = result(
          await client.POST("/api/brains/{id}/owner", {
            params: { path: { id } },
            body: { account_id: nextOwner! },
          }),
        ).effective_role;
      return remaining === undefined
        ? "Group mapping updated."
        : `Effective access ${kind === "owner" ? "for you " : ""}is now ${remaining ?? "none"}.`;
    },
    onSuccess: (notice) => {
      setMessage(notice);
      setEditor(null);
      setUsername("");
      setGroup("");
      void cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(id) || q.queryKey[0] === "brains",
      });
    },
  });
  useEffect(() => {
    if (access.error || !canEdit) setEditor(null);
  }, [access.error, canEdit]);
  const edit = (value: Action) => {
    action.reset();
    setEditor(value);
  };
  const member = access.data?.members.find(
    (m) => m.account.id === editor?.target,
  );
  const editingSelf = editor?.kind === "remove" && editor.target === actor;
  const destructive = editor?.kind === "remove" || editor?.kind === "ungroup";
  const stagger = useFirstLoadStagger(
    access.data?.members.map((m) => m.account.id),
  );
  const content = (
    <div className="grid">
      <CardBar className={cn(!embedded && "border-b-0 px-0")}>
        <h3>Members</h3>
        {access.data && !access.error && (
          <span className="mono text-caption text-muted-foreground">
            {access.data.members.length}
          </span>
        )}
        <div className="ml-auto flex items-center gap-2">
          {canEdit && options.data?.oidc_configured && (
            <Button
              variant="secondary"
              size="sm"
              disabled={!access.data || !!access.error}
              onClick={() => edit({ kind: "group" })}
            >
              <Users aria-hidden="true" />
              Add group
            </Button>
          )}
          {canEdit && (
            <Button
              size="sm"
              disabled={!access.data || !!access.error}
              onClick={() => {
                setUsername("");
                setRole("reader");
                edit({ kind: "grant" });
              }}
            >
              <Plus aria-hidden="true" />
              Add member
            </Button>
          )}
        </div>
      </CardBar>
      {(access.error || message) && (
        <div className={cn("grid gap-2 pt-3", embedded && "px-4")}>
          {access.error && (
            <Alert
              variant="destructive"
              actions={
                <Button
                  variant="destructive-ghost"
                  size="sm"
                  onClick={() => void access.refetch()}
                >
                  Retry members
                </Button>
              }
            >
              {access.error.message}
            </Alert>
          )}
          {message && (
            <Alert
              variant="brand"
              role="status"
              icon={false}
              className="items-center"
            >
              <div className="flex items-center justify-between gap-3">
                <span>{message}</span>
                <IconButton
                  label="Dismiss"
                  size="icon-sm"
                  onClick={() => setMessage("")}
                >
                  <X aria-hidden="true" />
                </IconButton>
              </div>
            </Alert>
          )}
        </div>
      )}
      {access.isPending ? (
        <div className={cn(embedded && "px-4")}>
          <SkeletonRows rows={3} label="Loading members…" />
        </div>
      ) : (
        !access.error && (
          <div className="grid">
            {access.data?.members.map((m, index) => (
              <div
                className={cn(
                  "flex items-center gap-3 border-b border-border py-2.5 last:border-b-0",
                  embedded && "px-4",
                  stagger(m.account.id, index).className,
                )}
                style={stagger(m.account.id, index).style}
                key={m.account.id}
                data-testid="brain-member"
              >
                <span
                  className="grid size-8 shrink-0 place-items-center rounded-full bg-secondary text-caption font-semibold text-foreground-secondary"
                  aria-hidden
                >
                  {m.account.username.slice(0, 2).toUpperCase()}
                </span>
                <div className="grid min-w-0 flex-1">
                  <span className="truncate text-body font-medium text-foreground">
                    {m.account.username}
                    {m.account.id === actor ? " (you)" : ""}
                  </span>
                  <span className="text-caption text-muted-foreground">
                    {m.owner
                      ? "Owner"
                      : m.direct_role
                        ? "Direct access"
                        : "Group access"}
                    {!m.account.enabled ? " · Account disabled" : ""}
                  </span>
                </div>
                <Badge
                  variant={m.effective_role ? "default" : "outline"}
                  className="capitalize"
                >
                  {m.effective_role ?? "No access"}
                </Badge>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <IconButton label={`Manage ${m.account.username}`}>
                      <MoreHorizontal aria-hidden="true" />
                    </IconButton>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuLabel>
                      {m.owner ? "Owner" : "Member"}
                      {m.direct_role
                        ? ` · Direct ${m.direct_role}`
                        : " · No direct grant"}
                    </DropdownMenuLabel>
                    {m.groups.map((g) => (
                      <DropdownMenuLabel key={g}>Group: {g}</DropdownMenuLabel>
                    ))}
                    {m.membership_until && (
                      <DropdownMenuLabel>
                        Valid until{" "}
                        <span className="mono">
                          {new Date(m.membership_until).toLocaleString()}
                        </span>
                      </DropdownMenuLabel>
                    )}
                    {canEdit && <DropdownMenuSeparator />}
                    {canEdit && (
                      <DropdownMenuItem
                        onSelect={() => {
                          setUsername(m.account.username);
                          setRole(m.direct_role ?? "reader");
                          edit({ kind: "grant", target: m.account.id });
                        }}
                      >
                        Edit direct role
                      </DropdownMenuItem>
                    )}
                    {canEdit &&
                      brain.owner_id === actor &&
                      m.account.enabled &&
                      m.account.id !== actor && (
                        <DropdownMenuItem
                          onSelect={() => {
                            setNextOwner(m.account.id);
                            edit({ kind: "owner" });
                          }}
                        >
                          Transfer ownership
                        </DropdownMenuItem>
                      )}
                    {canEdit && m.direct_role && (
                      <DropdownMenuItem
                        destructive
                        onSelect={() =>
                          edit({ kind: "remove", target: m.account.id })
                        }
                      >
                        Remove direct grant
                      </DropdownMenuItem>
                    )}
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            ))}
          </div>
        )
      )}
      {!access.error && !!access.data?.group_grants.length && (
        <div className={cn("grid gap-1 border-t border-border py-3", embedded && "px-4")}>
          <span className="text-label font-semibold text-foreground">
            Organization groups
          </span>
          {access.data.group_grants.map((g) => (
            <div
              key={g.id}
              className="flex items-center justify-between gap-3 py-1"
            >
              <span className="text-body text-foreground-secondary">
                {g.group_name} · <span className="capitalize">{g.role}</span>
              </span>
              {canEdit && (
                <Button
                  variant="destructive-ghost"
                  size="sm"
                  onClick={() => edit({ kind: "ungroup", target: g.id })}
                >
                  Remove group mapping
                </Button>
              )}
            </div>
          ))}
        </div>
      )}
      <p
        className={cn(
          "m-0 border-t border-border py-2.5 text-caption text-muted-foreground",
          embedded && "bg-muted/50 px-4",
        )}
      >
        Tool-use permissions are managed in Connections → Tool access.
      </p>
    </div>
  );
  return (
    <>
      {embedded ? (
        <section
          className="overflow-hidden rounded-lg border border-border bg-card"
          aria-label="Brain access"
        >
          {content}
        </section>
      ) : (
        <>
          <Button
            className="mt-4"
            variant="secondary"
            onClick={() => setOpened(true)}
          >
            Manage access
          </Button>
          <Modal
            opened={opened}
            onClose={() => !editor && setOpened(false)}
            closeOnEscape={!editor}
            closeOnClickOutside={!editor}
            title="Brain access"
            size="lg"
          >
            {content}
          </Modal>
        </>
      )}
      {editor && canEdit && !access.error && (
        <Modal
          opened
          onClose={() => !action.isPending && setEditor(null)}
          title={
            editor.kind === "grant"
              ? editor.target
                ? "Edit member"
                : "Add member"
              : editor.kind === "remove"
                ? "Remove direct access"
                : editor.kind === "owner"
                  ? "Transfer ownership"
                  : editor.kind === "group"
                    ? "Add organization group"
                    : "Remove organization group"
          }
          size="md"
          closeOnEscape={!action.isPending}
          closeOnClickOutside={!action.isPending}
        >
          <form
            className="grid gap-4"
            onSubmit={(e) => {
              e.preventDefault();
              action.mutate(editor);
            }}
          >
            {editor.kind === "grant" && (
              <>
                <TextField
                  label="Account username"
                  value={username}
                  disabled={!!editor.target || action.isPending}
                  onChange={(e) => setUsername(e.currentTarget.value)}
                  required
                  maxLength={120}
                />
                <Select
                  label="Direct role"
                  value={role}
                  data={roles}
                  onChange={(v) => setRole(v ?? "reader")}
                  disabled={action.isPending}
                />
                {member?.groups.length ? (
                  <p className="m-0 text-caption text-muted-foreground">
                    Group access remains independent of this role.
                  </p>
                ) : null}
              </>
            )}
            {editor.kind === "group" && (
              <>
                <TextField
                  label="Group name or ID"
                  value={group}
                  onChange={(e) => setGroup(e.currentTarget.value)}
                  required
                  maxLength={200}
                />
                <Select
                  label="Group role"
                  value={groupRole}
                  data={roles}
                  onChange={(v) => setGroupRole(v ?? "reader")}
                />
              </>
            )}
            {editor.kind === "remove" && (
              <p className="m-0 text-body text-foreground">
                Remove the direct grant for {member?.account.username}?{" "}
                {member?.owner || member?.groups.length
                  ? "Owner or group permissions will still apply."
                  : "This removes their direct Brain access."}
                {editingSelf && " You may lose your own access."}
              </p>
            )}
            {editor.kind === "ungroup" && (
              <p className="m-0 text-body text-foreground">
                Remove this organization's grant to the Brain? Direct grants
                remain.
              </p>
            )}
            {editor.kind === "owner" && (
              <>
                <Select
                  label="New owner"
                  value={nextOwner}
                  onChange={setNextOwner}
                  data={
                    access.data?.members
                      .filter(
                        (m) => m.account.enabled && m.account.id !== actor,
                      )
                      .map((m) => ({
                        value: m.account.id,
                        label: m.account.username,
                      })) ?? []
                  }
                />
                <p className="m-0 text-body text-foreground-secondary">
                  Your access will depend on your remaining direct or group
                  grants.
                </p>
              </>
            )}
            {action.error && (
              <Alert variant="destructive">{action.error.message}</Alert>
            )}
            <div className="-mx-5 -mb-5 flex justify-end gap-2 border-t border-border bg-muted px-5 py-3">
              <Button
                variant="secondary"
                disabled={action.isPending}
                onClick={() => setEditor(null)}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                loading={action.isPending}
                variant={destructive ? "destructive" : "default"}
                disabled={editor.kind === "owner" && !nextOwner}
              >
                {editor.kind === "grant"
                  ? "Save direct grant"
                  : editor.kind === "group"
                    ? "Save group mapping"
                    : editor.kind === "owner"
                      ? "Transfer ownership"
                      : "Remove grant"}
              </Button>
            </div>
          </form>
        </Modal>
      )}
    </>
  );
}
