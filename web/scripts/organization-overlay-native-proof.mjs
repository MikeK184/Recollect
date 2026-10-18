import { chromium, expect as baseExpect } from "@playwright/test";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

// Read-only proof against the ephemeral server supplied by the native unmatched
// conversion fixture. No route mocking, seeded browser cards, or provider calls.
const expect = baseExpect.configure({ timeout: 15_000 });
const fixture = JSON.parse(
  await readFile(process.env.RECOLLECT_ORGANIZATION_NATIVE_FIXTURE, "utf8"),
);
const origin = new URL(fixture.origin);
if (origin.protocol !== "http:" || origin.hostname !== "127.0.0.1")
  throw Error("Native overlay UI proof requires the owned loopback server");
const output = resolve(dirname(fileURLToPath(import.meta.url)), "../../.cache/ui");
await mkdir(output, { recursive: true });
const report = {
  fixture: "Actual unmatched native publication and unmocked browser/API",
  topic_id: fixture.topic_id,
  expected_sources: fixture.sources.map((source) => source.key).sort(),
  checks: [],
  screenshots: [],
  provider_calls: 0,
};
let browser;
try {
  browser = await chromium.launch({ channel: "chrome", headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 960 },
    reducedMotion: "reduce",
  });
  await context.addCookies([{
    ...fixture.session_cookie, url: origin.origin,
    httpOnly: true, sameSite: "Strict",
  }]);
  const page = await context.newPage();
  const errors = [], commands = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("request", (request) => {
    if (request.method() === "PUT" && request.url().includes("/organization/topics/"))
      commands.push(request.method());
  });
  const endpoint = `/api/brains/${fixture.brain_id}/organization/view`;
  const read = (center) => page.waitForResponse((response) =>
    new URL(response.url()).pathname === endpoint &&
    response.request().method() === "POST" && response.ok() &&
    (center ? response.request().postDataJSON()?.center?.id === fixture.topic_id :
      !response.request().postDataJSON()?.center),
  );
  const overviewRead = read(false);
  await page.goto(`${origin.origin}/brains/${fixture.brain_id}/graph`);
  const overview = await (await overviewRead).json();
  const stored = overview.topics.find((topic) => topic.id === fixture.topic_id);
  expect(stored?.topic_kind).toBe("manual_overlay");
  expect(stored?.label).toBe(fixture.label);
  const card = page.locator(`[data-topic-id="${fixture.topic_id}"]`);
  await expect(card.getByText("Manual group", { exact: true })).toBeVisible();
  await expect(card.getByRole("heading", { name: fixture.label, exact: true })).toBeVisible();
  report.checks.push("Actual overview card exposes the stored manual overlay kind and preserved human name");
  for (const width of [1440, 1120]) {
    await page.setViewportSize({ width, height: 960 });
    await card.scrollIntoViewIfNeeded();
    const path = resolve(output, `organization-overlay-native-overview-${width}.png`);
    await page.screenshot({ path, animations: "disabled" });
    report.screenshots.push(path);
  }
  await page.setViewportSize({ width: 1440, height: 960 });
  const expandedRead = read(true);
  await card.getByRole("button", { name: `Expand topic ${fixture.label}`, exact: true }).click();
  const expanded = await (await expandedRead).json();
  expect(expanded.topics.find((topic) => topic.id === fixture.topic_id)?.topic_kind)
    .toBe("manual_overlay");
  const contributors = expanded.source_memberships
    .filter((member) => member.topic_id === fixture.topic_id)
    .map((member) => member.source_key);
  expect([...new Set(contributors)].sort()).toEqual(report.expected_sources);
  expect(expanded.evidence.map((node) => node.key).sort()).toEqual(report.expected_sources);
  report.checks.push("Expanded unmocked API and UI retain exactly the adopted immutable source identities");
  const canvas = page.getByTestId("graph-canvas");
  await canvas.scrollIntoViewIfNeeded();
  await expect(canvas).toHaveAttribute("data-nodes", String(fixture.sources.length));
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  for (const source of fixture.sources) {
    await page.getByRole("textbox", { name: "Inspect graph entity", exact: true }).fill(source.title);
    await page.locator(`[role="option"][value="${source.key}"]`).click();
    const inspector = page.getByRole("complementary", { name: "Selected organization evidence", exact: true });
    await expect(inspector).toContainText(source.key);
    await expect(inspector).toContainText(`Source member of ${fixture.label}`);
  }
  report.checks.push("The actual source inspectors expose each retained canonical version");
  await page.getByRole("button", { name: `Organize topic ${fixture.label}`, exact: true }).click();
  const editor = page.getByRole("dialog", { name: "Organize manual group", exact: true });
  await expect(editor).toContainText("Changes apply to this manual group.");
  await expect(editor.getByRole("textbox", { name: "Topic name", exact: true })).toHaveValue(fixture.label);
  const titles = fixture.sources.map((source) => source.title);
  const sourceLabels = fixture.sources.map((source) =>
    titles.filter((title) => title === source.title).length > 1
      ? `${source.title} · version ${source.key.replace(/^source_version:/, "").slice(0, 8)}`
      : source.title,
  );
  for (const label of sourceLabels)
    await expect(editor.getByRole("checkbox", { name: label, exact: true })).toBeVisible();
  await editor.getByRole("textbox", { name: "Name supported by", exact: true }).click();
  for (const label of sourceLabels)
    await expect(page.getByRole("option", { name: label, exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  for (const width of [1440, 1120]) {
    await page.setViewportSize({ width, height: 960 });
    const path = resolve(output, `organization-overlay-native-editor-${width}.png`);
    await page.screenshot({ path, animations: "disabled" });
    report.screenshots.push(path);
  }
  expect(commands).toEqual([]);
  expect(errors).toEqual([]);
  report.checks.push("Manual group editor identifies human organization and disambiguates duplicate source titles by immutable version at desktop and laptop widths without issuing another command");
  report.status = "passed";
} catch (error) {
  report.status = "failed";
  report.error = String(error);
  throw error;
} finally {
  await writeFile(resolve(output, "organization-overlay-native-proof.json"), JSON.stringify(report, null, 2));
  await browser?.close();
}
