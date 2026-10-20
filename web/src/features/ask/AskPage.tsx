import { useEffect, useRef, useState, type ReactNode } from "react";
import { useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import {
  ArrowUp,
  BookOpen,
  ChevronDown,
  MessageSquare,
  RotateCcw,
  SlidersHorizontal,
  Sparkles,
  Square,
} from "lucide-react";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { subscribeAskHandoff, takeAskHandoff } from "../../app/askHandoff";
import { useBrain } from "../../app/context";
import { ErrorState } from "../../components/AsyncState";
import { BrainIcon } from "../../components/BrainIcon";
import { Page } from "../../components/Page";
import { useFeatureTab } from "../../components/FeatureTabs";
import {
  Alert,
  Badge,
  Button,
  Kbds,
  Popover,
  PopoverContent,
  PopoverTrigger,
  SegmentedControl,
  Textarea,
  modKey,
} from "../../components/ui";
import { pageEnter } from "../../design/motion";
import { cn } from "../../lib/utils";
import { RecallPanel } from "../../RecallPanel";
import { ClaimDialog, EvidenceDialog } from "../../ClaimsPanel";
import { recallEvidence } from "../../RecallResults";
import { useContentDeadline } from "../../useContentDeadline";
import { ManagedMemoryPanel } from "../settings/ManagedMemoryPanel";
import { usePollInterval } from "../../usePollInterval";
import { useWorkspaceCatalogue } from "../../catalogueQueries";
import { AnswerTurn, QuestionBubble, type Citation, type Turn } from "./AnswerTurn";
import { AskFilters } from "./AskFilters";
import { EvidenceAside } from "./EvidenceAside";

type Request = components["schemas"]["RecallRequest"];
type Answer = components["schemas"]["AnswerResponse"];
type Evidence = components["schemas"]["ClaimEvidenceChoice"];
type Mode = "ask" | "search";
const modes = [
  { value: "ask", label: "Answer" },
  { value: "search", label: "Find evidence" },
] as const;
const suggestions = [
  "What decisions have we made?",
  "What do we know about this project?",
  "Which issues remain unresolved?",
];
const label = (s: string) => s.replaceAll("_", " ");
const initialRequest = (): Request => ({
  project_brief: false,
  continuation: null,
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
  semantic_exact: false,
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
  model_not_selected:
    "No text model is selected for this Brain. An administrator can choose one under Settings → AI. Find evidence remains available.",
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

type Handoff = { id: string; question: string };

export function AskPage() {
  const brain = useBrain();
  const [draft, setDraft] = useState("");
  const [tab, setTab] = useFeatureTab(
    modes.map((m) => m.value),
    "ask",
  );
  const [handoff, setHandoff] = useState<Handoff | null>(null);
  const current = useRef(tab);
  current.current = tab;
  useEffect(() => setDraft(""), [brain.id]);
  // ⌘K → Ask: take a question handed off by the palette (in memory, never in
  // the URL) and submit it as an explicit Ask request.
  useEffect(() => {
    const take = () => {
      const question = takeAskHandoff(brain.id);
      if (!question) return;
      if (current.current !== "ask") setTab("ask");
      setHandoff({ id: crypto.randomUUID(), question });
    };
    take();
    return subscribeAskHandoff(take);
    // setTab is recreated each render; the subscription only needs the Brain.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [brain.id]);
  const mode = (
    <SegmentedControl
      label="Ask mode"
      data={modes}
      value={tab as Mode}
      onChange={(next) => setTab(next)}
    />
  );
  return (
    <Page className="flex-1 pb-0">
      {tab === "search" ? (
        <>
          <AskHeader
            description="Find the stored evidence behind a decision, system or past work. No answer is generated."
            actions={mode}
          />
          <RecallPanel
            key={brain.id}
            brain={brain}
            draft={draft}
            setDraft={setDraft}
          />
          <div className="h-4 shrink-0" />
        </>
      ) : (
        <AskConversation
          key={brain.id}
          brain={brain}
          mode={mode}
          question={draft}
          setQuestion={setDraft}
          handoff={handoff}
          clearHandoff={() => setHandoff(null)}
          search={(query) => {
            setDraft(query ?? "");
            setTab("search");
          }}
        />
      )}
    </Page>
  );
}

/** The compact sans header used once a thread exists and in Find evidence. */
function AskHeader({
  description,
  actions,
  hideTitle = false,
}: {
  description: string;
  actions: ReactNode;
  hideTitle?: boolean;
}) {
  return (
    <motion.div
      variants={pageEnter}
      initial="initial"
      animate="animate"
      className="flex items-center justify-between gap-6"
    >
      {hideTitle ? (
        <div />
      ) : (
        <div className="min-w-0">
          <h1 className="m-0 text-page font-semibold text-foreground">
            Ask your Brain
          </h1>
          <p className="m-0 mt-1 text-body text-muted-foreground">{description}</p>
        </div>
      )}
      <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">
        {actions}
      </div>
    </motion.div>
  );
}

function AskConversation({
  brain,
  mode,
  search,
  question,
  setQuestion,
  handoff,
  clearHandoff,
}: {
  brain: Brain;
  mode: ReactNode;
  search: (query?: string) => void;
  question: string;
  setQuestion: (value: string) => void;
  handoff: Handoff | null;
  clearHandoff: () => void;
}) {
  const pollSlow = usePollInterval("slow");
  const pollFast = usePollInterval("fast");
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
  const thread = useRef<HTMLDivElement>(null);
  const reduced = useReducedMotion();
  const policies = useQuery({
    queryKey: ["models-policy", brain.id],
    refetchInterval: pollSlow,
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
  const catalogue = useWorkspaceCatalogue(brain.id, undefined, {
    enabled: filtersOpen || turns.length > 0,
  });
  const validity = useQueries({
    queries: turns.map((turn) => ({
      queryKey: ["answer-status", brain.id, turn.id],
      gcTime: 0,
      retry: false,
      refetchInterval: pollFast,
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

  async function ask(asked?: string) {
    const source = asked ?? question;
    if (!allowed || brain.archived || pending || !source.trim()) return;
    const text = source.trim();
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
  // A palette hand-off is submitted once the Brain's answer policy is known,
  // exactly as if the person typed it and pressed Ask. When answering is not
  // allowed it stays in the composer, as a typed question would.
  useEffect(() => {
    if (!handoff || policies.isPending) return;
    clearHandoff();
    setQuestion(handoff.question);
    void ask(handoff.question);
  }, [handoff, policies.isPending]);
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
  // Keep the newest turn in view as it arrives.
  useEffect(() => {
    if (!pending && !turns.length) return;
    const last = thread.current?.lastElementChild;
    last?.scrollIntoView({
      block: "nearest",
      behavior: reduced ? "auto" : "smooth",
    });
  }, [pending, turns.length]);
  const selectedCount =
    (request.selection?.repository_ids?.length ?? 0) +
    (request.selection?.area_ids?.length ?? 0) +
    (request.selection?.environment_id ? 1 : 0) +
    (request.collection_id ? 1 : 0);
  const empty = !turns.length && !pending;
  const unavailable = !policies.isPending && !allowed && !policies.isError;
  const toolbar = (
    <>
      {request.mode !== "investigation" && (
        <Badge variant="brand" className="capitalize">
          {label(request.mode ?? "investigation")}
        </Badge>
      )}
      {(request.knowledge_at || request.fact_at) && (
        <Badge variant="warning">Time filtered</Badge>
      )}
      {mode}
      <Button
        variant="secondary"
        onClick={() => setFiltersOpen(true)}
        aria-haspopup="dialog"
        title="Refine the evidence this answer may use"
      >
        <SlidersHorizontal aria-hidden="true" />
        {selectedCount ? `${selectedCount} scope filters` : "Entire Brain"}
        <ChevronDown aria-hidden="true" className="text-muted-foreground" />
      </Button>
      <Button
        variant="ghost"
        onClick={() => {
          clear();
          setInterruptedId(null);
        }}
        disabled={!turns.length && !pending && !question}
      >
        <RotateCcw aria-hidden="true" />
        Clear conversation
      </Button>
    </>
  );
  return (
    <>
      <AskHeader
        hideTitle={empty}
        description="Ask this Brain a question and see the evidence behind the answer."
        actions={toolbar}
      />
      <ErrorState
        error={policies.error}
        retry={() => void policies.refetch()}
      />
      {notice && (
        <Alert variant="warning" role="status" className="animate-enter">
          {notice}
        </Alert>
      )}
      <div
        className={cn(
          "grid min-h-0 flex-1 gap-8",
          selected && "grid-cols-[minmax(0,1fr)_minmax(300px,360px)]",
        )}
      >
        <div
          className={cn(
            "flex min-w-0 flex-col",
            empty ? "justify-center gap-8" : "gap-8",
          )}
        >
          <div
            ref={thread}
            className="mx-auto flex w-full max-w-(--composer-width) flex-col gap-10"
            aria-live="polite"
            aria-busy={!!pending}
          >
            {empty && (
              <motion.div
                variants={pageEnter}
                initial="initial"
                animate="animate"
                className="mx-auto grid max-w-(--composer-width) justify-items-center gap-3 pt-6 text-center"
              >
                <span className="mb-1 grid size-11 place-items-center rounded-lg bg-brand-soft text-brand-text">
                  <Sparkles className="size-5" aria-hidden="true" />
                </span>
                <h1 className="m-0 font-serif text-hero font-normal text-foreground">
                  Ask your <em className="not-italic text-brand-text">Brain</em>
                </h1>
                <p className="m-0 max-w-[52ch] text-body text-muted-foreground">
                  Answers come only from the evidence {brain.name} holds, with
                  citations you can open.
                </p>
              </motion.div>
            )}
            {turns.map((turn) => (
              <AnswerTurn
                key={turn.id}
                brain={brain}
                turn={turn}
                selected={
                  selected?.turn.id === turn.id ? selected.citation.id : undefined
                }
                select={(citation) => setSelected({ citation, turn })}
              />
            ))}
            {pending && (
              <div className="flex flex-col gap-5">
                <QuestionBubble>{pending}</QuestionBubble>
                <div role="status" className="flex items-center gap-2">
                  <Sparkles className="size-4 shrink-0 text-brand-text" aria-hidden="true" />
                  <span className="shimmer-text text-body">
                    Gathering evidence from {brain.name} and preparing a cited
                    answer…
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="ml-auto"
                    onClick={stop}
                  >
                    <Square aria-hidden="true" className="size-3" />
                    Stop
                  </Button>
                </div>
              </div>
            )}
            {(error || interruptedId || receipt) && (
              <div className="grid gap-3">
                <ErrorState error={error} />
                {interruptedId && (
                  <div className="flex flex-wrap items-center gap-3">
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => void checkReceipt()}
                    >
                      Check request status
                    </Button>
                    <span className="text-caption text-muted-foreground">
                      No automatic retry. Sending again starts a new attempt.
                    </span>
                  </div>
                )}
                {receipt && (
                  <Alert
                    role="status"
                    variant={receipt.state === "no_evidence" ? "default" : "warning"}
                    title={
                      receipt.state === "no_evidence"
                        ? "Not enough supporting evidence"
                        : `Request ${label(receipt.state)}`
                    }
                    actions={
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => search(question)}
                      >
                        Find evidence
                      </Button>
                    }
                  >
                    <p>
                      {failureText[receipt.failure_code ?? ""] ??
                        (receipt.state === "completed"
                          ? "This request completed, but its temporary answer is no longer available. It was not regenerated."
                          : "This attempt did not produce a displayable answer. Check its status before starting another attempt.")}
                    </p>
                    {receipt.provider_may_have_run && (
                      <p className="mt-1 text-caption">
                        Provider work may have been charged.
                      </p>
                    )}
                    {receipt.recall && (
                      <p className="mono mt-1 text-caption">
                        {receipt.recall.coverage.examined} candidates examined ·{" "}
                        {receipt.recall.coverage.withheld} withheld ·{" "}
                        {receipt.recall.coverage.unavailable} unavailable
                      </p>
                    )}
                  </Alert>
                )}
              </div>
            )}
            {unavailable && (
              <div className="grid gap-3">
                <Alert
                  variant="default"
                  role="status"
                  title="Answering is not available for this Brain"
                  actions={
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => search(question)}
                    >
                      Find evidence
                    </Button>
                  }
                >
                  Find evidence still searches stored memory without a model.
                </Alert>
                <ManagedMemoryPanel brain={brain} compact />
              </div>
            )}
          </div>
          <div
            className={cn(
              "sticky bottom-0 z-10 mx-auto grid w-full max-w-(--composer-width) gap-3 bg-card pb-6 pt-2 before:pointer-events-none before:absolute before:inset-x-0 before:-top-6 before:h-6 before:bg-linear-to-t before:from-card before:to-transparent before:content-['']",
              !empty && "mt-auto",
            )}
          >
            {!pending && !unavailable && (
              <div className="flex flex-wrap items-center gap-1">
                <span className="mr-1.5 text-caption text-muted-foreground">
                  Suggested
                </span>
                {suggestions.map((q) => (
                  <Button
                    key={q}
                    variant="ghost"
                    size="sm"
                    onClick={() => setQuestion(q)}
                  >
                    <MessageSquare aria-hidden="true" />
                    {q}
                  </Button>
                ))}
              </div>
            )}
            <form
              className={cn(
                "w-full rounded-xl border border-border bg-card shadow-sm transition-[border-color,box-shadow] duration-fast ease-standard focus-within:border-ring focus-within:shadow-focus",
                pending &&
                  "border-brand shadow-glow focus-within:border-brand focus-within:shadow-glow",
              )}
              onSubmit={(e) => {
                e.preventDefault();
                void ask();
              }}
            >
              <Textarea
                aria-label="Ask a question about this Brain"
                placeholder={
                  empty
                    ? "Ask a question about this Brain…"
                    : "Ask a follow-up about this Brain…"
                }
                autosize
                minRows={2}
                maxRows={6}
                value={question}
                onChange={(e) => setQuestion(e.currentTarget.value)}
                maxLength={512}
                disabled={!!pending || brain.archived}
                className="min-h-[60px] rounded-none border-0 bg-transparent px-4 pb-1 pt-3.5 text-[16px] leading-[26px] shadow-none hover:border-0 focus-visible:shadow-none disabled:opacity-60"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                    e.preventDefault();
                    void ask();
                  }
                }}
              />
              <div className="flex items-center gap-1.5 px-2.5 pb-2 pt-2">
                <span
                  className="inline-flex h-7 min-w-0 items-center gap-1.5 rounded-full border border-border bg-card pl-1 pr-2.5 text-caption font-medium text-foreground-secondary"
                  title="Questions are answered only from this Brain"
                >
                  <span className="grid size-[18px] shrink-0 place-items-center overflow-hidden rounded-full [&>span]:size-[18px] [&>span]:rounded-full">
                    <BrainIcon id={brain.id} revision={brain.icon_revision} size={18} />
                  </span>
                  <span className="truncate">{brain.name}</span>
                </span>
                <Kbds keys={[modKey, "↵"]} className="ml-auto mr-1" />
                <button
                  type="submit"
                  aria-label="Ask Brain"
                  title="Ask Brain"
                  disabled={!allowed || !question.trim() || !!pending || brain.archived}
                  className="grid size-8 shrink-0 place-items-center rounded-md bg-brand text-brand-foreground transition-[background-color,transform] duration-fast ease-standard hover:bg-brand-hover active:scale-[0.94] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:cursor-default disabled:bg-secondary disabled:text-faint-foreground"
                >
                  <ArrowUp className="size-4" aria-hidden="true" />
                </button>
              </div>
            </form>
            <div className="flex items-center gap-2 text-caption text-muted-foreground">
              <span>Temporary · each question stands alone</span>
              <Popover>
                <PopoverTrigger asChild>
                  <Button variant="link" size="sm" className="ml-auto">
                    <BookOpen aria-hidden="true" />
                    How Ask works
                  </Button>
                </PopoverTrigger>
                <PopoverContent align="end" side="top" className="text-label text-foreground-secondary">
                  Uses stored evidence. Does not run tools or save memories.
                  Previous turns are not sent as context. {modKey} + Enter to
                  ask.
                </PopoverContent>
              </Popover>
            </div>
          </div>
        </div>
        <AnimatePresence>
          {selected && (
            <EvidenceAside
              citation={selected.citation}
              onClose={() => setSelected(null)}
              onInspectClaim={
                selected.citation.evidence.kind === "claim"
                  ? () =>
                      setDetail({
                        claim: selected.citation.evidence.id,
                        knowledge: selected.turn.response.recall?.knowledge_at,
                        fact: selected.turn.response.recall?.fact_at ?? undefined,
                      })
                  : undefined
              }
              onOpenProvenance={(index) =>
                setDetail({
                  evidence: recallEvidence(selected.citation.evidence, index),
                })
              }
            />
          )}
        </AnimatePresence>
      </div>
      <AskFilters
        brain={brain}
        opened={filtersOpen}
        close={() => setFiltersOpen(false)}
        request={request}
        change={(next) => {
          clear(undefined, true);
          setRequest(next);
        }}
        reset={() => {
          clear(undefined, true);
          setRequest(initialRequest());
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
    </>
  );
}
