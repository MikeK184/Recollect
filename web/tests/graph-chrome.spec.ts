import { loadGraph, openGraphDrawer } from "./desktop-helpers";
import { test, expect as baseExpect } from "@playwright/test";
const expect = baseExpect.configure({ timeout: 20_000 });

// Desktop knowledge surface, "Graph placement": the canvas owns the dominant
// region at 1440x900, its controls live in canvas chrome, coverage limitations
// collapse to one persistent indicator, the bounded default issues exactly one
// existing read (no model call, rebuild, analytics or traversal), and the
// bounded entity picker stays reachable through the tools menu.
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
      let claim = await post(`/api/brains/${brain.id}/claims`, {
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
          freshness: "needs_verification",
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
      const reviewed = await post(`/api/brains/${brain.id}/claims/${claim.claim_id}/review`, {
        base_revision: claim.id, action: "accept", reason: "Synthetic evidence checked by the fixture owner.",
      });
      claim = reviewed.claims[0].revision;
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
  // Baseline model usage before the navigation under test. Fixture source
  // processing may have charged models already; what the contract forbids is a
  // model call triggered by navigating to the graph view itself.
  const usageBefore = (
    await (
      await page.request.get(`/api/brains/${fixture.brain}/models/usage`)
    ).json()
  ).total;
  // --- Navigation under test: capture every graph request from a fresh load.
  const requests = { view: 0, rebuild: 0, path: 0, analytics: 0 };
  const exploreRequests: string[] = [];
  page.on("request", (request) => {
    if (request.method() !== "POST") return;
    const url = request.url();
    if (url.endsWith(`/api/brains/${fixture.brain}/graph/view`))
      requests.view++;
    if (url.endsWith(`/api/brains/${fixture.brain}/graph/rebuild`))
      requests.rebuild++;
    if (url.endsWith(`/api/brains/${fixture.brain}/graph/path`))
      requests.path++;
    if (url.includes(`/api/brains/${fixture.brain}/graph/analytics`))
      requests.analytics++;
    if (url.endsWith(`/api/brains/${fixture.brain}/graph/explore`))
      exploreRequests.push(request.postData() ?? "");
  });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/brains/${fixture.brain}/graph`);
  const canvas = page.getByTestId("graph-canvas");
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  // Navigation uses exactly one bounded exploration response, including its
  // canonical view, with no redundant complete-view read.
  expect(requests.view).toBe(0);
  expect(requests.rebuild).toBe(0);
  expect(requests.path).toBe(0);
  expect(requests.analytics).toBe(0);
  expect(exploreRequests).toHaveLength(1);
  for (const body of exploreRequests) {
    expect(JSON.parse(body).center ?? null).toBe(null);
  }
  const usage = await (
    await page.request.get(`/api/brains/${fixture.brain}/models/usage`)
  ).json();
  // Navigation must not start a model call: the total is unchanged from the
  // pre-navigation baseline.
  expect(usage.total).toBe(usageBefore);
  // --- Geometry: the canvas region exceeds the combined chrome height.
  const geometry = await page.evaluate(() => {
    const surface = document.querySelector(".graph-surface");
    const stage = document.querySelector(".graph-canvas");
    if (!surface || !stage) return null;
    const chrome = [
      ...surface.querySelectorAll<HTMLElement>("[data-graph-chrome]"),
    ].map((element) => element.getBoundingClientRect().height);
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
  // Fresh navigation at each width exercises the initial layout, not only a
  // resized graph whose positions were inherited from the previous viewport.
  for (const width of [1280, 1440, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    await page.reload();
    await expect(canvas).toHaveAttribute("data-layout-ready", "true");
    const overlapping = await canvas.evaluate((element) => {
      const cy = (element as HTMLElement & { _cyreg: { cy: import("cytoscape").Core } })._cyreg.cy;
      const nodes = cy.nodes().map((node) => ({ id: node.id(), box: node.boundingBox({ includeLabels: true }) }));
      return nodes.flatMap((a, index) => nodes.slice(index + 1).filter((b) =>
        a.box.x1 < b.box.x2 && b.box.x1 < a.box.x2 && a.box.y1 < b.box.y2 && b.box.y1 < a.box.y2,
      ).map((b) => [a.id, b.id]));
    });
    expect(overlapping, `Settled node and label bounds at ${width}px`).toEqual([]);
    await page.screenshot({ path: `../.cache/ui/graph-review-${width}.png` });
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  // The moved controls stay reachable in their new canvas-chrome locations.
  await expect(
    page.getByRole("textbox", { name: "Inspect graph entity", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "View", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Explore", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Filters", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Graph tools", exact: true }),
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
  const coverage = page.getByRole("button", {
    name: "Coverage limits",
    exact: true,
  });
  await expect(coverage).toBeVisible();
  await expect(coverage).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByText("source text unavailable")).not.toBeVisible();
  await coverage.click();
  await expect(coverage).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByText("source text unavailable")).toBeVisible();
  await page.unroute("**/graph/view");
  await page.unroute("**/graph/explore");
  // The flat graph remains the main view. The bounded entity picker stays
  // reachable by keyboard through the secondary tools menu.
  const tools = page.getByRole("button", { name: "Graph tools", exact: true });
  await tools.focus();
  await page.keyboard.press("Enter");
  const entitiesItem = page.getByRole("menuitem", {
    name: "Entities",
    exact: true,
  });
  await expect(entitiesItem).toBeVisible();
  await entitiesItem.focus();
  await page.keyboard.press("Enter");
  const entities = page.getByRole("dialog", { name: "Entities", exact: true });
  const choices = entities.getByTestId("graph-entity");
  await expect(choices).toHaveCount(4);
  const amber = choices.filter({ hasText: "Amber graph service" });
  await amber
    .getByRole("button", { name: "Inspect evidence", exact: true })
    .focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("dialog", {
      name: "Claim and knowledge history",
      exact: true,
    }),
  ).toContainText("Amber graph service");
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  expect(errors).toEqual([]);
});
