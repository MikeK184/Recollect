import { test, expect } from "@playwright/test";
import { formatToolDescription } from "../src/features/connections/toolDescription";

test("legacy descriptions gain lists without changing words or inline code", () => {
  const source =
    "Find a library. Each result includes: - Library ID: identifier - Name: title. Selection Process: 1. Check query 2. Return matches. Response Format: - Explain choice - Ask if ambiguous. Use `value: - one - two 1. keep 2. exact`.";
  const rendered = formatToolDescription(source);
  expect(rendered).toContain("includes:\n\n- Library ID:");
  expect(rendered).toContain("\n- Name:");
  expect(rendered).toContain("\n\nSelection Process:\n\n1. Check query");
  expect(rendered).toContain("\n\n2. Return matches.");
  expect(rendered).toContain("`value: - one - two 1. keep 2. exact`");
  expect(rendered.replace(/\s+/g, " ")).toBe(source);
  const codeList = "Parameters: - `foo`: first - `bar`: second";
  expect(formatToolDescription(codeList)).toBe(
    "Parameters:\n\n- `foo`: first\n- `bar`: second",
  );
});

test("structured code and source line breaks remain byte-exact", () => {
  const source =
    'Plain\nline\n\n```json\n{"example":"value: - one - two"}\n```\n\n- Existing list\n';
  expect(formatToolDescription(source)).toBe(source);
  expect(formatToolDescription("Version 1.2 supports A - B.")).toBe(
    "Version 1.2 supports A - B.",
  );
  const inline =
    "Use ``key: - first - `middle` - final`` and `key: - one - two`.";
  expect(formatToolDescription(inline)).toBe(inline);
});

test("vendor indented prose is readable without changing real code blocks", () => {
  const source =
    "Search the web.\n\n      Best for: Public documentation.\n      Returns: Search results.\n\n      Query tips:\n      Describe the ideal page.";
  expect(formatToolDescription(source)).toBe(
    "Search the web.\n\nBest for: Public documentation.\nReturns: Search results.\n\nQuery tips:\nDescribe the ideal page.",
  );
  const code = "Example:\n\n    const count = 4;\n    console.log(count);";
  expect(formatToolDescription(code)).toBe(code);
  const fenced = 'Search.\n\n    Best for: Code.\n\n```json\n{"x":1}\n```';
  expect(formatToolDescription(fenced)).toBe(fenced);
  const mixed = "Search.\n\n    Best for: Examples.\n    const example = 4;";
  expect(formatToolDescription(mixed)).toBe(mixed);
  const command =
    "Search.\n\n    Best for: API testing.\n    curl -X POST https://example.com/api";
  expect(formatToolDescription(command)).toBe(command);
  const commandInTips =
    "Search.\n\n    Best for: API testing.\n    Returns: Results.\n\n    Query tips:\n    curl -X POST https://example.com/api.";
  expect(formatToolDescription(commandInTips)).toBe(commandInTips);
});
