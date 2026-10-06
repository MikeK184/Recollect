import { useState } from "react";
import {
  Alert,
  ActionIcon,
  Badge,
  Button,
  Group,
  Modal,
  Menu,
  Tabs,
  Select,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Box,
  FileText,
  FileUp,
  Upload,
  Link2,
  Monitor,
  MoreHorizontal,
  Plus,
  Search,
  ShieldCheck,
  Unplug,
} from "lucide-react";
import { client, result } from "../../api";
import { useWorkspace } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
import { McpConnectorIcon } from "../../components/McpConnectorIcon";
import { CodeBlock } from "../../components/CodeBlock";
import { Markdown } from "../../components/Markdown";
import "./inline-management.css";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
import { McpConnectorSetup } from "../../McpConnectorSetup";

export function ConnectorsPage() {
  const session = useWorkspace();
  const cache = useQueryClient();
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState("approved");
  const [transport, setTransport] = useState<string | null>(null);
  const [addMethod, setAddMethod] = useState("config");
  const [adding, setAdding] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const query = useQuery({
    queryKey: ["global-connectors"],
    enabled: session.user.installation_owner,
    gcTime: 0,
    retry: false,
    refetchInterval: 5000,
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/mcp/definitions", { signal })),
  });
  const detail = useQuery({
    queryKey: ["global-connectors", selected],
    enabled: !!selected && session.user.installation_owner && !query.error,
    gcTime: 0,
    retry: false,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/mcp/definitions/{key}", {
          params: { path: { key: selected! } },
          signal,
        }),
      ),
  });
  if (!session.user.installation_owner)
    return (
      <Alert color="gray" title="Installation owner access">
        The installation owner manages reusable connector definitions.
      </Alert>
    );
  const definitions = query.isError ? [] : (query.data ?? []);
  const matches = definitions.filter(
    (d) =>
      (filter === "all" || (filter === "approved") === d.enabled) &&
      (!transport || d.transport === transport) &&
      (d.name + " " + d.description)
        .toLowerCase()
        .includes(search.trim().toLowerCase()),
  );
  return (
    <div className="connectors-page">
      <div className="connector-library-layout">
        <div className="connector-primary">
          <PageHeader
            title="Connectors"
            description="Approve reusable MCP connectors once, then use them across Brains."
            actions={
              <Button
                color="green"
                leftSection={<Plus size={21} />}
                onClick={() => {
                  setAddMethod("url");
                  setAdding(true);
                }}
              >
                Add connector
              </Button>
            }
          />
          <div className="connector-library-toolbar">
            <Tabs value={filter} onChange={(v) => setFilter(v ?? "approved")}>
              <Tabs.List>
                <Tabs.Tab value="approved">
                  Approved ({definitions.filter((d) => d.enabled).length})
                </Tabs.Tab>
                <Tabs.Tab value="disabled">
                  Disabled ({definitions.filter((d) => !d.enabled).length})
                </Tabs.Tab>
                <Tabs.Tab value="all">All ({definitions.length})</Tabs.Tab>
              </Tabs.List>
            </Tabs>
            <TextInput
              aria-label="Find a connector"
              placeholder="Find a connector…"
              value={search}
              onChange={(e) => setSearch(e.currentTarget.value)}
              leftSection={<Search size={18} />}
            />
            <Select
              aria-label="Transport"
              placeholder="All transports"
              clearable
              value={transport}
              onChange={setTransport}
              data={[
                { value: "streamable_http", label: "HTTP" },
                { value: "stdio", label: "Local" },
              ]}
            />
          </div>
          {query.isPending ? (
            <LoadingState label="Loading connectors…" />
          ) : query.error ? (
            <ErrorState error={query.error} />
          ) : !matches.length ? (
            <EmptyState
              icon={Unplug}
              title={
                definitions.length
                  ? "No matching connectors"
                  : "Your connector library starts here"
              }
              description={
                definitions.length
                  ? "Clear your search or change the approval filter to find an existing connector."
                  : "Add a server address or import a configuration to review your first connector."
              }
              action={
                definitions.length ? (
                  <Button
                    variant="default"
                    onClick={() => {
                      setSearch("");
                      setFilter("all");
                      setTransport(null);
                    }}
                  >
                    Clear filters
                  </Button>
                ) : undefined
              }
            />
          ) : (
            <div className="connector-library">
              {matches.map((d) => (
                <article
                  className="management-surface connector-tile"
                  key={d.key}
                >
                  <Menu position="bottom-end" withinPortal>
                    <Menu.Target>
                      <ActionIcon
                        className="connector-tile-actions"
                        variant="subtle"
                        size={26}
                        aria-label={"Actions for connector " + d.name}
                      >
                        <MoreHorizontal size={20} />
                      </ActionIcon>
                    </Menu.Target>
                    <Menu.Dropdown>
                      <Menu.Item
                        leftSection={<FileText size={16} />}
                        onClick={() => setSelected(d.key)}
                      >
                        View definition
                      </Menu.Item>
                    </Menu.Dropdown>
                  </Menu>
                  <header>
                    <McpConnectorIcon definition={d} />
                    <div>
                      <h3>{d.name}</h3>
                      <Badge
                        variant="light"
                        color={d.enabled ? "brand" : "gray"}
                      >
                        {d.enabled ? "Definition approved" : "Disabled"}
                      </Badge>
                    </div>
                  </header>
                  <p className="management-muted">
                    {d.description || "Approved MCP connector metadata."}
                  </p>
                  <span className="connector-transport-pill">
                    {d.transport === "stdio" ? (
                      <Monitor size={18} />
                    ) : (
                      <Link2 size={18} />
                    )}{" "}
                    {d.transport === "stdio" ? "Local" : "HTTP"}
                  </span>
                  <footer>
                    <Button
                      variant="subtle"
                      size="sm"
                      onClick={() => setSelected(d.key)}
                    >
                      View definition →
                    </Button>
                  </footer>
                </article>
              ))}
            </div>
          )}
        </div>
        <aside className="management-surface connector-context-panel">
          <h2 className="management-title">Add a connector</h2>
          <div className="connector-import-illustration">
            <FileUp size={56} />
          </div>
          <p className="management-muted">
            Import or paste a connector definition to make it available across
            all Brains.
          </p>
          <Button
            color="green"
            leftSection={<Upload size={21} />}
            onClick={() => {
              setAddMethod("config");
              setAdding(true);
            }}
          >
            Import definition
          </Button>
          <Button
            variant="default"
            leftSection={<FileText size={21} />}
            onClick={() => {
              setAddMethod("config");
              setAdding(true);
            }}
          >
            Paste configuration
          </Button>
          <p className="management-muted connector-review-explainer">
            Review transport, allowed targets, tools and credential
            requirements.
          </p>
          <ol className="connector-import-steps">
            <li>
              <strong>Import</strong>
              <span>Import or paste an MCP connector definition.</span>
            </li>
            <li>
              <strong>Review</strong>
              <span>
                Check transport, allowed targets, tools and credential
                requirements.
              </span>
            </li>
            <li>
              <strong>Approve</strong>
              <span>Make the definition available across all Brains.</span>
            </li>
          </ol>
        </aside>
      </div>
      <div className="connector-library-note">
        <ShieldCheck size={30} />
        <span>
          <strong>
            Connector approval controls what can be configured. Brain access
            controls who can use it.
          </strong>
          <small>
            Approved connectors are reusable definitions. Each Brain controls
            which approved connectors its agents can use.
          </small>
        </span>
      </div>
      {adding && !query.error && (
        <McpConnectorSetup
          initialMethod={addMethod}
          close={() => setAdding(false)}
          saved={() => {
            setAdding(false);
            void cache.invalidateQueries({ queryKey: ["global-connectors"] });
          }}
        />
      )}
      <Modal
        opened={!!selected && !query.error}
        onClose={() => setSelected(null)}
        title="Connector details"
        size="lg"
      >
        {detail.isPending ? (
          <LoadingState />
        ) : detail.error ? (
          <ErrorState error={detail.error} />
        ) : (
          detail.data && (
            <Stack>
              <Group gap="md">
                <McpConnectorIcon definition={detail.data.summary} />
                <h2 className="management-title">{detail.data.summary.name}</h2>
              </Group>
              <Text size="sm">{detail.data.summary.description}</Text>
              <div className="connection-setup-summary">
                <div>
                  <small>Transport</small>
                  <strong>{detail.data.manifest.transport}</strong>
                </div>
                <div>
                  <small>Tools</small>
                  <strong>{detail.data.manifest.tools.length}</strong>
                </div>
                <div>
                  <small>Runs on</small>
                  <strong>{detail.data.manifest.placements.join(" · ")}</strong>
                </div>
              </div>
              {detail.data.manifest.command && (
                <Text size="sm">
                  Command: {detail.data.manifest.command}{" "}
                  {detail.data.manifest.arguments?.join(" ")}
                </Text>
              )}
              <Text size="sm">
                Credential aliases:{" "}
                {detail.data.manifest.credential_aliases?.join(" · ") || "None"}
              </Text>
              {detail.data.manifest.tools.map((t) => (
                <div key={t.name} className="management-row">
                  <div className="management-row-main">
                    <strong>{t.name}</strong>
                    <p className="management-muted">
                      {(t.description ?? "").slice(0, 180)}
                      {(t.description ?? "").length > 180 ? "…" : ""}
                    </p>
                    {!!t.description && (
                      <details className="tool-description-details">
                        <summary>Full description</summary>
                        <Markdown text={t.description ?? ""} />
                      </details>
                    )}
                    <details>
                      <summary>Input schema</summary>
                      <CodeBlock
                        language="json"
                        code={JSON.stringify(t.inputSchema, null, 2)}
                      />
                    </details>
                  </div>
                </div>
              ))}
              <Text size="xs" c="dimmed">
                To use this connector, open a Brain’s Connections page and
                choose Add connection.
              </Text>
            </Stack>
          )
        )}
      </Modal>
    </div>
  );
}
