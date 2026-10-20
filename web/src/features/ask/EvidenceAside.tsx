import { useEffect, useRef } from "react";
import { motion } from "motion/react";
import { FileSearch, History, X } from "lucide-react";
import { EvidenceNotes } from "../../components/EvidenceNotes";
import { Markdown } from "../../components/Markdown";
import { Badge, Button, IconButton } from "../../components/ui";
import { paneIn } from "../../design/motion";
import { RecallKindChip } from "../../RecallResults";
import type { Citation } from "./AnswerTurn";

const label = (s: string) => s.replaceAll("_", " ");

/**
 * "Supporting evidence" for one citation, docked beside the thread (never over
 * it). It is a complementary region, not a dialog: the exact evidence it opens
 * is the dialog. Escape closes it and focus returns to the opener.
 */
export function EvidenceAside({
  citation,
  onClose,
  onInspectClaim,
  onOpenProvenance,
}: {
  citation: Citation;
  onClose: () => void;
  onInspectClaim?: () => void;
  onOpenProvenance: (index: number) => void;
}) {
  const evidence = citation.evidence;
  const opener = useRef<HTMLElement | null>(null);
  const panel = useRef<HTMLElement>(null);
  useEffect(() => {
    const active = document.activeElement as HTMLElement | null;
    if (active && !panel.current?.contains(active)) opener.current = active;
  }, [citation]);
  useEffect(() => {
    // Like the docked inspector: focus moves in on open and returns on close.
    panel.current?.focus({ preventScroll: true });
    return () => {
      const target = opener.current;
      if (target?.isConnected && !document.activeElement?.closest("[role=dialog]"))
        target.focus({ preventScroll: true });
    };
  }, []);
  return (
    <motion.aside
      ref={panel}
      tabIndex={-1}
      variants={paneIn}
      initial="initial"
      animate="animate"
      exit="exit"
      aria-label="Supporting evidence"
      onKeyDown={(event) => {
        if (event.key === "Escape" && !event.defaultPrevented) {
          event.stopPropagation();
          onClose();
        }
      }}
      className="sticky top-4 flex max-h-[calc(100vh-120px)] outline-none min-w-0 flex-col self-start overflow-hidden rounded-lg border border-border bg-card shadow-sm"
    >
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border pl-4 pr-2">
        <h2 className="m-0 min-w-0 flex-1 truncate text-title font-semibold text-foreground">
          Supporting evidence
        </h2>
        <IconButton label="Close citation" size="icon-sm" onClick={onClose}>
          <X aria-hidden="true" />
        </IconButton>
      </header>
      <div className="grid min-h-0 flex-1 content-start gap-3 overflow-auto p-4 scrollbar-thin">
        <div className="flex flex-wrap items-center gap-1.5">
          <Badge variant="brand" className="mono">
            {citation.id}
          </Badge>
          <RecallKindChip item={evidence} />
        </div>
        <h3 className="m-0 text-title font-semibold text-foreground [overflow-wrap:anywhere]">
          {evidence.label}
        </h3>
        <p className="m-0 text-caption text-muted-foreground">
          Exact retrieved {label(evidence.kind)} ·{" "}
          <span className="mono">
            {new Date(evidence.recorded_at).toLocaleString()}
          </span>
        </p>
        <div className="max-h-[min(50vh,560px)] overflow-auto rounded-md border border-border bg-background px-3.5 py-3 scrollbar-thin">
          <Markdown
            text={evidence.text}
            className={evidence.kind === "claim" ? "markdown-preserve-lines" : ""}
            raw
          />
        </div>
        <EvidenceNotes notes={evidence.qualifications} />
        <div className="grid gap-1.5">
          {onInspectClaim && (
            <Button variant="secondary" className="justify-start" onClick={onInspectClaim}>
              <History aria-hidden="true" />
              Inspect memory and history
            </Button>
          )}
          {evidence.provenance.map((p, i) => (
            <Button
              variant="ghost"
              className="h-auto min-h-8 justify-start whitespace-normal py-1.5 text-left"
              key={`${p.id}-${i}`}
              onClick={() => onOpenProvenance(i)}
            >
              <FileSearch aria-hidden="true" />
              Open {label(p.kind)}
              {p.line_from
                ? ` · lines ${p.line_from}–${p.line_to ?? p.line_from}`
                : ""}
            </Button>
          ))}
        </div>
      </div>
    </motion.aside>
  );
}
