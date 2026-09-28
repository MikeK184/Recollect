// Opt-in measured retrieval, never a quality pass/fail gate selected after a run.
import { test, expect } from "@playwright/test";
import { createHash, randomUUID } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

test("HotpotQA public pooled-corpus retrieval baseline", async ({
  request,
}) => {
  test.skip(
    process.env.RECOLLECT_PUBLIC_BENCHMARK !== "1",
    "Explicit public benchmark only",
  );
  test.setTimeout(900_000);
  const input = readFileSync(
    "../.cache/public-benchmark-input/hotpotqa-validation-50.json",
  );
  const hash = createHash("sha256").update(input).digest("hex");
  expect(hash).toBe(
    "32dd92947d6c50194bc2e76588bc78ad6ad08805a4b05728336ac70cd7e19b96",
  );
  type Row = {
    id: string;
    question: string;
    supporting_facts: { title: string[] };
    context: { title: string[]; sentences: string[][] };
  };
  const rows: Row[] = JSON.parse(input.toString()).rows.map(
    (r: { row: Row }) => r.row,
  );
  expect(rows).toHaveLength(50);
  const auth = await request.post("/api/auth/login", {
    data: {
      username: process.env.RECOLLECT_OWNER_USERNAME,
      password: process.env.RECOLLECT_OWNER_PASSWORD,
    },
  });
  expect(auth.ok()).toBeTruthy();
  const csrf = (await auth.json()).csrf_token;
  const api = async (
    path: string,
    body?: unknown,
    method = body === undefined ? "GET" : "POST",
  ) => {
    const response = await request.fetch(path, {
      method,
      data: body,
      headers: { "x-csrf-token": csrf, "Idempotency-Key": randomUUID() },
    });
    expect(response.ok(), `${path}: ${response.status()}`).toBeTruthy();
    return response.json();
  };
  const brain = await api("/api/brains", {
    name: "Public HotpotQA retrieval benchmark",
  });
  const base = `/api/brains/${brain.id}`;
  expect((await api(base + "/models/policy")).current.policy.enabled).toBe(
    false,
  );
  const documents = new Map<
    string,
    { title: string; content: string; source?: string; version?: string }
  >();
  for (const row of rows)
    row.context.title.forEach((title, index) => {
      const content = title + "\n" + row.context.sentences[index].join("");
      documents.set(title + "\0" + content, { title, content });
    });
  for (const doc of documents.values()) {
    const source = await api(base + "/sources", {
      title: doc.title,
      content: doc.content,
      media_type: "text/plain",
      retain_content: true,
    });
    doc.source = source.id;
    doc.version = source.version.id;
  }
  for (const doc of documents.values()) {
    await expect
      .poll(
        async () =>
          (await api(`${base}/sources/${doc.source}/versions/${doc.version}`))
            .version.processing,
        { timeout: 120_000 },
      )
      .toBe("ready");
  }
  const byVersion = new Map(
    [...documents.values()].map((d) => [d.version, d.title]),
  );
  const measure = async (channels: string[]) => {
    const report = {
      complete: false,
      dataset: "hotpotqa/hotpot_qa",
      split: "validation",
      config: "distractor",
      offset: 0,
      questions: 50,
      input_sha256: hash,
      documents: documents.size,
      lane: channels.join("+"),
      attempts: [] as {
        question: string;
        request_id: string | null;
        complete: boolean;
      }[],
      model_usage: {} as unknown,
      semantic_index: {} as unknown,
      limit: 10,
      context_bytes: 16384,
      model_requests: 0,
      timestamp: new Date().toISOString(),
      metrics: {} as Record<string, number>,
      results: [] as {
        id: string;
        relevant: string[];
        retrieved: string[];
        recall: number;
        precision: number;
        complete_support: boolean;
        elapsed_ms: number;
        context_bytes: number;
        semantic: unknown;
        coverage: unknown;
      }[],
    };
    const out = `../.cache/public-benchmark-${randomUUID()}`;
    mkdirSync(out, { recursive: true });
    const save = () =>
      writeFileSync(
        out + "/report.json",
        JSON.stringify(report, null, 2) + "\n",
      );
    save();
    for (const row of rows) {
      const attempt = {
        question: row.id,
        request_id: channels.includes("semantic") ? randomUUID() : null,
        complete: false,
      };
      report.attempts.push(attempt);
      save();
      const start = performance.now();
      const result = await api(base + "/recall", {
        query: row.question,
        channels,
        semantic_request_id: attempt.request_id,
        limit: 10,
        context_bytes: 16384,
        source_diversity: true,
      });
      const elapsed_ms = performance.now() - start;
      expect(result.context_bytes).toBeLessThanOrEqual(16384);
      expect(result.context.items.length).toBeLessThanOrEqual(10);
      const titles = new Set<string>();
      for (const item of result.context.items) {
        expect(
          byVersion.has(item.id),
          "Only corpus evidence is retrieved",
        ).toBeTruthy();
        titles.add(byVersion.get(item.id)!);
      }
      const gold = new Set(row.supporting_facts.title);
      const hits = [...gold].filter((t) => titles.has(t)).length;
      report.results.push({
        id: row.id,
        relevant: [...gold],
        retrieved: [...titles],
        recall: hits / gold.size,
        precision: titles.size ? hits / titles.size : 0,
        complete_support: hits === gold.size,
        elapsed_ms,
        context_bytes: result.context_bytes,
        semantic: result.semantic,
        coverage: result.coverage,
      });
      attempt.complete = true;
      save();
    }
    const times = report.results.map((r) => r.elapsed_ms).sort((a, b) => a - b);
    const mean = (values: number[]) =>
      values.reduce((a, b) => a + b, 0) / values.length;
    report.metrics = {
      supporting_document_recall_at_10: mean(
        report.results.map((r) => r.recall),
      ),
      complete_support_rate: mean(
        report.results.map((r) => Number(r.complete_support)),
      ),
      precision_at_returned: mean(report.results.map((r) => r.precision)),
      empty_queries: report.results.filter((r) => r.retrieved.length === 0)
        .length,
      p50_ms: times[24],
      p95_ms: times[47],
      max_ms: times[49],
    };
    report.model_usage = await api(base + "/models/usage");
    report.model_requests = (report.model_usage as { total: number }).total;
    if (!channels.includes("semantic")) expect(report.model_requests).toBe(0);
    else report.semantic_index = await api(base + "/semantic");
    report.complete = true;
    save();
    console.log(
      JSON.stringify({
        report: out + "/report.json",
        documents: report.documents,
        metrics: report.metrics,
        model_requests: report.model_requests,
        lane: report.lane,
      }),
    );
  };
  await measure(["exact", "lexical"]);
  if (process.env.RECOLLECT_PUBLIC_SEMANTIC === "1") {
    const current = (await api(base + "/models/policy")).current;
    await api(
      base + "/models/policy",
      {
        base_change: current.change_id,
        policy: {
          ...current.policy,
          enabled: true,
          automatic_learning: false,
          autonomous_memory: false,
          automatic_embedding: true,
          purposes: ["embedding"],
          content_classes: ["document", "query"],
          daily_token_limit: 500000,
          max_input_bytes: 16384,
          max_output_tokens: 1024,
          max_concurrent: 2,
        },
      },
      "PUT",
    );
    await expect
      .poll(
        async () => {
          const index = await api(base + "/semantic");
          if (index.counts.failed || index.counts.blocked)
            throw new Error(
              "Semantic preparation failed; inspect the retained run before any paid retry.",
            );
          return (
            index.counts.ready >= documents.size &&
            index.counts.pending +
              index.counts.queued +
              index.counts.running ===
              0
          );
        },
        { timeout: 600000, intervals: [2000] },
      )
      .toBe(true);
    await measure(["exact", "lexical", "semantic"]);
  }
});
