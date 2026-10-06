/** Validate JSON, then format its original tokens rather than serializing parsed
 * numbers. This preserves large integers, decimal precision and quoted content.
 */
export function formatJson(source: string, compact = false): string | null {
  if (source.length > 524288) return null;
  try {
    JSON.parse(source);
  } catch {
    return null;
  }
  const tokens = source.match(/"(?:[^"\\]|\\.)*"|[^\s]/g) ?? [];
  let depth = 0;
  let formatted = "";
  const newline = () => (compact ? "" : "\n" + "  ".repeat(depth));
  for (let i = 0; i < tokens.length; i++) {
    const token = tokens[i];
    if (token === "{" || token === "[") {
      formatted += token;
      depth++;
      if (depth > 64) return null;
      if (tokens[i + 1] !== (token === "{" ? "}" : "]")) formatted += newline();
    } else if (token === "}" || token === "]") {
      depth--;
      if (tokens[i - 1] !== (token === "}" ? "{" : "[")) formatted += newline();
      formatted += token;
    } else if (token === ",") {
      formatted += token + newline();
    } else if (token === ":") {
      formatted += compact ? ":" : ": ";
    } else {
      formatted += token;
    }
    if (formatted.length > 1048576) return null;
  }
  return formatted;
}
