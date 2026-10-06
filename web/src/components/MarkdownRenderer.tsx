import { isValidElement, type ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { CodeBlock } from "./CodeBlock";

export function MarkdownRenderer({
  text,
  className = "",
  raw = false,
}: {
  text: string;
  className?: string;
  raw?: boolean;
}) {
  return (
    <div className={`rc-markdown ${className}`}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: "h3",
          h2: "h3",
          h3: "h4",
          a: ({ node: _node, children, ...props }) => (
            <a {...props} target="_blank" rel="noopener noreferrer">
              {children}
            </a>
          ),
          // Evidence can contain external tracking URLs. A textual image label
          // preserves the source without fetching external resources on view.
          img: ({ alt }) => (
            <span className="markdown-image-label">
              [Image: {alt || "unlabeled"}]
            </span>
          ),
          table: ({ node: _node, ...props }) => (
            <div
              className="markdown-table"
              tabIndex={0}
              role="region"
              aria-label="Evidence table"
            >
              <table {...props} />
            </div>
          ),
          pre: ({ children }) => {
            if (
              !isValidElement<{ children?: ReactNode; className?: string }>(
                children,
              )
            )
              return <pre>{children}</pre>;
            const language = /language-([\w+-]+)/.exec(
              children.props.className ?? "",
            )?.[1];
            return (
              <CodeBlock
                code={String(children.props.children ?? "")}
                language={language}
              />
            );
          },
        }}
      >
        {text}
      </ReactMarkdown>
      {raw && (
        <details className="markdown-original">
          <summary>Original text</summary>
          <pre>{text}</pre>
        </details>
      )}
    </div>
  );
}
