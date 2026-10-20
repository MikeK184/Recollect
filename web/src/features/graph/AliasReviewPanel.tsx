import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Spinner,
  StatusBadge,
} from "../../components/ui";
import { OrganizationQuote } from "./OrganizationSources";
import type { components } from "../../api-schema";
import { client, result, type Brain } from "../../api";
import { useContentDeadline } from "../../useContentDeadline";
import { useIdempotency } from "../../useIdempotency";
import { useQualifiedRead } from "./useQualifiedRead";

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
  const [read, setRead] = useQualifiedRead<View>({
    key: requestKey,
    fallback: "Alias evidence could not be read.",
    read: async (signal) => {
      const next = result(
        await client.POST("/api/brains/{brain}/organization/aliases/view", {
          params: { path: { brain: brain.id } },
          body: { scope, cursor, limit: 25 },
          signal,
        }),
      );
      if (next.brain_id !== brain.id || next.epoch !== epoch)
        throw Error(
          "Organization inputs changed. Read the current organization again.",
        );
      return next;
    },
    onError: (error) => callbacks.current.onFailure(error),
  });
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
  useEffect(
    () => () => {
      active.current = "";
    },
    [],
  );
  const writable = brain.role === "admin" || brain.role === "writer";
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
      setRead({ key: "", pending: false });
      setNonce((value) => value + 1);
    } catch (error) {
      if (active.current !== key) return;
      const failure =
        error instanceof Error ? error : new Error("Alias review failed.");
      setRead({ key, error: failure, pending: false });
      callbacks.current.onFailure(failure);
    } finally {
      if (active.current === key) setBusy(false);
    }
  }
  const sources = new Map(
    view?.evidence.map((source) => [source.key, source]) ?? [],
  );
  return (
    <div className="grid gap-4" role="region" aria-label="Alias evidence review">
      <p className="m-0 text-body text-foreground-secondary">
        Review source-described alias navigation. Original entity identities
        stay separate; an alias decision does not establish accepted operational
        truth.
      </p>
      {!writable && (
        <p className="m-0 text-body text-muted-foreground">
          Reader access · evidence inspection is available. Writers can record
          alias decisions.
        </p>
      )}
      {read.error && (
        <Alert variant="destructive" title="Alias evidence unavailable">
          {read.error.message}
        </Alert>
      )}
      {!view && !read.error && !expired && (
        <p
          role="status"
          className="m-0 flex items-center gap-2 text-body text-foreground-secondary"
        >
          <Spinner />
          Reading current alias evidence…
        </p>
      )}
      {view && (
        <>
          {!view.candidates.length && (
            <p className="m-0 text-body text-muted-foreground">
              No qualified alias candidates in this page and scope.
            </p>
          )}
          {view.candidates.map((candidate) => {
            const source = sources.get(candidate.source_key);
            const supported =
              candidate.acceptance_state === "explicit_positive_statement";
            return (
              <Card
                key={candidate.id}
                className="grid gap-2.5 [overflow-wrap:anywhere]"
                data-testid="alias-candidate"
              >
                <div className="flex items-start justify-between gap-2">
                  <h3 className="m-0 text-title font-semibold">
                    {candidate.from.label} ↔ {candidate.to.label}
                  </h3>
                  <StatusBadge
                    state={
                      candidate.disposition === "accepted"
                        ? "positive"
                        : candidate.disposition === "rejected"
                          ? "negative"
                          : "neutral"
                    }
                  >
                    {candidate.disposition}
                  </StatusBadge>
                </div>
                <p className="m-0 text-caption text-muted-foreground">
                  {candidate.from.kind.replaceAll("_", " ")} ·{" "}
                  <span className="mono">{candidate.from.id}</span>
                  <br />
                  {candidate.to.kind.replaceAll("_", " ")} ·{" "}
                  <span className="mono">{candidate.to.id}</span>
                </p>
                <p className="m-0 text-body">
                  {support(candidate.acceptance_state)}
                </p>
                <p className="m-0 text-caption text-muted-foreground">
                  {source?.evidence.label ?? "Qualified source evidence"} ·
                  Exact UTF-8 bytes{" "}
                  <span className="mono">
                    {candidate.byte_start}–{candidate.byte_end}
                  </span>
                </p>
                <OrganizationQuote>{candidate.quote}</OrganizationQuote>
                <p className="mono m-0 text-caption text-muted-foreground">
                  Candidate {candidate.id}
                  <br />
                  Input {candidate.input_id}
                  <br />
                  Source {candidate.source_key}
                  <br />
                  Review version{" "}
                  {candidate.change_id ?? "No recorded decision"}
                </p>
                <div className="flex flex-wrap gap-2">
                  <Button
                    variant="secondary"
                    size="sm"
                    disabled={!source || busy}
                    onClick={() => source && onInspect(source)}
                  >
                    Inspect exact alias source
                  </Button>
                  {writable && (
                    <>
                      {candidate.disposition !== "accepted" && (
                        <Button
                          size="sm"
                          disabled={busy || !supported}
                          onClick={() => void decide(candidate, "accept")}
                        >
                          Accept alias navigation
                        </Button>
                      )}
                      {candidate.disposition !== "rejected" && (
                        <Button
                          variant="secondary"
                          size="sm"
                          disabled={busy}
                          onClick={() => void decide(candidate, "reject")}
                        >
                          Reject alias
                        </Button>
                      )}
                      {candidate.disposition !== "pending" && (
                        <Button
                          variant="ghost"
                          size="sm"
                          disabled={busy}
                          onClick={() => void decide(candidate, "revoke")}
                        >
                          Revoke alias decision
                        </Button>
                      )}
                    </>
                  )}
                </div>
              </Card>
            );
          })}
          <div className="flex flex-wrap items-center justify-between gap-2">
            <Button
              variant="ghost"
              size="sm"
              disabled={!history.length || busy}
              onClick={() => {
                setCursor(history[history.length - 1]);
                setHistory((old) => old.slice(0, -1));
              }}
            >
              Previous aliases
            </Button>
            <p className="m-0 text-caption text-muted-foreground">
              Page <span className="mono">{history.length + 1}</span> · up to
              25 candidates
            </p>
            <Button
              variant="ghost"
              size="sm"
              disabled={!view.next_cursor || busy}
              onClick={() => {
                setHistory((old) => [...old, cursor]);
                setCursor(view.next_cursor ?? null);
              }}
            >
              Next aliases
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
