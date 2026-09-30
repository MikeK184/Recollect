import { loadGraph, openGraphDrawer } from "./desktop-helpers";
import { test, expect as baseExpect } from "@playwright/test";
const expect = baseExpect.configure({ timeout: 20_000 });

// Desktop knowledge surface, "Graph placement": the canvas owns the dominant
// region at 1440x900, its controls live in canvas chrome, coverage limitations
// collapse to one persistent indicator, the bounded default issues exactly one
// existing read (no model call, rebuild, analytics or traversal), and the
// keyboard list alternative stays reachable from the canvas chrome.
test("graph canvas dominates its chrome with one bounded default read", async ({
  page,
}) => {
  test.setTimeout(180_000);
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
      if (!r.ok) throw new Error(`Graph chrome fixture ${r.status}`);
      return r.json();
    };
    const brain = await post("/api/brains", {
      name: "Synthetic graph chrome proof",
    });
    const rows = [];
    for (const name of ["Amber", "Cobalt"]) {
      const source = await post(`/api/brains/${brain.id}/sources`, {
        title: `${name} graph evidence`,
        media_type: "text/plain",
        retain_content: true,
        content: `${name} port 8080.\nGRAPH_INERT_EVIDENCE <script>window.graphInjected=true</script>\n`,
      });
      const claim = await post(`/api/brains/${brain.id}/claims`, {
        content: {
          kind: "claim",
          subject: `${name} graph service`,
          predicate: "port",
          value: "8080",
          rationale: "Synthetic graph declaration.",
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
    return { brain: brain.id, rows };
  });
  await expect
    .poll(
      async () =>
        (
          await (
            await page.request.get(
              `/api/brains/${fixture.brain}/sources/${fixture.rows[0].source}/versions/${fixture.rows[0].version}`,
            )
          ).json()
        ).version.processing,
    )
    .toBe("ready");
  // Materialize a ready knowledge generation before the navigation under test.
  await page.goto(`/brains/${fixture.brain}/graph`);
  const panel = page.locator("body");
  await expect(
    page.getByRole("region", { name: "Evidence graphs", exact: true }),
  ).toBeVisible();
  await openGraphDrawer(page, "Graph status and maintenance", "Graph status");
  await panel
    .getByRole("button", { name: "Rebuild graph", exact: true })
    .click();
  await expect(
    panel.getByText("Graph rebuild queued. Progress appears below."),
  ).toBeVisible();
  await expect
    .poll(async () =>
      (
        await (
          await page.request.get(`/api/brains/${fixture.brain}/graph`)
        ).json()
      ).generations.some((g: { state: string }) => g.state === "ready"),
    )
    .toBe(true);
  // --- Navigation under test: capture every graph request from a fresh load.
  const requests = { view: 0, rebuild: 0, path: 0, analytics: 0 };
  const exploreRequests: string[] = [];
  page.on("request", (request) => {
    if (request.method() !== "POST") return;
    const url = request.url();
    if (url.endsWith(`/api/brains/${fixture.brain}/graph/view`)) requests.view++;
    if (url.endsWith(`/api/brains/${fixture.brain}/graph/rebuild`))
      requests.rebuild++;
    if (url.endsWith(`/api/brains/${fixture.brain}/graph/path`)) requests.path++;
    if (url.includes(`/api/brains/${fixture.brain}/graph/analytics`))
      requests.analytics++;
    if (url.endsWith(`/api/brains/${fixture.brain}/graph/explore`))
      exploreRequests.push(request.postData() ?? "");
  });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/brains/${fixture.brain}/graph`);
  const canvas = page.getByTestId("graph-canvas");
  await expect(canvas).toHaveAttribute("data-ready", "true");
  // The bounded default invokes the existing read endpoint exactly once and
  // runs no model, rebuild, analytics or arbitrary traversal on navigation.
  // The explorer's own overview read is the pre-existing bounded display read
  // (null center, hop and display bounds); it must not become a centered or
  // unbounded traversal.
  expect(requests.view).toBe(1);
  expect(requests.rebuild).toBe(0);
  expect(requests.path).toBe(0);
  expect(requests.analytics).toBe(0);
  expect(exploreRequests.length).toBeLessThanOrEqual(1);
  for (const body of exploreRequests) {
    expect(JSON.parse(body).center ?? null).toBe(null);
  }
  const usage = await (
    await page.request.get(`/api/brains/${fixture.brain}/models/usage`)
  ).json();
  expect(usage.total).toBe(0);
  // --- Geometry: the canvas region exceeds the combined chrome height.
  const geometry = await page.evaluate(() => {
    const surface = document.querySelector(".graph-surface");
    const stage = document.querySelector(".graph-canvas");
    if (!surface || !stage) return null;
    const chrome = [...surface.querySelectorAll<HTMLElement>("[data-graph-chrome]")].map(
      (element) => element.getBoundingClientRect().height,
    );
    return {
      canvas: stage.getBoundingClientRect().height,
      chrome: chrome.reduce((sum, height) => sum + height, 0),
      parts: chrome,
    };
  });
  expect(geometry).toBeTruthy();
  console.info(
    `graph geometry @1440x900: canvas=${Math.round(geometry!.canvas)}px combined chrome=${Math.round(geometry!.chrome)}px parts=${geometry!.parts.map((h) => Math.round(h)).join("+")}px`,
  );
  expect(geometry!.canvas).toBeGreaterThan(geometry!.chrome);
  // The moved controls stay reachable in their new canvas-chrome locations.
  await expect(
    page.getByRole("textbox", { name: "Inspect graph entity", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("textbox", { name: "Inspect graph relationship", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Explore from an entity", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Filters", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Graph status", exact: true }),
  ).toBeVisible();
  // --- Coverage limitation: one persistent indicator, visible collapsed,
  // expanded on demand. The fixture responses are intercepted to report a
  // partial coverage; backend coverage semantics are covered natively. The
  // explorer renders the indicator from its bounded overview read's view.
  const markPartial = (body: {
    coverage?: { partial?: boolean; reasons?: string[] };
    view?: { coverage?: { partial?: boolean; reasons?: string[] } };
  }) => {
    for (const coverage of [body.coverage, body.view?.coverage]) {
      if (coverage) {
        coverage.partial = true;
        coverage.reasons = ["source_text_unavailable"];
      }
    }
    return body;
  };
  await page.route("**/graph/view", async (route) => {
    const response = await route.fetch();
    await route.fulfill({ json: markPartial(await response.json()) });
  });
  await page.route("**/graph/explore", async (route) => {
    const response = await route.fetch();
    await route.fulfill({ json: markPartial(await response.json()) });
  });
  await loadGraph(page);
  const coverage = page.getByRole("button", { name: "Coverage limits", exact: true });
  await expect(coverage).toBeVisible();
  await expect(coverage).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByText("source text unavailable")).not.toBeVisible();
  await coverage.click();
  await expect(coverage).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByText("source text unavailable")).toBeVisible();
  await page.unroute("**/graph/view");
  await page.unroute("**/graph/explore");
  // --- Keyboard-accessible list alternative, reachable from the canvas chrome.
  // The Canvas/List switch is a radiogroup in the canvas chrome; its items are
  // focusable radio inputs activated with Space.
  const listLabel = page.getByText("List", { exact: true });
  await expect(listLabel).toBeVisible();
  const listRadio = page.getByRole("radio", { name: "List", exact: true });
  await listRadio.focus();
  await page.keyboard.press("Space");
  const nodeButtons = page.locator(".graph-node-button");
  await expect(nodeButtons).toHaveCount(4);
  // List order is not guaranteed; select the Amber claim by its label.
  const amberButton = nodeButtons.filter({
    has: page.getByText("Amber graph service", { exact: true }),
  });
  await amberButton.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("exploration-node")).toContainText(
    "Amber graph service",
  );
  await page.keyboard.press("Escape");
  // The canvas representation returns from the chrome switch as well.
  const canvasRadio = page.getByRole("radio", { name: "Canvas", exact: true });
  await canvasRadio.focus();
  await page.keyboard.press("Space");
  await expect(canvas).toHaveAttribute("data-ready", "true");
  expect(errors).toEqual([]);
});
