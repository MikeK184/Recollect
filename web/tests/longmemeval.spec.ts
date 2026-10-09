// Paid opt-in, public inputs only, durable identities before dispatch and no retries.
import { test, expect } from "@playwright/test";
import { createHash, randomUUID } from "node:crypto";
import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";

test("frozen LongMemEval-S answer pipeline", async ({ request }) => {
  test.skip(
    process.env.RECOLLECT_LONGMEM_BENCHMARK !== "1",
    "Explicit paid benchmark only",
  );
  test.setTimeout(21_600_000);
  const input = readFileSync("../.cache/longmemeval-input/subset-50.json");
  const manifest = JSON.parse(
    readFileSync("../.cache/longmemeval-input/manifest.json", "utf8"),
  );
  expect(createHash("sha256").update(input).digest("hex")).toBe(
    manifest.sha256["subset-50.json"],
  );
  type Row = {
    question_id: string;
    question_type: string;
    question: string;
    question_date: string;
    answer: string;
    haystack_dates: string[];
    haystack_session_ids: string[];
    haystack_sessions: { role: string; content: string }[][];
    answer_session_ids: string[];
  };
  const rows: Row[] = JSON.parse(input.toString());
  const count = Number(process.env.RECOLLECT_LONGMEM_COUNT ?? 50);
  expect([7, 50]).toContain(count);
  const out =
    process.env.RECOLLECT_LONGMEM_RESUME_DIR ??
    `../.cache/longmemeval-${randomUUID()}`;
  mkdirSync(out, { recursive: true });
  const report: any = process.env.RECOLLECT_LONGMEM_RESUME_DIR
    ? JSON.parse(readFileSync(out + "/report.json", "utf8"))
    : {
        complete: false,
        manifest,
        count,
        provider: "openrouter",
        text_model: "z-ai/glm-4.7-flash",
        embedding_model: "qwen/qwen3-embedding-8b",
        embedding_dimensions: 1024,
        recall: {
          channels: ["exact", "lexical", "semantic"],
          limit: 10,
          context_bytes: 16384,
          source_diversity: true,
        },
        learning: false,
        extraction: false,
        prompt: "brain-answer-2",
        started_at: new Date().toISOString(),
        actual_cost_usd: 0,
        rows: [] as any[],
      };
  const save = () => {
    const order = new Map(rows.map((row, index) => [row.question_id, index]));
    report.rows.sort((a: any, b: any) => order.get(a.id)! - order.get(b.id)!);
    writeFileSync(
      out + "/report.json.tmp",
      JSON.stringify(report, null, 2) + "\n",
    );
    renameSync(out + "/report.json.tmp", out + "/report.json");
    writeFileSync(
      out + "/hypotheses.jsonl",
      report.rows
        .filter((r: any) => r.complete)
        .map((r: any) =>
          JSON.stringify({ question_id: r.id, hypothesis: r.hypothesis }),
        )
        .join("\n") + "\n",
    );
  };
  const budget = async () => {
    const response = await fetch("https://openrouter.ai/api/v1/key", {
      headers: { Authorization: `Bearer ${process.env.OPENROUTER_API_KEY}` },
      signal: AbortSignal.timeout(15_000),
    });
    expect(response.ok).toBeTruthy();
    const key = (await response.json()).data;
    const unresolved =
      report.rows.reduce(
        (sum: number, row: any) =>
          sum + (row.unreconciled_cost_requests?.length ?? 0),
        0,
      ) * 0.1;
    expect((report.prior_campaign_commitment_usd ?? 0) + report.actual_cost_usd + unresolved + 0.1)
      .toBeLessThanOrEqual(8);
    expect(
      key.usage +
        0.1 +
        unresolved +
        (report.warm_index?.unresolved_reservation_usd ?? 0),
    ).toBeLessThanOrEqual(8);
    expect(key.limit_remaining).toBeGreaterThan(2.1);
    return { usage: key.usage, remaining: key.limit_remaining };
  };
  save();
  const auth = await request.post("/api/auth/login", {
    data: {
      username: process.env.RECOLLECT_OWNER_USERNAME,
      password: process.env.RECOLLECT_OWNER_PASSWORD,
    },
  });
  expect(auth.ok()).toBeTruthy();
  const csrf = (await auth.json()).csrf_token;
  class ApiFailure extends Error {
    constructor(
      path: string,
      readonly status: number,
      readonly code: string,
    ) {
      super(`${path}: HTTP ${status} ${code}`);
    }
  }
  const api = async (
    path: string,
    data?: unknown,
    method = data === undefined ? "GET" : "POST",
    commandId = randomUUID(),
  ) => {
    const response = await request.fetch(path, {
      method,
      data,
      headers: { "x-csrf-token": csrf, "Idempotency-Key": commandId },
      timeout: 60_000,
    });
    if (!response.ok())
      throw new ApiFailure(
        path,
        response.status(),
        (await response.json()).code,
      );
    return response.json();
  };
  const date = (value: string) => {
    const match =
      /^(\d{4})\/(\d{2})\/(\d{2}) \([A-Za-z]+\) (\d{2}:\d{2})$/.exec(value);
    if (!match) throw new Error("Unexpected frozen date format");
    return `${match[1]}-${match[2]}-${match[3]}T${match[4]}:00Z`;
  };
  const ledger = async (base: string, record: any) => {
    const usage = await api(base + "/models/usage");
    const requests: any[] = [];
    for (let offset = 0; offset < usage.total; offset += 20)
      requests.push(
        ...(await api(base + `/models/usage?offset=${offset}`)).requests,
      );
    const baseline = new Set(record.baseline_request_ids ?? []);
    record.model_ledger = requests.filter((r: any) => !baseline.has(r.id));
    save();
  };
  for (const row of rows.slice(0, count)) {
    if (report.rows.some((r: any) => r.id === row.question_id && r.complete))
      continue;
    const pace = Number(process.env.RECOLLECT_LONGMEM_PACE_MS ?? 0);
    expect(Number.isFinite(pace) && pace >= 0 && pace <= 30000).toBeTruthy();
    if (pace) await new Promise((resolve) => setTimeout(resolve, pace));
    const previous = report.rows.find((r: any) => r.id === row.question_id);
    const record: any = previous ?? {
      id: row.question_id,
      type: row.question_type,
      abstention: row.question_id.endsWith("_abs"),
      sessions: row.haystack_sessions.length,
      question: row.question,
      asked_question: `As of ${date(row.question_date)}, ${row.question}`,
      started_at: new Date().toISOString(),
      complete: false,
      account_before: await budget(),
      reservation_usd: 0.1,
      model_ledger: [],
    };
    if (!previous) report.rows.push(record);
    record.account_before ??= await budget();
    record.reservation_usd ??= 0.1;
    save();
    const brain = record.brain
      ? { id: record.brain }
      : await api("/api/brains", {
          name: `LongMemEval public ${row.question_id}`,
          managed_memory: false,
        });
    record.brain = brain.id;
    save();
    const base = `/api/brains/${brain.id}`;
    const versions = new Map<string, string>();
    const sources: any[] = record.sources ?? [];
    record.sources = sources;
    record.source_request_ids ??= [];
    for (
      let i = 0;
      i < row.haystack_sessions.length &&
      record.pipeline_failure?.stage !== "ingestion";
      i++
    ) {
      if (sources[i]) {
        versions.set(sources[i].version.id, row.haystack_session_ids[i]);
        continue;
      }
      // Do not serialize has_answer, answer_session_ids, question or gold answer.
      const content =
        `Conversation date: ${row.haystack_dates[i]}\n` +
        row.haystack_sessions[i]
          .map((t) => `${t.role}: ${t.content}`)
          .join("\n\n");
      record.source_request_ids[i] ??= randomUUID();
      save();
      let source: any;
      try {
        source = await api(
          base + "/sources",
          {
            title: `Conversation ${row.haystack_dates[i]}`,
            content,
            media_type: "text/plain",
            retain_content: true,
            observed_at: date(row.haystack_dates[i]),
          },
          "POST",
          record.source_request_ids[i],
        );
      } catch (error) {
        record.import_failure = {
          session_index: i,
          command_id: record.source_request_ids[i],
          at: new Date().toISOString(),
          error: String(error),
          status: error instanceof ApiFailure ? error.status : null,
          code: error instanceof ApiFailure ? error.code : null,
        };
        if (
          error instanceof ApiFailure &&
          error.status === 400 &&
          error.code === "invalid_input"
        ) {
          record.pipeline_failure = {
            stage: "ingestion",
            code: error.code,
            status: error.status,
            session_index: i,
            command_id: record.source_request_ids[i],
            at: record.import_failure.at,
          };
          save();
          break;
        }
        save();
        throw error;
      }
      sources.push({ id: source.id, version: { id: source.version.id } });
      save();
      versions.set(source.version.id, row.haystack_session_ids[i]);
    }
    if (record.pipeline_failure?.stage === "ingestion") {
      // The product rejected the frozen history. Preserve the case and its
      // original inputs; do not bypass admission or dispatch an incomplete Ask.
      await ledger(base, record);
      expect(record.model_ledger).toHaveLength(0);
      record.pipeline_state = "failed";
      record.hypothesis = "";
      record.elapsed_ms = null;
      record.gold_sessions = row.answer_session_ids;
      record.retrieved_sessions = null;
      record.session_recall = null;
      record.actual_cost_usd = 0;
      record.unreconciled_cost_requests = [];
      record.account_after = await budget();
      record.complete = true;
      save();
      console.log(
        JSON.stringify({
          id: record.id,
          state: "failed",
          stage: "ingestion",
          code: record.pipeline_failure.code,
        }),
      );
      continue;
    }
    for (const source of sources)
      await expect
        .poll(
          async () =>
            (
              await api(
                `${base}/sources/${source.id}/versions/${source.version.id}`,
              )
            ).version.processing,
          { timeout: 120_000 },
        )
        .toBe("ready");
    if (!record.configured) {
      const current = (await api(base + "/models/policy")).current;
      await api(base + "/models/catalogue?provider=openrouter", null);
      await api(
        base + "/models/policy",
        {
          base_change: current.change_id,
          rebuild_embeddings: true,
          policy: {
            ...current.policy,
            enabled: true,
            provider: report.provider,
            text_model: report.text_model,
            embedding_model: report.embedding_model,
            embedding_dimensions: 1024,
            purposes: ["embedding", "answering"],
            content_classes: ["document", "query"],
            automatic_learning: false,
            autonomous_memory: false,
            automatic_embedding: true,
            max_input_bytes: 32768,
            max_output_tokens: 4096,
            daily_token_limit: 2_000_000,
            max_concurrent: 2,
          },
        },
        "PUT",
      );
      record.configured = true;
      save();
    }
    await expect
      .poll(
        async () => {
          const index = await api(base + "/semantic?summary=true");
          record.semantic = index;
          save();
          if (index.counts.failed || index.counts.blocked) {
            await ledger(base, record);
            if (record.model_ledger.some((r: any) => r.state === "uncertain"))
              throw new Error("Uncertain indexing attempt; no paid retry");
            const failed = record.model_ledger.filter(
              (r: any) => r.purpose === "embedding" && r.state === "failed",
            );
            if (!index.counts.failed || !failed.length)
              throw new Error(
                "Indexing failure requires request reconciliation",
              );
            record.pipeline_failure ??= {
              stage: "indexing",
              code: failed[0].error_code,
              failed_request_ids: failed.map((r: any) => r.id),
              at: new Date().toISOString(),
              semantic: index,
            };
            save();
            return true;
          }
          return index.state === "ready";
        },
        { timeout: 900_000, intervals: [2000] },
      )
      .toBe(true);
    if (record.pipeline_failure?.stage === "indexing") {
      // Keep the failed case in the denominator. Cancel its remaining index
      // work through the product policy fence; never retry a failed batch.
      if (!record.index_cancellation) {
        const current = (await api(base + "/models/policy")).current;
        record.index_cancellation = await api(
          base + "/models/policy",
          {
            base_change: current.change_id,
            rebuild_embeddings: false,
            policy: { ...current.policy, automatic_embedding: false },
          },
          "PUT",
        );
        save();
      }
      await expect
        .poll(
          async () => {
            await ledger(base, record);
            if (record.model_ledger.some((r: any) => r.state === "uncertain"))
              throw new Error("Uncertain indexing attempt; retain reservation");
            return record.model_ledger.every(
              (r: any) => r.state === "succeeded" || r.state === "failed",
            );
          },
          { timeout: 120_000 },
        )
        .toBe(true);
      record.pipeline_state = "failed";
      record.hypothesis = "";
      record.elapsed_ms = null;
      record.gold_sessions = row.answer_session_ids;
      record.retrieved_sessions = null;
      record.session_recall = null;
    } else {
      if (record.request_id && !record.answer_response)
        throw new Error(
          "An answer identity is already reserved; reconcile before any paid retry",
        );
      record.request_id ??= randomUUID();
      save();
      const retainedAnswer = record.answer_response;
      const start = performance.now();
      const answer =
        record.answer_response ??
        (await api(base + "/answer-requests", {
          request_id: record.request_id,
          question: record.asked_question,
          recall: report.recall,
        }));
      if (!retainedAnswer) record.elapsed_ms = performance.now() - start;
      record.answer_response = answer;
      record.pipeline_state = answer.state;
      record.hypothesis =
        answer.state === "failed"
          ? ""
          : answer.answer
            ? [
                answer.answer.summary,
                ...answer.answer.statements.map((s: any) => s.text),
                ...answer.answer.limitations,
              ].join("\n")
            : "Unable to answer: no supporting evidence was found.";
      const retrieved = [
        ...new Set<string>(
          (answer.recall?.context.items ?? [])
            .map((i: any) => versions.get(i.id))
            .filter(Boolean),
        ),
      ];
      record.gold_sessions = row.answer_session_ids;
      record.retrieved_sessions = retrieved;
      record.session_recall = row.answer_session_ids.length
        ? row.answer_session_ids.filter((id) => retrieved.includes(id)).length /
          row.answer_session_ids.length
        : null;
      // Persist the original response and elapsed observation even when its
      // uncertain outcome stops the experiment after ledger reconciliation.
      save();
    }
    await ledger(base, record);
    for (const receipt of record.model_ledger.filter((r: any) => r.purpose === "answering"))
      expect(receipt.prompt_label).toBe(report.prompt);
    record.actual_cost_usd = record.model_ledger.reduce(
      (sum: number, r: any) => sum + (r.cost_usd ?? 0),
      0,
    );
    record.unreconciled_cost_requests = record.model_ledger
      .filter((r: any) => r.cost_usd === null)
      .map((r: any) => r.id);
    report.actual_cost_usd = report.rows.reduce(
      (sum: number, r: any) => sum + (r.actual_cost_usd ?? 0),
      0,
    );
    record.account_after = await budget();
    save();
    if (!["completed", "no_evidence", "failed"].includes(record.pipeline_state))
      throw new Error(
        `Answer pipeline failed: ${record.answer_response?.failure_code}; no automatic retry`,
      );
    expect(
      record.model_ledger.every(
        (r: any) => r.state === "succeeded" || r.state === "failed",
      ),
    ).toBeTruthy();
    record.complete = true;
    save();
    console.log(
      JSON.stringify({
        report: out + "/report.json",
        id: record.id,
        state: record.pipeline_state,
        session_recall: record.session_recall,
        cost_usd: record.actual_cost_usd,
        elapsed_ms: record.elapsed_ms,
      }),
    );
  }
  report.complete = true;
  report.pipeline_states = Object.fromEntries(
    ["completed", "no_evidence", "failed"].map((state) => [
      state,
      report.rows.filter(
        (r: any) => (r.pipeline_state ?? r.answer_response?.state) === state,
      ).length,
    ]),
  );
  save();
});
