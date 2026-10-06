import { test, expect } from "@playwright/test";
import { formatJson } from "../src/components/jsonFormatting";

test("JSON formatting preserves precision, strings and original number tokens", () => {
  const source =
    '{"big":9007199254740993,"decimal":0.1234567890123456789,"exponent":1e999,"text":"quoted \\"value\\" and [,:]","nested":[{},[]]}';
  const pretty = formatJson(source)!;
  expect(pretty).toContain("9007199254740993");
  expect(pretty).toContain("0.1234567890123456789");
  expect(pretty).toContain("1e999");
  expect(pretty).toContain('"quoted \\"value\\" and [,:]"');
  expect(formatJson(pretty, true)).toBe(source);
  expect(pretty).toContain('\n  "nested": [\n    {},\n    []\n  ]');
});

test("invalid and oversized JSON stays unchanged by refusing formatting", () => {
  expect(formatJson('{"bad":')).toBeNull();
  expect(formatJson("/* not JSON */ {}")).toBeNull();
  expect(formatJson('"' + "x".repeat(524288) + '"')).toBeNull();
  expect(formatJson("[".repeat(65) + "0" + "]".repeat(65))).toBeNull();
  expect(formatJson(' "unchanged string" ')).toBe('"unchanged string"');
});
