import { openGraphDrawer, loadGraph } from "./desktop-helpers";
import { test, expect, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

const input = process.env.RECOLLECT_TEST_RECOVERY_CONFIG;
const saved = input ? JSON.parse(readFileSync(input, "utf8")) : undefined;
test.skip(
  !saved,
  "Select the private credentials from the owned recovery drill",
);

test("restored desktop preserves scoped evidence, graph and uncertain effect receipt", async ({
  page,
  browser,
}) => {
  test.setTimeout(90_000);
  expect(saved.instance).toMatch(/^proof-recovery-/);
  const errors: string[] = [];
  const screenshots = dirname(input!);
  const login = async (
    view: Page,
    account: { username: string; password: string },
  ) => {
    view.on("pageerror", (error) => errors.push(error.message));
    await view.goto(saved.origin);
    await view.getByLabel(/^Username/).fill(account.username);
    await view.getByLabel(/^Password/).fill(account.password);
    await view.getByRole("button", { name: "Sign in", exact: true }).click();
    await expect(
      view.getByRole("heading", { name: "Your Brains", exact: true }),
    ).toBeVisible();
  };
  await login(page, saved.owner);
  await expect(
    page.getByText(saved.member_brain.name, { exact: true }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "Runtime diagnostics", exact: true })
    .click();
  await expect(
    page.getByRole("dialog").getByText("Requests started", { exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: resolve(screenshots, "restored-diagnostics.png"),
    animations: "disabled",
  });
  await page.keyboard.press("Escape");
  await page.goto(`${saved.origin}/brains/${saved.brain.id}/ask?tab=search`);
  await expect(
    page.getByRole("heading", {
      name: "Ask your Brain",
      level: 1,
      exact: true,
    }),
  ).toBeVisible();
  const recall = page.getByRole("region", {
    name: "Recall memory",
    exact: true,
  });
  await recall
    .getByLabel("Search memory", { exact: true })
    .fill("RetainedCapturedAmber");
  await recall.getByRole("button", { name: "Recall", exact: true }).click();
  await expect(recall.getByTestId("recall-result")).toHaveCount(1);
  await expect(recall.getByTestId("recall-result")).toContainText(
    "RetainedCapturedAmber",
  );
  await recall.screenshot({
    path: resolve(screenshots, "restored-recall.png"),
    animations: "disabled",
  });
  await recall
    .getByLabel("Search memory", { exact: true })
    .fill("EraseCapturedZircon");
  const erased = page.waitForResponse(
    (response) =>
      response.url().endsWith("/recall") &&
      response.request().method() === "POST",
  );
  await recall.getByRole("button", { name: "Recall", exact: true }).click();
  expect((await (await erased).json()).context.items).toEqual([]);
  await expect(recall.getByTestId("recall-result")).toHaveCount(0);

  await page.goto(`${saved.origin}/brains/${saved.brain.id}/graph`);
  const graph = page.locator("body");
  await page.getByText("Repository", { exact: true }).click();
  await graph
    .getByRole("textbox", { name: "Graph repositories", exact: true })
    .click();
  await page
    .getByRole("option", { name: "example.test/team/recovery", exact: true })
    .click();
  await page.keyboard.press("Escape");
  await loadGraph(page);
  await openGraphDrawer(
    page,
    "Eligible entities",
    "Browse eligible entity pages",
  );
  await expect(graph.getByTestId("graph-entity")).toHaveCount(2);
  await page.keyboard.press("Escape");
  await openGraphDrawer(page, "Graph status and maintenance", "Graph status");
  await expect(
    graph.getByText("2 eligible entities · 1 eligible relationships", {
      exact: true,
    }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(graph.getByTestId("graph-canvas")).toHaveAttribute(
    "data-ready",
    "true",
  );
  await graph.screenshot({
    path: resolve(screenshots, "restored-graph.png"),
    animations: "disabled",
  });

  await page.goto(
    `${saved.origin}/brains/${saved.brain.id}/activity?tab=tools`,
  );
  const activity = page.getByRole("region", {
    name: "MCP activity",
    exact: true,
  });
  await activity
    .getByRole("row")
    .filter({ has: page.getByRole("cell", { name: "effect", exact: true }) })
    .getByRole("button", { name: "Inspect call", exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: "Tool call", exact: true });
  await expect(dialog).toContainText("Completion unknown");
  await expect(dialog).toContainText("connector receipt · succeeded");
  await dialog.screenshot({
    path: resolve(screenshots, "restored-receipt.png"),
    animations: "disabled",
  });
  await page.keyboard.press("Escape");

  const memberContext = await browser.newContext({
    viewport: { width: 1440, height: 960 },
  });
  try {
    const member = await memberContext.newPage();
    await login(member, saved.member);
    await expect(
      member.getByText(saved.member_brain.name, { exact: true }),
    ).toBeVisible();
    await expect(
      member.getByRole("button", { name: "Runtime diagnostics", exact: true }),
    ).toHaveCount(0);
    await member.goto(`${saved.origin}/brains/${saved.brain.id}/sources`);
    await expect(
      member.getByRole("button", { name: "Add source", exact: true }),
    ).toHaveCount(0);
    await member
      .getByRole("button", { name: /RetainedIndependentCobalt/ })
      .first()
      .click();
    await expect(member.getByTestId("source-content")).toContainText(
      "RetainedIndependentCobalt",
    );
    await member.screenshot({
      path: resolve(screenshots, "restored-member.png"),
      animations: "disabled",
    });
  } finally {
    await memberContext.close();
  }
  expect(errors).toEqual([]);
});
