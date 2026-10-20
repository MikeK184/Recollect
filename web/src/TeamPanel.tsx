import { useEffect, useId, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Info, Mail, UserPlus } from "lucide-react";
import { client, result } from "./api";
import { useWorkspace } from "./app/context";
import { EmptyState, ErrorState } from "./components/AsyncState";
import { Page } from "./components/Page";
import { PageHeader } from "./components/PageHeader";
import { SkeletonRows } from "./components/Skeleton";
import { staggerStyle } from "./components/Motion";
import {
  Alert,
  Badge,
  Button,
  Count,
  Drawer,
  Field,
  Select,
  StatusBadge,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableWrap,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  Textarea,
  TextField,
} from "./components/ui";
import { cn } from "./lib/utils";
import { usePollInterval } from "./usePollInterval";

const timestamp = (value: string) => new Date(value).toLocaleString();

function Avatar({ name }: { name: string }) {
  return (
    <span
      aria-hidden="true"
      className="grid size-7 shrink-0 place-items-center rounded-full bg-kind-claim-soft text-[11px] font-semibold leading-none text-kind-claim"
    >
      {name.slice(0, 2).toUpperCase()}
    </span>
  );
}

export function TeamPanel() {
  const pollNormal = usePollInterval("normal");
  const session = useWorkspace();
  const cache = useQueryClient();
  const formId = `team-invite-${useId()}`;
  const [tab, setTab] = useState("accounts");
  const [opened, setOpened] = useState(false);
  const [username, setUsername] = useState("");
  const [kind, setKind] = useState("local");
  const [subject, setSubject] = useState("");
  const [link, setLink] = useState("");
  const usernameInput = useRef<HTMLInputElement>(null);
  // Focus the username once the sheet has mounted; the sheet then returns
  // focus to Add teammate when it closes.
  useEffect(() => {
    if (!opened) return;
    const timer = setTimeout(() => usernameInput.current?.focus(), 0);
    return () => clearTimeout(timer);
  }, [opened]);
  const team = useQuery({
    queryKey: ["team"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/team", { signal })),
    gcTime: 0,
    refetchInterval: pollNormal,
  });
  const options = useQuery({
    queryKey: ["auth-options"],
    queryFn: async () => result(await client.GET("/api/auth/options")),
  });
  const history = useQuery({
    queryKey: ["team-audit"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/team/audit", { signal })),
    gcTime: 0,
  });
  const refresh = () => {
    void cache.invalidateQueries({ queryKey: ["team"] });
    void cache.invalidateQueries({ queryKey: ["team-audit"] });
  };
  const invite = useMutation({
    mutationFn: async () => {
      if (kind === "oidc") {
        result(
          await client.POST("/api/team/oidc-accounts", {
            body: { username, subject },
          }),
        );
        return null;
      }
      return result(
        await client.POST("/api/team/invitations", { body: { username } }),
      );
    },
    onSuccess: (value) => {
      setLink(value ? `${window.location.origin}/#invite=${value.token}` : "");
      setOpened(false);
      setUsername("");
      setSubject("");
      refresh();
    },
  });
  const action = useMutation({
    mutationFn: async ({
      id,
      kind,
      enabled,
    }: {
      id: string;
      kind: "status" | "reset" | "revoke";
      enabled?: boolean;
    }) => {
      if (kind === "status")
        result(
          await client.PATCH("/api/team/accounts/{id}", {
            params: { path: { id } },
            body: { enabled: !!enabled },
          }),
        );
      if (kind === "revoke")
        result(
          await client.DELETE("/api/team/invitations/{id}", {
            params: { path: { id } },
          }),
        );
      if (kind === "reset") {
        const value = result(
          await client.POST("/api/team/accounts/{id}/reset", {
            params: { path: { id } },
          }),
        );
        setLink(`${window.location.origin}/#invite=${value.token}`);
      }
    },
    onSuccess: refresh,
  });
  useEffect(() => {
    if (team.isError) {
      setOpened(false);
      setLink("");
    }
  }, [team.isError]);
  const header = (withAction: boolean) => (
    <PageHeader
      meta={<Badge variant="outline">People and permissions</Badge>}
      title="Team"
      description="Invite people, manage their sign-in and keep access current."
      actions={
        withAction && (
          <Button
            onClick={() => {
              invite.reset();
              setOpened(true);
            }}
          >
            <UserPlus aria-hidden="true" />
            Add teammate
          </Button>
        )
      }
    />
  );
  if (team.isError)
    return (
      <Page className="text-body">
        {header(false)}
        <ErrorState error={team.error} retry={() => void team.refetch()} />
      </Page>
    );
  const accounts = team.data?.accounts ?? [];
  const invitations = team.data?.invitations ?? [];
  return (
    <Page className="text-body">
      {header(true)}
      {action.error && (
        <Alert
          variant="destructive"
          title="Team action failed"
          actions={
            <Button
              variant="destructive-ghost"
              size="sm"
              onClick={() => void team.refetch()}
            >
              Refresh
            </Button>
          }
        >
          {action.error.message}
        </Alert>
      )}
      <Tabs value={tab} onValueChange={setTab} className="gap-4">
        <TabsList aria-label="Team sections">
          <TabsTrigger value="accounts">
            Accounts
            {team.data && <Count aria-hidden="true">{accounts.length}</Count>}
          </TabsTrigger>
          <TabsTrigger value="invitations">
            Invitations
            {team.data && (
              <Count aria-hidden="true">{invitations.length}</Count>
            )}
          </TabsTrigger>
          <TabsTrigger value="activity">Account activity</TabsTrigger>
        </TabsList>
        <TabsContent value="accounts">
          {team.isPending ? (
            <TableWrap className="px-3">
              <SkeletonRows rows={3} label="Loading accounts…" />
            </TableWrap>
          ) : (
            <TableWrap>
              <Table>
                <TableHead>
                  <tr>
                    <TableHeader>Account</TableHeader>
                    <TableHeader className="w-[140px]">Status</TableHeader>
                    <TableHeader className="w-[240px]" aria-label="Actions" />
                  </tr>
                </TableHead>
                <TableBody>
                  {accounts.map((account, index) => (
                    <TableRow
                      key={account.id}
                      data-slot="card"
                      className="animate-enter"
                      style={staggerStyle(index)}
                    >
                      <TableCell className="py-2.5">
                        <div className="flex items-center gap-3">
                          <Avatar name={account.username} />
                          <div className="min-w-0">
                            <div className="flex items-center gap-1.5">
                              <span className="truncate font-medium text-foreground">
                                {account.username}
                              </span>
                              {account.id === session.user.id && (
                                <Badge variant="outline" className="h-5">
                                  You
                                </Badge>
                              )}
                            </div>
                            <span className="block text-caption text-muted-foreground">
                              {account.installation_owner
                                ? "Installation owner"
                                : account.auth_kind === "oidc"
                                  ? "Organization identity"
                                  : "Local account"}
                            </span>
                          </div>
                        </div>
                      </TableCell>
                      <TableCell>
                        <StatusBadge
                          state={account.enabled ? "positive" : "neutral"}
                        >
                          {account.enabled ? "Enabled" : "Disabled"}
                        </StatusBadge>
                      </TableCell>
                      <TableCell>
                        {!account.installation_owner && (
                          <div className="flex items-center justify-end gap-1.5">
                            <Button
                              variant="ghost"
                              size="sm"
                              loading={
                                action.isPending &&
                                action.variables?.id === account.id &&
                                action.variables.kind === "status"
                              }
                              disabled={action.isPending}
                              onClick={() =>
                                action.mutate({
                                  id: account.id,
                                  kind: "status",
                                  enabled: !account.enabled,
                                })
                              }
                            >
                              {account.enabled ? "Disable" : "Enable"}
                            </Button>
                            {account.auth_kind === "local" && (
                              <Button
                                variant="secondary"
                                size="sm"
                                disabled={action.isPending}
                                onClick={() =>
                                  action.mutate({
                                    id: account.id,
                                    kind: "reset",
                                  })
                                }
                              >
                                Reset sign-in
                              </Button>
                            )}
                          </div>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableWrap>
          )}
        </TabsContent>
        <TabsContent value="invitations">
          {team.isPending ? (
            <TableWrap className="px-3">
              <SkeletonRows rows={2} label="Loading invitations…" />
            </TableWrap>
          ) : invitations.length === 0 ? (
            <div className="rounded-lg border border-border bg-card">
              <EmptyState
                icon={Mail}
                title="No invitations yet"
                description="Add a teammate to create a private, single-use invitation link."
              />
            </div>
          ) : (
            <TableWrap>
              <Table>
                <TableHead>
                  <tr>
                    <TableHeader>Invitation</TableHeader>
                    <TableHeader className="w-[140px]">State</TableHeader>
                    <TableHeader className="w-[220px]">Expires</TableHeader>
                    <TableHeader className="w-[180px]" aria-label="Actions" />
                  </tr>
                </TableHead>
                <TableBody>
                  {invitations.map((invitation, index) => (
                    <TableRow
                      key={invitation.id}
                      className="animate-enter"
                      style={staggerStyle(index)}
                    >
                      <TableCell primary>
                        <div className="flex items-center gap-3">
                          <Avatar name={invitation.username} />
                          <span className="truncate">
                            {invitation.username}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <StatusBadge
                          state={
                            invitation.state === "pending"
                              ? "attention"
                              : invitation.state === "accepted"
                                ? "positive"
                                : "neutral"
                          }
                        >
                          {invitation.state.charAt(0).toUpperCase() +
                            invitation.state.slice(1)}
                        </StatusBadge>
                      </TableCell>
                      <TableCell>
                        <span className="mono text-caption text-muted-foreground">
                          {timestamp(invitation.expires_at)}
                        </span>
                      </TableCell>
                      <TableCell>
                        {invitation.state === "pending" && (
                          <div className="flex justify-end">
                            <Button
                              size="sm"
                              variant="destructive-ghost"
                              disabled={action.isPending}
                              onClick={() =>
                                action.mutate({
                                  id: invitation.id,
                                  kind: "revoke",
                                })
                              }
                            >
                              Revoke invitation
                            </Button>
                          </div>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableWrap>
          )}
        </TabsContent>
        <TabsContent value="activity">
          {history.error ? (
            <Alert variant="destructive">{history.error.message}</Alert>
          ) : history.isPending ? (
            <TableWrap className="px-3">
              <SkeletonRows rows={4} label="Loading account activity…" />
            </TableWrap>
          ) : !history.data?.length ? (
            <div className="rounded-lg border border-border bg-card">
              <EmptyState
                title="No account activity yet"
                description="Sign-in, invitation and account changes appear here."
              />
            </div>
          ) : (
            <TableWrap>
              <Table>
                <TableHead>
                  <tr>
                    <TableHeader>Event</TableHeader>
                    <TableHeader className="w-[220px]">Time</TableHeader>
                  </tr>
                </TableHead>
                <TableBody>
                  {history.data.slice(0, 12).map((event, index) => (
                    <TableRow
                      key={event.id}
                      className="animate-enter"
                      style={staggerStyle(index)}
                    >
                      <TableCell>
                        <span className="mono text-label text-foreground-secondary">
                          {event.action
                            .replaceAll(".", " ")
                            .replaceAll("_", " ")}{" "}
                          · {event.disposition.replaceAll("_", " ")}
                        </span>
                      </TableCell>
                      <TableCell>
                        <span className="mono text-caption text-muted-foreground">
                          {timestamp(event.created_at)}
                        </span>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableWrap>
          )}
        </TabsContent>
      </Tabs>
      <Alert
        variant="info"
        role="note"
        icon={<Info aria-hidden="true" />}
        title="Accounts sign in to this installation."
      >
        Share each Brain separately from its Access panel.
      </Alert>
      <Drawer
        opened={opened}
        onClose={() => setOpened(false)}
        title="Add a teammate"
        description="Create a private invitation or enroll an organization identity."
        size={480}
        footer={
          <>
            <Button variant="secondary" onClick={() => setOpened(false)}>
              Cancel
            </Button>
            <Button type="submit" form={formId} loading={invite.isPending}>
              {kind === "oidc" ? "Enroll identity" : "Create invitation"}
            </Button>
          </>
        }
      >
        <form
          id={formId}
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            invite.mutate();
          }}
        >
          <TextField
            label="Teammate username"
            value={username}
            required
            maxLength={120}
            ref={usernameInput}
            onChange={(e) => setUsername(e.target.value)}
          />
          <Select
            label="Sign-in method"
            value={kind}
            onChange={(v) => setKind(v ?? "local")}
            data={[
              { value: "local", label: "Local invitation" },
              ...(options.data?.oidc_configured
                ? [{ value: "oidc", label: "Organization identity" }]
                : []),
            ]}
          />
          {kind === "oidc" ? (
            <TextField
              label="Provider subject"
              description="Use the exact stable subject from your configured identity provider."
              value={subject}
              required
              maxLength={255}
              onChange={(e) => setSubject(e.target.value)}
            />
          ) : (
            <p className="m-0 rounded-md bg-muted px-3 py-2.5 text-label text-muted-foreground">
              You will receive a private invitation link to share. It expires
              after 24 hours and can be used once.
            </p>
          )}
          {invite.error && (
            <Alert variant="destructive">{invite.error.message}</Alert>
          )}
        </form>
      </Drawer>
      <Drawer
        opened={!!link}
        onClose={() => setLink("")}
        title="Your invitation is ready"
        description="Share this link privately with your teammate. It is shown here once."
        size={480}
        footer={<Button onClick={() => setLink("")}>Done</Button>}
      >
        <div className="flex flex-col gap-4">
          <Field label="Invitation link">
            <Textarea
              value={link}
              readOnly
              autosize
              minRows={3}
              className={cn("mono text-label")}
            />
          </Field>
          <p className="m-0 border-t border-border pt-4 text-caption text-muted-foreground">
            Resetting sign-in revokes existing sessions and requires a new
            invitation acceptance.
          </p>
        </div>
      </Drawer>
    </Page>
  );
}
