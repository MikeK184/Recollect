// Explicit local proof against a separately started repository-owned database.
// Every paid query identity is persisted before dispatch; an interrupted run
// refuses automatic replay. Only the checked-in synthetic corpus is sent.
import { request } from "../web/node_modules/@playwright/test/index.mjs";
import { readFile, writeFile, mkdir, rename } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import assert from "node:assert/strict";
import path from "node:path";

const origin = process.env.RECOLLECT_FUSION_PROOF_ORIGIN;
assert.equal(
  origin,
  "http://127.0.0.1:8789",
  "Use the isolated fusion proof listener",
);
const root = path.resolve(process.argv[2] ?? ".cache/fusion-live-proof");
assert(
  root.startsWith(path.resolve(".cache") + path.sep),
  "Proof files stay in this repository cache",
);
await mkdir(root, { recursive: true });
try {
  await readFile(root + "/state.json");
  throw new Error(
    "Existing proof state: inspect it before an explicit new run; paid attempts are never replayed automatically.",
  );
} catch (e) {
  if (e.code !== "ENOENT") throw e;
}
const corpus = JSON.parse(
  await readFile("crates/server/tests/fixtures/semantic-corpus.json", "utf8"),
);
const evaluation = JSON.parse(
  await readFile("crates/server/tests/fixtures/fusion-corpus.json", "utf8"),
);
const state = {
  run: randomUUID(),
  stage: "starting",
  complete: false,
  documents: {},
  claims: {},
  attempts: [],
};
const save = async () => {
  await writeFile(root + "/state.tmp", JSON.stringify(state, null, 2) + "\n", {
    mode: 0o600,
  });
  await rename(root + "/state.tmp", root + "/state.json");
};
await save();
const client = await request.newContext({ baseURL: origin, timeout: 100000 });
try {
  const response = await client.post("/api/auth/login", {
    data: {
      username: process.env.RECOLLECT_OWNER_USERNAME,
      password: process.env.RECOLLECT_OWNER_PASSWORD,
    },
  });
  assert.equal(response.status(), 200, "Proof login failed");
  const csrf = (await response.json()).csrf_token;
  const api = async (url, method = "GET", data) => {
    const response = await client.fetch(url, {
      method,
      data,
      headers: { "x-csrf-token": csrf, "Idempotency-Key": randomUUID() },
    });
    const wire = await response.text();
    const body = JSON.parse(wire);
    if (!response.ok())
      throw new Error(
        `Proof ${method} ${url}: ${response.status()} ${body.code}`,
      );
    if (url.endsWith("/recall")) {
      // Check the bytes actually serialized by Rust: JS reserialization may
      // shorten an integral float such as 0.0 without changing its value.
      const marker = '"context":';
      const offset = wire.indexOf(marker);
      const end = wire.indexOf(',"context_bytes":', offset + marker.length);
      assert(offset >= 0 && end > offset, "Recall wire context was not found");
      assert.equal(
        Buffer.byteLength(wire.slice(offset + marker.length, end)),
        body.context_bytes,
      );
    }
    return body;
  };
  const brain = await api("/api/brains", "POST", {
    name: "Synthetic retrieval fusion evaluation",
  });
  state.brain = brain.id;
  const base = `/api/brains/${brain.id}`;
  await save();
  for (const doc of corpus.documents) {
    const source = await api(base + "/sources", "POST", {
      title: doc.title,
      content: doc.content,
      media_type: "text/plain",
      retain_content: true,
    });
    state.documents[doc.key] = {
      source: source.id,
      version: source.version.id,
    };
    await save();
  }
  for (const row of evaluation.claims) {
    const result = await api(base + "/claims", "POST", {
      content: {
        kind: "claim",
        subject: row.subject,
        predicate: row.predicate,
        value: row.value,
        rationale: "Synthetic declared evidence for fixed-corpus evaluation.",
        selection: { repository_ids: [], area_ids: [], environment_id: null },
        manifest_revision_id: null,
        validity: {
          kind: "unknown",
          from: null,
          to: null,
          precision: "unknown",
        },
        freshness: "current",
        operational: "declared",
        observed_at: null,
        observation: "",
        supports: [
          {
            kind: "source_version",
            id: state.documents[row.source].version,
            line_from: null,
            line_to: null,
          },
        ],
      },
    });
    state.claims[row.key] = {
      id: result.claim_id,
      revision: result.id,
      source: row.source,
    };
    await save();
  }
  const waitFor = async (read, ready, label) => {
    const deadline = Date.now() + 120000;
    while (Date.now() < deadline) {
      const value = await read();
      if (ready(value)) return value;
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
    throw new Error(`Proof readiness timed out: ${label}`);
  };
  for (const doc of Object.values(state.documents))
    await waitFor(
      () => api(`${base}/sources/${doc.source}/versions/${doc.version}`),
      (r) => r.version.processing === "ready",
      "source processing",
    );
  const graph = await api(base + "/graph/rebuild", "POST", {
    kind: "knowledge",
  });
  state.graph_generation = graph.id;
  await save();
  await waitFor(
    () => api(base + "/graph"),
    (r) => r.generations.some((g) => g.id === graph.id && g.state === "ready"),
    "graph",
  );
  const policy = await api(base + "/models/policy");
  const selected = {
    ...policy.current.policy,
    enabled: true,
    automatic_learning: false,
    autonomous_memory: false,
    automatic_embedding: true,
    purposes: ["embedding"],
    content_classes: ["document", "claim", "query"],
    daily_token_limit: 100000,
  };
  state.stage = "embedding approved synthetic corpus";
  await save();
  await api(base + "/models/policy", "PUT", {
    base_change: policy.current.change_id,
    policy: selected,
  });
  const indexed = await waitFor(
    () => api(base + "/semantic"),
    (r) => r.counts.ready === 17,
    "semantic representations",
  );
  assert.equal(indexed.profile.model, "text-embedding-3-large");
  assert.equal(indexed.profile.dimensions, 3072);
  state.profile = indexed.profile;
  state.index_usage = await api(base + "/models/usage");
  state.stage = "fixed channel ablations";
  await save();
  for (const question of evaluation.questions)
    for (const [channel, channels] of [
      ["baseline", ["exact", "lexical"]],
      ["graph", ["exact", "lexical", "graph"]],
      ["semantic", ["exact", "lexical", "semantic"]],
      ["semantic_graph", ["exact", "lexical", "semantic", "graph"]],
    ]) {
      for (const diversity of [false, true]) {
        const input = {
          query: question.query,
          channels,
          limit: evaluation.limit,
          context_bytes: evaluation.context_bytes,
          source_diversity: diversity,
          semantic_request_id: channels.includes("semantic")
            ? randomUUID()
            : null,
        };
        const attempt = {
          question: question.key,
          channel,
          diversity,
          input,
          expected: question.target ? state.claims[question.target].id : null,
          complete: false,
        };
        state.attempts.push(attempt);
        await save();
        const result = await api(base + "/recall", "POST", input);
        assert(result.context_bytes <= evaluation.context_bytes);
        const relevant = new Set(
          question.relevant_sources.flatMap((key) => [
            state.documents[key].version,
            ...Object.values(state.claims)
              .filter((c) => c.source === key)
              .map((c) => c.id),
          ]),
        );
        Object.assign(attempt, {
          complete: true,
          hit:
            attempt.expected !== null &&
            result.context.items.some((i) => i.id === attempt.expected),
          irrelevant: result.context.items.filter((i) => !relevant.has(i.id))
            .length,
          elapsed_ms: result.elapsed_ms,
          context_bytes: result.context_bytes,
          status: result.status,
          source_groups: result.context_selection.distinct_source_groups,
          coverage: result.coverage,
          graph: result.graph
            ? {
                state: result.graph.state,
                anchors: result.graph.anchors,
                candidates: result.graph.candidates,
              }
            : null,
          model_request_id: result.semantic?.model_request_id ?? null,
          returned: result.context.items.map((i) => ({
            id: i.id,
            label: i.label,
            channels: i.channels,
            score: i.score,
            semantic_similarity: i.semantic_similarity,
            graph_hops: i.graph_match?.edges.length ?? null,
          })),
        });
        await save();
      }
    }
  state.usage = await api(base + "/models/usage");
  state.complete = true;
  state.stage = "complete";
  await save();
  console.log(
    JSON.stringify({
      complete: true,
      brain: state.brain,
      queries: state.attempts.length,
      model_requests: state.usage.total,
    }),
  );
} finally {
  await client.dispose();
}
