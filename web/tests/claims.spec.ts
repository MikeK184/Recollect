import { openDetails } from "./desktop-helpers";
import { test, expect } from "@playwright/test";

test("author claims, inspect exact evidence and preserve fact and knowledge history", async ({
  page,
  context,
}) => {
  test.setTimeout(90_000);
  page.setDefaultTimeout(10_000);
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
        throw new Error(`Fixture request failed (${response.status})`);
      return response.json();
    };
    const brain = await send("/api/brains", { name: "Claims history" });
    const source = await send(`/api/brains/${brain.id}/sources`, {
      title: "Authentication inspection",
      media_type: "text/plain",
      content: "January declares JWT.\nFebruary changes to OIDC.\n",
      retain_content: true,
      source_uri: "https://example.test/auth-evidence",
      group_ids: [],
    });
    return { brain: brain.id, source: source.id, version: source.version.id };
  });
  await page.goto(`/brains/${fixture.brain}/memory`);
  await expect(
    page.getByText("Your knowledge will gather here", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Add memory", exact: true }).click();
  await page
    .getByRole("button", { name: "Structured entry", exact: true })
    .click();
  let dialog = page.getByRole("dialog");
  await dialog
    .getByRole("textbox", { name: "Subject", exact: true })
    .fill("Vault production");
  await dialog
    .getByRole("textbox", { name: "Property or relationship", exact: true })
    .fill("authentication method");
  await dialog
    .getByRole("textbox", { name: "Claim value", exact: true })
    .fill("JWT");
  await dialog
    .getByRole("textbox", { name: "Validity kind", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Interval [from, to)", exact: true })
    .click();
  await dialog
    .getByLabel("Valid from (UTC; blank means unknown)", { exact: true })
    .fill("2026-01-01T00:00");
  await dialog
    .getByLabel("Valid until (UTC; exclusive, blank means unknown)", {
      exact: true,
    })
    .fill("2026-02-01T00:00");
  await dialog
    .getByRole("button", { name: "Use evidence", exact: true })
    .click();
  await dialog
    .getByLabel("First supporting line (optional)", { exact: true })
    .fill("1");
  await dialog
    .getByLabel("Last supporting line (inclusive)", { exact: true })
    .fill("1");
  await dialog
    .getByRole("button", { name: "Save proposal", exact: true })
    .click();
  await expect(dialog.getByTestId("claim-value")).toHaveText("JWT");
  await expect(
    dialog.getByText("Review: proposed", { exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText("Does not qualify for strict accepted context", {
      exact: true,
    }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: /Authentication inspection · source version/ })
    .click();
  await expect(dialog.getByTestId("claim-evidence-text")).toHaveText(
    "January declares JWT.\nFebruary changes to OIDC.\n",
  );
  await dialog
    .getByRole("button", { name: "Back to claim", exact: true })
    .click();
  // A second editor must not overwrite the first editor's later knowledge revision.
  const other = await context.newPage();
  await other.goto(`/brains/${fixture.brain}/memory`);
  await other
    .getByRole("button", {
      name: "Vault production · authentication method",
      exact: true,
    })
    .click();
  await other
    .getByRole("button", { name: "Memory history & actions", exact: true })
    .click();
  await other
    .getByRole("dialog")
    .getByRole("button", { name: "Revise proposal", exact: true })
    .click();
  await other
    .getByRole("dialog")
    .getByRole("textbox", { name: "Claim value", exact: true })
    .fill("Stale competing value");
  await dialog
    .getByRole("button", { name: "Revise proposal", exact: true })
    .click();
  await dialog
    .getByRole("textbox", { name: "Claim value", exact: true })
    .fill("OIDC");
  await dialog
    .getByLabel("Valid from (UTC; blank means unknown)", { exact: true })
    .fill("2026-02-01T00:00");
  await dialog
    .getByLabel("Valid until (UTC; exclusive, blank means unknown)", {
      exact: true,
    })
    .fill("2026-03-01T00:00");
  await page.route(
    "**/api/brains/*/claims/*",
    (route) =>
      route.request().method() === "PUT" ? route.abort() : route.continue(),
    { times: 1 },
  );
  await dialog
    .getByRole("button", { name: "Save new proposal", exact: true })
    .click();
  await expect(
    dialog.getByText("Request failed", { exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Save new proposal", exact: true })
    .click();
  await expect(dialog.getByTestId("claim-value")).toHaveText("OIDC");
  await other
    .getByRole("dialog")
    .getByRole("button", { name: "Save new proposal", exact: true })
    .click();
  await expect(
    other.getByText(
      "This claim changed. Reload its latest revision before saving.",
      { exact: true },
    ),
  ).toBeVisible();
  await other.close();
  await openDetails(dialog, "Version history");
  await dialog
    .getByRole("textbox", { name: "Knowledge revision", exact: true })
    .click();
  await page.getByRole("option", { name: /· JWT$/ }).click();
  await expect(dialog.getByTestId("claim-value")).toHaveText("JWT");
  await expect(
    dialog.getByText(
      "Historical knowledge revision. Later knowledge may differ.",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "Revise proposal", exact: true }),
  ).toHaveCount(0);
  await dialog
    .getByRole("heading", { name: "Vault production", exact: true })
    .scrollIntoViewIfNeeded();
  await page.screenshot({
    path: "../.cache/ui/claim-history.png",
    animations: "disabled",
  });
  await openDetails(dialog, "Version history");
  await dialog
    .getByRole("textbox", { name: "Knowledge revision", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Latest knowledge", exact: true })
    .click();
  await page.evaluate(async (fixture) => {
    const me = await (await fetch("/api/auth/me")).json();
    const response = await fetch(
      `/api/brains/${fixture.brain}/sources/${fixture.source}/versions`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify({
          title: "Authentication inspection",
          media_type: "text/plain",
          content: "The source was updated and claims need reassessment.\n",
          retain_content: true,
          source_uri: "https://example.test/auth-evidence",
          base_version: fixture.version,
          group_ids: [],
        }),
      },
    );
    if (!response.ok)
      throw new Error(`Source update failed (${response.status})`);
  }, fixture);
  await expect(
    dialog.getByText("Freshness: needs verification", { exact: true }),
  ).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 800 });
  await dialog
    .getByRole("heading", { name: "Vault production", exact: true })
    .scrollIntoViewIfNeeded();
  await page.screenshot({
    path: "../.cache/ui/claim-1280.png",
    animations: "disabled",
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.getByRole("button", { name: /^Filters/ }).click();
  await page
    .getByLabel("Fact time (UTC)", { exact: true })
    .fill("2026-01-15T00:00");
  await page
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  await expect(
    page.getByText("No matching memory", {
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: /^Filters/ }).click();
  await page
    .getByLabel("Fact time (UTC)", { exact: true })
    .fill("2026-02-15T00:00");
  await page
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Vault production · authentication method",
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: /^Filters/ }).click();
  await page.getByRole("textbox", { name: "Claim view", exact: true }).click();
  await page
    .getByRole("option", { name: "Strict accepted", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  await expect(
    page.getByText("No matching memory", {
      exact: true,
    }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});
