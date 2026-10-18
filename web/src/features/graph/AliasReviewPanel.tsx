import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Loader,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import type { components } from "../../api-schema";
import { client, result, type Brain } from "../../api";
import { useContentDeadline } from "../../useContentDeadline";
import { useIdempotency } from "../../useIdempotency";

type Scope = components["schemas"]["GraphSelection"];
type View = components["schemas"]["AliasView"];
type Cursor = components["schemas"]["AliasCursor"];
type Candidate = components["schemas"]["AliasCandidateView"];
type Action = components["schemas"]["AliasAction"];
type Source = components["schemas"]["GraphNode"];

function support(state: string) {
  if (state === "explicit_positive_statement")
    return "Supported standalone positive statement";
  if (state === "incompatible_identity_scope")
    return "The entity kinds or identity scopes are incompatible. Acceptance is unavailable.";
  if (state === "same_identity")
    return "Both mentions already identify the same entity. An alias decision is unnecessary.";
  if (state === "source_unavailable")
    return "The canonical source is unavailable for exact statement verification. Acceptance is unavailable.";
  if (state === "unsupported_context")
    return "This source does not match the supported standalone positive statement. Negative, historical, quoted and uncertain contexts remain available for review; acceptance is unavailable.";
  return `Acceptance unavailable: ${state.replaceAll("_", " ")}`;
}

/** Human review of qualified alias evidence; entity identities stay separate. */
export function AliasReviewPanel({
  brain,
  scope,
  epoch,
  qualification,
  onInspect,
  onChanged,
  onFailure,
}: {
  brain: Brain;
  scope: Scope;
  epoch: number;
  qualification: string;
  onInspect: (source: Source) => void;
  onChanged: () => void;
  onFailure: (error: Error) => void;
}) {
  const [cursor, setCursor] = useState<Cursor | null>(null),
    [history, setHistory] = useState<(Cursor | null)[]>([]),
    [nonce, setNonce] = useState(0),
    [busy, setBusy] = useState(false);
  const [read, setRead] = useState<{ key: string; view?: View; error?: Error }>(
    { key: "" },
  );
  const requestKey = JSON.stringify([
    brain.id,
    scope,
    qualification,
    cursor,
    nonce,
  ]);
  const active = useRef(requestKey);
  active.current = requestKey;
  const callbacks = useRef({ onChanged, onFailure });
  callbacks.current = { onChanged, onFailure };
  const idem = useIdempotency();
  const expired = useContentDeadline(
    read.key === requestKey ? read.view?.expires_at : null,
  );
  const view =
    read.key === requestKey &&
    !read.error &&
    !expired &&
    read.view?.brain_id === brain.id &&
    read.view.epoch === epoch
      ? read.view
      : undefined;
  const writable = brain.role === "admin" || brain.role === "writer";
  useEffect(() => {
    const controller = new AbortController();
    active.current = requestKey;
    setRead({ key: requestKey });
    void client
      .POST("/api/brains/{brain}/organization/aliases/view", {
        params: { path: { brain: brain.id } },
        body: { scope, cursor, limit: 25 },
        signal: controller.signal,
      })
      .then((response) => {
        const next = result(response);
        if (controller.signal.aborted || active.current !== requestKey) return;
        if (next.brain_id !== brain.id || next.epoch !== epoch)
          throw Error(
            "Organization inputs changed. Read the current organization again.",
          );
        setRead({ key: requestKey, view: next });
      })
      .catch((error) => {
        if (controller.signal.aborted || active.current !== requestKey) return;
        const failure =
          error instanceof Error
            ? error
            : new Error("Alias evidence could not be read.");
        setRead({ key: requestKey, error: failure });
        callbacks.current.onFailure(failure);
      });
    return () => {
      controller.abort();
      if (active.current === requestKey) active.current = "";
    };
  }, [requestKey]);
  useEffect(() => {
    if (expired)
      callbacks.current.onFailure(
        new Error(
          "Alias evidence reached its retention deadline. Read the current organization again.",
        ),
      );
  }, [expired]);
  async function decide(candidate: Candidate, action: Action) {
    if (!view || busy || !writable) return;
    const key = requestKey;
    const body = { base_change_id: candidate.change_id ?? null, action };
    setBusy(true);
    try {
      result(
        await client.PUT("/api/brains/{brain}/organization/aliases/{id}", {
          params: { path: { brain: brain.id, id: candidate.id } },
          body,
          headers: {
            "Idempotency-Key": idem.forInput({
              brain: brain.id,
              id: candidate.id,
              body,
            }),
          },
        }),
      );
      // A completed human command changes canonical state even if its drawer
      // was closed while the response was in flight. Invalidate that Brain's
      // reads without restoring a former drawer, selection, or scope.
      callbacks.current.onChanged();
      if (active.current !== key) return;
      idem.reset();
      setRead({ key: "" });
      setNonce((value) => value + 1);
    } catch (error) {
      if (active.current !== key) return;
      const failure =
        error instanceof Error ? error : new Error("Alias review failed.");
      setRead({ key, error: failure });
      callbacks.current.onFailure(failure);
    } finally {
      if (active.current === key) setBusy(false);
    }
  }
  const sources = new Map(
    view?.evidence.map((source) => [source.key, source]) ?? [],
  );
  return (
    <Stack gap="md" role="region" aria-label="Alias evidence review">
      <Text size="sm">
        Review source-described alias navigation. Original entity identities
        stay separate; an alias decision does not establish accepted operational
        truth.
      </Text>
      {!writable && (
        <Text size="sm" c="dimmed">
          Reader access · evidence inspection is available. Writers can record
          alias decisions.
        </Text>
      )}
      {read.error && (
        <Alert color="red" title="Alias evidence unavailable">
          {read.error.message}
        </Alert>
      )}
      {!view && !read.error && !expired && (
        <Group role="status">
          <Loader size="sm" />
          <Text>Reading current alias evidence…</Text>
        </Group>
      )}
      {view && (
        <>
          {!view.candidates.length && (
            <Text size="sm" c="dimmed">
              No qualified alias candidates in this page and scope.
            </Text>
          )}
          {view.candidates.map((candidate) => {
            const source = sources.get(candidate.source_key);
            const supported =
              candidate.acceptance_state === "explicit_positive_statement";
            return (
              <Card
                key={candidate.id}
                withBorder
                padding="md"
                data-testid="alias-candidate"
              >
                <Stack gap="sm">
                  <Group justify="space-between">
                    <Title order={4}>
                      {candidate.from.label} ↔ {candidate.to.label}
                    </Title>
                    <Badge
                      variant="light"
                      color={
                        candidate.disposition === "accepted" ? "teal" : "gray"
                      }
                    >
                      {candidate.disposition}
                    </Badge>
                  </Group>
                  <Text size="xs" c="dimmed">
                    {candidate.from.kind.replaceAll("_", " ")} ·{" "}
                    {candidate.from.id}
                    <br />
                    {candidate.to.kind.replaceAll("_", " ")} · {candidate.to.id}
                  </Text>
                  <Text size="sm">{support(candidate.acceptance_state)}</Text>
                  <Text size="xs" c="dimmed">
                    {source?.evidence.label ?? "Qualified source evidence"} ·
                    Exact UTF-8 bytes {candidate.byte_start}–
                    {candidate.byte_end}
                  </Text>
                  <pre className="organization-quote">{candidate.quote}</pre>
                  <Text size="xs" c="dimmed">
                    Candidate {candidate.id}
                    <br />
                    Input {candidate.input_id}
                    <br />
                    Source {candidate.source_key}
                    <br />
                    Review version{" "}
                    {candidate.change_id ?? "No recorded decision"}
                  </Text>
                  <Group>
                    <Button
                      variant="light"
                      size="xs"
                      disabled={!source || busy}
                      onClick={() => source && onInspect(source)}
                    >
                      Inspect exact alias source
                    </Button>
                    {writable && (
                      <>
                        {candidate.disposition !== "accepted" && (
                          <Button
                            size="xs"
                            disabled={busy || !supported}
                            onClick={() => void decide(candidate, "accept")}
                          >
                            Accept alias navigation
                          </Button>
                        )}
                        {candidate.disposition !== "rejected" && (
                          <Button
                            variant="default"
                            size="xs"
                            disabled={busy}
                            onClick={() => void decide(candidate, "reject")}
                          >
                            Reject alias
                          </Button>
                        )}
                        {candidate.disposition !== "pending" && (
                          <Button
                            variant="subtle"
                            size="xs"
                            disabled={busy}
                            onClick={() => void decide(candidate, "revoke")}
                          >
                            Revoke alias decision
                          </Button>
                        )}
                      </>
                    )}
                  </Group>
                </Stack>
              </Card>
            );
          })}
          <Group justify="space-between">
            <Button
              variant="subtle"
              size="xs"
              disabled={!history.length || busy}
              onClick={() => {
                setCursor(history[history.length - 1]);
                setHistory((old) => old.slice(0, -1));
              }}
            >
              Previous aliases
            </Button>
            <Text size="xs" c="dimmed">
              Page {history.length + 1} · up to 25 candidates
            </Text>
            <Button
              variant="subtle"
              size="xs"
              disabled={!view.next_cursor || busy}
              onClick={() => {
                setHistory((old) => [...old, cursor]);
                setCursor(view.next_cursor ?? null);
              }}
            >
              Next aliases
            </Button>
          </Group>
        </>
      )}
    </Stack>
  );
}
