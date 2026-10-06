import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Checkbox,
  Code,
  Drawer,
  Group,
  Loader,
  MultiSelect,
  Select,
  SimpleGrid,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from "@mantine/core";
import { useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
  ArrowUp,
  Check,
  Copy,
  FileText,
  MessageSquare,
  RotateCcw,
  Search,
  SlidersHorizontal,
  Square,
  X,
} from "lucide-react";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { useBrain } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
import { FeatureTabs, useFeatureTab } from "../../components/FeatureTabs";
import { ErrorState } from "../../components/AsyncState";
import { Brand } from "../../components/Brand";
import { RecallPanel } from "../../RecallPanel";
import { ClaimDialog, EvidenceDialog } from "../../ClaimsPanel";
import { recallEvidence, RecallScope } from "../../RecallResults";
import { useContentDeadline } from "../../useContentDeadline";
import { ManagedMemoryPanel } from "../settings/ManagedMemoryPanel";
import { Markdown } from "../../components/Markdown";
import { EvidenceNotes } from "../../components/EvidenceNotes";
import "./ask.css";

type Request = components["schemas"]["RecallRequest"];
type Answer = components["schemas"]["AnswerResponse"];
type Citation = components["schemas"]["AnswerCitation"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type Turn = {
  id: string;
  question: string;
  request: Request;
  response: Answer;
};
const tabs = [
  { value: "ask", label: "Answer" },
  { value: "search", label: "Find evidence" },
] as const;
const label = (s: string) => s.replaceAll("_", " ");
const initialRequest = (): Request => ({
  query: "",
  exact: null,
  operation_id: null,
  selection: { repository_ids: [], area_ids: [], environment_id: null },
  collection_id: null,
  manifest_revision_id: null,
  knowledge_at: null,
  fact_at: null,
  mode: "investigation",
  channels: ["exact", "lexical"],
  strategy: "auto",
  semantic_request_id: null,
  semantic_min_similarity: null,
  graph: null,
  source_diversity: true,
  limit: 10,
  context_bytes: 8192,
});
const failureText: Record<string, string> = {
  answer_insufficient_support:
    "There is not enough eligible evidence to answer this question. Try naming a specific subject or use Find evidence.",
  model_policy_denied:
    "This Brain’s model policy does not permit this answer. Find evidence remains available.",
  model_credentials_missing:
    "The installed model has no available credential. An administrator can check the installation.",
  model_budget_exhausted:
    "The model budget is exhausted. No additional attempt will be made automatically.",
  model_concurrency_limit:
    "The model is busy with this Brain’s other requests. Try a new attempt when it has finished.",
  provider_timeout:
    "The provider did not finish in time. It may have processed this request; no automatic retry was made.",
  answer_stale:
    "Evidence or permissions changed during this answer. Its content was discarded.",
  answer_citations_invalid:
    "The model returned unsupported citations. Its answer was discarded.",
};

export function AskPage() {
  const brain = useBrain();
  const [draft, setDraft] = useState("");
  const [tab, setTab] = useFeatureTab(
    tabs.map((t) => t.value),
    "ask",
  );
  useEffect(() => setDraft(""), [brain.id]);
  return (
    <>
      <PageHeader title="Ask" />
      <div className="ask-intents">
        <FeatureTabs tabs={tabs} value={tab} onChange={setTab}>
          {tab === "search" ? (
            <RecallPanel
              key={brain.id}
              brain={brain}
              draft={draft}
              setDraft={setDraft}
            />
          ) : (
            <AskConversation
              key={brain.id}
              brain={brain}
              question={draft}
              setQuestion={setDraft}
              search={(query) => {
                setDraft(query ?? "");
                setTab("search");
              }}
            />
          )}
        </FeatureTabs>
      </div>
    </>
  );
}

function AskConversation({
  brain,
  search,
  question,
  setQuestion,
}: {
  brain: Brain;
  search: (query?: string) => void;
  question: string;
  setQuestion: (value: string) => void;
}) {
  const cache = useQueryClient();
  const [request, setRequest] = useState<Request>(initialRequest);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [turns, setTurns] = useState<Turn[]>([]);
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [receipt, setReceipt] = useState<Answer | null>(null);
  const [interruptedId, setInterruptedId] = useState<string | null>(null);
  const [selected, setSelected] = useState<{
    citation: Citation;
    turn: Turn;
  } | null>(null);
  const [detail, setDetail] = useState<{
    claim?: string;
    evidence?: Evidence;
    knowledge?: string;
    fact?: string;
  } | null>(null);
  const active = useRef<{ id: string; controller: AbortController } | null>(
    null,
  );
  const statusCheck = useRef(0);
  const mounted = useRef(true);
  const policies = useQuery({
    queryKey: ["models-policy", brain.id],
    refetchInterval: 4000,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/models/policy", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
  });
  const policy = !policies.isError ? policies.data?.current.policy : undefined;
  const allowed =
    !!policy?.enabled &&
    policy.purposes.includes("answering") &&
    policy.content_classes.includes("query") &&
    !!policies.data?.installed.credentials_present;
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id, "ask"],
    enabled: filtersOpen || turns.length > 0,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
  });
  const validity = useQueries({
    queries: turns.map((turn) => ({
      queryKey: ["answer-status", brain.id, turn.id],
      gcTime: 0,
      retry: false,
      refetchInterval: 3000,
      queryFn: async ({ signal }: { signal: AbortSignal }) =>
        result(
          await client.GET("/api/brains/{brain}/answer-requests/{id}", {
            params: { path: { brain: brain.id, id: turn.id } },
            signal,
          }),
        ),
    })),
  });
  const deadline = turns
    .map((t) => t.response.expires_at)
    .filter((d): d is string => !!d)
    .sort()[0];
  const expired = useContentDeadline(deadline);
  const validityKey = validity
    .map((v) => `${v.dataUpdatedAt}:${v.errorUpdatedAt}:${v.data?.state}`)
    .join("|");

  function stop() {
    const running = active.current;
    active.current = null;
    if (!running) return;
    running.controller.abort();
    setPending(null);
    setInterruptedId(running.id);
    setNotice(
      "Stopped displaying this request. Any provider work already sent may still be charged.",
    );
    void client.POST("/api/brains/{brain}/answer-requests/{id}/cancel", {
      params: { path: { brain: brain.id, id: running.id } },
    });
  }
  function clear(message?: string, keepDraft = false) {
    statusCheck.current += 1;
    stop();
    setTurns([]);
    setSelected(null);
    setDetail(null);
    setReceipt(null);
    setError(null);
    if (!keepDraft) setQuestion("");
    setNotice(message ?? null);
  }
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      statusCheck.current += 1;
      const running = active.current;
      active.current = null;
      if (running) {
        running.controller.abort();
        void client.POST("/api/brains/{brain}/answer-requests/{id}/cancel", {
          params: { path: { brain: brain.id, id: running.id } },
        });
      }
    };
  }, [brain.id]);
  useEffect(() => {
    if (
      expired ||
      validity.some(
        (v) =>
          v.isError ||
          (v.data && !["completed", "no_evidence"].includes(v.data.state)),
      )
    ) {
      clear(
        "This conversation’s evidence, access, or model policy changed. Ask again for current information.",
      );
    }
  }, [validityKey, expired]);
  useEffect(() => {
    if (
      (policies.isError ||
        (!policies.isPending && !allowed) ||
        brain.archived) &&
      (turns.length || pending)
    )
      clear(
        "Answering is unavailable under the current Brain policy. Previous answers have been cleared.",
      );
  }, [policies.dataUpdatedAt, policies.errorUpdatedAt, brain.archived]);
  useEffect(
    () =>
      cache.getQueryCache().subscribe((event) => {
        if (event.query.queryKey[1] === brain.id && event.type === "updated") {
          const key = String(event.query.queryKey[0]);
          if (
            event.action.type === "invalidate" &&
            ["recall", "claims", "models-policy"].includes(key)
          ) {
            clear(
              "Memory or its policy changed. Ask again for current information.",
            );
          }
          if (
            event.action.type === "error" &&
            ["claim", "claim-evidence-detail"].includes(key)
          ) {
            clear(
              "Supporting evidence could not be revalidated. Ask again for current information.",
            );
          }
        }
      }),
    [cache, brain.id],
  );

  async function ask() {
    if (!allowed || brain.archived || pending || !question.trim()) return;
    const text = question.trim();
    if (new TextEncoder().encode(text).length > 512) {
      setError(
        new Error(
          "Keep the question within 512 UTF-8 bytes. Shorten it and try again.",
        ),
      );
      return;
    }
    const id = crypto.randomUUID();
    const submittedRequest: Request = {
      ...request,
      ...(request.strategy === "auto"
        ? {
            query: "",
            channels: ["exact", "lexical"],
            graph: null,
            semantic_min_similarity: null,
          }
        : {}),
      semantic_request_id:
        request.strategy === "auto" || request.channels?.includes("semantic")
          ? id
          : null,
    };
    statusCheck.current += 1;
    const controller = new AbortController();
    active.current = { id, controller };
    setPending(text);
    setQuestion("");
    setError(null);
    setNotice(null);
    setReceipt(null);
    setInterruptedId(null);
    setSelected(null);
    setDetail(null);
    try {
      const response = result(
        await client.POST("/api/brains/{brain}/answer-requests", {
          params: { path: { brain: brain.id } },
          body: {
            request_id: id,
            question: text,
            recall: submittedRequest,
          },
          signal: controller.signal,
        }),
      );
      if (!mounted.current || active.current?.id !== id) return;
      if (
        response.state === "completed" &&
        response.answer &&
        response.recall
      ) {
        setTurns((prior) =>
          [
            ...prior.filter(
              (t) => t.response.memory_epoch === response.memory_epoch,
            ),
            { id, question: text, request: submittedRequest, response },
          ].slice(-4),
        );
      } else {
        setReceipt(response);
        setQuestion(text);
      }
    } catch (e) {
      if (!mounted.current || active.current?.id !== id) return;
      setError(
        e instanceof Error ? e : new Error("The answer could not be received."),
      );
      setInterruptedId(id);
      setQuestion(text);
    } finally {
      if (mounted.current && active.current?.id === id) {
        active.current = null;
        setPending(null);
      }
    }
  }
  async function checkReceipt() {
    if (!interruptedId) return;
    const version = ++statusCheck.current;
    try {
      const value = result(
        await client.GET("/api/brains/{brain}/answer-requests/{id}", {
          params: { path: { brain: brain.id, id: interruptedId } },
        }),
      );
      if (!mounted.current || version !== statusCheck.current) return;
      setReceipt(value);
      setError(null);
    } catch (e) {
      if (mounted.current && version === statusCheck.current)
        setError(
          e instanceof Error ? e : new Error("Request status is unavailable."),
        );
    }
  }
  const selectedCount =
    (request.selection?.repository_ids?.length ?? 0) +
    (request.selection?.area_ids?.length ?? 0) +
    (request.selection?.environment_id ? 1 : 0) +
    (request.collection_id ? 1 : 0);
  return (
    <div className={`ask-page ${!turns.length && !pending ? "is-empty" : ""}`}>
      <Group justify="space-between" className="ask-toolbar">
        <Group gap="xs">
          <Badge color="gray">
            {selectedCount ? `${selectedCount} scope filters` : "Entire Brain"}
          </Badge>
          {request.mode !== "investigation" && (
            <Badge color="brand">
              {label(request.mode ?? "investigation")}
            </Badge>
          )}
          {(request.knowledge_at || request.fact_at) && (
            <Badge color="yellow">Time filtered</Badge>
          )}
          <Button
            variant="default"
            size="xs"
            leftSection={<SlidersHorizontal size={15} />}
            onClick={() => setFiltersOpen(true)}
          >
            Refine
          </Button>
        </Group>
        <Button
          variant="subtle"
          size="xs"
          leftSection={<RotateCcw size={15} />}
          onClick={() => {
            clear();
            setInterruptedId(null);
          }}
          disabled={!turns.length && !pending && !question}
        >
          Clear conversation
        </Button>
      </Group>
      <ErrorState
        error={policies.error}
        retry={() => void policies.refetch()}
      />
      {notice && (
        <Alert color="yellow" mt="md">
          {notice}
        </Alert>
      )}
      <div className={`ask-body ${selected ? "with-inspector" : ""}`}>
        <div
          className="ask-conversation"
          aria-live="polite"
          aria-busy={!!pending}
        >
          {!turns.length && !pending && (
            <div className="ask-welcome">
              <Brand compact />
              <Title order={2}>What would you like to know?</Title>
              <div className="question-suggestions">
                {[
                  "What decisions have we made?",
                  "What do we know about this project?",
                  "Which issues remain unresolved?",
                ].map((q) => (
                  <button key={q} onClick={() => setQuestion(q)}>
                    <MessageSquare size={16} />
                    {q}
                  </button>
                ))}
              </div>
            </div>
          )}
          {turns.map((turn) => (
            <AnswerTurn
              key={turn.id}
              turn={turn}
              select={(citation) => setSelected({ citation, turn })}
            />
          ))}
          {pending && (
            <div className="answer-turn">
              <div className="question-bubble">{pending}</div>
              <Group role="status" className="answer-progress">
                <Loader size="sm" />
                <Text>Finding evidence and preparing a cited answer…</Text>
                <Button
                  size="xs"
                  variant="subtle"
                  leftSection={<Square size={13} />}
                  onClick={stop}
                >
                  Stop
                </Button>
              </Group>
            </div>
          )}
          <ErrorState error={error} />
          {interruptedId && (
            <Group mt="sm">
              <Button
                size="xs"
                variant="default"
                onClick={() => void checkReceipt()}
              >
                Check request status
              </Button>
              <Text size="xs" c="dimmed">
                No automatic retry. Sending again starts a new attempt.
              </Text>
            </Group>
          )}
          {receipt && (
            <Alert
              color={receipt.state === "no_evidence" ? "gray" : "yellow"}
              mt="md"
              title={
                receipt.state === "no_evidence"
                  ? "Not enough supporting evidence"
                  : `Request ${label(receipt.state)}`
              }
            >
              {failureText[receipt.failure_code ?? ""] ??
                (receipt.state === "completed"
                  ? "This request completed, but its temporary answer is no longer available. It was not regenerated."
                  : "This attempt did not produce a displayable answer. Check its status before starting another attempt.")}
              {receipt.provider_may_have_run && (
                <Text size="xs" mt="xs">
                  Provider work may have been charged.
                </Text>
              )}
              {receipt.recall && (
                <Text size="xs" mt="xs">
                  {receipt.recall.coverage.examined} candidates examined ·{" "}
                  {receipt.recall.coverage.withheld} withheld ·{" "}
                  {receipt.recall.coverage.unavailable} unavailable
                </Text>
              )}
              <Button
                variant="subtle"
                size="xs"
                mt="sm"
                onClick={() => search(question)}
              >
                Find evidence
              </Button>
            </Alert>
          )}
        </div>
        {selected && (
          <aside className="answer-inspector" aria-label="Supporting evidence">
            <Group justify="space-between" mb="lg">
              <Title order={2}>Supporting evidence</Title>
              <Button
                variant="subtle"
                size="xs"
                aria-label="Close citation"
                onClick={() => setSelected(null)}
              >
                <X size={16} />
              </Button>
            </Group>
            <Badge variant="light">{selected.citation.id}</Badge>
            <Title order={3} mt="md">
              {selected.citation.evidence.label}
            </Title>
            <Text size="xs" c="dimmed" mt="xs">
              Exact retrieved {label(selected.citation.evidence.kind)} ·{" "}
              {new Date(
                selected.citation.evidence.recorded_at,
              ).toLocaleString()}
            </Text>
            <div className="answer-evidence-content">
              <Markdown
                text={selected.citation.evidence.text}
                className={
                  selected.citation.evidence.kind === "claim"
                    ? "markdown-preserve-lines"
                    : ""
                }
                raw
              />
            </div>
            <EvidenceNotes notes={selected.citation.evidence.qualifications} />
            <Stack gap="xs">
              {selected.citation.evidence.kind === "claim" && (
                <Button
                  variant="default"
                  onClick={() =>
                    setDetail({
                      claim: selected.citation.evidence.id,
                      knowledge: selected.turn.response.recall?.knowledge_at,
                      fact: selected.turn.response.recall?.fact_at ?? undefined,
                    })
                  }
                >
                  Inspect memory and history
                </Button>
              )}
              {selected.citation.evidence.provenance.map((p, i) => (
                <Button
                  variant="light"
                  key={`${p.id}-${i}`}
                  onClick={() =>
                    setDetail({
                      evidence: recallEvidence(selected.citation.evidence, i),
                    })
                  }
                >
                  Open {label(p.kind)}
                  {p.line_from
                    ? ` · lines ${p.line_from}–${p.line_to ?? p.line_from}`
                    : ""}
                </Button>
              ))}
            </Stack>
          </aside>
        )}
      </div>
      <form
        className="ask-composer"
        onSubmit={(e) => {
          e.preventDefault();
          void ask();
        }}
      >
        <Textarea
          aria-label="Ask a question about this Brain"
          placeholder="Ask a question about this Brain…"
          autosize
          minRows={2}
          maxRows={6}
          value={question}
          onChange={(e) => setQuestion(e.currentTarget.value)}
          maxLength={512}
          disabled={!!pending || brain.archived}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              void ask();
            }
          }}
        />
        <Button
          type="submit"
          aria-label="Ask Brain"
          disabled={!allowed || !question.trim() || !!pending || brain.archived}
          leftSection={<ArrowUp size={17} />}
        >
          Ask Brain
        </Button>
      </form>
      <Group justify="space-between" mt="sm" className="ask-privacy">
        <Text size="xs" c="dimmed">
          Temporary · Each question stands alone
        </Text>
        <details>
          <summary>How Ask works</summary>
          <Text size="xs" c="dimmed">
            Uses stored evidence. Does not run tools or save memories. Previous
            turns are not sent as context. ⌘/Ctrl + Enter to ask.
          </Text>
        </details>
      </Group>
      {!policies.isPending && !allowed && !policies.isError && (
        <Stack gap="sm" mt="lg">
          <Button
            variant="subtle"
            w="fit-content"
            onClick={() => search(question)}
          >
            Find evidence
          </Button>
          <ManagedMemoryPanel brain={brain} compact />
        </Stack>
      )}
      <AskFilters
        brain={brain}
        opened={filtersOpen}
        close={() => setFiltersOpen(false)}
        request={request}
        change={(next) => {
          clear(undefined, true);
          setRequest(next);
        }}
      />
      {detail?.claim && (
        <ClaimDialog
          key={`${detail.claim}-${detail.knowledge}`}
          brain={brain}
          id={detail.claim}
          catalogue={catalogue.data}
          knowledgeAt={detail.knowledge}
          factAt={detail.fact}
          onClose={() => setDetail(null)}
          onSaved={() =>
            clear("Memory changed. Ask again for current information.")
          }
        />
      )}
      {detail?.evidence && (
        <EvidenceDialog
          brain={brain}
          evidence={detail.evidence}
          returnLabel="Back to answer"
          onClose={() => setDetail(null)}
        />
      )}
    </div>
  );
}

function AnswerTurn({
  turn,
  select,
}: {
  turn: Turn;
  select: (citation: Citation) => void;
}) {
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState(false);
  const answer = turn.response.answer!;
  async function copy() {
    try {
      await navigator.clipboard.writeText(
        [
          answer.summary,
          ...answer.statements.map(
            (s) => `${s.text} [${s.citation_ids.join(", ")}]`,
          ),
          ...answer.limitations,
          ...turn.response.citations.map(
            (c) =>
              `${c.id}: ${c.evidence.label} · ${c.evidence.kind}:${c.evidence.revision_id}`,
          ),
        ].join("\n\n"),
      );
      setCopied(true);
      setCopyError(false);
    } catch {
      setCopyError(true);
    }
  }
  return (
    <article className="answer-turn">
      <div className="question-bubble">{turn.question}</div>
      <div className="answer-content">
        <div className="answer-mark">
          <Brand compact />
        </div>
        <div className="answer-text">
          <Title order={2}>{answer.summary}</Title>
          {answer.statements.map((statement, index) => (
            <div className="answer-statement" key={index}>
              <Markdown text={statement.text} />
              <span className="statement-citations">
                {statement.citation_ids.map((id) => {
                  const citation = turn.response.citations.find(
                    (c) => c.id === id,
                  );
                  return citation ? (
                    <button
                      className="citation-marker"
                      key={id}
                      onClick={() => select(citation)}
                      aria-label={`Open citation ${id}`}
                    >
                      {id.slice(1)}
                    </button>
                  ) : null;
                })}
              </span>
            </div>
          ))}
          {!answer.statements.length && (
            <Text c="dimmed" my="md">
              The available evidence does not support a factual answer to this
              question.
            </Text>
          )}
          <EvidenceNotes
            important={answer.limitations}
            notes={turn.response.recall?.coverage.reasons ?? []}
            partial={turn.response.recall?.coverage.partial}
          />
          <div className="answer-citations">
            {turn.response.citations.map((c) => (
              <button key={c.id} onClick={() => select(c)}>
                <FileText size={16} />
                <span>
                  <small>{c.id}</small>
                  {c.evidence.label}
                </span>
              </button>
            ))}
          </div>
          <Group gap="sm">
            <Button
              variant="default"
              size="xs"
              leftSection={copied ? <Check size={15} /> : <Copy size={15} />}
              onClick={() => void copy()}
            >
              {copied ? "Copied" : "Copy answer"}
            </Button>
            <Text size="xs" c="dimmed">
              {turn.response.citations.length} evidence{" "}
              {turn.response.citations.length === 1 ? "record" : "records"}
            </Text>
          </Group>
          {copyError && (
            <Text size="xs" c="red" mt="xs">
              Clipboard access failed. You can select and copy the text.
            </Text>
          )}
          <details className="answer-scope">
            <summary>Scope and freshness</summary>
            {turn.response.recall && (
              <Stack gap="xs" mt="sm">
                <RecallScope selection={turn.response.recall.selection} />
                <Text size="xs">
                  Matched by:{" "}
                  {Array.from(
                    new Set(
                      turn.response.recall.context.items.flatMap(
                        (item) => item.channels,
                      ),
                    ),
                  )
                    .map((channel) =>
                      channel === "lexical"
                        ? "text"
                        : channel === "semantic"
                          ? "meaning"
                          : channel === "graph"
                            ? "relationships"
                            : channel,
                    )
                    .join(" + ") || "No eligible matches"}
                </Text>
                <Text size="xs">
                  Knowledge cutoff:{" "}
                  {new Date(turn.response.recall.knowledge_at).toLocaleString()}{" "}
                  · {label(turn.response.recall.context.mode)}
                </Text>
                {turn.response.recall.fact_at && (
                  <Text size="xs">
                    Fact time:{" "}
                    {new Date(turn.response.recall.fact_at).toLocaleString()}
                  </Text>
                )}
                {turn.response.expires_at && (
                  <Text size="xs">
                    Display expires:{" "}
                    {new Date(turn.response.expires_at).toLocaleString()}
                  </Text>
                )}
              </Stack>
            )}
          </details>
        </div>
      </div>
    </article>
  );
}

function AskFilters({
  brain,
  opened,
  close,
  request,
  change,
}: {
  brain: Brain;
  opened: boolean;
  close: () => void;
  request: Request;
  change: (value: Request) => void;
}) {
  const catalogue = useQuery({
    queryKey: ["workspace", brain.id, "ask"],
    enabled: opened,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/workspace", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
  });
  const groups = useQuery({
    queryKey: ["evidence", brain.id, "ask"],
    enabled: opened,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/evidence", {
          params: { path: { brain: brain.id } },
          signal,
        }),
      ),
  });
  const [manifestOffset, setManifestOffset] = useState(0);
  const manifests = useQuery({
    queryKey: [
      "ask-manifests",
      brain.id,
      request.selection?.environment_id,
      manifestOffset,
    ],
    enabled: opened,
    queryFn: async ({ signal }) =>
      result(
        await client.GET("/api/brains/{brain}/revision-manifests", {
          params: {
            path: { brain: brain.id },
            query: {
              environment_id: request.selection?.environment_id ?? undefined,
              offset: manifestOffset,
            },
          },
          signal,
        }),
      ),
  });
  const manifestOptions = !manifests.isError
    ? (manifests.data?.items.map((m) => ({
        value: m.id,
        label: `${m.name} · ${label(m.kind)}`,
      })) ?? [])
    : [];
  if (
    request.manifest_revision_id &&
    !manifestOptions.some((m) => m.value === request.manifest_revision_id)
  )
    manifestOptions.unshift({
      value: request.manifest_revision_id,
      label: `Selected manifest · ${request.manifest_revision_id.slice(0, 8)}`,
    });
  const patch = (value: Partial<Request>) => change({ ...request, ...value });
  return (
    <Drawer opened={opened} onClose={close} title="Refine evidence" size={480}>
      <Stack gap="lg">
        <Text size="sm" c="dimmed">
          These limits apply to retrieval and the answer. Changing them clears
          the current conversation.
        </Text>
        <ErrorState
          error={catalogue.error ?? groups.error ?? manifests.error}
        />
        <MultiSelect
          label="Repositories"
          placeholder="All in this Brain"
          searchable
          clearable
          data={
            catalogue.data?.repositories.map((r) => ({
              value: r.id,
              label: r.canonical_origin,
            })) ?? []
          }
          value={request.selection?.repository_ids ?? []}
          onChange={(repository_ids) =>
            patch({ selection: { ...request.selection, repository_ids } })
          }
        />
        <MultiSelect
          label="Areas"
          placeholder="All in this Brain"
          searchable
          clearable
          data={
            catalogue.data?.areas.map((a) => ({
              value: a.id,
              label: a.name,
            })) ?? []
          }
          value={request.selection?.area_ids ?? []}
          onChange={(area_ids) =>
            patch({ selection: { ...request.selection, area_ids } })
          }
        />
        <Select
          label="Environment"
          placeholder="All in this Brain"
          clearable
          data={
            catalogue.data?.environments.map((a) => ({
              value: a.id,
              label: a.name,
            })) ?? []
          }
          value={request.selection?.environment_id ?? null}
          onChange={(environment_id) => {
            setManifestOffset(0);
            patch({
              selection: { ...request.selection, environment_id },
              manifest_revision_id: null,
            });
          }}
        />
        <Select
          label="Collection"
          placeholder="All collections"
          clearable
          data={
            groups.data?.groups
              .filter((g) => g.kind === "collection")
              .map((g) => ({ value: g.id, label: g.name })) ?? []
          }
          value={request.collection_id ?? null}
          onChange={(collection_id) => patch({ collection_id })}
        />
        <Select
          label="Revision manifest"
          placeholder="Select exact revisions when needed"
          clearable
          data={manifestOptions}
          value={request.manifest_revision_id ?? null}
          onChange={(manifest_revision_id) =>
            patch({
              manifest_revision_id,
              selection: {
                ...request.selection,
                environment_id:
                  manifests.data?.items.find(
                    (m) => m.id === manifest_revision_id,
                  )?.environment_id ?? request.selection?.environment_id,
              },
            })
          }
        />
        {(manifests.data?.total ?? 0) > 20 && (
          <Group>
            <Button
              variant="subtle"
              size="xs"
              disabled={!manifestOffset}
              onClick={() =>
                setManifestOffset(Math.max(0, manifestOffset - 20))
              }
            >
              Previous manifests
            </Button>
            <Button
              variant="subtle"
              size="xs"
              disabled={manifestOffset + 20 >= (manifests.data?.total ?? 0)}
              onClick={() => setManifestOffset(manifestOffset + 20)}
            >
              More manifests
            </Button>
          </Group>
        )}
        <Select
          label="Evidence eligibility"
          value={request.mode}
          allowDeselect={false}
          data={[
            {
              value: "investigation",
              label: "Include qualified and disputed evidence",
            },
            { value: "strict_accepted", label: "Accepted and current only" },
            {
              value: "strict_operational",
              label: "Operationally verified only",
            },
            { value: "history", label: "Qualified history" },
          ]}
          onChange={(v) => patch({ mode: v ?? "investigation" })}
        />
        <TextInput
          label="Knowledge cutoff (UTC)"
          type="datetime-local"
          value={request.knowledge_at?.slice(0, 16) ?? ""}
          onChange={(e) =>
            patch({
              knowledge_at: e.currentTarget.value
                ? new Date(`${e.currentTarget.value}Z`).toISOString()
                : null,
            })
          }
        />
        <TextInput
          label="Fact time (UTC)"
          type="datetime-local"
          value={request.fact_at?.slice(0, 16) ?? ""}
          onChange={(e) =>
            patch({
              fact_at: e.currentTarget.value
                ? new Date(`${e.currentTarget.value}Z`).toISOString()
                : null,
            })
          }
        />
        <Select
          label="Retrieval strategy"
          value={request.strategy ?? "auto"}
          allowDeselect={false}
          data={[
            { value: "auto", label: "Automatic" },
            { value: "manual", label: "Choose methods" },
          ]}
          onChange={(value) =>
            patch({ strategy: value === "manual" ? "manual" : "auto" })
          }
        />
        {request.strategy === "manual" && (
          <Checkbox
            label="Also search by meaning"
            description="Uses the separately permitted embedding model and its budget."
            checked={request.channels?.includes("semantic") ?? false}
            onChange={(e) =>
              patch({
                channels: e.currentTarget.checked
                  ? ["exact", "lexical", "semantic"]
                  : ["exact", "lexical"],
              })
            }
          />
        )}
        <Text size="xs" c="dimmed">
          Automatic retrieval uses permitted methods within this scope. Each
          submission may use the approved embedding model; no request is sent
          while editing.
        </Text>
        {request.strategy === "manual" && (
          <TextInput
            label="Exact text search expression (optional)"
            description="Leave empty to find the subjects named in your question."
            value={request.query ?? ""}
            maxLength={512}
            onChange={(e) => patch({ query: e.currentTarget.value })}
          />
        )}

        <Button variant="default" onClick={() => change(initialRequest())}>
          Reset filters
        </Button>
        <Button onClick={close}>Done</Button>
      </Stack>
    </Drawer>
  );
}
