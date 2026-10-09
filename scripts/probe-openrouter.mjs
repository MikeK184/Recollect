// Explicit paid preflight: synthetic inputs only; never print credentials or vectors.
import { mkdir, writeFile } from "node:fs/promises";
const key = process.env.OPENROUTER_API_KEY;
if (!key) throw new Error("OPENROUTER_API_KEY is missing");
const dir = ".cache/openrouter-preflight";
await mkdir(dir, { recursive: true });
const report = { checked_at: new Date().toISOString(), requests: [] };
async function call(path, body) {
  const response = await fetch(`https://openrouter.ai/api/v1/${path}`, {
    method: body ? "POST" : "GET",
    headers: { Authorization: `Bearer ${key}`, "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
    signal: AbortSignal.timeout(45000),
  });
  if (!response.ok) {
    const value = await response.json().catch(() => ({}));
    return { error: `http_${response.status}`, error_message: String(value.error?.message ?? "").replaceAll(key, "[REDACTED]").slice(0,400) };
  }
  return await response.json();
}
const account = await call("key");
report.account = { limit: account.data?.limit, remaining: account.data?.limit_remaining, usage: account.data?.usage };
console.log(JSON.stringify({ account: report.account }));
const schemas = { type: "object", properties: { language: { type: "string" } }, required: ["language"], additionalProperties: false };
for (const model of ["meta/muse-spark-1.3-contributor", "z-ai/glm-5.3-flash"]) {
  const reply = await call("chat/completions", {
    model, messages: [{ role: "user", content: "The fictional user prefers Rust. Extract the programming language." }],
    max_tokens: 256, temperature: 0,
    provider: { require_parameters: true, allow_fallbacks: false },
    response_format: { type: "json_schema", json_schema: { name: "preflight", strict: true, schema: schemas } },
  });
  const row = { requested: model, returned: reply.model, error: reply.error, error_message: reply.error_message, finish_reason: reply.choices?.[0]?.finish_reason, usage: reply.usage, valid: reply.choices?.[0]?.message?.content === '{"language":"Rust"}' };
  if (reply.choices?.[0]?.message?.content) {
    try { row.valid = JSON.parse(reply.choices[0].message.content).language === "Rust"; } catch { row.valid = false; }
  }
  report.requests.push(row);
  await writeFile(`${dir}/report.json`, JSON.stringify(report, null, 2));
  console.log(JSON.stringify(row));
}
const model = "qwen/qwen3-embedding-8b";
const reply = await call("embeddings", { model, input: ["A fictional project uses Rust."], dimensions: 1024, encoding_format: "float", provider: { allow_fallbacks: false } });
const vector = reply.data?.[0]?.embedding;
const row = { requested: model, returned: reply.model, error: reply.error, dimensions: vector?.length, usage: reply.usage, valid: Array.isArray(vector) && vector.length === 1024 && vector.every(Number.isFinite) && vector.some(v => v !== 0) };
report.requests.push(row);
await writeFile(`${dir}/report.json`, JSON.stringify(report, null, 2));
console.log(JSON.stringify(row));
