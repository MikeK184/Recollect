import { Fragment, useEffect, useMemo, useState, type ReactNode } from "react";
import { Check, Copy } from "lucide-react";
import type { RootContent } from "hast";
import "./markdown.css";
import { formatJson } from "./jsonFormatting";

function highlighted(nodes: RootContent[]): ReactNode {
  return nodes.map((node, index) => {
    if (node.type === "text")
      return <Fragment key={index}>{node.value}</Fragment>;
    // Lowlight emits spans and text. Only carry token classes into React; no
    // source-supplied HTML, attributes or executable markup is interpreted.
    if (node.type === "element")
      return (
        <span
          key={index}
          className={(node.properties.className as string[] | undefined)?.join(
            " ",
          )}
        >
          {highlighted(node.children)}
        </span>
      );
    return null;
  });
}

export function CodeSyntax({
  code,
  language,
}: {
  code: string;
  language?: string;
}) {
  const [tokens, setTokens] = useState<{
    code: string;
    language: string;
    nodes: RootContent[];
  } | null>(null);
  const bounded = code.length <= 16384 && code.split("\n").length <= 500;
  useEffect(() => {
    let active = true;
    setTokens(null);
    if (language && bounded) {
      void import("./markdownHighlight")
        .then(({ highlighter }) => {
          if (active && highlighter.registered(language))
            setTokens({
              code,
              language,
              nodes: highlighter.highlight(language, code).children,
            });
        })
        .catch(() => {
          // Readable exact code remains available if a grammar cannot load.
        });
    }
    return () => {
      active = false;
    };
  }, [code, language, bounded]);
  return (
    <>
      {tokens?.code === code && tokens.language === language
        ? highlighted(tokens.nodes)
        : code}
    </>
  );
}

export function CodeBlock({
  code,
  language,
  copyCode,
  copyLabel = "Copy code",
}: {
  code: string;
  language?: string;
  copyCode?: string;
  copyLabel?: string;
}) {
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState(false);
  const [compact, setCompact] = useState(false);
  const formatted = useMemo(
    () =>
      language?.toLowerCase() === "json" ? formatJson(code, compact) : null,
    [code, language, compact],
  );
  const displayed = formatted ?? code;
  useEffect(() => {
    setCopied(false);
    setCopyError(false);
    setCompact(false);
  }, [code, language, copyCode]);
  async function copy() {
    try {
      await navigator.clipboard.writeText(copyCode ?? displayed);
      setCopied(true);
      setCopyError(false);
    } catch {
      setCopyError(true);
    }
  }
  return (
    <div className="markdown-code my-3 max-w-full overflow-hidden rounded-lg border border-border bg-card">
      <div className="mono flex h-10 items-center gap-2 border-b border-border bg-muted pl-3.5 pr-1.5 text-caption text-muted-foreground">
        <span>{language ?? "Code"}</span>
        <div className="ml-auto flex items-center gap-0.5">
          {formatted !== null && (
            <button
              type="button"
              aria-label={compact ? "Format JSON" : "Compact JSON"}
              className="inline-flex h-7 items-center gap-1.5 rounded-sm px-2 font-sans text-caption font-medium text-foreground-secondary transition-colors hover:bg-accent hover:text-foreground"
              onClick={() => {
                setCompact(!compact);
                setCopied(false);
              }}
            >
              {compact ? "Format" : "Compact"}
            </button>
          )}
          <button
            type="button"
            onClick={() => void copy()}
            aria-label={copyLabel}
            className="inline-flex h-7 items-center gap-1.5 rounded-sm px-2 font-sans text-caption font-medium text-foreground-secondary transition-colors hover:bg-accent hover:text-foreground"
          >
            {copied ? (
              <Check size={14} className="text-brand-text" aria-hidden="true" />
            ) : (
              <Copy size={14} aria-hidden="true" />
            )}
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
      <pre
        tabIndex={0}
        aria-label={`${language ?? "Plain text"} code`}
        className="mono m-0 max-h-[420px] max-w-full overflow-auto p-3.5 text-label leading-5 text-foreground scrollbar-thin"
      >
        <code className="block whitespace-pre">
          <CodeSyntax code={displayed} language={language} />
        </code>
      </pre>
      {copyError && (
        <span role="status" className="block px-3 py-1.5 text-caption text-foreground-secondary">
          Code could not be copied.
        </span>
      )}
    </div>
  );
}
