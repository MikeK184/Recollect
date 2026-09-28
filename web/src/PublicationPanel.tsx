import { Link } from "@tanstack/react-router";
import { iconSize } from "./design/tokens";
import { useState } from "react";
import "./features/feature-views.css";
import { EraseAction } from "./RetentionPanel";
import {
  Alert,
  Badge,
  Button,
  Card,
  Code,
  Divider,
  Group,
  Loader,
  Modal,
  Select,
  Stack,
  Switch,
  Tabs,
  Text,
  Textarea,
  TextInput,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FileCode2, GitCommitHorizontal, Plus } from "lucide-react";
import { client, result, type Brain } from "./api";
import type { components } from "./api-schema";
import { useIdempotency } from "./useIdempotency";
type Repository = components["schemas"]["Repository"];
type Manifest = components["schemas"]["ManifestRevision"];
type Entry = components["schemas"]["ManifestEntry"];
type Catalogue = components["schemas"]["WorkspaceCatalogue"];
function Failure({ error }: { error: Error | null }) {
  return error ? (
    <Alert color="red" title="Request failed">
      {error.message}
    </Alert>
  ) : null;
}
const time = (value: string) => new Date(value).toLocaleString();
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
    <Group justify="space-between">
      <Button
        variant="subtle"
        disabled={!offset}
        onClick={() => onChange(Math.max(0, offset - size))}
      >
        Previous page
      </Button>
      <Text size="xs">
        {offset + 1}–{Math.min(offset + size, total)} of {total}
      </Text>
      <Button
        variant="subtle"
        disabled={offset + size >= total}
        onClick={() => onChange(offset + size)}
      >
        Next page
      </Button>
    </Group>
  );
}
function JsonView({ value }: { value: unknown }) {
  return <pre className="source-content">{JSON.stringify(value, null, 2)}</pre>;
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
    refetchInterval: 4000,
  });
  if (selected)
    return (
      <SnapshotDialog
        brain={brain}
        id={selected}
        onClose={() => setSelected(null)}
      />
    );
  return (
    <Modal opened onClose={onClose} size="xl" title="Committed snapshots">
      <Stack className="publication-panel">
        <Text fw={600}>{repository.canonical_origin}</Text>
        <Text size="sm" c="dimmed">
          Published committed evidence remains available when its contributor is
          offline. File text is retained only when explicitly selected under the
          Brain policy.
        </Text>
        <Failure error={snapshots.error} />
        {snapshots.isPending && <Loader size="sm" />}
        {snapshots.data?.items.length === 0 && (
          <Alert title="No snapshots published">
            Use a paired companion with a task scoped to this repository.
            Publish an exact locally available commit from your checkout.
          </Alert>
        )}
        {!snapshots.error &&
          snapshots.data?.items.map((s) => (
            <Card withBorder key={s.id}>
              <Stack gap="xs">
                <Group justify="space-between">
                  <Button
                    variant="subtle"
                    px={0}
                    onClick={() => setSelected(s.id)}
                  >
                    Inspect snapshot
                  </Button>
                  <Badge color={s.processing === "ready" ? "teal" : "orange"}>
                    {s.processing}
                  </Badge>
                </Group>
                <Code className="revision-code">{s.revision}</Code>
                <Link
                  to="/brains/$brainId/memory"
                  params={{ brainId: brain.id }}
                  search={{ repository: s.repository_id }}
                >
                  Memory in this repository
                </Link>
                <Text size="xs" c="dimmed">
                  Shows current memories whose scope includes this repository.
                  Inspect their exact support before attributing them to this
                  snapshot.
                </Text>
                <Text size="sm">
                  {s.fact_count} facts · {s.file_count} files ·{" "}
                  {s.retained_file_count} retained
                </Text>
                <Text size="xs" c="dimmed">
                  {time(s.created_at)} · {s.adapter} {s.adapter_build}
                </Text>
              </Stack>
            </Card>
          ))}
        {snapshots.data && (
          <Pages
            offset={offset}
            total={snapshots.data.total}
            onChange={setOffset}
          />
        )}
        <Button variant="default" onClick={() => void snapshots.refetch()}>
          Refresh snapshots
        </Button>
      </Stack>
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
  const [tab, setTab] = useState<string | null>("files");
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
    refetchInterval: 3000,
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
    refetchInterval: 4000,
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
    refetchInterval: 4000,
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
  return (
    <Modal opened onClose={onClose} size="xl" title="Repository snapshot">
      <Stack className="publication-panel">
        <Failure error={details.error} />
        {details.isPending && <Loader />}
        {s && (
          <>
            <Group justify="space-between">
              <Badge color={s.processing === "ready" ? "teal" : "orange"}>
                {s.processing}
              </Badge>
              <Text size="xs">Published {time(s.created_at)}</Text>
            </Group>
            <Code className="revision-code">{s.revision}</Code>
            <EraseAction
              brain={brain.id}
              permitted={brain.role === "admin"}
              target={{ kind: "snapshot", id }}
              name={"Snapshot " + s.revision}
            />
            <Text size="sm">
              {s.fact_count} structural facts · {s.file_count} files ·{" "}
              {s.retained_file_count} retained texts
            </Text>
            <Text size="xs" c="dimmed">
              Snapshot {id} · {s.adapter} {s.adapter_build} · extractor{" "}
              {s.extractor_version}
            </Text>
            {brain.role !== "reader" && !brain.archived && (
              <Button
                variant="light"
                loading={process.isPending}
                disabled={
                  s.processing === "queued" || s.processing === "running"
                }
                onClick={() => process.mutate()}
              >
                Reprocess retained artifacts
              </Button>
            )}
            <Failure error={process.error} />
            <Tabs
              value={tab}
              onChange={(v) => {
                setTab(v);
                setSelectedFile(null);
              }}
            >
              <Tabs.List>
                <Tabs.Tab value="files">Files</Tabs.Tab>
                <Tabs.Tab value="facts">Facts</Tabs.Tab>
                <Tabs.Tab value="coverage">Coverage</Tabs.Tab>
                <Tabs.Tab value="contributors">Contributors</Tabs.Tab>
                <Tabs.Tab value="insights">Insights</Tabs.Tab>
                <Tabs.Tab value="receipt">Receipt</Tabs.Tab>
              </Tabs.List>
              <Tabs.Panel value="files" pt="md">
                <Stack>
                  <Failure error={files.error} />
                  {files.isPending && <Loader size="sm" />}
                  {selectedFile ? (
                    <>
                      <Button
                        variant="subtle"
                        onClick={() => setSelectedFile(null)}
                      >
                        Back to files
                      </Button>
                      <Failure error={content.error} />
                      {content.isPending && <Loader size="sm" />}
                      {!content.error && content.data && (
                        <>
                          <Text fw={600}>{content.data.file.path}</Text>
                          {content.data.content !== null &&
                          content.data.content !== undefined ? (
                            <pre className="source-content">
                              {content.data.content}
                            </pre>
                          ) : (
                            <Alert
                              color="yellow"
                              title="File content unavailable"
                            >
                              {content.data.file.availability.replaceAll(
                                "_",
                                " ",
                              )}
                              . The committed file reference is preserved;
                              Recollect does not fetch it from a contributor.
                            </Alert>
                          )}
                        </>
                      )}
                    </>
                  ) : (
                    <>
                      {!files.error && files.data?.items.length === 0 && (
                        <Text>No files in this snapshot.</Text>
                      )}
                      {!files.error &&
                        files.data?.items.map((f) => (
                          <button
                            className="source-row"
                            key={f.id}
                            onClick={() => setSelectedFile(f.id)}
                          >
                            <FileCode2 size={iconSize.navigation} />
                            <span className="source-row-body">
                              <Text size="sm" fw={500}>
                                {f.path}
                              </Text>
                              <Text size="xs" c="dimmed">
                                {f.status.replaceAll("_", " ")} ·{" "}
                                {f.extraction.replaceAll("_", " ")} ·{" "}
                                {f.availability.replaceAll("_", " ")}
                              </Text>
                            </span>
                          </button>
                        ))}
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
                </Stack>
              </Tabs.Panel>
              <Tabs.Panel value="facts" pt="md">
                <Stack>
                  <Failure error={facts.error} />
                  {facts.isPending && <Loader size="sm" />}
                  {!facts.error &&
                    facts.data &&
                    facts.data.processing !== "ready" && (
                      <Alert color="yellow">
                        Fact processing is {facts.data.processing}. Retained
                        extraction artifacts remain available separately.
                      </Alert>
                    )}
                  {!facts.error && facts.data?.items.length === 0 && (
                    <Text>No materialized facts available.</Text>
                  )}
                  {!facts.error &&
                    facts.data?.items.map((f) => (
                      <Card withBorder key={f.id}>
                        <Text size="xs" mb="xs">
                          Record {f.ordinal + 1} · {f.id}
                        </Text>
                        <JsonView value={f.record} />
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
                </Stack>
              </Tabs.Panel>
              <Tabs.Panel value="coverage" pt="md">
                <Stack>
                  <Alert title="Extraction limits">
                    Static code and configuration facts are evidence. Kubernetes
                    YAML has file inventory and optional text, with no supported
                    semantic extraction in this adapter. Unresolved targets
                    remain unresolved.
                  </Alert>
                  <JsonView value={s.coverage} />
                  <Text fw={500}>Extraction settings</Text>
                  <JsonView value={s.settings} />
                </Stack>
              </Tabs.Panel>
              <Tabs.Panel value="contributors" pt="md">
                <Stack>
                  {details.data?.contributors.map((c) => (
                    <Card key={c.id} withBorder>
                      <Stack gap="xs">
                        <Text size="sm">Account {c.actor_id}</Text>
                        <Text size="xs">Device {c.device_id}</Text>
                        <Text size="sm">
                          {c.dirty
                            ? "Working copy was dirty; committed bytes were published."
                            : "Working copy reported clean."}
                        </Text>
                        <Text size="xs">
                          Branch {c.branch || "detached"} · captured{" "}
                          {time(c.captured_at)}
                        </Text>
                        <Text size="xs">
                          Operation {c.operation_id} · scope {c.scope.id}
                        </Text>
                        <Text size="xs">
                          Environment:{" "}
                          {c.scope.environment?.name || "All in this Brain"}
                        </Text>
                      </Stack>
                    </Card>
                  ))}
                  {details.data && (
                    <Pages
                      offset={contributorOffset}
                      total={details.data.contributor_total}
                      onChange={setContributorOffset}
                    />
                  )}
                </Stack>
              </Tabs.Panel>
              {(["insights", "receipt"] as const).map((kind) => (
                <Tabs.Panel key={kind} value={kind} pt="md">
                  <Stack>
                    <Failure error={artifact.error} />
                    {artifact.isPending && <Loader size="sm" />}
                    {!artifact.error &&
                    artifact.data?.content !== null &&
                    artifact.data?.content !== undefined ? (
                      <JsonView value={artifact.data.content} />
                    ) : (
                      !artifact.error &&
                      artifact.data && (
                        <Alert color="yellow">
                          Artifact {artifact.data.availability}.
                        </Alert>
                      )
                    )}
                  </Stack>
                </Tabs.Panel>
              ))}
            </Tabs>
          </>
        )}
      </Stack>
    </Modal>
  );
}

export function PublicationPanel({
  brain,
  section = "environments",
}: {
  brain: Brain;
  section?: "repositories" | "environments";
}) {
  const cache = useQueryClient();
  const [environment, setEnvironment] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const catalogue = useQuery({
    queryKey: ["publication-catalogue", brain.id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
        }),
      ),
    refetchInterval: 5000,
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
    refetchInterval: 4000,
  });
  const refresh = () => {
    void cache.invalidateQueries({
      queryKey: ["revision-manifests", brain.id],
    });
    void cache.invalidateQueries({ queryKey: ["revision-manifest", brain.id] });
  };
  return (
    <section className="feature-view publication-panel">
      <Stack>
        <Group justify="space-between">
          <Group gap="sm">
            <GitCommitHorizontal size={iconSize.action} />
            <Title order={3}>Environment revisions</Title>
          </Group>
          {brain.role !== "reader" && !brain.archived && (
            <Button
              leftSection={<Plus size={iconSize.small} />}
              disabled={
                !catalogue.data?.environments.length ||
                !catalogue.data?.repositories.length
              }
              onClick={() => setCreating(true)}
            >
              New manifest
            </Button>
          )}
        </Group>
        <Text size="sm" c="dimmed">
          Choose exact repository revisions for an environment. Committed code,
          desired configuration and recorded deployment observations keep
          separate provenance.
        </Text>
        <Failure error={catalogue.error} />
        <Select
          clearable
          label="Filter by environment"
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
        <Failure error={manifests.error} />
        {manifests.isPending && <Loader size="sm" />}
        {manifests.data?.items.length === 0 && (
          <Text size="sm" c="dimmed">
            No revision manifests yet. Add an environment and register a
            repository to create an explicit selection.
          </Text>
        )}
        {!manifests.error &&
          !catalogue.error &&
          manifests.data?.items.map((m) => (
            <button
              key={m.manifest_id}
              className="source-row"
              onClick={() => setSelected(m.manifest_id)}
            >
              <GitCommitHorizontal size={iconSize.navigation} />
              <span className="source-row-body">
                <Text fw={500}>{m.name}</Text>
                <Text size="sm">
                  {catalogue.data?.environments.find(
                    (e) => e.id === m.environment_id,
                  )?.name || m.environment_id}{" "}
                  ·{" "}
                  {m.kind === "observed_deployed"
                    ? "Recorded deployment observation"
                    : m.kind}{" "}
                  · {m.entries.length} repositories
                </Text>
                <Text size="xs" c="dimmed">
                  Recorded {time(m.created_at)}
                </Text>
              </span>
            </button>
          ))}
        {manifests.data && (
          <Pages
            offset={offset}
            total={manifests.data.total}
            onChange={setOffset}
          />
        )}
        {manifests.error && (
          <Button variant="subtle" onClick={() => void manifests.refetch()}>
            Retry manifests
          </Button>
        )}
      </Stack>
      {creating && catalogue.data && (
        <ManifestForm
          brain={brain}
          catalogue={catalogue.data}
          onClose={() => setCreating(false)}
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
          onClose={() => setSelected(null)}
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
    refetchInterval: 4000,
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
      <Stack className="publication-panel">
        <Failure error={detail.error} />
        {detail.isPending && <Loader />}
        {selected && (
          <>
            <Group justify="space-between">
              <Title order={4}>{selected.name}</Title>
              <EraseAction
                brain={brain.id}
                permitted={brain.role === "admin"}
                target={{ kind: "manifest", id: selected.manifest_id }}
                name={selected.name}
              />
              <Badge>
                {historical ? "Historical revision" : "Current revision"}
              </Badge>
            </Group>
            <Text>
              {catalogue.environments.find(
                (e) => e.id === selected.environment_id,
              )?.name || selected.environment_id}
            </Text>
            <Text size="sm">
              {selected.kind === "observed_deployed"
                ? "Recorded deployment observation"
                : selected.kind}{" "}
              · {time(selected.created_at)}
            </Text>
            {selected.kind === "observed_deployed" && (
              <Alert color="blue" title="Recorded observation">
                Observed{" "}
                {selected.observed_at ? time(selected.observed_at) : "unknown"}.
                Reference: {selected.observation_reference}. This is
                contributor-provided evidence; Recollect has not independently
                verified deployment.
              </Alert>
            )}
            <Text size="sm">{selected.notes}</Text>
            {selected.entries.map((e) => (
              <Card withBorder key={e.repository_id}>
                <Stack gap="xs">
                  <Text fw={500} size="sm">
                    {catalogue.repositories.find(
                      (r) => r.id === e.repository_id,
                    )?.canonical_origin || e.repository_id}
                  </Text>
                  <Code className="revision-code">{e.revision}</Code>
                  {e.snapshot_id ? (
                    <Button
                      variant="light"
                      onClick={() => setSnapshot(e.snapshot_id!)}
                    >
                      Inspect selected snapshot
                    </Button>
                  ) : (
                    <Text size="sm" c="dimmed">
                      Snapshot unavailable — this revision is recorded without
                      retained extraction.
                    </Text>
                  )}
                  <Text size="xs">
                    Config paths:{" "}
                    {(e.config_paths ?? []).join(", ") || "None specified"}
                  </Text>
                </Stack>
              </Card>
            ))}
            <Text size="xs">
              Recorded by {selected.actor_id} · revision {selected.id}
            </Text>
            {brain.role !== "reader" && !brain.archived && detail.data && (
              <Button onClick={() => setEdit(detail.data!.current)}>
                Edit current selection
              </Button>
            )}
            <Divider />
            <Text fw={600}>Revision history</Text>
            <Button variant="subtle" onClick={() => setHistorical(null)}>
              Show current revision
            </Button>
            {detail.data?.history.map((m) => (
              <Button
                variant={historical?.id === m.id ? "light" : "subtle"}
                key={m.id}
                onClick={() => setHistorical(m)}
              >
                {time(m.created_at)} · {m.kind.replaceAll("_", " ")}
              </Button>
            ))}
            {detail.data && (
              <Pages
                offset={offset}
                total={detail.data.total}
                onChange={setOffset}
              />
            )}
          </>
        )}
      </Stack>
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
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate();
        }}
      >
        <Stack className="publication-panel">
          <TextInput
            label="Manifest name"
            required
            maxLength={120}
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
            disabled={!!current}
          />
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
          {kind === "observed_deployed" && (
            <>
              <TextInput
                label="Observation time (ISO date and timezone)"
                placeholder="2026-09-14T12:00:00Z"
                required
                value={observed}
                onChange={(e) => setObserved(e.currentTarget.value)}
              />
              <TextInput
                label="Supporting observation reference"
                required
                value={reference}
                onChange={(e) => setReference(e.currentTarget.value)}
                maxLength={2000}
              />
            </>
          )}
          {entries.map((entry, index) => (
            <EntryForm
              key={index}
              brain={brain}
              catalogue={catalogue}
              entry={entry}
              index={index}
              onChange={(value) =>
                setEntries((v) =>
                  v.map((old, i) => (i === index ? value : old)),
                )
              }
              onRemove={() =>
                setEntries((v) => v.filter((_, i) => i !== index))
              }
            />
          ))}
          <Button
            variant="light"
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
            Add repository entry
          </Button>
          <Textarea
            label="Notes"
            value={notes}
            onChange={(e) => setNotes(e.currentTarget.value)}
            maxLength={2048}
          />
          <Failure error={save.error} />
          {save.error && current && (
            <Text size="sm">
              If this manifest changed, close this editor and reopen the current
              selection before saving again.
            </Text>
          )}
          <Group justify="flex-end">
            <Button variant="default" type="button" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={!valid} loading={save.isPending}>
              Save manifest revision
            </Button>
          </Group>
        </Stack>
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
    <Card withBorder>
      <Stack>
        <Group justify="space-between">
          <Text fw={500}>Repository entry {index + 1}</Text>
          <Button variant="subtle" color="red" type="button" onClick={onRemove}>
            Remove entry
          </Button>
        </Group>
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
        <TextInput
          label={`Exact commit ${index + 1}`}
          required
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
          <Text size="xs" c="dimmed">
            This selection records a revision without retained snapshot
            evidence.
          </Text>
        )}
        <Textarea
          label={`Configuration paths ${index + 1}`}
          description="One relative path per line"
          value={(entry.config_paths ?? []).join("\n")}
          onChange={(e) =>
            onChange({
              ...entry,
              config_paths: e.currentTarget.value.split("\n"),
            })
          }
        />
      </Stack>
    </Card>
  );
}

export function RepositoryStoragePolicy({ brain }: { brain: Brain }) {
  const cache = useQueryClient();
  const policy = useQuery({
    queryKey: ["repository-policy", brain.id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/repositories/policy", {
          params: { path: { brain: brain.id } },
        }),
      ),
  });
  const updatePolicy = useMutation({
    mutationFn: async (allowed: boolean) =>
      result(
        await client.PUT("/api/brains/{brain}/repositories/policy", {
          params: { path: { brain: brain.id } },
          body: { allow_file_content: allowed },
        }),
      ),
    onSuccess: (value) =>
      cache.setQueryData(["repository-policy", brain.id], value),
  });

  return (
    <section className="feature-setting">
      <Stack gap="md">
        <Title order={3}>Repository file storage</Title>
        <Text size="sm" c="dimmed">
          Published structure is separate from retained file text. Files require
          explicit companion selection and this Brain's permission.
        </Text>
        <Failure error={policy.error} />
        <Failure error={updatePolicy.error} />
        {!policy.error &&
          policy.data &&
          (brain.role === "admin" && !brain.archived ? (
            <Switch
              label="Allow explicitly selected repository file text"
              description="Applies to new publications. Turning this off preserves previously retained evidence."
              checked={
                updatePolicy.isPending
                  ? updatePolicy.variables
                  : policy.data.allow_file_content
              }
              disabled={updatePolicy.isPending}
              onChange={(e) => updatePolicy.mutate(e.currentTarget.checked)}
            />
          ) : (
            <Text size="sm">
              Repository text capture:{" "}
              {policy.data.allow_file_content
                ? "allowed for explicitly selected files"
                : "disabled"}
            </Text>
          ))}
      </Stack>
    </section>
  );
}
