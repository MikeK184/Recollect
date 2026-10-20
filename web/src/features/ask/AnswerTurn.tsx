import { useState } from "react";
import { motion, useReducedMotion } from "motion/react";
import { Check, Copy, Sparkles } from "lucide-react";
import type { Brain } from "../../api";
import type { components } from "../../api-schema";
import { EvidenceNotes } from "../../components/EvidenceNotes";
import { Markdown } from "../../components/Markdown";
import { Button } from "../../components/ui";
import { arrive, reveal, staggerDelay } from "../../design/motion";
import { RecallDetails, RecallKindChip, RecallScope } from "../../RecallResults";

type Request = components["schemas"]["RecallRequest"];
type Answer = components["schemas"]["AnswerResponse"];
export type Citation = components["schemas"]["AnswerCitation"];
export type Turn = {
  id: string;
  question: string;
  request: Request;
  response: Answer;
};
const label = (s: string) => s.replaceAll("_", " ");

/** The person's question, right-aligned like a chat message. */
export function QuestionBubble({ children }: { children: string }) {
  return (
    <p className="m-0 max-w-[520px] self-end whitespace-pre-wrap rounded-lg bg-muted px-3.5 py-2.5 text-[15px] leading-[22px] text-foreground [overflow-wrap:anywhere]">
      {children}
    </p>
  );
}

/** Small brand-soft mono citation chip; opens the supporting evidence. */
export function CitationChip({
  id,
  onClick,
}: {
  id: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={`Open citation ${id}`}
      className="mx-px inline-grid h-[18px] min-w-[18px] place-items-center rounded-xs bg-brand-soft px-1 align-[2px] font-mono text-[11px] font-medium leading-4 text-brand-text transition-colors duration-fast ease-standard hover:bg-brand hover:text-brand-foreground focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring"
    >
      {id.replace(/^C/, "")}
    </button>
  );
}

/**
 * One validated answer. It appears whole (motion.md "Revealing": 4px blur to
 * sharp over motion-slow); citations follow 60ms later and source cards
 * stagger in. Nothing streams.
 */
export function AnswerTurn({
  brain,
  turn,
  selected,
  select,
}: {
  brain: Brain;
  turn: Turn;
  selected?: string;
  select: (citation: Citation) => void;
}) {
  const reduced = useReducedMotion();
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState(false);
  const answer = turn.response.answer!;
  const citations = turn.response.citations;
  async function copy() {
    try {
      await navigator.clipboard.writeText(
        [
          answer.summary,
          ...answer.statements.map(
            (s) => `${s.text} [${s.citation_ids.join(", ")}]`,
          ),
          ...answer.limitations,
          ...citations.map(
            (c) =>
              `${c.id}: ${c.evidence.label} · ${c.evidence.kind}:${c.evidence.revision_id}`,
          ),
        ].join("\n\n"),
      );
      setCopied(true);
      setCopyError(false);
    } catch {
      setCopied(false);
      setCopyError(true);
    }
  }
  const later = (delay: number) =>
    reduced
      ? { initial: false as const }
      : {
          initial: { opacity: 0 },
          animate: { opacity: 1, transition: { ...arrive, delay } },
        };
  return (
    <article className="flex flex-col gap-5" aria-label={`Answer to: ${turn.question}`}>
      <QuestionBubble>{turn.question}</QuestionBubble>
      <motion.div
        variants={reveal}
        initial={reduced ? false : "initial"}
        animate="animate"
        className="grid gap-3"
      >
        <div className="flex items-center gap-2 text-label text-muted-foreground">
          <Sparkles className="size-4 text-brand-text" aria-hidden="true" />
          <span className="min-w-0 truncate">Answer from {brain.name}</span>
          <span className="mono ml-auto shrink-0 text-caption">
            {citations.length} {citations.length === 1 ? "source" : "sources"}
          </span>
        </div>
        <h2 className="m-0 text-title font-semibold text-foreground">
          {answer.summary}
        </h2>
        {answer.statements.map((statement, index) => (
          <div
            key={index}
            className="answer-statement max-w-(--reading-measure) font-serif text-answer text-foreground [&>.rc-markdown:has(>p:only-child)]:inline [&>.rc-markdown>p:only-child]:inline"
          >
            <Markdown text={statement.text} className="markdown-answer" />
            <motion.span {...later(0.06)} className="ml-1 inline-flex gap-0.5 align-baseline">
              {statement.citation_ids.map((id) => {
                const citation = citations.find((c) => c.id === id);
                return citation ? (
                  <CitationChip key={id} id={id} onClick={() => select(citation)} />
                ) : null;
              })}
            </motion.span>
          </div>
        ))}
        {!answer.statements.length && (
          <p className="m-0 font-serif text-answer text-muted-foreground">
            The available evidence does not support a factual answer to this
            question.
          </p>
        )}
        <EvidenceNotes
          important={answer.limitations}
          notes={turn.response.recall?.coverage.reasons ?? []}
          partial={turn.response.recall?.coverage.partial}
        />
        {!!citations.length && (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-2">
            {citations.map((c, index) => (
              <motion.button
                key={c.id}
                type="button"
                {...(reduced
                  ? { initial: false }
                  : {
                      initial: { opacity: 0, y: 4 },
                      animate: {
                        opacity: 1,
                        y: 0,
                        transition: { ...arrive, delay: 0.06 + staggerDelay(index) },
                      },
                    })}
                onClick={() => select(c)}
                aria-pressed={selected === c.id}
                className="grid min-w-0 gap-1.5 rounded-md border border-border bg-card px-3 py-2.5 text-left text-label text-foreground transition-[border-color,background-color] duration-fast ease-standard hover:border-border-strong aria-pressed:border-brand/60 aria-pressed:bg-brand-soft/40 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
              >
                <span className="flex min-w-0 items-center gap-1.5 font-medium">
                  <span
                    aria-hidden="true"
                    className="inline-grid h-[18px] min-w-[18px] shrink-0 place-items-center rounded-xs bg-brand-soft px-1 font-mono text-[11px] leading-4 text-brand-text"
                  >
                    {c.id.replace(/^C/, "")}
                  </span>
                  <span className="truncate">{c.evidence.label}</span>
                </span>
                <span className="flex min-w-0 items-center gap-2">
                  <RecallKindChip item={c.evidence} />
                  <span className="mono truncate text-caption text-muted-foreground">
                    {new Date(c.evidence.recorded_at).toLocaleDateString()}
                  </span>
                </span>
              </motion.button>
            ))}
          </div>
        )}
        <motion.div {...later(0.12)} className="flex flex-wrap items-center gap-1">
          <Button variant="ghost" size="sm" onClick={() => void copy()}>
            {copied ? (
              <Check aria-hidden="true" className="text-brand-text" />
            ) : (
              <Copy aria-hidden="true" />
            )}
            {copied ? "Copied" : "Copy answer"}
          </Button>
          <span className="mono px-2 text-caption text-muted-foreground">
            {citations.length} evidence{" "}
            {citations.length === 1 ? "record" : "records"}
          </span>
          {copyError && (
            <span role="alert" className="text-caption text-destructive-text">
              Clipboard access failed. You can select and copy the text.
            </span>
          )}
        </motion.div>
        <RecallDetails summary="Scope and freshness">
          {turn.response.recall && (
            <>
              <RecallScope selection={turn.response.recall.selection} />
              <p className="m-0">
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
              </p>
              <p className="m-0">
                Knowledge cutoff:{" "}
                <span className="mono">
                  {new Date(turn.response.recall.knowledge_at).toLocaleString()}
                </span>{" "}
                · {label(turn.response.recall.context.mode)}
              </p>
              {turn.response.recall.fact_at && (
                <p className="m-0">
                  Fact time:{" "}
                  <span className="mono">
                    {new Date(turn.response.recall.fact_at).toLocaleString()}
                  </span>
                </p>
              )}
              {turn.response.expires_at && (
                <p className="m-0">
                  Display expires:{" "}
                  <span className="mono">
                    {new Date(turn.response.expires_at).toLocaleString()}
                  </span>
                </p>
              )}
            </>
          )}
        </RecallDetails>
      </motion.div>
    </article>
  );
}
