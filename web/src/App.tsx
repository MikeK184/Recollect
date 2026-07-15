import { TeamPanel } from "./TeamPanel";
import { DevicesPanel } from "./DevicesPanel";
import { EvidencePanel } from "./EvidencePanel";
import { WorkspacePanel } from "./WorkspacePanel";
import { PublicationPanel } from "./PublicationPanel";
import { ClaimsPanel } from "./ClaimsPanel";
import { RetentionPanel } from "./RetentionPanel";
import { ModelsPanel } from "./ModelsPanel";
import { CapturePanel } from "./CapturePanel";
import { RecallPanel } from "./RecallPanel";
import { GraphPanel } from "./GraphPanel";
import { McpPanel } from "./McpPanel";
import { HandoversPanel } from "./HandoversPanel";
import { AccessPanel } from "./AccessPanel";
import { Enrollment, initialInvitation } from "./Enrollment";
import { JobsPanel } from "./JobsPanel";
import { OperationsDialog } from "./OperationsDialog";
import { useIdempotency } from "./useIdempotency";
import { useEffect, useState, type FormEvent } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Divider,
  Group,
  Loader,
  Modal,
  PasswordInput,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from "@mantine/core";
import {
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
} from "@tanstack/react-query";
import { Link, useNavigate, useRouterState } from "@tanstack/react-router";
import {
  ArrowLeft,
  ArrowRight,
  Archive,
  BookOpen,
  Boxes,
  Check,
  Database,
  Layers3,
  LogOut,
  Laptop,
  Plus,
  RefreshCw,
  Settings2,
  Shield,
  Sparkles,
} from "lucide-react";
import {
  client,
  result,
  session,
  setCsrf,
  RequestError,
  type Brain,
  type Session,
} from "./api";

async function replaceSession(cache: QueryClient, value: Session | null) {
  await cache.cancelQueries();
  cache.removeQueries({
    predicate: (query) => query.queryKey[0] !== "session",
  });
  setCsrf(value?.csrf_token ?? "");
  cache.setQueryData(["session"], value);
}

const activityLabels: Record<string, string> = {
  "source.import": "Source imported",
  "source.excerpt": "Supporting excerpt retained",
  "retention.update": "Retention policy updated",
  "memory.erase": "Memory erasure requested",
  "source.update": "Source version added",
  "source.process": "Source processing completed",
  "source.reprocess": "Source processing requested",
  "source.organize": "Source views updated",
  "evidence.group.create": "View created",
  "evidence.group.update": "View updated",
  "evidence.group.remove": "View removed",
  "evidence.policy": "Document capture policy updated",
  "workspace.refresh": "Checkout catalogue refreshed",
  "repository.alias": "Repository origin attached",
  "task.create": "Task started",
  "task.scope": "Task scope changed",
  "task.close": "Task closed",
  "operation.bind": "Operation scope recorded",
  "repository.publish": "Repository snapshot published",
  "repository.policy": "Repository capture policy changed",
  "repository.process": "Repository facts processed",
  "repository.reprocess": "Repository reprocessing requested",
  "manifest.revise": "Revision manifest recorded",
  "claim.propose": "Claim proposal recorded",
  "claim.review": "Claim review recorded",
};

function Failure({
  error,
  retry,
}: {
  error: Error | null;
  retry?: () => void;
}) {
  if (!error) return null;
  return (
    <Alert color="red" title="Something needs attention">
      <Text size="sm">{error.message}</Text>
      {retry && (
        <Button variant="subtle" color="red" size="xs" mt="sm" onClick={retry}>
          Try again
        </Button>
      )}
    </Alert>
  );
}

export function App() {
  const cache = useQueryClient();
  const navigate = useNavigate();
  const [invitation, setInvitation] = useState(initialInvitation);
  useEffect(() => {
    const readInvite = () => {
      const token = new URLSearchParams(window.location.hash.slice(1)).get(
        "invite",
      );
      if (token) {
        setInvitation(token);
        window.history.replaceState(
          null,
          "",
          window.location.pathname + window.location.search,
        );
      }
    };
    window.addEventListener("hashchange", readInvite);
    return () => window.removeEventListener("hashchange", readInvite);
  }, []);
  const auth = useQuery({
    queryKey: ["session"],
    queryFn: session,
    refetchInterval: 15_000,
  });
  useEffect(() => {
    if (!auth.data) return;
    try {
      const saved = sessionStorage.getItem("recollect-pairing-return");
      if (!saved) return;
      sessionStorage.removeItem("recollect-pairing-return");
      const pending = JSON.parse(saved);
      if (/^[a-f0-9]{8}$/i.test(pending.code) && pending.until > Date.now()) {
        void navigate({ to: "/devices", search: { code: pending.code } });
      }
    } catch {
      /* A browser with storage disabled keeps the current local URL. */
    }
  }, [auth.data, navigate]);
  if (invitation)
    return (
      <Enrollment
        token={invitation}
        cancel={() => setInvitation("")}
        complete={async (data) => {
          await replaceSession(cache, data);
          setInvitation("");
        }}
      />
    );
  if (auth.isPending)
    return (
      <div className="full-center">
        <Loader color="teal" />
        <Text c="dimmed">Opening Recollect…</Text>
      </div>
    );
  if (auth.error)
    return (
      <div className="full-center">
        <Failure error={auth.error} retry={() => void auth.refetch()} />
      </div>
    );
  if (!auth.data) return <Login />;
  return <Workspace user={auth.data} />;
}

function Login() {
  const options = useQuery({
    queryKey: ["auth-options"],
    queryFn: async () => result(await client.GET("/api/auth/options")),
  });
  const organizationError = new URLSearchParams(window.location.search).has(
    "auth_error",
  );
  const cache = useQueryClient();
  const [username, setUsername] = useState("owner");
  const [password, setPassword] = useState("");
  const login = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/auth/login", { body: { username, password } }),
      ),
    onSuccess: async (data) => {
      await replaceSession(cache, data);
      setPassword("");
    },
  });
  return (
    <div className="login-page">
      <section className="login-story">
        <Brand />
        <div>
          <span className="eyebrow">YOUR ENGINEERING MEMORY</span>
          <h1>
            Keep the context.
            <br />
            Build on what
            <br />
            you know.
          </h1>
          <p>
            A home for your team’s knowledge, decisions and the evidence behind
            them.
          </p>
          <div className="story-lines">
            <span />
            <span />
            <span />
          </div>
        </div>
        <Text size="xs" c="#afc1b5">
          Your knowledge. Your infrastructure.
        </Text>
      </section>
      <section className="login-form">
        <div className="form-inner">
          <Badge variant="light" color="teal">
            Local workspace
          </Badge>
          <Title order={1} mt="lg">
            Welcome back
          </Title>
          <Text c="dimmed" mt="xs" mb="xl">
            Sign in to your Recollect installation.
          </Text>
          {organizationError && (
            <Alert color="red" mb="md">
              Organization sign-in was not completed. Check your enrolled
              identity or retry.
            </Alert>
          )}
          <form
            onSubmit={(event: FormEvent) => {
              event.preventDefault();
              login.mutate();
            }}
          >
            <Stack gap="md">
              <TextInput
                label="Username"
                autoComplete="username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                required
              />
              <PasswordInput
                label="Password"
                autoComplete="current-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
              <Failure error={login.error} />
              <Button
                type="submit"
                size="md"
                loading={login.isPending}
                rightSection={<ArrowRight size={16} />}
              >
                Sign in
              </Button>
            </Stack>
          </form>
          {options.data?.oidc_configured && (
            <Button
              component="a"
              href="/api/auth/oidc/start"
              onClick={() => {
                const code = new URLSearchParams(window.location.search).get(
                  "code",
                );
                if (window.location.pathname === "/devices" && code) {
                  try {
                    sessionStorage.setItem(
                      "recollect-pairing-return",
                      JSON.stringify({ code, until: Date.now() + 300_000 }),
                    );
                  } catch {
                    /* The companion still displays the verification link. */
                  }
                }
              }}
              variant="default"
              fullWidth
              mt="md"
            >
              Sign in with your organization
            </Button>
          )}
          <Text size="xs" c="dimmed" mt="xl">
            Use your local account or the owner credentials from setup.
          </Text>
        </div>
      </section>
    </div>
  );
}

function Brand() {
  return (
    <div className="brand">
      <div className="brand-mark">
        <Layers3 size={23} strokeWidth={1.6} />
      </div>
      <span>
        recollect<span className="brand-dot">.</span>
      </span>
    </div>
  );
}

function Workspace({ user }: { user: Session }) {
  const cache = useQueryClient();
  const navigate = useNavigate();
  const path = useRouterState({ select: (state) => state.location.pathname });
  const teamView = path === "/team";
  const devicesView = path === "/devices";
  const brainId = path.startsWith("/brains/") ? path.split("/")[2] : undefined;
  const [creating, setCreating] = useState(false);
  const brains = useQuery({
    queryKey: ["brains"],
    queryFn: async () => result(await client.GET("/api/brains")),
    refetchInterval: 5000,
  });
  const health = useQuery({
    queryKey: ["status"],
    queryFn: async () => result(await client.GET("/api/status")),
    refetchInterval: 30_000,
  });
  const logout = useMutation({
    mutationFn: async () => {
      const response = await client.POST("/api/auth/logout");
      if (response.response.status !== 401) result(response);
    },
    onSuccess: async () => {
      await replaceSession(cache, null);
      void navigate({ to: "/" });
    },
  });
  const refresh = () => {
    void cache.invalidateQueries();
  };
  const expired = [brains.error, health.error].some(
    (error) => error instanceof RequestError && error.status === 401,
  );
  return (
    <div className="workspace">
      <aside className="sidebar">
        <Brand />
        <span className="nav-label">WORKSPACE</span>
        <nav className="workspace-nav" aria-label="Workspace navigation">
          <Link
            to="/"
            className={`nav-item ${!brainId && !teamView && !devicesView ? "selected" : ""}`}
          >
            <Boxes size={18} /> Your Brains{" "}
            <span>{brains.data?.length ?? "—"}</span>
          </Link>
          {user.user.installation_owner && (
            <Link
              to="/team"
              className={`nav-item ${teamView ? "selected" : ""}`}
            >
              <Shield size={18} /> Team
            </Link>
          )}
          <Link
            to="/devices"
            search={{ code: undefined }}
            className={`nav-item ${devicesView ? "selected" : ""}`}
          >
            <Laptop size={18} /> Devices
          </Link>
        </nav>
        <div className="sidebar-note">
          <BookOpen size={19} />
          <Text size="sm" fw={500} c="#dae6dd">
            A place for context
          </Text>
          <Text size="xs" c="#a8bdb0">
            Organize knowledge in Brains. Each has its own members and access.
          </Text>
        </div>
        <div className="sidebar-bottom">
          <div className="user-avatar">
            {user.user.username[0]?.toUpperCase()}
          </div>
          <div>
            <Text c="white" size="sm" fw={500}>
              {user.user.username}
            </Text>
            <Text size="xs" c="#9fb6a7">
              {user.user.installation_owner ? "Installation owner" : "Member"}
            </Text>
          </div>
          <button
            aria-label="Sign out"
            title="Sign out"
            onClick={() => logout.mutate()}
            disabled={logout.isPending}
          >
            <LogOut size={18} />
          </button>
        </div>
      </aside>
      <main className="main">
        <header className="topbar">
          <div>
            <span className="breadcrumb">Workspace</span>
            <span className="crumb-divider">/</span>
            {brainId
              ? "Brain overview"
              : teamView
                ? "Team"
                : devicesView
                  ? "Devices"
                  : "Your Brains"}
          </div>
          <Group gap="sm">
            <span
              className={`status-dot ${health.data?.ready ? "healthy" : ""}`}
            />
            <Text size="xs" c="dimmed">
              {health.isPending
                ? "Checking services"
                : health.data?.ready
                  ? "All services connected"
                  : "Services need attention"}
            </Text>
            <button
              className="icon-button"
              aria-label="Refresh workspace"
              onClick={refresh}
            >
              <RefreshCw size={15} />
            </button>
          </Group>
        </header>
        <div className="content">
          <Failure error={logout.error} />
          {expired ? (
            <Alert color="yellow" title="Your session has expired">
              <Button
                mt="sm"
                variant="light"
                onClick={() => {
                  void replaceSession(cache, null);
                }}
              >
                Sign in again
              </Button>
            </Alert>
          ) : teamView ? (
            <TeamPanel />
          ) : devicesView ? (
            <DevicesPanel />
          ) : brainId ? (
            <BrainDetail id={brainId} actor={user.user.id} />
          ) : (
            <>
              <div className="page-heading">
                <div>
                  <span className="eyebrow">YOUR KNOWLEDGE, CONNECTED</span>
                  <Title order={1}>Your Brains</Title>
                  <Text c="dimmed" mt="xs">
                    A shared home for the things worth remembering.
                  </Text>
                </div>
                <Button
                  leftSection={<Plus size={17} />}
                  onClick={() => setCreating(true)}
                >
                  Create Brain
                </Button>
              </div>
              <div className="intro-banner">
                <div className="intro-icon">
                  <Sparkles size={25} strokeWidth={1.5} />
                </div>
                <div>
                  <Text fw={600}>Good work starts with good context.</Text>
                  <Text c="dimmed" size="sm" mt={4}>
                    Give a project, team or customer a Brain. Keep its knowledge
                    and access in one place.
                  </Text>
                </div>
              </div>
              <Group justify="space-between" mb="lg" mt={36}>
                <Text size="sm" fw={600}>
                  All Brains{" "}
                  <span className="count-pill">{brains.data?.length ?? 0}</span>
                </Text>
                <Text size="xs" c="dimmed">
                  Only Brains you can access
                </Text>
              </Group>
              <Failure
                error={brains.error}
                retry={() => void brains.refetch()}
              />
              {brains.isPending ? (
                <div className="empty-state">
                  <Loader />
                  <Text>Loading your Brains…</Text>
                </div>
              ) : brains.data?.length === 0 ? (
                <Card className="empty-state" withBorder>
                  <div className="empty-icon">
                    <Boxes size={30} strokeWidth={1.5} />
                  </div>
                  <Title order={3}>Room for your first idea</Title>
                  <Text c="dimmed" size="sm" maw={340} ta="center">
                    Create a Brain for your personal knowledge, a project or
                    your team.
                  </Text>
                  <Button
                    variant="light"
                    leftSection={<Plus size={16} />}
                    onClick={() => setCreating(true)}
                  >
                    Create your first Brain
                  </Button>
                </Card>
              ) : (
                <div className="brain-grid">
                  {brains.data?.map((brain) => (
                    <Link
                      to="/brains/$brainId"
                      params={{ brainId: brain.id }}
                      key={brain.id}
                      className="brain-link"
                    >
                      <Card withBorder padding="lg" className="brain-card">
                        <Group justify="space-between">
                          <div className="brain-symbol">
                            <Layers3 size={24} strokeWidth={1.5} />
                          </div>
                          <Badge
                            variant="light"
                            color={brain.archived ? "gray" : "teal"}
                          >
                            {brain.archived ? "Archived" : "Active"}
                          </Badge>
                        </Group>
                        <Title order={3} mt="lg">
                          {brain.name}
                        </Title>
                        <Text
                          c="dimmed"
                          size="sm"
                          mt="xs"
                          lineClamp={2}
                          mih={42}
                        >
                          {brain.description ||
                            "A space for knowledge and context."}
                        </Text>
                        <Divider mt="lg" mb="md" />
                        <Group justify="space-between">
                          <Group gap={6}>
                            <Shield size={13} />
                            <Text size="xs" c="dimmed">
                              {brain.owner_id === user.user.id
                                ? "Owner"
                                : brain.role}
                            </Text>
                          </Group>
                          <ArrowRight size={16} />
                        </Group>
                      </Card>
                    </Link>
                  ))}
                </div>
              )}
              <section className="services">
                <Group justify="space-between" mb="md">
                  <Text fw={600} size="sm">
                    Installation health
                  </Text>
                  <Group gap="xs">
                    {user.user.installation_owner && <OperationsDialog />}
                    <Text c="dimmed" size="xs">Checked through live queries</Text>
                  </Group>
                </Group>
                <Failure
                  error={health.error}
                  retry={() => void health.refetch()}
                />
                <div className="service-grid">
                  {health.data?.dependencies.map((service) => (
                    <div className="service" key={service.name}>
                      <Database size={19} />
                      <div>
                        <Text size="sm" fw={500}>
                          {service.name}
                        </Text>
                        <Text size="xs" c="dimmed">
                          {service.detail}
                        </Text>
                      </div>
                      <span
                        className={`status-dot ${service.connected ? "healthy" : ""}`}
                        title={service.connected ? "Connected" : "Unavailable"}
                      />
                    </div>
                  ))}
                </div>
              </section>
            </>
          )}
        </div>
        <footer className="footer">
          <span>Recollect · Engineering memory</span>
          <span>Local development</span>
        </footer>
      </main>
      <BrainForm
        opened={creating}
        close={() => setCreating(false)}
        saved={(brain) => {
          setCreating(false);
          void cache.invalidateQueries({ queryKey: ["brains"] });
          void navigate({
            to: "/brains/$brainId",
            params: { brainId: brain.id },
          });
        }}
      />
    </div>
  );
}

function BrainForm({
  opened,
  close,
  saved,
  brain,
}: {
  opened: boolean;
  close: () => void;
  saved: (brain: Brain) => void;
  brain?: Brain;
}) {
  const command = useIdempotency();
  const [name, setName] = useState(brain?.name ?? "");
  const [description, setDescription] = useState(brain?.description ?? "");
  const save = useMutation({
    mutationFn: async () =>
      brain
        ? result(
            await client.PATCH("/api/brains/{id}", {
              params: { path: { id: brain.id } },
              body: { name, description },
              headers: {
                "Idempotency-Key": command.forInput({ name, description }),
              },
            }),
          )
        : result(
            await client.POST("/api/brains", {
              body: { name, description },
              headers: {
                "Idempotency-Key": command.forInput({ name, description }),
              },
            }),
          ),
    onSuccess: (data) => {
      command.reset();
      saved(data);
      if (!brain) {
        setName("");
        setDescription("");
      }
    },
  });
  return (
    <Modal
      opened={opened}
      onClose={close}
      title={brain ? "Edit Brain" : "Create a Brain"}
      centered
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          save.mutate();
        }}
      >
        <Stack>
          <Text c="dimmed" size="sm">
            Choose a name that makes this space easy to recognize.
          </Text>
          <TextInput
            label="Name"
            placeholder="e.g. Platform engineering"
            required
            maxLength={120}
            value={name}
            onChange={(event) => setName(event.target.value)}
            data-autofocus
          />
          <Textarea
            label="Description"
            placeholder="What belongs in this Brain?"
            maxLength={2000}
            minRows={3}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
          <Failure error={save.error} />
          <Group justify="flex-end">
            <Button variant="default" onClick={close}>
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={!name.trim()}
              loading={save.isPending}
            >
              {brain ? "Save changes" : "Create Brain"}
            </Button>
          </Group>
        </Stack>
      </form>
    </Modal>
  );
}

function BrainDetail({ id, actor }: { id: string; actor: string }) {
  const cache = useQueryClient();
  const [editing, setEditing] = useState(false);
  const brain = useQuery({
    queryKey: ["brain", id],
    refetchInterval: 5000,
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{id}", { params: { path: { id } } }),
      ),
  });
  const audit = useQuery({
    queryKey: ["audit", id],
    enabled: brain.data?.role === "admin",
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{id}/audit", {
          params: { path: { id } },
        }),
      ),
  });
  const refresh = () => {
    void cache.invalidateQueries({ queryKey: ["brain", id] });
    void cache.invalidateQueries({ queryKey: ["brains"] });
    void cache.invalidateQueries({ queryKey: ["audit", id] });
    void cache.invalidateQueries({ queryKey: ["jobs", id] });
    void cache.invalidateQueries({ queryKey: ["processing", id] });
  };
  const archiveCommand = useIdempotency();
  const archive = useMutation({
    mutationFn: async () =>
      result(
        await client.PATCH("/api/brains/{id}", {
          params: { path: { id } },
          body: { archived: !brain.data?.archived },
          headers: {
            "Idempotency-Key": archiveCommand.forInput({
              archived: !brain.data?.archived,
            }),
          },
        }),
      ),
    onSuccess: () => {
      archiveCommand.reset();
      refresh();
    },
  });
  if (brain.isPending)
    return (
      <div className="empty-state">
        <Loader />
        <Text>Opening Brain…</Text>
      </div>
    );
  if (brain.error)
    return (
      <Stack>
        <Link to="/" className="back-link">
          <ArrowLeft size={15} /> Back to Brains
        </Link>
        <Failure error={brain.error} retry={() => void brain.refetch()} />
      </Stack>
    );
  if (!brain.data) return null;
  const data = brain.data;
  return (
    <>
      <Link to="/" className="back-link">
        <ArrowLeft size={15} /> Back to Brains
      </Link>
      <div className="page-heading detail-heading">
        <div>
          <Group gap="sm" mb="sm">
            <span className="eyebrow">BRAIN OVERVIEW</span>
            <Badge variant="light" color={data.archived ? "gray" : "teal"}>
              {data.archived ? "Archived" : "Active"}
            </Badge>
          </Group>
          <Title order={1}>{data.name}</Title>
          <Text c="dimmed" mt="sm">
            {data.description || "Your space for knowledge and context."}
          </Text>
        </div>
        {data.role === "admin" && (
          <Button
            variant="default"
            leftSection={<Settings2 size={16} />}
            onClick={() => setEditing(true)}
          >
            Edit Brain
          </Button>
        )}
      </div>
      {data.archived && (
        <Alert color="yellow" mb="lg" title="This Brain is archived">
          Its content and history are preserved. Reopen it to continue using
          this space.
        </Alert>
      )}
      <div className="detail-grid">
        <EvidencePanel brain={data} />
        <Card withBorder p="xl">
          <Group gap="sm" mb="lg">
            <Shield size={19} />
            <Text fw={600}>Brain access</Text>
          </Group>
          <Text size="sm">
            Your role: <strong>{data.role}</strong>
          </Text>
          <Text size="xs" c="dimmed" mt="xs">
            Access applies only to this Brain.
          </Text>
          {data.role === "admin" && <AccessPanel brain={data} actor={actor} />}
          <Divider my="lg" />
          <Text size="xs" c="dimmed">
            Created {new Date(data.created_at).toLocaleDateString()}
          </Text>
          {data.role === "admin" && (
            <Button
              mt="lg"
              color={data.archived ? "teal" : "gray"}
              variant="light"
              leftSection={
                data.archived ? <RefreshCw size={15} /> : <Archive size={15} />
              }
              loading={archive.isPending}
              onClick={() => archive.mutate()}
            >
              {data.archived ? "Reopen Brain" : "Archive Brain"}
            </Button>
          )}
          <Failure error={archive.error} />
        </Card>
      </div>
      <RecallPanel key={data.id} brain={data} />
      <GraphPanel key={`graph-${data.id}`} brain={data} />
      <WorkspacePanel brain={data} />
      <McpPanel key={`mcp-${data.id}`} brain={data} actor={actor} />
      <PublicationPanel brain={data} />
      <CapturePanel brain={data} />
      <ClaimsPanel brain={data} />
      <ModelsPanel brain={data} />
      <HandoversPanel brain={data} />
      <RetentionPanel brain={data} />
      <JobsPanel id={id} admin={data.role === "admin"} />
      {data.role === "admin" && (
        <section className="audit-section">
          <Text fw={600} mb="lg">
            Activity
          </Text>
          <Failure error={audit.error} retry={() => void audit.refetch()} />
          {audit.isPending ? (
            <Loader size="sm" />
          ) : audit.data?.length === 0 ? (
            <Text c="dimmed" size="sm">
              No activity yet.
            </Text>
          ) : (
            <Card withBorder p="lg">
              {audit.data?.map((event) => (
                <div className="audit-row" key={event.id}>
                  <div className="audit-icon">
                    <Check size={14} />
                  </div>
                  <div>
                    <Text size="sm" fw={500}>
                      {event.action === "brain.create"
                        ? "Brain created"
                        : event.action === "brain.update"
                          ? "Brain updated"
                          : event.action === "projection.refresh"
                            ? "Brain views updated"
                            : event.action === "job.cancel"
                              ? "Background job cancelled"
                              : event.action === "job.retry"
                                ? "Background job retried"
                                : (activityLabels[event.action] ??
                                  event.action)}
                    </Text>
                    <Text size="xs" c="dimmed">
                      {event.disposition.replaceAll("_", " ")} ·{" "}
                      {event.actor_id === data.owner_id
                        ? "Brain owner"
                        : "Member"}
                    </Text>
                  </div>
                  <Text size="xs" c="dimmed" ml="auto">
                    {new Date(event.created_at).toLocaleString()}
                  </Text>
                </div>
              ))}
            </Card>
          )}
        </section>
      )}
      <BrainForm
        key={data.updated_at}
        opened={editing}
        close={() => setEditing(false)}
        brain={data}
        saved={() => {
          setEditing(false);
          refresh();
        }}
      />
    </>
  );
}
