import { expect, test } from "@playwright/test";

test("AI cards edit inline, preserve Cancel and require explicit embedding rebuild", async ({
  page,
}) => {
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
  const session = await (await page.request.get("/api/auth/me")).json();
  const created = await page.request.post("/api/brains", {
    data: { name: "Inline AI settings fixture", managed_memory: false },
    headers: { "x-csrf-token": session.csrf_token },
  });
  expect(created.ok()).toBeTruthy();
  const brain = await created.json();
  const base = `/api/brains/${brain.id}`;
  const initial = await (
    await page.request.get(`${base}/models/policy`)
  ).json();
  let saved = structuredClone(initial);
  let revision = 0;
  let failNext = false;
  let refreshes = 0;
  const writes: Record<string, unknown>[] = [];
  const models = [
    {
      id: "gpt-5.6-luna",
      kind: "text",
      max_dimensions: null,
      input_usd_per_million: 0.2,
      cached_input_usd_per_million: 0.02,
      output_usd_per_million: 1.2,
    },
    {
      id: "gpt-4.1-mini",
      kind: "text",
      max_dimensions: null,
      input_usd_per_million: 0.4,
      cached_input_usd_per_million: 0.1,
      output_usd_per_million: 1.6,
    },
    {
      id: "text-embedding-3-large",
      kind: "embedding",
      max_dimensions: 3072,
      input_usd_per_million: 0.13,
      cached_input_usd_per_million: null,
      output_usd_per_million: null,
    },
    {
      id: "text-embedding-3-small",
      kind: "embedding",
      max_dimensions: 1536,
      input_usd_per_million: 0.02,
      cached_input_usd_per_million: null,
      output_usd_per_million: null,
    },
  ].map((model) => ({
    ...model,
    available: true,
    selectable: true,
    checked_on: "2026-10-05",
    source_url: `https://developers.openai.com/api/docs/models/${model.id}`,
    pricing_stale: false,
    pricing_tier: "standard",
  }));
  await page.route(`**${base}/models/catalogue`, async (route) => {
    if (route.request().method() === "POST") refreshes++;
    await route.fulfill({
      json: {
        models,
        observed_at: refreshes ? "2026-10-05T12:00:00Z" : null,
        stale: !refreshes,
        error_code: null,
      },
    });
  });
  await page.route(`**${base}/models/policy`, async (route) => {
    if (route.request().method() === "PUT") {
      const body = route.request().postDataJSON();
      writes.push(body);
      if (failNext) {
        failNext = false;
        await route.fulfill({
          status: 409,
          json: {
            code: "model_policy_changed",
            message: "This model policy changed. Reload it before saving.",
          },
        });
        return;
      }
      saved.current = {
        ...saved.current,
        policy: body.policy,
        change_id: `fixture-revision-${++revision}`,
      };
      await route.fulfill({ json: saved.current });
      return;
    }
    await route.fulfill({ json: saved });
  });
  await page.goto(`/brains/${brain.id}/settings?tab=ai`);
  await expect(
    page.getByRole("heading", { name: "Selected models", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Optional recommended defaults", { exact: true }),
  ).toHaveCount(0);
  expect(refreshes).toBe(0);
  await page
    .getByRole("button", { name: "Edit AI permissions", exact: true })
    .click();
  await expect.poll(() => refreshes).toBe(1);
  await expect(
    page.getByRole("dialog", { name: "AI permissions", exact: true }),
  ).toHaveCount(0);
  await page.getByLabel("Learn from sources", { exact: true }).uncheck();
  await expect(
    page.getByLabel("Automatic memory", { exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(writes).toHaveLength(0);
  await expect(
    page.getByLabel("Learn from sources", { exact: true }),
  ).toBeChecked();
  await page
    .getByRole("button", { name: "Edit AI permissions", exact: true })
    .click();
  await page.getByLabel("Text model", { exact: true }).click();
  await page.getByRole("option", { name: "gpt-4.1-mini", exact: true }).click();
  await expect(
    page.getByText(/\$0.40 input · \$0.10 cached · \$1.60 output/),
  ).toBeVisible();
  await page.getByLabel("Embedding model", { exact: true }).click();
  await page
    .getByRole("option", { name: "text-embedding-3-small", exact: true })
    .click();
  await expect(
    page.getByLabel("Embedding dimensions", { exact: true }),
  ).toHaveValue("1,536");
  await expect(
    page.getByText(/Saving rebuilds search embeddings/),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Save and rebuild", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Edit AI permissions", exact: true }),
  ).toBeVisible();
  expect(writes[0]).toMatchObject({
    rebuild_embeddings: true,
    base_change: initial.current.change_id,
  });
  expect(saved.current.policy.embedding_dimensions).toBe(1536);
  await page
    .getByRole("button", { name: "Edit AI permissions", exact: true })
    .click();
  await page.getByLabel("Answer questions", { exact: true }).check();
  failNext = true;
  await page
    .getByRole("button", { name: "Save AI permissions", exact: true })
    .click();
  await expect(page.getByText(/This model policy changed/)).toBeVisible();
  await expect(
    page.getByLabel("Answer questions", { exact: true }),
  ).toBeChecked();
  await expect(
    page.getByRole("button", { name: "Cancel", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "../.cache/ui/inline-ai-settings.png",
    fullPage: true,
  });
});
