import { expect, type Locator, type Page } from "@playwright/test";

/** Open a named progressive-disclosure group without toggling an open group shut. */
export async function openDetails(scope: Page | Locator, label: string) {
  const summary = scope
    .locator("summary")
    .filter({ hasText: new RegExp(`^${label}$`) });
  await summary.first().waitFor({ state: "visible" });
  if (await summary.count()) {
    const details = summary.first().locator("..");
    if ((await details.getAttribute("open")) === null)
      await summary.first().click();
  }
}

/** Move between the graph's secondary tools as a keyboard user would. */
export async function openGraphDrawer(
  page: Page,
  title: string,
  button: string,
) {
  const drawer = page.getByRole("dialog", { name: title, exact: true });
  // A drawer that is closing is still "visible" during its exit transition.
  // Let the animation settle so we never interact with an unmounting dialog
  // or skip the open click for a drawer that is already gone.
  if (await drawer.isVisible()) {
    await page.waitForTimeout(300);
    if (await drawer.isVisible()) return drawer;
  }
  for (let i = 0; i < 4 && (await page.getByRole("dialog").count()); i++) {
    await page.keyboard.press("Escape");
    await page.waitForTimeout(180);
  }
  await page.getByRole("button", { name: button, exact: true }).click();
  await expect(drawer).toBeVisible();
  return drawer;
}

export async function loadGraph(page: Page) {
  const filters = await openGraphDrawer(page, "Graph filters", "Filters");
  await filters
    .getByRole("button", { name: "Apply graph filters", exact: true })
    .click();
}
