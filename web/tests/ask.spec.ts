import { test, expect, type Page } from "@playwright/test";
import type { components } from "../src/api-schema";

type Answer = components["schemas"]["AnswerResponse"];
type Recall = components["schemas"]["RecallResponse"];

async function command(
  page: Page,
  path: string,
  data?: unknown,
  method = data === undefined ? "GET" : "POST",
) {
  const session = await (await page.request.get("/api/auth/me")).json();
  const response = await page.request.fetch(path, {
    method,
    data,
    headers: { "x-csrf-token": session.csrf_token },
  });
  if (!response.ok())
    throw new Error(
      `Ask fixture command failed: ${method} ${path} ${response.status()}`,
    );
  return response.status() === 204 ? null : response.json();
}

async function setup(page: Page, enabled = true) {
  await page.goto("/");
  await page
    .getByLabel(/^Username/)
    .fill(process.env.RECOLLECT_OWNER_USERNAME!);
  await page
    .getByLabel(/^Password/)
    .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Your Brains", exact: true }),
  ).toBeVisible();
  const brain = await command(page, "/api/brains", {
    name: "Temporary answer browser proof",
  });
  const base = `/api/brains/${brain.id}`;
  const source = await command(page, `${base}/sources`, {
    title: "AmberDesktop exact evidence",
    media_type: "text/plain",
    retain_content: true,
    content:
      "AmberDesktop uses port 8080.\nBROWSER_EXACT_ANSWER_EVIDENCE\n<script>window.answerEvidenceExecuted=true</script>\n",
  });
  await command(page, `${base}/claims`, {
    content: {
      kind: "claim",
      subject: "AmberDesktop",
      predicate: "port",
      value: "8080",
      rationale: "Synthetic declaration, not a verified deployment.",
      selection: { repository_ids: [], area_ids: [], environment_id: null },
      manifest_revision_id: null,
      validity: { kind: "unknown", from: null, to: null, precision: "unknown" },
      freshness: "current",
      operational: "declared",
      observed_at: null,
      observation: "",
      supports: [
        {
          kind: "source_version",
          id: source.version.id,
          line_from: 1,
          line_to: 1,
        },
      ],
    },
  });
  const settings = await command(page, `${base}/models/policy`);
  if (enabled)
    await command(
      page,
      `${base}/models/policy`,
      {
        base_change: settings.current.change_id,
        policy: {
          ...settings.current.policy,
          enabled: true,
          autonomous_memory: false,
          purposes: ["answering"],
          content_classes: ["claim", "query", "document"],
        },
      },
      "PUT",
    );
  return { brain: brain.id as string, base, source };
}

/**
 * Browser transport fixture only: evidence and its exact revisions come from
 * production Recall. The provider-backed answer POST and receipt are controlled
 * here so UI failures incur no external model calls. Production answer/provider
 * authority and durable metadata are independently covered by Rust HTTP tests.
 */
async function transport(page: Page, base: string) {
  const requests: { request_id: string; question: string }[] = [];
  const receipts = new Map<string, Answer>();
  const control = {
    statusFailure: false,
    expiresIn: 60_000,
    hold: null as Promise<void> | null,
  };
  // Presence is an installation fact; this transport test needs a usable button
  // even on a development installation without a real provider credential.
  await page.route(`**${base}/models/policy`, async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    if (route.request().method() === "GET")
      body.installed.credentials_present = true;
    await route.fulfill({ response, json: body });
  });
  await page.route(`**${base}/answer-requests`, async (route) => {
    const body = route.request().postDataJSON();
    requests.push({ request_id: body.request_id, question: body.question });
    const recall: Recall = await command(page, `${base}/recall`, {
      ...body.recall,
      query: "AmberDesktop",
    });
    expect(recall.context.items.length).toBeGreaterThan(0);
    const supported = recall.context.items.find(
      (item) => item.kind === "claim",
    )!;
    expect(supported).toBeTruthy();
    const now = new Date().toISOString();
    const answer: Answer = {
      request_id: body.request_id,
      state: "completed",
      answer: {
        summary: "Recorded configuration",
        statements: [
          {
            text: "BROWSER_TEMPORARY_ANSWER: AmberDesktop declares port 8080.\n\n```yaml\nport: 8080\n```\n\n| Setting | Value |\n| --- | --- |\n| Port | 8080 |",
            citation_ids: ["C1"],
          },
        ],
        limitations: ["This declaration does not establish deployed behavior."],
      },
      citations: [{ id: "C1", evidence: supported }],
      recall,
      model_request_id: null,
      failure_code: null,
      memory_epoch: recall.memory_epoch,
      expires_at: new Date(Date.now() + control.expiresIn).toISOString(),
      provider_may_have_run: false,
      created_at: now,
      finished_at: now,
    };
    receipts.set(body.request_id, {
      ...answer,
      answer: undefined,
      citations: [],
      recall: undefined,
    });
    if (control.hold) await control.hold;
    await route.fulfill({ json: answer }).catch(() => {});
  });
  await page.route(`**${base}/answer-requests/*`, async (route) => {
    if (control.statusFailure)
      return route.fulfill({
        status: 503,
        json: {
          code: "test_status_unavailable",
          message: "Synthetic validity read failed.",
        },
      });
    const id = new URL(route.request().url()).pathname.split("/").at(-1)!;
    const receipt = receipts.get(id);
    if (!receipt) return route.continue();
    const graph = await command(page, `${base}/graph`);
    await route.fulfill({
      json:
        graph.memory_epoch !== receipt.memory_epoch
          ? { ...receipt, state: "suppressed", failure_code: "answer_stale" }
          : receipt,
    });
  });
  return { requests, control };
}

test("disabled answering offers useful canonical search without enabling transmission", async ({
  page,
}) => {
  const f = await setup(page, false);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto(`/brains/${f.brain}/ask`);
  await expect(page).toHaveURL(new RegExp(`/brains/${f.brain}/ask$`));
  await expect(
    page.getByRole("button", { name: "Ask Brain", exact: true }),
  ).toBeInViewport();
  await expect(
    page.getByRole("region", { name: "Autonomous memory setup" }),
  ).toBeVisible();
  await page
    .getByLabel("Ask a question about this Brain", { exact: true })
    .fill("AmberDesktop");
  await expect(
    page.getByRole("button", { name: "Ask Brain", exact: true }),
  ).toBeDisabled();
  await page
    .getByRole("button", { name: "Find evidence", exact: true })
    .click();
  const search = page.getByRole("region", {
    name: "Recall memory",
    exact: true,
  });
  await expect(
    search.getByLabel("Search evidence", { exact: true }),
  ).toHaveValue("AmberDesktop");
  await search
    .getByRole("button", { name: "Find evidence", exact: true })
    .click();
  await expect(search.getByTestId("recall-result").first()).toBeVisible();
  expect(
    (await command(page, `${f.base}/models/policy`)).current.policy.enabled,
  ).toBe(false);
  expect((await command(page, `${f.base}/models/usage`)).total).toBe(0);
});

test("temporary answers render literal questions, exact citations, independent follow-ups and reset", async ({
  page,
  context,
}) => {
  const f = await setup(page);
  const proof = await transport(page, f.base);
  await page.goto(`/brains/${f.brain}/ask`);
  const question =
    "<script>window.answerQuestionExecuted=true</script> What is AmberDesktop port?";
  await page
    .getByLabel("Ask a question about this Brain", { exact: true })
    .fill(question);
  await page.getByRole("button", { name: "Ask Brain", exact: true }).click();
  await expect(page.getByText(question, { exact: true })).toBeVisible();
  await expect(page.locator(".answer-statement")).toContainText(
    "BROWSER_TEMPORARY_ANSWER: AmberDesktop declares port 8080.",
  );
  const statement = page.locator(".answer-statement").first();
  await expect(statement.getByRole("table")).toContainText("Port");
  await expect(statement.getByRole("table")).toContainText("8080");
  await expect(statement.getByLabel("yaml code")).toContainText("port: 8080");
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await statement
    .getByRole("button", { name: "Copy code", exact: true })
    .click();
  await expect
    .poll(() => page.evaluate(() => navigator.clipboard.readText()))
    .toBe("port: 8080\n");
  await expect(
    page.getByText("This declaration does not establish deployed behavior.", {
      exact: true,
    }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Open citation C1", exact: true })
    .click();
  const inspector = page.getByRole("complementary", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(inspector).toContainText("AmberDesktop");
  await page.screenshot({ path: "../.cache/ask-formatted-proof.png" });
  await inspector.getByRole("button", { name: /^Open source version/ }).click();
  const evidence = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(evidence.getByTestId("claim-evidence-text")).toContainText(
    "BROWSER_EXACT_ANSWER_EVIDENCE",
  );
  expect(
    await page.evaluate(
      () =>
        "answerQuestionExecuted" in window ||
        "answerEvidenceExecuted" in window,
    ),
  ).toBe(false);
  await evidence
    .getByRole("button", { name: "Back to answer", exact: true })
    .click();
  await page
    .getByLabel("Ask a question about this Brain", { exact: true })
    .fill("What evidence supports the AmberDesktop port?");
  await page.getByRole("button", { name: "Ask Brain", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Recorded configuration", exact: true }),
  ).toHaveCount(2);
  expect(proof.requests).toHaveLength(2);
  expect(
    new Set(proof.requests.map((request) => request.request_id)).size,
  ).toBe(2);
  expect(proof.requests[0].question).toBe(question);
  expect(new URL(page.url()).search).not.toContain("AmberDesktop");
  const stored = await page.evaluate(() =>
    [...Object.values(localStorage), ...Object.values(sessionStorage)].join(
      " ",
    ),
  );
  expect(stored).not.toContain("BROWSER_TEMPORARY_ANSWER");
  expect(stored).not.toContain(question);
  await page
    .getByRole("button", { name: "Clear conversation", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Recorded configuration", exact: true }),
  ).toHaveCount(0);
  await expect(inspector).toHaveCount(0);
  await page.reload();
  await expect(
    page.getByText("BROWSER_TEMPORARY_ANSWER", { exact: false }),
  ).toHaveCount(0);
  expect((await command(page, `${f.base}/models/usage`)).total).toBe(0);
});

test("failed answer validity reads, evidence epochs, policy revocation and expiry hide protected conversation payload", async ({
  page,
}) => {
  test.setTimeout(60_000);
  const f = await setup(page);
  const proof = await transport(page, f.base);
  await page.goto(`/brains/${f.brain}/ask`);
  const ask = async () => {
    await page
      .getByLabel("Ask a question about this Brain", { exact: true })
      .fill("What is AmberDesktop port?");
    await page.getByRole("button", { name: "Ask Brain", exact: true }).click();
    await expect(
      page.getByRole("heading", {
        name: "Recorded configuration",
        exact: true,
      }),
    ).toBeVisible();
  };
  await ask();
  await page
    .getByRole("button", { name: "Open citation C1", exact: true })
    .click();
  proof.control.statusFailure = true;
  await expect(
    page.getByText("BROWSER_TEMPORARY_ANSWER", { exact: false }),
  ).toHaveCount(0, { timeout: 8000 });
  await expect(
    page.getByRole("complementary", {
      name: "Supporting evidence",
      exact: true,
    }),
  ).toHaveCount(0);
  proof.control.statusFailure = false;
  await ask();
  // The server mutation really advances the canonical evidence epoch. The
  // transport fixture turns that into the same metadata-only suppressed receipt
  // that the independently tested answer handler publishes.
  await command(page, `${f.base}/sources/${f.source.id}/versions`, {
    title: "AmberDesktop updated evidence",
    media_type: "text/plain",
    retain_content: true,
    content: "AmberDesktop now declares port 9090.\n",
    base_version: f.source.version.id,
  });
  await expect(
    page.getByText("BROWSER_TEMPORARY_ANSWER", { exact: false }),
  ).toHaveCount(0, { timeout: 8000 });
  await ask();
  const settings = await command(page, `${f.base}/models/policy`);
  await command(
    page,
    `${f.base}/models/policy`,
    {
      base_change: settings.current.change_id,
      policy: { ...settings.current.policy, enabled: false },
    },
    "PUT",
  );
  await expect(
    page.getByText("BROWSER_TEMPORARY_ANSWER", { exact: false }),
  ).toHaveCount(0, { timeout: 8000 });
  await expect(
    page.getByRole("region", { name: "Autonomous memory setup" }),
  ).toBeVisible();
  const disabled = await command(page, `${f.base}/models/policy`);
  await command(
    page,
    `${f.base}/models/policy`,
    {
      base_change: disabled.current.change_id,
      policy: { ...disabled.current.policy, enabled: true },
    },
    "PUT",
  );
  await page.reload();
  proof.control.expiresIn = 1800;
  await ask();
  await expect(
    page.getByText("BROWSER_TEMPORARY_ANSWER", { exact: false }),
  ).toHaveCount(0, { timeout: 5000 });
  expect(proof.requests).toHaveLength(4);
  expect((await command(page, `${f.base}/models/usage`)).total).toBe(0);
});

test("production no-evidence answer and metadata replay never call a model", async ({
  page,
}) => {
  const f = await setup(page);
  const request_id = crypto.randomUUID();
  const question = "NO_MATCHING_SYNTHETIC_UI_SUBJECT_73629";
  const first: Answer = await command(page, `${f.base}/answer-requests`, {
    request_id,
    question,
    recall: { query: question, channels: ["exact", "lexical"] },
  });
  // Ordinary browser fixtures run without a paid provider credential. That
  // authority gate precedes retrieval; the Rust provider fixture separately
  // proves the ready-provider no-evidence branch without an external call.
  const installed = (await command(page, `${f.base}/models/policy`)).installed;
  const terminal = installed.credentials_present
    ? "no_evidence"
    : "unavailable";
  expect(first.state).toBe(terminal);
  if (!installed.credentials_present)
    expect(first.failure_code).toBe("model_credentials_missing");
  expect(first.provider_may_have_run).toBe(false);
  const replay: Answer = await command(page, `${f.base}/answer-requests`, {
    request_id,
    question,
    recall: { query: question, channels: ["exact", "lexical"] },
  });
  expect(replay.state).toBe(terminal);
  expect(replay.answer).toBeUndefined();
  expect(replay.recall).toBeUndefined();
  expect(
    JSON.stringify(
      await command(page, `${f.base}/answer-requests/${request_id}`),
    ),
  ).not.toContain(question);
  expect((await command(page, `${f.base}/models/usage`)).total).toBe(0);
});

test("switching Brains cancels a pending answer and ignores its late payload", async ({
  page,
}) => {
  const f = await setup(page);
  const second = await command(page, "/api/brains", {
    name: "Another answer scope",
  });
  const proof = await transport(page, f.base);
  let release!: () => void;
  proof.control.hold = new Promise<void>((resolve) => {
    release = resolve;
  });
  let cancellations = 0;
  page.on("request", (request) => {
    if (
      request.method() === "POST" &&
      request.url().includes(`${f.base}/answer-requests/`) &&
      request.url().endsWith("/cancel")
    )
      cancellations++;
  });
  await page.goto(`/brains/${f.brain}/ask`);
  await page
    .getByLabel("Ask a question about this Brain", { exact: true })
    .fill("What is AmberDesktop port?");
  await page.getByRole("button", { name: "Ask Brain", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Stop", exact: true }),
  ).toBeVisible();
  await expect.poll(() => proof.requests.length).toBe(1);
  await page
    .getByLabel("Switch Brain", { exact: true })
    .selectOption(second.id);
  await expect(page).toHaveURL(new RegExp(`/brains/${second.id}/dashboard$`));
  release();
  await page.goto(`/brains/${second.id}/ask`);
  await expect(
    page.getByRole("region", { name: "Autonomous memory setup" }),
  ).toBeVisible();
  await expect(
    page.getByText("BROWSER_TEMPORARY_ANSWER", { exact: false }),
  ).toHaveCount(0);
  await expect(
    page.getByText("What is AmberDesktop port?", { exact: true }),
  ).toHaveCount(0);
  await expect.poll(() => cancellations).toBe(1);
  expect((await command(page, `${f.base}/models/usage`)).total).toBe(0);
});
