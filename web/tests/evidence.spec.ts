import { openDetails } from "./desktop-helpers";
import { test, expect } from "@playwright/test";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const original =
  "# Vault production evidence\n\nThe repository declares a dedicated production role.\nObserved deployment still needs verification.\n\n<script>window.evidenceExecuted=true</script>\n";
const replacement =
  "# Vault production evidence\n\nThe latest inspection confirms the production role exists.\nIts permissions require a separate review.\n";

test("import evidence, preserve history and share one source across collection views", async ({
  page,
}) => {
  test.setTimeout(75_000);
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
  await page.getByRole("button", { name: "Create Brain", exact: true }).click();
  let dialog = page.getByRole("dialog");
  await dialog.getByLabel(/^Name/).fill("Operational evidence");
  await dialog
    .getByRole("button", { name: "Create Brain", exact: true })
    .click();
  await expect(page.getByLabel("Switch Brain", { exact: true })).toBeVisible();
  await page
    .getByRole("navigation", { name: "Brain navigation" })
    .getByRole("link", { name: "Sources", exact: true })
    .click();
  const brain = new URL(page.url()).pathname.split("/")[2];
  await expect(
    page.getByText("Start with your evidence", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Manage views", exact: true }).click();
  for (const [kind, name] of [
    ["Collection", "Runbooks"],
    ["Collection", "Shared notes"],
    ["Area", "Vault"],
    ["Environment", "Production"],
  ]) {
    await dialog
      .getByRole("textbox", { name: "View kind", exact: true })
      .click();
    await page.getByRole("option", { name: kind, exact: true }).click();
    await dialog.getByLabel(/^View name/).fill(name);
    await dialog
      .getByRole("button", { name: "Create view", exact: true })
      .click();
    await expect(dialog.getByText(name, { exact: true })).toBeVisible();
  }
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Filters", exact: true }).click();
  for (const [kind, name] of [
    ["Collection", "Runbooks"],
    ["Area", "Vault"],
    ["Environment", "Production"],
  ]) {
    await page.getByRole("textbox", { name: kind, exact: true }).click();
    await page.getByRole("option", { name, exact: true }).click();
  }
  await page.getByRole("button", { name: "Show sources", exact: true }).click();
  await page.getByRole("button", { name: "Add source", exact: true }).click();
  await dialog.getByLabel(/^Source title/).fill("Production Vault");
  await dialog.locator('input[type="file"]').setInputFiles({
    name: "vault.md",
    mimeType: "text/markdown",
    buffer: Buffer.from(original),
  });
  await expect(dialog.getByLabel(/^Source text/)).toHaveValue(original);
  await dialog
    .getByLabel(/^Source reference/)
    .fill("https://docs.example.test/vault");
  await dialog
    .getByRole("checkbox", {
      name: "Retain this document’s text in this Brain",
      exact: true,
    })
    .check();
  await page.route("**/api/brains/*/sources", (route) => route.abort(), {
    times: 1,
  });
  await dialog
    .getByRole("button", { name: "Import source", exact: true })
    .click();
  await expect(
    dialog.getByText("Source could not be saved", { exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Import source", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  await page.getByRole("button", { name: /Production Vault/ }).click();
  await page
    .getByRole("button", { name: "Source history & actions", exact: true })
    .click();
  await expect(dialog.getByTestId("source-content")).toHaveText(original);
  expect(await page.evaluate(() => "evidenceExecuted" in window)).toBe(false);
  await promisify(execFile)("../target/debug/recollect-server", [
    "worker-once",
    "capture",
  ]);
  await expect(dialog.getByText("Processed", { exact: true })).toBeVisible();
  await page.screenshot({
    path: "../.cache/ui/source-evidence.png",
    fullPage: false,
    animations: "disabled",
  });
  await dialog
    .getByRole("button", { name: "Edit source", exact: true })
    .click();
  await dialog.getByLabel(/^Source text/).fill(replacement);
  await dialog
    .getByRole("checkbox", {
      name: "Retain this document’s text in this Brain",
      exact: true,
    })
    .check();
  await dialog
    .getByRole("button", { name: "Save new version", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  await page.getByRole("button", { name: /Production Vault/ }).click();
  await page
    .getByRole("button", { name: "Source history & actions", exact: true })
    .click();
  await expect(dialog.getByTestId("source-content")).toHaveText(replacement);
  await openDetails(dialog, "Version history");
  await dialog
    .getByRole("textbox", { name: "Evidence version", exact: true })
    .click();
  await page.getByRole("option", { name: /Earlier/ }).click();
  await expect(dialog.getByTestId("source-content")).toHaveText(original);
  await expect(
    dialog.getByText(
      "You are viewing an earlier source version. Current knowledge may refer to newer evidence.",
    ),
  ).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "Edit source", exact: true }),
  ).toBeDisabled();
  await openDetails(dialog, "Version history");
  await dialog
    .getByRole("textbox", { name: "Evidence version", exact: true })
    .click();
  await page.getByRole("option", { name: /Current/ }).click();
  await dialog
    .getByRole("button", { name: "Organize source", exact: true })
    .click();
  await dialog
    .getByRole("textbox", { name: "Source views", exact: true })
    .click();
  await page.getByRole("option", { name: "Shared notes", exact: true }).click();
  await page.keyboard.press("Escape");
  await dialog
    .getByRole("button", { name: "Save associations", exact: true })
    .click();
  await expect(
    dialog.getByRole("button", { name: "Save associations", exact: true }),
  ).toHaveCount(0);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Manage views", exact: true }).click();
  await dialog.getByRole("textbox", { name: "View kind", exact: true }).click();
  await page.getByRole("option", { name: "Collection", exact: true }).click();
  const runbooks = dialog
    .locator(".mantine-Card-root")
    .filter({ has: page.getByText("Runbooks", { exact: true }) });
  await runbooks
    .getByRole("button", { name: "Remove view", exact: true })
    .click();
  await runbooks
    .getByRole("button", { name: "Remove this view", exact: true })
    .click();
  await expect(dialog.getByText("Runbooks", { exact: true })).toHaveCount(0);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: /^Filters/ }).click();
  await page.getByRole("textbox", { name: "Collection", exact: true }).click();
  await page.getByRole("option", { name: "Shared notes", exact: true }).click();
  await page.getByRole("button", { name: "Show sources", exact: true }).click();
  await expect(
    page.getByRole("button", { name: /Production Vault/ }),
  ).toBeVisible();
  await promisify(execFile)("../target/debug/recollect-server", [
    "worker-once",
    "capture",
  ]);
  await expect(
    page.locator(".source-row").getByText("Processed", { exact: true }),
  ).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({
    path: "../.cache/ui/collections.png",
    fullPage: false,
    animations: "disabled",
  });
  await page.goto(`/brains/${brain}/settings?tab=privacy`);
  const captureSwitch = page.getByRole("switch", {
    name: /^Allow document content retention/,
  });
  await captureSwitch.click();
  await expect(captureSwitch).not.toBeChecked();
  await page.goto(`/brains/${brain}/sources`);
  await page.getByRole("button", { name: "Add source", exact: true }).click();
  await expect(
    dialog.getByText("This Brain currently allows reference-only imports."),
  ).toBeVisible();
  await dialog
    .getByLabel(/^Source title/)
    .fill("Controlled external reference");
  await dialog
    .getByLabel(/^Source reference/)
    .fill("https://docs.example.test/controlled");
  await dialog
    .getByRole("button", { name: "Import source", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  await page
    .getByRole("button", { name: /Controlled external reference/ })
    .click();
  await page
    .getByRole("button", { name: "Source history & actions", exact: true })
    .click();
  await expect(
    dialog.getByText("Reference without retained text", { exact: true }),
  ).toBeVisible();
  await expect(dialog.getByTestId("source-content")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 1280, height: 800 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
  ).toBe(false);
  await page.screenshot({
    path: "../.cache/ui/collections-1280.png",
    fullPage: true,
    animations: "disabled",
  });
  expect(errors).toEqual([]);
});
