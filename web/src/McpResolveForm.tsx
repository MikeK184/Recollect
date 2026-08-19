import { useState } from "react";
import { Alert, Button, Group, Select, Stack, Textarea } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { client, result, RequestError } from "./api";
import type { components } from "./api-schema";
type Input = components["schemas"]["McpResolveInput"];
export function McpResolveForm({
  brain,
  call,
  saved,
}: {
  brain: string;
  call: string;
  saved: () => void;
}) {
  const [offset, setOffset] = useState(0);
  const [versionOffset, setVersionOffset] = useState(0);
  const [source, setSource] = useState<string | null>(null);
  const [version, setVersion] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<string | null>("unknown");
  const [explanation, setExplanation] = useState("");
  const [pending, setPending] = useState<Input | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sources = useQuery({
    queryKey: ["mcp", brain, "resolution-sources", offset],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          params: { path: { brain }, query: { offset } },
          signal,
        }),
      ),
    retry: false,
    gcTime: 0,
  });
  const versions = useQuery({
    queryKey: ["mcp", brain, "resolution-versions", source, versionOffset],
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/sources/{source}/versions", {
          params: {
            path: { brain, source: source! },
            query: { offset: versionOffset },
          },
          signal,
        }),
      ),
    enabled: !!source,
    retry: false,
    gcTime: 0,
  });
  const submit = async () => {
    if (!version || !outcome) return;
    setBusy(true);
    setError(null);
    try {
      const input = pending ?? {
        request_id: crypto.randomUUID(),
        outcome,
        explanation,
        source_version_id: version,
      };
      setPending(input);
      result(
        await client.POST("/api/brains/{brain}/mcp/calls/{id}/resolve", {
          params: { path: { brain, id: call } },
          body: input,
        }),
      );
      saved();
    } catch (e) {
      if (e instanceof RequestError && e.status === 400) setPending(null);
      setError(
        e instanceof Error ? e.message : "Evidence could not be recorded.",
      );
    } finally {
      setBusy(false);
    }
  };
  return (
    <Stack aria-label="Reconciliation evidence">
      <Select
        label="Evidence source"
        searchable
        value={source}
        disabled={busy || !!pending}
        data={
          sources.error
            ? []
            : (sources.data?.sources.map((s) => ({
                value: s.id,
                label: s.version.title,
              })) ?? [])
        }
        onChange={(v) => {
          setSource(v);
          setVersion(null);
          setVersionOffset(0);
        }}
      />
      {(offset > 0 || (sources.data?.total ?? 0) > 50) && (
        <Group>
          <Button
            size="xs"
            variant="light"
            disabled={offset === 0 || !!pending}
            onClick={() => {
              setOffset(Math.max(0, offset - 50));
              setSource(null);
              setVersion(null);
            }}
          >
            Previous sources
          </Button>
          <Button
            size="xs"
            variant="light"
            disabled={offset + 50 >= (sources.data?.total ?? 0) || !!pending}
            onClick={() => {
              setOffset(offset + 50);
              setSource(null);
              setVersion(null);
            }}
          >
            More sources
          </Button>
        </Group>
      )}
      <Select
        label="Retained evidence version"
        value={version}
        disabled={busy || !!pending}
        data={
          versions.error
            ? []
            : (versions.data?.versions
                .filter((v) => v.availability === "retained")
                .map((v) => ({
                  value: v.id,
                  label: `${v.title} · ${new Date(v.created_at).toLocaleString()}`,
                })) ?? [])
        }
        onChange={setVersion}
      />
      {(versionOffset > 0 || (versions.data?.total ?? 0) > 20) && (
        <Group>
          <Button
            size="xs"
            variant="light"
            disabled={versionOffset === 0 || !!pending}
            onClick={() => {
              setVersionOffset(Math.max(0, versionOffset - 20));
              setVersion(null);
            }}
          >
            Newer versions
          </Button>
          <Button
            size="xs"
            variant="light"
            disabled={
              versionOffset + 20 >= (versions.data?.total ?? 0) || !!pending
            }
            onClick={() => {
              setVersionOffset(versionOffset + 20);
              setVersion(null);
            }}
          >
            Older versions
          </Button>
        </Group>
      )}
      <Select
        label="Observed outcome"
        value={outcome}
        disabled={busy || !!pending}
        onChange={setOutcome}
        data={[
          { value: "succeeded", label: "Succeeded" },
          { value: "failed", label: "Failed" },
          { value: "not_executed", label: "Not executed" },
          { value: "unknown", label: "Still unknown" },
        ]}
      />
      <Textarea
        label="Evidence explanation"
        description="Explain what this retained source establishes. The original missing response remains recorded."
        maxLength={2000}
        value={explanation}
        disabled={busy || !!pending}
        onChange={(e) => setExplanation(e.currentTarget.value)}
      />
      {(error || sources.error || versions.error) && (
        <Alert color="red">
          {error ?? sources.error?.message ?? versions.error?.message}
        </Alert>
      )}
      <Button
        disabled={
          !version || !explanation.trim() || !!sources.error || !!versions.error
        }
        loading={busy}
        onClick={() => void submit()}
      >
        {pending ? "Check evidence submission" : "Save evidence resolution"}
      </Button>
    </Stack>
  );
}
