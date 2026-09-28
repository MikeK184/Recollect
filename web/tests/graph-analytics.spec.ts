import { openGraphDrawer } from "./desktop-helpers";
import { test, expect as baseExpect } from "@playwright/test";
const expect = baseExpect.configure({ timeout: 20_000 });

test("native analytical reports, evidence, invalidation, errors at 1280 pixels layout", async ({
  page,
}) => {
  test.setTimeout(120_000);
  page.setDefaultTimeout(12_000);
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
    const post = async (path: string, body: unknown) => {
      const r = await fetch(path, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-csrf-token": me.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error(`Analytical fixture ${r.status}`);
      return r.json();
    };
    const brain = await post("/api/brains", {
      name: "Synthetic analytics browser proof",
    });
    const rows = [];
    for (const name of ["Amber", "Cobalt"]) {
      const source = await post(`/api/brains/${brain.id}/sources`, {
        title: `${name} analytical evidence`,
        media_type: "text/plain",
        retain_content: true,
        content: `${name} port 8080.\nANALYTICS_INERT_EVIDENCE <script>window.analyticsInjected=true</script>\n`,
      });
      const claim = await post(`/api/brains/${brain.id}/claims`, {
        content: {
          kind: "claim",
          subject: `${name} analytical service${name === "Cobalt" ? " module.deeply_nested_repository_symbol_without_breaks_for_mobile_layout" : ""}`,
          predicate: "port",
          value: "8080",
          rationale: "Synthetic analytical declaration.",
          selection: { repository_ids: [], area_ids: [], environment_id: null },
          manifest_revision_id: null,
          validity: {
            kind: "unknown",
            from: null,
            to: null,
            precision: "unknown",
          },
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
      rows.push({
        source: source.id,
        version: source.version.id,
        claim: claim.claim_id,
        revision: claim.id,
      });
    }
    return { brain: brain.id, rows, csrf: me.csrf_token };
  });
  for (const row of fixture.rows) {
    await expect
      .poll(
        async () =>
          (
            await (
              await page.request.get(
                `/api/brains/${fixture.brain}/sources/${row.source}/versions/${row.version}`,
              )
            ).json()
          ).version.processing,
      )
      .toBe("ready");
  }
  await page.goto(`/brains/${fixture.brain}/graph`);
  const panel = page.locator("body");
  await openGraphDrawer(page, "Graph status and maintenance", "Graph status");
  await panel
    .getByRole("button", { name: "Rebuild graph", exact: true })
    .click();
  const currentKnowledgeReady = async () => {
    const status = await (
      await page.request.get(`/api/brains/${fixture.brain}/graph`)
    ).json();
    return status.generations.some(
      (g: { kind: string; state: string; input_epoch: number }) =>
        g.kind === "knowledge" &&
        g.state === "ready" &&
        g.input_epoch === status.memory_epoch,
    );
  };
  await expect.poll(currentKnowledgeReady).toBe(true);
  await openGraphDrawer(page, "Graph insights", "Insights");
  const analytics = panel.getByTestId("graph-analytics");
  const queue = analytics.getByRole("button", {
    name: "Queue analysis",
    exact: true,
  });
  await expect(queue).toBeDisabled();
  await openGraphDrawer(page, "Graph filters", "Filters");
  await panel
    .getByRole("textbox", { name: "Graph relations", exact: true })
    .click();
  await page.getByRole("option", { name: "supported by", exact: true }).click();
  await page.keyboard.press("Escape");
  await openGraphDrawer(page, "Graph insights", "Insights");
  await analytics
    .getByRole("textbox", { name: "Analysis", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Connected components", exact: true })
    .click();
  let queues = 0;
  page.on("request", (r) => {
    if (r.method() === "POST" && r.url().endsWith("/graph/analytics")) queues++;
  });
  await queue.click();
  const report = analytics.getByTestId("analytics-report").first();
  await expect(report.getByText("ready", { exact: true })).toBeVisible({
    timeout: 20_000,
  });
  await report.getByRole("button", { name: "Open analytical report" }).click();
  const detail = analytics.getByTestId("analytics-detail");
  await expect(detail.getByTestId("analytics-result")).toHaveCount(4);
  await detail
    .getByRole("button", { name: "Exact analytical inputs and parameters" })
    .click();
  await expect(detail.getByText(/"supported_by"/)).toBeVisible();
  const amber = detail.getByTestId("analytics-result").filter({
    has: page.getByText("Amber analytical evidence", { exact: true }),
  });
  await amber
    .getByRole("button", { name: "Inspect analytical evidence" })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(dialog.getByText(/ANALYTICS_INERT_EVIDENCE/)).toBeVisible();
  expect(await page.evaluate(() => "analyticsInjected" in window)).toBe(false);
  await dialog
    .getByRole("button", { name: "Back to graph", exact: true })
    .click();
  await detail.getByRole("button", { name: "Refresh report" }).click();
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await page.waitForTimeout(3400);
  expect(queues).toBe(1);
  await detail
    .getByRole("button", { name: "Exact analytical inputs and parameters" })
    .click();
  await page.setViewportSize({ width: 1280, height: 800 });
  await detail.scrollIntoViewIfNeeded();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "../.cache/analytics-proof/1280.png",
    fullPage: false,
  });
  await page.setViewportSize({ width: 1440, height: 960 });
  await detail.scrollIntoViewIfNeeded();
  await page.screenshot({
    path: "../.cache/analytics-proof/desktop.png",
    fullPage: false,
  });
  const response = await page.request.post(
    `/api/brains/${fixture.brain}/claims/${fixture.rows[0].claim}/review`,
    {
      headers: { "x-csrf-token": fixture.csrf },
      data: {
        base_revision: fixture.rows[0].revision,
        action: "reject",
        reason: "Synthetic analytical contributor invalidation.",
      },
    },
  );
  expect(response.ok()).toBe(true);
  await expect(report.getByText("stale", { exact: true })).toBeVisible();
  await expect(detail.getByTestId("analytics-result")).toHaveCount(0);
  await expect(
    detail.getByText(/Current scores are unavailable/),
  ).toBeVisible();
  // A replacement ready generation invalidates reports on the old generation.
  // Wait for the automatic correction rebuild before explicitly recomputing.
  await expect.poll(currentKnowledgeReady, { timeout: 20_000 }).toBe(true);
  await queue.click();
  await expect(
    analytics
      .getByTestId("analytics-report")
      .first()
      .getByText("ready", { exact: true }),
  ).toBeVisible({ timeout: 20_000 });
  await analytics
    .getByTestId("analytics-report")
    .first()
    .getByRole("button", { name: "Open analytical report" })
    .click();
  await expect(detail.getByTestId("analytics-result")).toHaveCount(2);
  await openGraphDrawer(page, "Graph filters", "Filters");
  await panel
    .getByRole("textbox", { name: "Graph evidence mode", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Accepted claims only", exact: true })
    .click();
  await openGraphDrawer(page, "Graph insights", "Insights");
  await analytics
    .getByRole("textbox", { name: "Analysis", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Connected components", exact: true })
    .click();
  await queue.click();
  await expect(
    analytics.getByText(
      "No eligible vertices exist in this selection. Choose a ready input with retained evidence.",
      {
        exact: true,
      },
    ),
  ).toBeVisible();
  expect(errors).toEqual([]);
});
