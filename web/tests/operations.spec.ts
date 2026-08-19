import { randomUUID } from "node:crypto";
import { test, expect } from "@playwright/test";

test("desktop owner diagnostics clear failed observations and deny ordinary members", async ({
  page,
  browser,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await page
    .getByLabel(/^Username/)
    .fill(process.env.RECOLLECT_OWNER_USERNAME!);
  await page
    .getByLabel(/^Password/)
    .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();

  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/operations", async (route) => {
    await pending;
    await route.continue();
  });
  await page
    .getByRole("button", { name: "Runtime diagnostics", exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: "Runtime diagnostics" });
  const response = page.waitForResponse(
    (value) => new URL(value.url()).pathname === "/api/operations",
  );
  try {
    await expect(
      dialog.getByRole("status", { name: "Loading runtime diagnostics" }),
    ).toBeVisible();
  } finally {
    release();
  }
  expect((await response).status()).toBe(200);
  await expect(
    dialog.getByText("Requests started", { exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText("Time until response headers", { exact: true }),
  ).toBeVisible();
  await expect(dialog.getByText("Any duration", { exact: true })).toBeVisible();
  await expect(
    dialog.getByText(/Job counts cover only Brains you can access/),
  ).toBeVisible();
  await page.unroute("**/api/operations");
  await page.route("**/api/operations", (route) =>
    route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({
        message: "The diagnostic dependency is unavailable.",
      }),
    }),
  );
  // An actual subsequent poll must remove the previously displayed observation.
  await expect(
    dialog.getByText("Diagnostics unavailable", { exact: true }),
  ).toBeVisible({
    timeout: 20_000,
  });
  await expect(
    dialog.getByText("Requests started", { exact: true }),
  ).toHaveCount(0);
  await expect(dialog.getByText("Queued jobs", { exact: true })).toHaveCount(0);
  await page.unroute("**/api/operations");
  await dialog.getByRole("button", { name: "Retry", exact: true }).click();
  await expect(
    dialog.getByText("Requests started", { exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText("Diagnostics unavailable", { exact: true }),
  ).toHaveCount(0);
  await page.screenshot({
    path: "../.cache/installation-desktop.png",
    fullPage: false,
  });
  await page.keyboard.press("Escape");

  // Invite through the delivered product path; never mock account authority.
  await page.getByRole("link", { name: "Team", exact: true }).click();
  await page.getByRole("button", { name: "Add teammate", exact: true }).click();
  const invite = page.getByRole("dialog");
  await invite
    .getByLabel(/^Teammate username/)
    .fill(`operations-${randomUUID().slice(0, 8)}`);
  await invite
    .getByRole("button", { name: "Create invitation", exact: true })
    .click();
  const invitation = await invite.getByLabel("Invitation link").inputValue();
  await invite.getByRole("button", { name: "Done", exact: true }).click();
  const context = await browser.newContext();
  try {
    const member = await context.newPage();
    member.on("pageerror", (error) => errors.push(error.message));
    await member.goto(invitation);
    await member.getByLabel(/^Choose a password/).fill(randomUUID());
    await member
      .getByRole("button", { name: "Accept invitation", exact: true })
      .click();
    await expect(
      member.getByRole("heading", { name: "Your Brains", exact: true }),
    ).toBeVisible();
    await expect(
      member.getByRole("button", { name: "Runtime diagnostics", exact: true }),
    ).toHaveCount(0);
    expect(
      (
        await member.request.get(new URL("/api/operations", invitation).href)
      ).status(),
    ).toBe(403);
  } finally {
    await context.close();
  }
  expect(errors).toEqual([]);
});
