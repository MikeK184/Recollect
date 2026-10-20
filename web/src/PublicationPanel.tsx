import { Link } from "@tanstack/react-router";
import { useId, useRef, useState, type ReactNode } from "react";
import { EraseAction } from "./RetentionPanel";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  ArrowRight,
  ChevronRight,
  FileCode2,
  FolderGit2,
  GitCommitHorizontal,
  Plus,
  RefreshCw,
} from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
import { usePollInterval } from "./usePollInterval";
import { useWorkspaceCatalogue } from "./catalogueQueries";
import { CodeBlock } from "./components/CodeBlock";
import { SectionHeader } from "./components/PageHeader";
import { PropList } from "./components/Inspector";
import { Skeleton } from "./components/Skeleton";
import { StatusDot } from "./components/StatusDot";
import { staggerStyle } from "./components/Motion";
import { Toolbar, useOpener } from "./features/repositories/Toolbar";
import {
  Alert,
  Badge,
  Button,
  Card,
  Code,
  Modal,
  Select,
  Separator,
  StatusBadge,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  TextareaField,
  TextField,
  Tile,
} from "./components/ui";
import { cn } from "./lib/utils";
type Repository = components["schemas"]["Repository"];
type Manifest = components["schemas"]["ManifestRevision"];
type Entry = components["schemas"]["ManifestEntry"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];
function Failure({ error }: { error: Error | null }) {
  return error ? (
    <Alert variant="destructive" title="Request failed">
      {error.message}
    </Alert>
  ) : null;
}
const time = (value: string) => new Date(value).toLocaleString();
const words = (value: string) => value.replaceAll("_", " ");
const sentence = (value: string) => {
  const text = words(value);
  return text.charAt(0).toUpperCase() + text.slice(1);
};
/** Revision meanings use the same names as the manifest form's choices. */
const meanings: Record<string, string> = {
  committed: "Committed code",
  desired: "Desired configuration",
  observed_deployed: "Recorded deployment observation",
};
const meaning = (kind: string) => meanings[kind] ?? words(kind);
function Pages({
  offset,
  total,
  size = 20,
  onChange,
}: {
  offset: number;
  total: number;
  size?: number;
  onChange: (n: number) => void;
}) {
  if (total <= size) return null;
  return (
    <div className="flex items-center gap-2">
      <Button
        variant="secondary"
        size="sm"
        disabled={!offset}
        onClick={() => onChange(Math.max(0, offset - size))}
      >
        Previous page
      </Button>
      <span className="mono mx-auto text-caption text-muted-foreground">
        {offset + 1}–{Math.min(offset + size, total)} of {total}
      </span>
      <Button
        variant="secondary"
        size="sm"
        disabled={offset + size >= total}
        onClick={() => onChange(offset + size)}
      >
        Next page
      </Button>
    </div>
  );
}
/** Recorded processing state, its raw word kept beside the icon. */
function Processing({ value }: { value: string }) {
  if (value === "running")
    return (
      <Badge variant="brand">
        <StatusDot tone="accent" live size={6} />
        {value}
      </Badge>
    );
  return (
    <StatusBadge
      state={
        value === "ready"
          ? "positive"
          : value === "failed"
            ? "negative"
            : "neutral"
      }
    >
      {value}
    </StatusBadge>
  );
}
/** Ids present on the first non-empty render: only these stagger in, so a
 * refetch or a later page never replays the entrance. */
function useFirstLoad(ids: readonly string[] | undefined) {
  const first = useRef<Set<string> | null>(null);
  if (!first.current && ids?.length) first.current = new Set(ids);
  return (id: string, index: number) =>
    first.current?.has(id)
      ? { className: "animate-enter", style: staggerStyle(index) }
      : {};
}
function Loading({ label }: { label: string }) {
  return (
    <div className="grid gap-2" role="status">
      <Skeleton height={14} width="40%" />
      <Skeleton height={56} className="rounded-md" />
      <span className="sr-only">{label}</span>
    </div>
  );
}
function JsonView({ value }: { value: unknown }) {
  return <CodeBlock code={JSON.stringify(value, null, 2)} language="json" />;
}
// Highlighting follows the file's extension only; JSON stays unformatted so
// the committed bytes are shown exactly.
const languages: Record<string, string> = {
  rs: "rust",
  ts: "typescript",
  tsx: "typescript",
  mts: "typescript",
  js: "javascript",
  jsx: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  py: "python",
  go: "go",
  java: "java",
  kt: "kotlin",
  rb: "ruby",
  php: "php",
  c: "c",
  h: "c",
  cc: "cpp",
  cpp: "cpp",
  hpp: "cpp",
  cs: "csharp",
  swift: "swift",
  sh: "bash",
  bash: "bash",
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  ini: "ini",
  md: "markdown",
  css: "css",
  scss: "scss",
  sql: "sql",
  xml: "xml",
  html: "xml",
  graphql: "graphql",
  lua: "lua",
};
const languageOf = (path: string) => {
  const name = path.split("/").at(-1) ?? "";
  if (name === "Makefile") return "makefile";
  if (name === "Dockerfile") return "dockerfile";
  const extension = name.includes(".") ? name.split(".").at(-1) : undefined;
  return extension ? languages[extension.toLowerCase()] : undefined;
};
function FactRecord({ value }: { value: unknown }) {
  const fact =
    value && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : null;
  const relations = Array.isArray(fact?.relations) ? fact.relations : [];
  return (
    <div className="grid gap-2">
      <div className="flex items-center justify-between gap-3">
        <p className="m-0 min-w-0 text-body font-medium text-foreground [overflow-wrap:anywhere]">
          {typeof fact?.name === "string" ? fact.name : "Structured fact"}
        </p>
        {typeof fact?.kind === "string" && (
          <Badge variant="outline">{fact.kind}</Badge>
        )}
      </div>
      {typeof fact?.file === "string" && (
        <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
          {fact.file}
          {typeof fact.line === "number" ? `:${fact.line}` : ""}
        </p>
      )}
      {relations.length > 0 && (
        <ul className="m-0 grid list-none gap-1 p-0">
          {relations.map((relation, index) => {
            const r =
              relation && typeof relation === "object"
                ? (relation as Record<string, unknown>)
                : {};
            return (
              <li
                key={index}
                className="flex items-center gap-1.5 text-caption text-foreground-secondary"
              >
                <span>
                  {typeof r.kind === "string" ? r.kind : "Relationship"}
                </span>
                <ArrowRight
                  aria-label="to"
                  className="size-3 shrink-0 text-muted-foreground"
                />
                <span className="mono min-w-0 [overflow-wrap:anywhere]">
                  {typeof r.target === "string"
                    ? r.target
                    : typeof r.target_id === "string"
                      ? r.target_id
                      : "Target not recorded"}
                </span>
              </li>
            );
          })}
        </ul>
      )}
      <Disclosure label="Raw record">
        <JsonView value={value} />
      </Disclosure>
    </div>
  );
}
/** A quiet, keyboard-reachable disclosure for secondary detail. */
function Disclosure({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <details className="group/disclosure">
      <summary className="inline-flex cursor-pointer list-none items-center gap-1 text-label font-medium text-foreground-secondary hover:text-foreground [&::-webkit-details-marker]:hidden">
        <ChevronRight
          aria-hidden="true"
          className="size-3.5 transition-transform duration-fast group-open/disclosure:rotate-90"
        />
        {label}
      </summary>
      <div className="pt-2">{children}</div>
    </details>
  );
}
export function RepositoryDialog({
  brain,
  repository,
  onClose,
}: {
  brain: Brain;
  repository: Repository;
  onClose: () => void;
}) {
  const pollNormal = usePollInterval("normal");
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const snapshots = useQuery({
    queryKey: ["repository-snapshots", brain.id, repository.id, offset],
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/repositories/{repository}/snapshots",
          {
            params: {
              path: { brain: brain.id, repository: repository.id },
              query: { offset },
            },
          },
        ),
      ),
    refetchInterval: pollNormal,
  });
  const enter = useFirstLoad(snapshots.data?.items.map((s) => s.id));
  if (selected)
    return (
      <SnapshotDialog
        brain={brain}
        id={selected}
        onClose={() => setSelected(null)}
      />
    );
  return (
    <Modal
      opened
      onClose={onClose}
      size="xl"
      title="Committed snapshots"
      footer={
        <Button variant="secondary" onClick={() => void snapshots.refetch()}>
          <RefreshCw aria-hidden="true" />
          Refresh snapshots
        </Button>
      }
    >
      <div className="grid gap-4">
        <div className="flex items-start gap-3">
          <Tile>
            <FolderGit2 aria-hidden="true" />
          </Tile>
          <div className="grid min-w-0 gap-1">
            <p className="mono m-0 text-label font-medium text-foreground [overflow-wrap:anywhere]">
              {repository.canonical_origin}
            </p>
            <p className="m-0 text-label text-muted-foreground">
              Published committed evidence remains available when its
              contributor is offline. File text is retained only when explicitly
              selected under the Brain policy.
            </p>
          </div>
        </div>
        <Failure error={snapshots.error} />
        {snapshots.isPending && <Loading label="Loading snapshots…" />}
        {snapshots.data?.items.length === 0 && (
          <Alert title="No snapshots published" role="status">
            Use a connected host with a task scoped to this repository. Publish
            an exact locally available commit from your checkout.
          </Alert>
        )}
        {!snapshots.error &&
          snapshots.data?.items.map((s, index) => {
            const entrance = enter(s.id, index);
            return (
              <Card
                key={s.id}
                className={cn("grid gap-3", entrance.className)}
                style={entrance.style}
              >
                <div className="flex items-center justify-between gap-3">
                  <span className="flex min-w-0 items-center gap-2">
                    <GitCommitHorizontal
                      aria-hidden="true"
                      className="size-4 shrink-0 text-muted-foreground"
                    />
                    <Code className="truncate">{s.revision}</Code>
                  </span>
                  <Processing value={s.processing} />
                </div>
                <p className="mono m-0 text-caption text-foreground-secondary">
                  {s.fact_count} facts · {s.file_count} files ·{" "}
                  {s.retained_file_count} retained
                </p>
                <p className="mono m-0 text-caption text-muted-foreground">
                  {time(s.created_at)} · {s.adapter} {s.adapter_build}
                </p>
                <div className="flex flex-wrap items-center gap-x-4 gap-y-2 border-t border-border pt-3">
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => setSelected(s.id)}
                  >
                    Inspect snapshot
                  </Button>
                  <Link
                    to="/brains/$brainId/memory"
                    params={{ brainId: brain.id }}
                    search={{ repository: s.repository_id }}
                    className="inline-flex items-center gap-1 text-label font-medium text-brand-text underline-offset-[3px] hover:underline"
                  >
                    Memory in this repository
                    <ArrowRight aria-hidden="true" className="size-3.5" />
                  </Link>
                </div>
                <p className="m-0 text-caption text-muted-foreground">
                  Shows current memories whose scope includes this repository.
                  Inspect their exact support before attributing them to this
                  snapshot.
                </p>
              </Card>
            );
          })}
        {snapshots.data && (
          <Pages
            offset={offset}
            total={snapshots.data.total}
            onChange={setOffset}
          />
        )}
      </div>
    </Modal>
  );
}
export function SnapshotDialog({
  brain,
  id,
  onClose,
}: {
  brain: Brain;
  id: string;
  onClose: () => void;
}) {
  const pollFast = usePollInterval("fast");
  const pollNormal = usePollInterval("normal");
  const [tab, setTab] = useState<string>("files");
  const [fileOffset, setFileOffset] = useState(0);
  const [factOffset, setFactOffset] = useState(0);
  const [contributorOffset, setContributorOffset] = useState(0);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const path = { brain: brain.id, snapshot: id };
  const details = useQuery({
    queryKey: ["repository-snapshot", brain.id, id, contributorOffset],
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/repository-snapshots/{snapshot}",
          { params: { path, query: { offset: contributorOffset } } },
        ),
      ),
    refetchInterval: pollFast,
  });
  const files = useQuery({
    queryKey: ["repository-files", brain.id, id, fileOffset],
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/repository-snapshots/{snapshot}/files",
          { params: { path, query: { offset: fileOffset } } },
        ),
      ),
    enabled: tab === "files",
  });
  const facts = useQuery({
    queryKey: ["repository-facts", brain.id, id, factOffset],
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/repository-snapshots/{snapshot}/facts",
          { params: { path, query: { offset: factOffset } } },
        ),
      ),
    enabled: tab === "facts",
    refetchInterval: pollNormal,
  });
  const artifactKind = tab === "insights" ? "insights" : "receipt";
  const artifact = useQuery({
    queryKey: ["repository-artifact", brain.id, id, artifactKind],
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/repository-snapshots/{snapshot}/artifacts/{kind}",
          { params: { path: { ...path, kind: artifactKind } } },
        ),
      ),
    enabled: tab === "insights" || tab === "receipt",
  });
  const content = useQuery({
    queryKey: ["repository-file", brain.id, id, selectedFile],
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/repository-snapshots/{snapshot}/files/{file}",
          { params: { path: { ...path, file: selectedFile! } } },
        ),
      ),
    enabled: !!selectedFile && tab === "files",
    refetchInterval: pollNormal,
  });
  const key = useIdempotency();
  const process = useMutation({
    mutationFn: async () =>
      result(
        await client.POST(
          "/api/brains/{brain}/repository-snapshots/{snapshot}/process",
          {
            params: { path },
            headers: { "Idempotency-Key": key.forInput(id) },
          },
        ),
      ),
    onSuccess: () => {
      key.reset();
      void details.refetch();
      void facts.refetch();
    },
  });
  const s = details.error ? undefined : details.data?.snapshot;
  const segments = content.data?.file.path.split("/").filter(Boolean) ?? [];
  return (
    <Modal
      opened
      onClose={onClose}
      size="full"
      title="Committed snapshot"
      className="h-[min(880px,calc(100vh-64px))]"
    >
      <div className="grid gap-4">
        <Failure error={details.error} />
        {details.isPending && <Loading label="Loading snapshot…" />}
        {s && (
          <>
            <div className="grid gap-3 rounded-lg border border-border bg-muted/50 p-4">
              <div className="flex flex-wrap items-center gap-3">
                <Processing value={s.processing} />
                <Code className="min-w-0 truncate">{s.revision}</Code>
                <span className="mono ml-auto shrink-0 whitespace-nowrap text-caption text-muted-foreground">
                  Published {time(s.created_at)}
                </span>
              </div>
              <p className="mono m-0 text-caption text-foreground-secondary">
                {s.fact_count} structural facts · {s.file_count} files ·{" "}
                {s.retained_file_count} retained texts
              </p>
              <div className="flex flex-wrap items-center gap-2">
                {brain.role !== "reader" && !brain.archived && (
                  <Button
                    variant="secondary"
                    size="sm"
                    loading={process.isPending}
                    disabled={
                      s.processing === "queued" || s.processing === "running"
                    }
                    onClick={() => process.mutate()}
                  >
                    <RefreshCw aria-hidden="true" />
                    Reprocess retained artifacts
                  </Button>
                )}
                <EraseAction
                  brain={brain.id}
                  permitted={brain.role === "admin"}
                  target={{ kind: "snapshot", id }}
                  name={"Snapshot " + s.revision}
                />
              </div>
              <Disclosure label="Extraction details">
                <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
                  Snapshot {id} · {s.adapter} {s.adapter_build} · extractor{" "}
                  {s.extractor_version}
                </p>
              </Disclosure>
            </div>
            <Failure error={process.error} />
            <Tabs
              value={tab}
              onValueChange={(v) => {
                setTab(v);
                setSelectedFile(null);
              }}
              className="gap-4"
            >
              <div className="flex items-end gap-3 border-b border-border">
                <TabsList className="min-w-0 flex-1 border-b-0">
                  <TabsTrigger value="files">Files</TabsTrigger>
                  <TabsTrigger value="facts">Facts</TabsTrigger>
                  <TabsTrigger value="coverage">Coverage</TabsTrigger>
                  <TabsTrigger value="contributors">Contributors</TabsTrigger>
                </TabsList>
                <div className="w-[160px] shrink-0 pb-1.5">
                  <Select
                    size="sm"
                    aria-label="Snapshot artifacts"
                    placeholder="Artifacts"
                    clearable
                    value={tab === "insights" || tab === "receipt" ? tab : null}
                    onChange={(value) => {
                      setTab(value ?? "files");
                      setSelectedFile(null);
                    }}
                    data={[
                      { value: "insights", label: "Insights" },
                      { value: "receipt", label: "Receipt" },
                    ]}
                  />
                </div>
              </div>
              <TabsContent value="files">
                <div className="grid gap-3">
                  <Failure error={files.error} />
                  {files.isPending && <Loading label="Loading files…" />}
                  {selectedFile ? (
                    <>
                      <div className="flex min-w-0 items-center gap-2">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setSelectedFile(null)}
                        >
                          <ArrowLeft aria-hidden="true" />
                          Back to files
                        </Button>
                        {segments.length > 0 && (
                          <nav
                            aria-label="File path"
                            className="mono flex min-w-0 flex-wrap items-center gap-1 text-caption text-muted-foreground"
                          >
                            {segments.map((segment, index) => (
                              <span
                                key={index}
                                className="flex items-center gap-1"
                              >
                                {index > 0 && (
                                  <ChevronRight
                                    aria-hidden="true"
                                    className="size-3"
                                  />
                                )}
                                <span
                                  aria-current={
                                    index === segments.length - 1
                                      ? "page"
                                      : undefined
                                  }
                                  className={cn(
                                    index === segments.length - 1 &&
                                      "font-medium text-foreground",
                                  )}
                                >
                                  {segment}
                                </span>
                              </span>
                            ))}
                          </nav>
                        )}
                      </div>
                      <Failure error={content.error} />
                      {content.isPending && <Loading label="Opening file…" />}
                      {!content.error && content.data && (
                        <>
                          {content.data.content !== null &&
                          content.data.content !== undefined ? (
                            <CodeBlock
                              code={content.data.content}
                              language={languageOf(content.data.file.path)}
                              copyLabel="Copy file text"
                            />
                          ) : (
                            <Alert
                              variant="warning"
                              role="status"
                              title="File content unavailable"
                            >
                              {sentence(content.data.file.availability)}. The
                              committed file reference is preserved; Recollect
                              does not fetch it from a contributor.
                            </Alert>
                          )}
                        </>
                      )}
                    </>
                  ) : (
                    <>
                      {!files.error && files.data?.items.length === 0 && (
                        <p className="m-0 text-body text-muted-foreground">
                          No files in this snapshot.
                        </p>
                      )}
                      {!files.error && !!files.data?.items.length && (
                        <ul className="m-0 grid list-none divide-y divide-border overflow-hidden rounded-lg border border-border bg-card p-0">
                          {files.data.items.map((f) => (
                            <li key={f.id}>
                              <button
                                type="button"
                                className="flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors duration-fast hover:bg-accent focus-visible:bg-accent focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring"
                                onClick={() => setSelectedFile(f.id)}
                              >
                                <FileCode2
                                  aria-hidden="true"
                                  className="size-4 shrink-0 text-muted-foreground"
                                />
                                <span className="grid min-w-0 flex-1 gap-0.5">
                                  <span className="mono truncate text-label font-medium text-foreground">
                                    {f.path}
                                  </span>
                                  <span className="text-caption text-muted-foreground">
                                    {words(f.status)} · {words(f.extraction)} ·{" "}
                                    {words(f.availability)}
                                  </span>
                                </span>
                                <ChevronRight
                                  aria-hidden="true"
                                  className="size-4 shrink-0 text-faint-foreground"
                                />
                              </button>
                            </li>
                          ))}
                        </ul>
                      )}
                      {!files.error && files.data && (
                        <Pages
                          offset={fileOffset}
                          total={files.data.total}
                          size={100}
                          onChange={setFileOffset}
                        />
                      )}
                    </>
                  )}
                </div>
              </TabsContent>
              <TabsContent value="facts">
                <div className="grid gap-3">
                  <Failure error={facts.error} />
                  {facts.isPending && <Loading label="Loading facts…" />}
                  {!facts.error &&
                    facts.data &&
                    facts.data.processing !== "ready" && (
                      <Alert variant="warning" role="status">
                        Fact processing is {facts.data.processing}. Retained
                        extraction artifacts remain available separately.
                      </Alert>
                    )}
                  {!facts.error && facts.data?.items.length === 0 && (
                    <p className="m-0 text-body text-muted-foreground">
                      No materialized facts available.
                    </p>
                  )}
                  {!facts.error &&
                    facts.data?.items.map((f) => (
                      <Card key={f.id}>
                        <FactRecord value={f.record} />
                      </Card>
                    ))}
                  {!facts.error && facts.data && (
                    <Pages
                      offset={factOffset}
                      total={facts.data.total}
                      size={100}
                      onChange={setFactOffset}
                    />
                  )}
                </div>
              </TabsContent>
              <TabsContent value="coverage">
                <div className="grid gap-3">
                  <Alert variant="info" role="note" title="Extraction limits">
                    Static code and configuration facts are evidence. Kubernetes
                    YAML has file inventory and optional text, with no supported
                    semantic extraction in this adapter. Unresolved targets
                    remain unresolved.
                  </Alert>
                  <h3 className="section-label m-0 mt-1">Coverage</h3>
                  <JsonView value={s.coverage} />
                  <h3 className="section-label m-0 mt-1">
                    Extraction settings
                  </h3>
                  <JsonView value={s.settings} />
                </div>
              </TabsContent>
              <TabsContent value="contributors">
                <div className="grid gap-3">
                  {details.data?.contributors.map((c) => (
                    <Card key={c.id} className="grid gap-3">
                      <div className="flex items-center justify-between gap-3">
                        <p className="m-0 text-body font-medium text-foreground">
                          {c.actor_name || "Recorded contributor"}
                        </p>
                      </div>
                      <p className="m-0 text-body text-foreground-secondary">
                        {c.dirty
                          ? "Working copy was dirty; committed bytes were published."
                          : "Working copy reported clean."}
                      </p>
                      <PropList
                        items={[
                          {
                            term: "Branch",
                            value: (
                              <span className="mono">
                                {c.branch || "detached"}
                              </span>
                            ),
                          },
                          {
                            term: "Captured",
                            value: (
                              <span className="mono">
                                {time(c.captured_at)}
                              </span>
                            ),
                          },
                          {
                            term: "Environment",
                            value:
                              c.scope.environment?.name || "All in this Brain",
                          },
                          {
                            term: "Device",
                            value: (
                              <span className="mono text-caption">
                                {c.device_id}
                              </span>
                            ),
                          },
                          {
                            term: "Operation",
                            value: (
                              <span className="mono text-caption">
                                {c.operation_id}
                              </span>
                            ),
                          },
                          {
                            term: "Scope",
                            value: (
                              <span className="mono text-caption">
                                {c.scope.id}
                              </span>
                            ),
                          },
                        ]}
                      />
                    </Card>
                  ))}
                  {details.data && (
                    <Pages
                      offset={contributorOffset}
                      total={details.data.contributor_total}
                      onChange={setContributorOffset}
                    />
                  )}
                </div>
              </TabsContent>
              {(["insights", "receipt"] as const).map((kind) => (
                <TabsContent key={kind} value={kind}>
                  <div className="grid gap-3">
                    <Failure error={artifact.error} />
                    {artifact.isPending && (
                      <Loading label="Loading artifact…" />
                    )}
                    {!artifact.error &&
                    artifact.data?.content !== null &&
                    artifact.data?.content !== undefined ? (
                      <JsonView value={artifact.data.content} />
                    ) : (
                      !artifact.error &&
                      artifact.data && (
                        <Alert variant="warning" role="status">
                          Artifact {artifact.data.availability}.
                        </Alert>
                      )
                    )}
                  </div>
                </TabsContent>
              ))}
            </Tabs>
          </>
        )}
      </div>
    </Modal>
  );
}

export function PublicationPanel({
  brain,
  section = "environments",
  toolbar,
}: {
  brain: Brain;
  section?: "repositories" | "environments";
  /** The page's switch row for the filter and actions; inline if absent. */
  toolbar?: HTMLElement | null;
}) {
  const pollNormal = usePollInterval("normal");
  const cache = useQueryClient();
  const [environment, setEnvironment] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const catalogue = useWorkspaceCatalogue(brain.id, undefined, {
    poll: "normal",
  });
  const manifests = useQuery({
    queryKey: ["revision-manifests", brain.id, environment, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/revision-manifests", {
          params: {
            path: { brain: brain.id },
            query: { environment_id: environment ?? undefined, offset },
          },
        }),
      ),
    refetchInterval: pollNormal,
  });
  const refresh = () => {
    void cache.invalidateQueries({
      queryKey: ["revision-manifests", brain.id],
    });
    void cache.invalidateQueries({ queryKey: ["revision-manifest", brain.id] });
  };
  const enter = useFirstLoad(manifests.data?.items.map((m) => m.manifest_id));
  const opener = useOpener();
  const environmentName = (id: string) =>
    catalogue.data?.environments.find((e) => e.id === id)?.name || id;
  return (
    <section className="flex flex-col gap-4">
      <Toolbar target={toolbar}>
        <div className="min-w-0 flex-[0_1_240px]">
          <Select
            clearable
            aria-label="Filter by environment"
            placeholder="All environments"
            data={
              catalogue.data?.environments.map((e) => ({
                value: e.id,
                label: e.name,
              })) ?? []
            }
            value={environment}
            onChange={(v) => {
              setEnvironment(v);
              setOffset(0);
            }}
          />
        </div>
        <span
          className="mono ml-auto shrink-0 whitespace-nowrap text-caption text-muted-foreground"
          aria-live="polite"
        >
          {manifests.data
            ? `${manifests.data.total} manifest${manifests.data.total === 1 ? "" : "s"}`
            : ""}
        </span>
        {brain.role !== "reader" && !brain.archived && (
          <Button
            disabled={
              !catalogue.data?.environments.length ||
              !catalogue.data?.repositories.length
            }
            onClick={() => {
              opener.remember();
              setCreating(true);
            }}
          >
            <Plus aria-hidden="true" />
            New manifest
          </Button>
        )}
      </Toolbar>
      <SectionHeader
        title="Environment revisions"
        description="Choose exact repository revisions for an environment. Committed code, desired configuration and recorded deployment observations keep separate provenance."
      />
      <Failure error={catalogue.error} />
      <Failure error={manifests.error} />
      {manifests.error && (
        <div>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void manifests.refetch()}
          >
            Retry manifests
          </Button>
        </div>
      )}
      {manifests.isPending && <Loading label="Loading manifests…" />}
      {manifests.data?.items.length === 0 && (
        <div className="grid justify-items-center gap-2 rounded-lg border border-dashed border-border px-6 py-10 text-center">
          <Tile size="lg">
            <GitCommitHorizontal aria-hidden="true" />
          </Tile>
          <p className="m-0 max-w-[420px] text-body text-muted-foreground">
            No revision manifests yet. Add an environment and register a
            repository to create an explicit selection.
          </p>
        </div>
      )}
      {!manifests.error &&
        !catalogue.error &&
        !!manifests.data?.items.length && (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3">
            {manifests.data.items.map((m, index) => (
              <div key={m.manifest_id} {...enter(m.manifest_id, index)}>
                <button
                  type="button"
                  className="grid h-full w-full content-start gap-3 rounded-lg border border-border bg-card p-4 text-left transition-[border-color,box-shadow,transform] duration-fast ease-standard hover:-translate-y-px hover:border-border-strong hover:shadow-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                  onClick={() => {
                    opener.remember();
                    setSelected(m.manifest_id);
                  }}
                >
                  <span className="flex items-start gap-3">
                    <Tile>
                      <GitCommitHorizontal aria-hidden="true" />
                    </Tile>
                    <span className="grid min-w-0 gap-0.5">
                      <span className="text-title font-semibold text-foreground [overflow-wrap:anywhere]">
                        {m.name}
                      </span>
                      <span className="text-label text-muted-foreground">
                        {meaning(m.kind)}
                      </span>
                    </span>
                  </span>
                  <span className="flex flex-wrap gap-1.5">
                    <Badge variant="outline">
                      {environmentName(m.environment_id)}
                    </Badge>
                    <Badge>
                      {m.entries.length} repositor
                      {m.entries.length === 1 ? "y" : "ies"}
                    </Badge>
                  </span>
                  <span className="mono border-t border-border pt-3 text-caption text-muted-foreground">
                    Recorded {time(m.created_at)}
                  </span>
                </button>
              </div>
            ))}
          </div>
        )}
      {manifests.data && (
        <Pages
          offset={offset}
          total={manifests.data.total}
          onChange={setOffset}
        />
      )}
      {creating && catalogue.data && (
        <ManifestForm
          brain={brain}
          catalogue={catalogue.data}
          onClose={() => {
            setCreating(false);
            opener.restore();
          }}
          onSaved={(m) => {
            setCreating(false);
            setSelected(m.manifest_id);
            refresh();
          }}
        />
      )}
      {selected && catalogue.data && (
        <ManifestDialog
          brain={brain}
          id={selected}
          catalogue={catalogue.data}
          onClose={() => {
            setSelected(null);
            opener.restore();
          }}
          onSaved={refresh}
        />
      )}
    </section>
  );
}
function ManifestDialog({
  brain,
  id,
  catalogue,
  onClose,
  onSaved,
}: {
  brain: Brain;
  id: string;
  catalogue: Catalogue;
  onClose: () => void;
  onSaved: () => void;
}) {
  const pollNormal = usePollInterval("normal");
  const [offset, setOffset] = useState(0);
  const [historical, setHistorical] = useState<Manifest | null>(null);
  const [edit, setEdit] = useState<Manifest | null>(null);
  const [snapshot, setSnapshot] = useState<string | null>(null);
  const detail = useQuery({
    queryKey: ["revision-manifest", brain.id, id, offset],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/revision-manifests/{manifest}", {
          params: {
            path: { brain: brain.id, manifest: id },
            query: { offset },
          },
        }),
      ),
    refetchInterval: pollNormal,
  });
  if (snapshot)
    return (
      <SnapshotDialog
        brain={brain}
        id={snapshot}
        onClose={() => setSnapshot(null)}
      />
    );
  if (edit)
    return (
      <ManifestForm
        brain={brain}
        catalogue={catalogue}
        current={edit}
        onClose={() => setEdit(null)}
        onSaved={() => {
          setEdit(null);
          setHistorical(null);
          void detail.refetch();
          onSaved();
        }}
      />
    );
  const selected = detail.error
    ? undefined
    : (historical ?? detail.data?.current);
  return (
    <Modal opened onClose={onClose} size="xl" title="Revision manifest">
      <div className="grid gap-4">
        <Failure error={detail.error} />
        {detail.isPending && <Loading label="Loading manifest…" />}
        {selected && (
          <>
            <div className="flex flex-wrap items-center gap-3">
              <h3 className="m-0 min-w-0 flex-1 text-section font-semibold text-foreground [overflow-wrap:anywhere]">
                {selected.name}
              </h3>
              <Badge variant={historical ? "warning" : "default"}>
                {historical ? "Historical revision" : "Current revision"}
              </Badge>
              <EraseAction
                brain={brain.id}
                permitted={brain.role === "admin"}
                target={{ kind: "manifest", id: selected.manifest_id }}
                name={selected.name}
              />
            </div>
            <PropList
              items={[
                {
                  term: "Environment",
                  value:
                    catalogue.environments.find(
                      (e) => e.id === selected.environment_id,
                    )?.name || selected.environment_id,
                },
                {
                  term: "Meaning",
                  value: meaning(selected.kind),
                },
                {
                  term: "Recorded",
                  value: (
                    <span className="mono">{time(selected.created_at)}</span>
                  ),
                },
              ]}
            />
            {selected.kind === "observed_deployed" && (
              <Alert variant="info" role="status" title="Recorded observation">
                Observed{" "}
                {selected.observed_at ? time(selected.observed_at) : "unknown"}.
                Reference: {selected.observation_reference}. This is
                contributor-provided evidence; Recollect has not independently
                verified deployment.
              </Alert>
            )}
            {selected.notes && (
              <p className="m-0 text-body text-foreground-secondary">
                {selected.notes}
              </p>
            )}
            <div className="grid gap-2">
              <h4 className="section-label m-0">
                Repositories
                <span className="mono ml-1.5">{selected.entries.length}</span>
              </h4>
              {selected.entries.map((e) => (
                <Card key={e.repository_id} className="grid gap-2.5 p-3">
                  <p className="mono m-0 text-label font-medium text-foreground [overflow-wrap:anywhere]">
                    {catalogue.repositories.find(
                      (r) => r.id === e.repository_id,
                    )?.canonical_origin || e.repository_id}
                  </p>
                  <span className="flex min-w-0 items-center gap-2">
                    <GitCommitHorizontal
                      aria-hidden="true"
                      className="size-4 shrink-0 text-muted-foreground"
                    />
                    <Code className="truncate">{e.revision}</Code>
                  </span>
                  <p className="m-0 text-caption text-muted-foreground">
                    Config paths:{" "}
                    <span className="mono">
                      {(e.config_paths ?? []).join(", ") || "None specified"}
                    </span>
                  </p>
                  {e.snapshot_id ? (
                    <div>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => setSnapshot(e.snapshot_id!)}
                      >
                        Inspect selected snapshot
                      </Button>
                    </div>
                  ) : (
                    <p className="m-0 text-caption text-muted-foreground">
                      Snapshot unavailable — this revision is recorded without
                      retained extraction.
                    </p>
                  )}
                </Card>
              ))}
            </div>
            <p className="mono m-0 text-caption text-muted-foreground [overflow-wrap:anywhere]">
              Recorded by {selected.actor_id} · revision {selected.id}
            </p>
            {brain.role !== "reader" && !brain.archived && detail.data && (
              <div>
                <Button onClick={() => setEdit(detail.data!.current)}>
                  Edit current selection
                </Button>
              </div>
            )}
            <Separator />
            <div className="grid gap-2">
              <h4 className="section-label m-0">Revision history</h4>
              <div className="flex flex-wrap gap-1.5">
                <Button
                  variant={historical ? "ghost" : "secondary"}
                  size="sm"
                  onClick={() => setHistorical(null)}
                >
                  Show current revision
                </Button>
                {detail.data?.history.map((m) => (
                  <Button
                    variant={historical?.id === m.id ? "secondary" : "ghost"}
                    size="sm"
                    key={m.id}
                    aria-pressed={historical?.id === m.id}
                    onClick={() => setHistorical(m)}
                  >
                    <span className="mono">{time(m.created_at)}</span> ·{" "}
                    {meaning(m.kind)}
                  </Button>
                ))}
              </div>
              {detail.data && (
                <Pages
                  offset={offset}
                  total={detail.data.total}
                  onChange={setOffset}
                />
              )}
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}
function ManifestForm({
  brain,
  catalogue,
  current,
  onClose,
  onSaved,
}: {
  brain: Brain;
  catalogue: Catalogue;
  current?: Manifest;
  onClose: () => void;
  onSaved: (m: Manifest) => void;
}) {
  const formId = useId();
  const [name, setName] = useState(current?.name ?? "");
  const [environment, setEnvironment] = useState<string | null>(
    current?.environment_id ?? null,
  );
  const [kind, setKind] = useState(current?.kind ?? "desired");
  const [entries, setEntries] = useState<Entry[]>(
    current?.entries ?? [
      { repository_id: "", revision: "", snapshot_id: null, config_paths: [] },
    ],
  );
  const [notes, setNotes] = useState(current?.notes ?? "");
  const [observed, setObserved] = useState(current?.observed_at ?? "");
  const [reference, setReference] = useState(
    current?.observation_reference ?? "",
  );
  const key = useIdempotency();
  const save = useMutation({
    mutationFn: async () => {
      const body = {
        name,
        environment_id: environment!,
        kind,
        entries: entries.map((e) => ({
          ...e,
          config_paths: (e.config_paths ?? [])
            .map((p) => p.trim())
            .filter(Boolean),
        })),
        notes,
        base_revision: current?.id ?? null,
        operation_id: null,
        observed_at:
          kind === "observed_deployed"
            ? new Date(observed).toISOString()
            : null,
        observation_reference: kind === "observed_deployed" ? reference : null,
      };
      const headers = { "Idempotency-Key": key.forInput(body) };
      return current
        ? result(
            await client.PUT(
              "/api/brains/{brain}/revision-manifests/{manifest}",
              {
                params: {
                  path: { brain: brain.id, manifest: current.manifest_id },
                },
                body,
                headers,
              },
            ),
          )
        : result(
            await client.POST("/api/brains/{brain}/revision-manifests", {
              params: { path: { brain: brain.id } },
              body,
              headers,
            }),
          );
    },
    onSuccess: (m) => {
      key.reset();
      onSaved(m);
    },
  });
  const valid =
    !!environment &&
    !!name.trim() &&
    entries.length > 0 &&
    entries.every(
      (e) =>
        e.repository_id && /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(e.revision),
    ) &&
    (kind !== "observed_deployed" ||
      (!!reference.trim() && !Number.isNaN(Date.parse(observed))));
  return (
    <Modal
      opened
      onClose={onClose}
      size="xl"
      title={current ? "Edit revision selection" : "New revision manifest"}
      footer={
        <>
          <Button variant="secondary" type="button" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            form={formId}
            disabled={!valid}
            loading={save.isPending}
          >
            Save manifest revision
          </Button>
        </>
      }
    >
      <form
        id={formId}
        className="grid gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate();
        }}
      >
        <TextField
          label="Manifest name"
          required
          maxLength={120}
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          disabled={!!current}
        />
        <div className="grid grid-cols-2 items-start gap-4">
          <Select
            label="Environment"
            required
            data={catalogue.environments.map((e) => ({
              value: e.id,
              label: e.name,
            }))}
            value={environment}
            onChange={setEnvironment}
            disabled={!!current}
          />
          <Select
            label="Revision meaning"
            required
            value={kind}
            onChange={(v) => setKind(v ?? "desired")}
            data={[
              { value: "committed", label: "Committed code" },
              { value: "desired", label: "Desired configuration" },
              {
                value: "observed_deployed",
                label: "Recorded deployment observation",
              },
            ]}
          />
        </div>
        {kind === "observed_deployed" && (
          <div className="grid grid-cols-2 items-start gap-4">
            <TextField
              label="Observation time (ISO date and timezone)"
              placeholder="2026-09-14T12:00:00Z"
              required
              inputClassName="mono"
              value={observed}
              onChange={(e) => setObserved(e.currentTarget.value)}
            />
            <TextField
              label="Supporting observation reference"
              required
              value={reference}
              onChange={(e) => setReference(e.currentTarget.value)}
              maxLength={2000}
            />
          </div>
        )}
        {entries.map((entry, index) => (
          <EntryForm
            key={index}
            brain={brain}
            catalogue={catalogue}
            entry={entry}
            index={index}
            onChange={(value) =>
              setEntries((v) => v.map((old, i) => (i === index ? value : old)))
            }
            onRemove={() => setEntries((v) => v.filter((_, i) => i !== index))}
          />
        ))}
        <div>
          <Button
            variant="secondary"
            type="button"
            disabled={entries.length >= 100}
            onClick={() =>
              setEntries((v) => [
                ...v,
                {
                  repository_id: "",
                  revision: "",
                  snapshot_id: null,
                  config_paths: [],
                },
              ])
            }
          >
            <Plus aria-hidden="true" />
            Add repository entry
          </Button>
        </div>
        <TextareaField
          label="Notes"
          value={notes}
          onChange={(e) => setNotes(e.currentTarget.value)}
          maxLength={2048}
        />
        <Failure error={save.error} />
        {save.error && current && (
          <p className="m-0 text-body text-foreground-secondary">
            If this manifest changed, close this editor and reopen the current
            selection before saving again.
          </p>
        )}
      </form>
    </Modal>
  );
}
function EntryForm({
  brain,
  catalogue,
  entry,
  index,
  onChange,
  onRemove,
}: {
  brain: Brain;
  catalogue: Catalogue;
  entry: Entry;
  index: number;
  onChange: (e: Entry) => void;
  onRemove: () => void;
}) {
  const [offset, setOffset] = useState(0);
  const snapshots = useQuery({
    queryKey: ["repository-snapshots", brain.id, entry.repository_id, offset],
    queryFn: async () =>
      result(
        await client.GET(
          "/api/brains/{brain}/repositories/{repository}/snapshots",
          {
            params: {
              path: { brain: brain.id, repository: entry.repository_id },
              query: { offset },
            },
          },
        ),
      ),
    enabled: !!entry.repository_id,
  });
  const choices =
    snapshots.data?.items.map((s) => ({
      value: s.id,
      label: `${s.revision} · ${s.retained_file_count} retained files`,
    })) ?? [];
  if (entry.snapshot_id && !choices.some((v) => v.value === entry.snapshot_id))
    choices.unshift({
      value: entry.snapshot_id,
      label: `Selected snapshot ${entry.snapshot_id}`,
    });
  return (
    <Card className="grid gap-4 bg-muted/40">
      <div className="flex items-center justify-between gap-3">
        <h3 className="m-0 text-body font-semibold text-foreground">
          Repository entry {index + 1}
        </h3>
        <Button
          variant="destructive-ghost"
          size="sm"
          type="button"
          onClick={onRemove}
        >
          Remove entry
        </Button>
      </div>
      <Select
        label={`Repository ${index + 1}`}
        required
        searchable
        data={catalogue.repositories.map((r) => ({
          value: r.id,
          label: r.canonical_origin,
        }))}
        value={entry.repository_id || null}
        onChange={(id) => {
          setOffset(0);
          onChange({
            ...entry,
            repository_id: id ?? "",
            revision: "",
            snapshot_id: null,
          });
        }}
      />
      <Failure error={snapshots.error} />
      <Select
        label={`Published snapshot ${index + 1}`}
        clearable
        searchable
        placeholder="Optional: select retained evidence"
        data={choices}
        value={entry.snapshot_id ?? null}
        onChange={(id) => {
          const s = snapshots.data?.items.find((s) => s.id === id);
          onChange({
            ...entry,
            snapshot_id: id,
            revision: s?.revision ?? entry.revision,
          });
        }}
        disabled={!entry.repository_id}
      />
      {snapshots.data && (
        <Pages
          offset={offset}
          total={snapshots.data.total}
          onChange={setOffset}
        />
      )}
      <TextField
        label={`Exact commit ${index + 1}`}
        required
        inputClassName="mono"
        value={entry.revision}
        onChange={(e) =>
          onChange({
            ...entry,
            revision: e.currentTarget.value.trim(),
            snapshot_id: null,
          })
        }
      />
      {!entry.snapshot_id && (
        <p className="m-0 text-caption text-muted-foreground">
          This selection records a revision without retained snapshot evidence.
        </p>
      )}
      <TextareaField
        label={`Configuration paths ${index + 1}`}
        description="One relative path per line"
        inputClassName="mono text-label"
        value={(entry.config_paths ?? []).join("\n")}
        onChange={(e) =>
          onChange({
            ...entry,
            config_paths: e.currentTarget.value.split("\n"),
          })
        }
      />
    </Card>
  );
}
