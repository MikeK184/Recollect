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
    <div className="markdown-code">
      <div className="markdown-code-toolbar">
        <span>{language ?? "Code"}</span>
        <div className="markdown-code-actions">
          {formatted !== null && (
            <button
              type="button"
              aria-label={compact ? "Format JSON" : "Compact JSON"}
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
          >
            {copied ? <Check size={14} /> : <Copy size={14} />}
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
      <pre tabIndex={0} aria-label={`${language ?? "Plain text"} code`}>
        <code>
          <CodeSyntax code={displayed} language={language} />
        </code>
      </pre>
      {copyError && (
        <span role="status" className="markdown-copy-error">
          Code could not be copied.
        </span>
      )}
    </div>
  );
}
