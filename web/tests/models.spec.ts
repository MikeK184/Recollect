import { test, expect } from "@playwright/test";

test("OpenAI policy, selected models and governed source learning", async ({
  page,
}) => {
  test.skip(
    process.env.RECOLLECT_TEST_OPENAI !== "1",
    "Opt in to fixed-synthetic actual OpenAI calls with RECOLLECT_TEST_OPENAI=1.",
  );
  test.setTimeout(180_000);
  page.setDefaultTimeout(15_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await page
    .getByLabel(/^Username/)
    .fill(process.env.RECOLLECT_OWNER_USERNAME!);
  await page
    .getByLabel(/^Password/)
    .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Create Brain", exact: true }),
  ).toBeVisible();
  const fixture = await page.evaluate(async () => {
    const me = await (await fetch("/api/auth/me")).json();
    const send = async (url: string, body: unknown) => {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!response.ok)
        throw new Error("Synthetic model fixture failed: " + response.status);
      return response.json();
    };
    const brain = await send("/api/brains", {
      name: "OpenAI browser learning proof",
    });
    const source = await send("/api/brains/" + brain.id + "/sources", {
      title: "Synthetic literal configuration",
      media_type: "text/plain",
      retain_content: true,
      content: "Amber.port = 8080\n",
    });
    return { brain: brain.id, source: source.id };
  });
  await page.goto("/brains/" + fixture.brain);
  await expect(
    page.getByText("Transmission disabled", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Edit model policy", exact: true })
    .click();
  let dialog = page.getByRole("dialog", { name: "Model policy", exact: true });
  await expect(dialog.getByLabel("Text model", { exact: true })).toHaveValue(
    "gpt-5.6-luna",
  );
  await expect(
    dialog.getByLabel("Embedding model", { exact: true }),
  ).toHaveValue("text-embedding-3-large");
  await dialog.getByLabel("Allow model transmission", { exact: true }).check();
  await dialog.getByLabel("Maintain memory autonomously", { exact: true }).uncheck();
  await dialog
    .getByRole("textbox", { name: "Allowed content classes", exact: true })
    .click();
  await page.keyboard.press("Escape");
  await dialog
    .getByLabel("Accept permitted literal configuration facts", { exact: true })
    .check();
  await dialog
    .getByLabel("Acceptance rule name", { exact: true })
    .fill("literal-ports");
  await dialog
    .getByLabel("Allowed literal properties", { exact: true })
    .fill("port");
  await dialog
    .getByRole("button", { name: "Save model policy", exact: true })
    .click();
  await expect(
    page.getByText("Transmission enabled", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Check selected models", exact: true })
    .click();
  await expect(
    page.getByText(/Both selected models responded successfully/),
  ).toBeVisible({ timeout: 100_000 });
  await expect(
    page.getByText(/Embeddings have 3,072 dimensions/),
  ).toBeVisible();
  await page
    .getByRole("button", { name: /Synthetic literal configuration/ })
    .click();
  await page
    .getByRole("button", { name: "Learn from this source", exact: true })
    .click();
  dialog = page.getByRole("dialog", { name: "Learn from source", exact: true });
  await dialog
    .getByRole("button", { name: "Queue learning", exact: true })
    .click();
  await expect(dialog.getByText(/Learning queued/)).toBeVisible();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await expect(page.getByText(/1 accepted by policy · 0 proposed/)).toBeVisible(
    { timeout: 70_000 },
  );
  await page
    .getByRole("button", { name: "Inspect learned claim 1", exact: true })
    .click();
  dialog = page.getByRole("dialog", {
    name: "Claim and knowledge history",
    exact: true,
  });
  await expect(
    dialog.getByText(/Model derivation: gpt-5.6-luna/),
  ).toBeVisible();
  await expect(
    dialog.getByText("Review: accepted", { exact: true }),
  ).toBeVisible();
  const proof = await page.evaluate(async (brain) => {
    const runs = await (
      await fetch("/api/brains/" + brain + "/learning")
    ).json();
    const detail = await (
      await fetch(
        "/api/brains/" + brain + "/claims/" + runs.items[0].claim_ids[0],
      )
    ).json();
    const usage = await (
      await fetch("/api/brains/" + brain + "/models/usage")
    ).json();
    return {
      revision: detail.selected.revision,
      eligibility: detail.selected.eligibility,
      usage,
    };
  }, fixture.brain);
  expect(proof.revision.reviewer_id).toBeNull();
  expect(proof.revision.acceptance_policy).toMatch(/^literal-ports@/);
  expect(proof.revision.origin).toBe("model_extracted");
  expect(proof.eligibility.strict_accepted).toBe(true);
  expect(proof.eligibility.strict_operational).toBe(false);
  expect(proof.usage.total).toBe(3);
  expect(
    proof.usage.requests.every(
      (r: { state: string }) => r.state === "succeeded",
    ),
  ).toBe(true);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: "../.cache/ui-model-claim-mobile.png" });
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 1440, height: 960 });
  await page
    .getByRole("heading", { name: "Model learning", exact: true })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: "../.cache/ui-model-learning-desktop.png" });
  await page
    .getByRole("button", { name: "Edit model policy", exact: true })
    .click();
  dialog = page.getByRole("dialog", { name: "Model policy", exact: true });
  await dialog
    .getByLabel("Daily token allowance", { exact: true })
    .fill("1000");
  await dialog
    .getByRole("button", { name: "Save model policy", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Check selected models", exact: true })
    .click();
  await expect(
    page
      .getByRole("alert")
      .filter({ hasText: "remaining daily token allowance" }),
  ).toBeVisible();
  const calls = await page.evaluate(
    async (brain) =>
      (await (await fetch("/api/brains/" + brain + "/models/usage")).json())
        .total,
    fixture.brain,
  );
  expect(calls).toBe(3);
  expect(errors).toEqual([]);
});
