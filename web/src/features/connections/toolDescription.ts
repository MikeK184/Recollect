/** Restore only explicit list boundaries in old flattened metadata for display.
 * Stored metadata and code spans remain unchanged. Structured descriptions keep
 * their original whitespace and are rendered by the shared safe Markdown view.
 */
export function formatToolDescription(source: string): string {
  if (/[\r\n]/.test(source)) return normalizeVendorProse(source);
  const codeSpans = Array.from(
    source.matchAll(/(?<!`)(`+)(?!`)[\s\S]*?\1(?!`)/g),
  );
  const prose = source.replace(/(?<!`)(`+)(?!`)[\s\S]*?\1(?!`)/g, "");
  const lists = {
    bullets: /: - /.test(prose),
    numbered: /\b1\. /.test(prose) && /\b2\. /.test(prose),
  };
  let rendered = "";
  let cursor = 0;
  for (const match of codeSpans) {
    rendered += paragraphs(source.slice(cursor, match.index), lists) + match[0];
    cursor = match.index + match[0].length;
  }
  return rendered + paragraphs(source.slice(cursor), lists);
}

/** Some vendors indent all prose after the lead paragraph. Only unwrap a
 * uniformly indented labelled prose tail; real code/fences keep exact bytes.
 */
function normalizeVendorProse(source: string): string {
  const start = /(?:\r?\n){2}([ \t]{4,})(?:Best for|Returns|Query tips):/.exec(
    source,
  );
  if (!start || /(?:^|\n)[ \t]*(?:```|~~~)/.test(source.slice(start.index)))
    return source;
  const tail = source.slice(start.index + start[0].indexOf(start[1]));
  const lines = tail.split(/\r?\n/);
  const content = lines.filter((line) => line.trim());
  if (!content.every((line) => /^ {4,}\S/.test(line))) return source;
  // The audited vendor structure is a pair of labelled prose sentences followed
  // by a Query tips paragraph. Other indented examples stay code.
  const tips = content.findIndex((line) => /^\s+Query tips:\s*$/.test(line));
  if (
    tips !== 2 ||
    !/^\s+Best for:.+[.!?]$/.test(content[0]) ||
    !/^\s+Returns:.+[.!?]$/.test(content[1]) ||
    content.slice(tips + 1).some((line) => !/[.!?]["')]*$/.test(line.trim()))
  )
    return source;
  if (
    content.some((line) =>
      /^\s*(?:[{}\[\]<>$#]|(?:const|let|var|import|export|def|class|function|return|if|for|while|curl|wget|jq|npm|npx|node|python|python3|sh|bash|zsh|printf|echo|SELECT|INSERT|UPDATE|DELETE|CREATE)\b|[\w$.]+\s*(?:=|\())/.test(
        line,
      ),
    )
  )
    return source;
  const indent = Math.min(
    ...content.map((line) => /^ */.exec(line)![0].length),
  );
  return (
    source.slice(0, source.length - tail.length) +
    lines
      .map((line) => (line.trim() ? line.slice(indent) : line))
      .join(source.includes("\r\n") ? "\r\n" : "\n")
  );
}

function paragraphs(
  source: string,
  lists: { bullets: boolean; numbered: boolean },
): string {
  let text = source.replace(/(?<=[.!?)]) (?=[A-Z][A-Za-z ]{1,32}: )/g, "\n\n");
  if (lists.bullets) {
    text = text.replace(/: - /g, ":\n\n- ").replace(/ - /g, "\n- ");
  }
  if (lists.numbered) {
    text = text.replace(/ (\d+\. )/g, "\n\n$1");
  }
  return text;
}
