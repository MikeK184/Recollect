import { BrainIcon } from "./components/BrainIcon";
import { useEffect, useState, type FormEvent } from "react";
import {
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  PasswordInput,
  Stack,
  Text,
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
  ChevronDown,
  LogOut,
  RefreshCw,
  Users,
} from "lucide-react";
import { Enrollment, initialInvitation } from "./Enrollment";
import {
  client,
  result,
  session,
  setCsrf,
  RequestError,
  type Session,
} from "./api";
import { Brand } from "./components/Brand";
import { ErrorState as Failure } from "./components/AsyncState";
import { PageTransition } from "./components/PageTransition";
import { StatusDot } from "./components/StatusDot";
import {
  brainNavigation,
  brainTiers,
  globalNavigation,
} from "./app/navigation";
import { WorkspaceContext } from "./app/context";
import { OperationsDialog } from "./OperationsDialog";

async function replaceSession(cache: QueryClient, value: Session | null) {
  await cache.cancelQueries();
  cache.removeQueries({
    predicate: (query) => query.queryKey[0] !== "session",
  });
  setCsrf(value?.csrf_token ?? "");
  cache.setQueryData(["session"], value);
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
        <Text size="xs" c="dimmed">
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
                if (
                  ["/devices", "/agents"].includes(window.location.pathname) &&
                  code
                ) {
                  try {
                    sessionStorage.setItem(
                      "recollect-pairing-return",
                      JSON.stringify({ code, until: Date.now() + 300_000 }),
                    );
                  } catch {
                    /* The host CLI still displays the verification link. */
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

function Workspace({ user }: { user: Session }) {
  const cache = useQueryClient();
  const navigate = useNavigate();
  const path = useRouterState({ select: (state) => state.location.pathname });
  const brainId = path.startsWith("/brains/") ? path.split("/")[2] : undefined;
  const section = path.split("/")[3] || "dashboard";
  const [lastBrain, setLastBrain] = useState<{
    actor: string;
    id: string;
  } | null>(null);
  const brains = useQuery({
    queryKey: ["brains"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/brains", { signal })),
    refetchInterval: 5000,
  });
  const health = useQuery({
    queryKey: ["status"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/status", { signal })),
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
  const visibleBrains = brains.isError ? [] : (brains.data ?? []);
  const current = visibleBrains.find((b) => b.id === brainId);
  useEffect(() => {
    if (brainId && current && !current.archived && !brains.error)
      setLastBrain({ actor: user.user.id, id: brainId });
    else if (
      brains.error ||
      (lastBrain &&
        (lastBrain.actor !== user.user.id ||
          !visibleBrains.some((b) => b.id === lastBrain.id && !b.archived)))
    )
      setLastBrain(null);
  }, [
    brainId,
    current?.id,
    current?.archived,
    brains.error,
    user.user.id,
    lastBrain?.actor,
    lastBrain?.id,
    visibleBrains,
  ]);
  const retainedBrain =
    path === "/connectors" && lastBrain?.actor === user.user.id && !brains.error
      ? visibleBrains.find((b) => b.id === lastBrain.id && !b.archived)
      : undefined;
  const navigationBrain = brainId ? current : retainedBrain;
  const navigationBrainId = brainId ?? retainedBrain?.id;
  const navigationSection = brainId ? section : undefined;

  const selectedLabel = brainId
    ? section === "tv"
      ? "Ambient display"
      : (brainNavigation.find((n) => n.section === section)?.label ?? "Ask")
    : (globalNavigation.find((n) => n.to === path)?.label ?? "Workspace");
  const expired = [brains.error, health.error].some(
    (e) => e instanceof RequestError && e.status === 401,
  );
  useEffect(() => {
    if (expired) setLastBrain(null);
  }, [expired]);
  useEffect(() => {
    document.title = `${current ? `${current.name} · ` : ""}${selectedLabel} · Recollect`;
  }, [current?.name, selectedLabel]);
  return (
    <WorkspaceContext.Provider value={user}>
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <div className="workspace">
        <aside className="sidebar rc-enter">
          <Link to="/" className="brand-link" aria-label="Recollect home">
            <Brand />
          </Link>
          {navigationBrainId ? (
            <>
              <nav
                aria-label="Workspace navigation"
                className="workspace-nav global-nav brain-global-nav"
              >
                {globalNavigation
                  .filter(
                    (n) =>
                      n.to !== "/team" &&
                      (n.to !== "/connectors" || user.user.installation_owner),
                  )
                  .map(({ to, label, icon: Icon }) => (
                    <Link
                      key={to}
                      to={to}
                      search={{}}
                      className={`nav-item ${path === to ? "selected" : ""}`}
                      aria-current={path === to ? "page" : undefined}
                    >
                      <Icon size={18} />
                      <span>{label}</span>
                    </Link>
                  ))}
              </nav>
              <div
                className={`brain-switcher ${navigationBrain?.icon_revision ? "has-artwork" : ""}`}
              >
                {navigationBrain?.icon_revision && (
                  <BrainIcon
                    id={navigationBrain.id}
                    revision={navigationBrain.icon_revision}
                    size={28}
                  />
                )}
                <label htmlFor="brain-switcher" className="sr-only">
                  Switch Brain
                </label>
                <select
                  id="brain-switcher"
                  value={navigationBrainId}
                  onChange={(e) => {
                    void navigate({
                      to: "/brains/$brainId/dashboard",
                      params: { brainId: e.currentTarget.value },
                      search: {},
                    });
                  }}
                >
                  {!navigationBrain && (
                    <option value={navigationBrainId}>
                      {brains.isPending ? "Opening Brain…" : "Current Brain"}
                    </option>
                  )}
                  {visibleBrains.map((b) => (
                    <option key={b.id} value={b.id}>
                      {b.name}
                      {b.archived ? " (archived)" : ""}
                    </option>
                  ))}
                </select>
                <ChevronDown size={15} aria-hidden="true" />
                <span className="brain-switcher-meta">
                  {navigationBrain?.archived
                    ? "Archived Brain"
                    : navigationBrain
                      ? `${navigationBrain.role} access`
                      : "Knowledge workspace"}
                </span>
              </div>
              <nav aria-label="Brain navigation" className="workspace-nav">
                {brainTiers.map((tier) =>
                  tier === "Manage" ? (
                    <details
                      key={tier}
                      className="nav-group nav-management"
                      open={
                        !brainId ||
                        navigationSection === "agents" ||
                        navigationSection === "connections" ||
                        navigationSection === "settings"
                      }
                      role="group"
                      aria-labelledby={`nav-tier-${tier.toLowerCase()}`}
                    >
                      <summary
                        className="nav-label"
                        id={`nav-tier-${tier.toLowerCase()}`}
                      >
                        <span>{tier}</span>
                        <ChevronDown size={14} aria-hidden="true" />
                      </summary>
                      {brainNavigation
                        .filter((n) => n.tier === tier && !n.hidden)
                        .map(({ section: value, label, icon: Icon }) => (
                          <Link
                            key={value}
                            to={`/brains/$brainId/${value}`}
                            params={{ brainId: navigationBrainId }}
                            search={{}}
                            className={`nav-item ${navigationSection === value ? "selected" : ""}`}
                            aria-current={
                              navigationSection === value ? "page" : undefined
                            }
                          >
                            <Icon size={18} />
                            <span>{label}</span>
                          </Link>
                        ))}
                    </details>
                  ) : (
                    <div
                      key={tier}
                      className="nav-group"
                      role="group"
                      aria-labelledby={`nav-tier-${tier.toLowerCase()}`}
                    >
                      <span
                        className="nav-label"
                        id={`nav-tier-${tier.toLowerCase()}`}
                      >
                        {tier}
                      </span>
                      {brainNavigation
                        .filter((n) => n.tier === tier && !n.hidden)
                        .map(({ section: value, label, icon: Icon }) => (
                          <Link
                            key={value}
                            to={`/brains/$brainId/${value}`}
                            params={{ brainId: navigationBrainId }}
                            search={{}}
                            className={`nav-item ${navigationSection === value || (value === "explore" && ["memory", "sources", "repositories"].includes(navigationSection ?? "")) ? "selected" : ""}`}
                            aria-current={
                              navigationSection === value ||
                              (value === "explore" &&
                                ["memory", "sources", "repositories"].includes(
                                  section,
                                ))
                                ? "page"
                                : undefined
                            }
                          >
                            <Icon size={18} />
                            <span>{label}</span>
                          </Link>
                        ))}
                    </div>
                  ),
                )}
              </nav>
            </>
          ) : (
            <nav
              aria-label="Workspace navigation"
              className="workspace-nav global-nav"
            >
              <span className="nav-label">Workspace</span>
              {globalNavigation
                .filter(
                  (n) =>
                    n.to !== "/team" &&
                    (n.to !== "/connectors" || user.user.installation_owner),
                )
                .map(({ to, label, icon: Icon }) => (
                  <Link
                    key={to}
                    to={to}
                    search={{}}
                    className={`nav-item ${path === to ? "selected" : ""}`}
                    aria-current={path === to ? "page" : undefined}
                  >
                    <Icon size={18} />
                    <span>{label}</span>
                    {to === "/" && <small>{visibleBrains.length}</small>}
                  </Link>
                ))}
            </nav>
          )}
          {user.user.installation_owner && (
            <nav
              className="sidebar-utilities"
              aria-label="Installation navigation"
            >
              <Link
                to="/team"
                className={`nav-item ${path === "/team" ? "selected" : ""}`}
                aria-current={path === "/team" ? "page" : undefined}
              >
                <Users size={18} />
                <span>Team</span>
              </Link>
            </nav>
          )}
          <div className="sidebar-bottom">
            <span className="user-avatar">
              {user.user.username[0]?.toUpperCase()}
            </span>
            <div className="account-name">
              <Text size="sm" fw={600}>
                {user.user.username}
              </Text>
              <Text size="xs" c="dimmed">
                {user.user.installation_owner ? "Installation owner" : "Member"}
              </Text>
            </div>
            <button
              className="icon-button"
              aria-label="Sign out"
              title="Sign out"
              onClick={() => logout.mutate()}
              disabled={logout.isPending}
            >
              <LogOut size={18} />
            </button>
          </div>
        </aside>
        <main className="main" id="main-content" tabIndex={-1}>
          <header className="topbar rc-enter">
            <div className="breadcrumb">
              <Link to="/">
                {path === "/connectors"
                  ? "Installation"
                  : (current?.name ?? "Workspace")}
              </Link>
              <span aria-hidden="true">/</span>
              <strong>{selectedLabel}</strong>
            </div>
            <Group gap="sm" wrap="nowrap">
              <Group
                gap="sm"
                wrap="nowrap"
                className={
                  health.isError || (health.data && !health.data.ready)
                    ? undefined
                    : "healthy-service-status"
                }
              >
                <StatusDot
                  tone={
                    !health.isError && health.data?.ready ? "accent" : "idle"
                  }
                  live={false}
                  size={7}
                />
                <Text size="xs" c="dimmed">
                  {health.isPending
                    ? "Checking services"
                    : health.isError
                      ? "Health unavailable"
                      : health.data?.ready
                        ? "Core stores reachable"
                        : "Services need attention"}
                </Text>
                <button
                  className="icon-button"
                  aria-label="Refresh service status"
                  onClick={() => void health.refetch()}
                >
                  <RefreshCw size={15} />
                </button>
              </Group>
              {user.user.installation_owner && <OperationsDialog />}
            </Group>
          </header>
          <div className="content">
            <Failure error={logout.error} />
            {expired ? (
              <Alert color="yellow" title="Your session has expired">
                <Button
                  mt="sm"
                  variant="light"
                  onClick={() => void replaceSession(cache, null)}
                >
                  Sign in again
                </Button>
              </Alert>
            ) : (
              <PageTransition />
            )}
          </div>
          <footer className="footer">
            <span>Recollect · A place for what you know.</span>
            <span>Private by design.</span>
          </footer>
        </main>
      </div>
    </WorkspaceContext.Provider>
  );
}
