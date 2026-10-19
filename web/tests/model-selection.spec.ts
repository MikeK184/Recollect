import { expect, test, type Page } from "@playwright/test";

// scripts/test-ui.sh runs this file against an installation that names no text
// model (no RECOLLECT_TEXT_MODEL) and no provider credential.
async function signIn(page: Page) {
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
}

test("an installation without a text model captures but asks the admin to choose one", async ({
  page,
}) => {
  await signIn(page);
  await page.getByRole("button", { name: "Create Brain", exact: true }).click();
  const create = page.getByRole("dialog");
  await create.getByLabel(/^Name/).fill("No installation model fixture");
  await expect(
    create.getByText(/learning, semantic search\s+and Ask run after a model/),
  ).toBeVisible();
  await create
    .getByRole("button", { name: "Create Brain", exact: true })
    .click();
  await expect(page).toHaveURL(/\/brains\/[^/]+\/dashboard/);
  const brain = new URL(page.url()).pathname.split("/")[2];
  // Capture is on and the preset's limits are saved, but nothing may be sent.
  const settings = await (
    await page.request.get(`/api/brains/${brain}/automation`)
  ).json();
  expect(settings.models.installed.text_model ?? null).toBeNull();
  expect(settings.models.current.policy).toMatchObject({
    text_model: "",
    enabled: false,
    automatic_learning: false,
    automatic_embedding: false,
    daily_token_limit: 1_000_000,
  });
  expect(settings.capture.policy).toMatchObject({
    enabled: true,
    managed_tools: true,
  });

  // Ask names the missing choice instead of a silent "Off".
  await page.goto(`/brains/${brain}/ask`);
  await expect(
    page.getByRole("heading", { name: "AI permissions", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText(/No text model is selected for this Brain/),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Choose a model", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Ask Brain" })).toBeDisabled();
  await page.screenshot({
    path: "../.cache/ui/choose-model-ask.png",
    fullPage: true,
  });

  // The AI card shows the same state with the action to pick a model.
  await page.goto(`/brains/${brain}/settings?tab=ai`);
  await expect(
    page.getByRole("heading", { name: "Selected models", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("textbox", { name: "Text model", exact: true }),
  ).toHaveValue("");
  await expect(
    page.getByRole("textbox", { name: "Text model", exact: true }),
  ).toHaveAttribute("placeholder", "Choose a model");
  await expect(page.getByText("Choose a text model first.")).toBeVisible();
  await page.screenshot({
    path: "../.cache/ui/choose-model-settings.png",
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "Choose a model", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Choose a model",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  // No provider credential here, so the model list cannot load and nothing
  // can be chosen: the dialog says so and keeps the action disabled.
  await expect(
    dialog.getByText(/The model list could not be loaded/),
  ).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "Use this model", exact: true }),
  ).toBeDisabled();
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(dialog).toHaveCount(0);

  // The gateway refuses before any credential or provider is considered.
  const session = await (await page.request.get("/api/auth/me")).json();
  const refused = await page.request.post(`/api/brains/${brain}/models/check`, {
    data: { operation_id: crypto.randomUUID() },
    headers: { "x-csrf-token": session.csrf_token },
  });
  expect(refused.status()).toBe(409);
  expect(await refused.json()).toMatchObject({
    code: "model_not_selected",
    message: "Choose a text model for this Brain",
  });
  const after = await (
    await page.request.get(`/api/brains/${brain}/automation`)
  ).json();
  expect(after.models.current.change_id).toBe(
    settings.models.current.change_id,
  );
});
