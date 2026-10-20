import { Alert, Button } from "./components/ui";

export const label = (s: string) =>
  (
    ({
      extraction: "Source learning",
      synthesis: "Memory maintenance",
      embedding: "Semantic search",
      reranking: "Result ordering",
      answering: "Ask answers",
      document: "Documents",
      raw_session: "Captured sessions",
      tool_output: "Tool results",
      support_excerpt: "Supporting excerpts",
      repository: "Repository text",
      claim: "Memory",
      query: "Questions",
      provider_incomplete: "The model reached its output limit",
      provider_shape: "The model returned an invalid structured result",
      model_input_too_large: "The source exceeds the input byte limit",
      model_input_sensitive: "The source contains sensitive input",
      model_budget_exhausted:
        "The request exceeds the remaining daily token allowance",
    }) as Record<string, string>
  )[s] ?? s.replaceAll("_", " ");
export const time = (s: string) => new Date(s).toLocaleString();
export function Failure({ error }: { error: Error | null }) {
  return error ? <Alert variant="destructive">{error.message}</Alert> : null;
}
export function Pages({
  offset,
  total,
  change,
}: {
  offset: number;
  total: number;
  change: (v: number) => void;
}) {
  return total > 20 ? (
    <div className="flex items-center gap-2">
      <Button
        variant="ghost"
        size="sm"
        disabled={!offset}
        onClick={() => change(Math.max(0, offset - 20))}
      >
        Previous
      </Button>
      <span className="mono text-caption text-muted-foreground">
        {offset + 1}–{Math.min(total, offset + 20)} of {total}
      </span>
      <Button
        variant="ghost"
        size="sm"
        disabled={offset + 20 >= total}
        onClick={() => change(offset + 20)}
      >
        Next
      </Button>
    </div>
  ) : null;
}
