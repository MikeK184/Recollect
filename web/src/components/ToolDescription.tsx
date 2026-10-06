import { Markdown } from "./Markdown";
import { formatToolDescription } from "../features/connections/toolDescription";

/** The compact summary is replaced by the full description while expanded. */
export function ToolDescription({ text }: { text: string }) {
  const formatted = formatToolDescription(text);
  if (!formatted.trim()) return null;
  const summary = formatted
    .split(/\r?\n[ \t]*\r?\n/)
    .find(
      (paragraph) =>
        paragraph.trim() && !/^(?:```|~~~|#{1,6}\s| {4}|\t)/.test(paragraph),
    );
  const hasMore =
    !summary || summary.length > 160 || formatted.trim() !== summary.trim();
  return (
    <div className="tool-description">
      {summary && (
        <Markdown text={summary} className="tool-description-preview" />
      )}
      {hasMore && (
        <details className="tool-description-details">
          <summary>Full description</summary>
          <Markdown text={formatted} className="markdown-preserve-lines" />
        </details>
      )}
    </div>
  );
}
