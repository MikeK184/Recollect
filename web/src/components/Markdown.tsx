import { lazy, Suspense } from "react";
import "./markdown.css";

const Renderer = lazy(() =>
  import("./MarkdownRenderer").then((module) => ({
    default: module.MarkdownRenderer,
  })),
);

export function Markdown(props: {
  text: string;
  className?: string;
  raw?: boolean;
}) {
  return (
    <Suspense
      fallback={
        <div className={`rc-markdown ${props.className ?? ""}`}>
          <pre className="markdown-loading">{props.text}</pre>
        </div>
      }
    >
      <Renderer {...props} />
    </Suspense>
  );
}

export { CodeBlock } from "./CodeBlock";
