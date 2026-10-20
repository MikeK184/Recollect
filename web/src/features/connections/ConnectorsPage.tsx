import { useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ChevronDown,
  FileText,
  Globe,
  Monitor,
  MoreHorizontal,
  Plus,
  ShieldCheck,
  ShieldAlert,
  Unplug,
  Upload,
} from "lucide-react";
import { client, result } from "../../api";
import { useWorkspace } from "../../app/context";
import { Page } from "../../components/Page";
import { PageHeader } from "../../components/PageHeader";
import { McpConnectorIcon } from "../../components/McpConnectorIcon";
import { CodeBlock } from "../../components/CodeBlock";
import { Markdown } from "../../components/Markdown";
import { staggerStyle } from "../../components/Motion";
import { SkeletonRows } from "../../components/Skeleton";
import {
  DetailInspector,
  InspectorSection,
  PropList,
} from "../../components/Inspector";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";
import {
  Alert,
  Button,
  Card,
  CardDescription,
  CardTitle,
  Count,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  IconButton,
  NativeSelect,
  SearchInput,
  StatusBadge,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableWrap,
  Tabs,
  TabsList,
  TabsTrigger,
} from "../../components/ui";
import { cn } from "../../lib/utils";
import { McpConnectorSetup } from "../../McpConnectorSetup";
import { usePollInterval } from "../../usePollInterval";

const steps = [
  { title: "Import", detail: "Import or paste an MCP connector definition." },
  {
    title: "Review",
    detail:
      "Check transport, allowed targets, tools and credential requirements.",
  },
  {
    title: "Approve",
    detail: "Make the definition available across all Brains.",
  },
] as const;

function Transport({ value }: { value: string }) {
  const local = value === "stdio";
  return (
    <span className="inline-flex items-center gap-1.5 text-foreground-secondary">
      {local ? (
        <Monitor
          className="size-3.5 text-muted-foreground"
          aria-hidden="true"
        />
      ) : (
        <Globe className="size-3.5 text-muted-foreground" aria-hidden="true" />
      )}
      <span className="mono text-label">{local ? "Local" : "HTTP"}</span>
    </span>
  );
}

export function ConnectorsPage() {
  const pollNormal = usePollInterval("normal");
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
    refetchInterval: pollNormal,
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
  // Rows on the first successful read stagger in; polling never replays it.
  const firstLoad = useRef<Set<string> | null>(null);
  if (!firstLoad.current && query.data)
    firstLoad.current = new Set(query.data.map((d) => d.key));
  if (!session.user.installation_owner)
    return (
      <Page className="text-body">
        <Alert
          variant="default"
          role="status"
          icon={<ShieldAlert aria-hidden="true" />}
          title="Installation owner access"
        >
          The installation owner manages reusable connector definitions.
        </Alert>
      </Page>
    );
  const definitions = query.isError ? [] : (query.data ?? []);
  const approvedCount = definitions.filter((d) => d.enabled).length;
  const matches = definitions.filter(
    (d) =>
      (filter === "all" || (filter === "approved") === d.enabled) &&
      (!transport || d.transport === transport) &&
      (d.name + " " + d.description)
        .toLowerCase()
        .includes(search.trim().toLowerCase()),
  );
  const add = (method: string) => {
    setAddMethod(method);
    setAdding(true);
  };
  return (
    <Page className="text-body">
      <PageHeader
        title="Connectors"
        description="Approve reusable MCP connectors once, then use them across Brains."
        actions={
          <Button onClick={() => add("url")}>
            <Plus aria-hidden="true" />
            Add connector
          </Button>
        }
      />
      {/* The add-a-connector rail sits beside the library when the panel is
          wide enough and drops below it when the docked inspector narrows it. */}
      <div className="@container">
        <div className="grid items-start gap-6 @min-[960px]:grid-cols-[minmax(0,1fr)_340px] @min-[1100px]:grid-cols-[minmax(0,1fr)_360px]">
          <div className="flex min-w-0 flex-col gap-4">
            <Tabs value={filter} onValueChange={setFilter}>
              <TabsList aria-label="Approval status">
                <TabsTrigger value="approved">
                  Approved
                  {query.data && (
                    <Count aria-hidden="true">{approvedCount}</Count>
                  )}
                </TabsTrigger>
                <TabsTrigger value="disabled">
                  Disabled
                  {query.data && (
                    <Count aria-hidden="true">
                      {definitions.length - approvedCount}
                    </Count>
                  )}
                </TabsTrigger>
                <TabsTrigger value="all">
                  All
                  {query.data && (
                    <Count aria-hidden="true">{definitions.length}</Count>
                  )}
                </TabsTrigger>
              </TabsList>
            </Tabs>
            <div className="flex items-center gap-2">
              <div className="w-72">
                <SearchInput
                  aria-label="Find a connector"
                  placeholder="Find a connector"
                  value={search}
                  onChange={(e) => setSearch(e.currentTarget.value)}
                />
              </div>
              <div className="w-44">
                <NativeSelect
                  aria-label="Transport"
                  value={transport ?? ""}
                  onChange={(e) => setTransport(e.currentTarget.value || null)}
                >
                  <option value="">All transports</option>
                  <option value="streamable_http">HTTP</option>
                  <option value="stdio">Local</option>
                </NativeSelect>
              </div>
              {query.data && (
                <span className="mono ml-auto whitespace-nowrap text-label text-muted-foreground">
                  {matches.length}{" "}
                  {matches.length === 1 ? "connector" : "connectors"}
                </span>
              )}
            </div>
            {query.isPending ? (
              <TableWrap className="px-3">
                <SkeletonRows rows={3} label="Loading connectors…" />
              </TableWrap>
            ) : query.error ? (
              <ErrorState error={query.error} />
            ) : !matches.length ? (
              <div className="rounded-lg border border-border bg-card">
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
                        variant="secondary"
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
              </div>
            ) : (
              <TableWrap>
                <Table>
                  <TableHead>
                    <tr>
                      <TableHeader>Connector</TableHeader>
                      <TableHeader className="w-[112px]">Transport</TableHeader>
                      <TableHeader className="w-[184px]">Status</TableHeader>
                      <TableHeader className="w-[168px]" aria-label="Actions" />
                    </tr>
                  </TableHead>
                  <TableBody>
                    {matches.map((d, index) => (
                      <TableRow
                        key={d.key}
                        selected={selected === d.key}
                        className={cn(
                          firstLoad.current?.has(d.key) && "animate-enter",
                        )}
                        style={
                          firstLoad.current?.has(d.key)
                            ? staggerStyle(index)
                            : undefined
                        }
                      >
                        <TableCell className="py-3">
                          <div className="flex items-start gap-3">
                            <McpConnectorIcon definition={d} />
                            <div className="min-w-0">
                              <h3 className="m-0 text-body font-medium text-foreground">
                                {d.name}
                              </h3>
                              <p className="m-0 mt-0.5 line-clamp-2 text-caption text-muted-foreground">
                                {d.description ||
                                  "Approved MCP connector metadata."}
                              </p>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell>
                          <Transport value={d.transport} />
                        </TableCell>
                        <TableCell>
                          <StatusBadge
                            state={d.enabled ? "positive" : "neutral"}
                          >
                            {d.enabled ? "Definition approved" : "Disabled"}
                          </StatusBadge>
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center justify-end gap-1">
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => setSelected(d.key)}
                            >
                              View definition
                            </Button>
                            <DropdownMenu>
                              <DropdownMenuTrigger asChild>
                                <IconButton
                                  size="icon-sm"
                                  label={"Actions for connector " + d.name}
                                >
                                  <MoreHorizontal aria-hidden="true" />
                                </IconButton>
                              </DropdownMenuTrigger>
                              <DropdownMenuContent align="end">
                                <DropdownMenuItem
                                  onSelect={() => setSelected(d.key)}
                                >
                                  <FileText aria-hidden="true" />
                                  View definition
                                </DropdownMenuItem>
                              </DropdownMenuContent>
                            </DropdownMenu>
                          </div>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableWrap>
            )}
            <Alert
              variant="info"
              role="note"
              icon={<ShieldCheck aria-hidden="true" />}
              title="Connector approval controls what can be configured. Brain access controls who can use it."
            >
              Each Brain chooses which approved connectors its agents can use.
            </Alert>
          </div>
          <Card className="flex flex-col gap-4">
            <div>
              <CardTitle as="h2">Add a connector</CardTitle>
              <CardDescription>
                Import or paste a connector definition to make it available
                across all Brains.
              </CardDescription>
            </div>
            <div className="flex flex-col gap-2">
              <Button className="w-full" onClick={() => add("config")}>
                <Upload aria-hidden="true" />
                Import definition
              </Button>
              <Button
                variant="secondary"
                className="w-full"
                onClick={() => add("config")}
              >
                <FileText aria-hidden="true" />
                Paste configuration
              </Button>
            </div>
            <div className="flex flex-col gap-3.5 border-t border-border pt-4">
              <h3 className="m-0 text-caption font-medium text-muted-foreground">
                How approval works
              </h3>
              <ol className="m-0 flex list-none flex-col gap-3.5 p-0">
                {steps.map((step, index) => (
                  <li key={step.title} className="flex gap-3">
                    <span className="mono grid size-6 shrink-0 place-items-center rounded-full border border-border-strong text-caption text-foreground-secondary">
                      {index + 1}
                    </span>
                    <div>
                      <div className="text-body font-medium text-foreground">
                        {step.title}
                      </div>
                      <div className="mt-0.5 text-label text-muted-foreground">
                        {step.detail}
                      </div>
                    </div>
                  </li>
                ))}
              </ol>
            </div>
          </Card>
        </div>
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
      <DetailInspector
        opened={!!selected && !query.error}
        onClose={() => setSelected(null)}
        title="Connector details"
      >
        {detail.isPending ? (
          <LoadingState />
        ) : detail.error ? (
          <ErrorState error={detail.error} />
        ) : (
          detail.data && (
            <>
              <InspectorSection>
                <div className="flex items-center gap-3">
                  <McpConnectorIcon definition={detail.data.summary} />
                  <h2 className="m-0 min-w-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
                    {detail.data.summary.name}
                  </h2>
                </div>
                {detail.data.summary.description && (
                  <p className="m-0 text-body text-foreground-secondary">
                    {detail.data.summary.description}
                  </p>
                )}
              </InspectorSection>
              <InspectorSection label="Definition">
                <PropList
                  items={[
                    {
                      term: "Transport",
                      value: (
                        <span className="mono">
                          {detail.data.manifest.transport}
                        </span>
                      ),
                    },
                    {
                      term: "Tools",
                      value: (
                        <span className="mono">
                          {detail.data.manifest.tools.length}
                        </span>
                      ),
                    },
                    {
                      term: "Runs on",
                      value: detail.data.manifest.placements.join(" · "),
                    },
                    ...(detail.data.manifest.command
                      ? [
                          {
                            term: "Command",
                            value: (
                              <span className="mono">
                                {detail.data.manifest.command}{" "}
                                {detail.data.manifest.arguments?.join(" ")}
                              </span>
                            ),
                          },
                        ]
                      : []),
                    {
                      term: "Credential aliases",
                      value:
                        detail.data.manifest.credential_aliases?.join(" · ") ||
                        "None",
                    },
                  ]}
                />
              </InspectorSection>
              <InspectorSection label="Tools">
                <ul className="m-0 flex list-none flex-col gap-2 p-0">
                  {detail.data.manifest.tools.map((t) => (
                    <li
                      key={t.name}
                      className="flex flex-col gap-1.5 rounded-md border border-border p-3"
                    >
                      <strong className="mono text-label font-medium text-foreground [overflow-wrap:anywhere]">
                        {t.name}
                      </strong>
                      {!!t.description && (
                        <p className="m-0 text-label text-muted-foreground">
                          {t.description.slice(0, 180)}
                          {t.description.length > 180 ? "…" : ""}
                        </p>
                      )}
                      {!!t.description && (
                        <details className="group/details">
                          <summary className="inline-flex cursor-pointer list-none items-center gap-1 text-caption font-medium text-foreground-secondary hover:text-foreground [&::-webkit-details-marker]:hidden">
                            <ChevronDown
                              className="size-3.5 -rotate-90 transition-transform duration-fast group-open/details:rotate-0"
                              aria-hidden="true"
                            />
                            Full description
                          </summary>
                          <div className="mt-2">
                            <Markdown text={t.description ?? ""} />
                          </div>
                        </details>
                      )}
                      <details className="group/details">
                        <summary className="inline-flex cursor-pointer list-none items-center gap-1 text-caption font-medium text-foreground-secondary hover:text-foreground [&::-webkit-details-marker]:hidden">
                          <ChevronDown
                            className="size-3.5 -rotate-90 transition-transform duration-fast group-open/details:rotate-0"
                            aria-hidden="true"
                          />
                          Input schema
                        </summary>
                        <div className="mt-2">
                          <CodeBlock
                            language="json"
                            code={JSON.stringify(t.inputSchema, null, 2)}
                          />
                        </div>
                      </details>
                    </li>
                  ))}
                </ul>
              </InspectorSection>
              <p className="m-0 pt-3 text-caption text-muted-foreground">
                To use this connector, open a Brain’s Connections page and
                choose Add connection.
              </p>
            </>
          )
        )}
      </DetailInspector>
    </Page>
  );
}
