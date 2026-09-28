import { loadGraph, openGraphDrawer } from "./desktop-helpers";
import { test, expect as baseExpect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
const expect = baseExpect.configure({ timeout: 20_000 });

test("graph processing, evidence, qualified paths and desktop canvas interactions", async ({
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
      if (!r.ok) throw new Error(`Graph fixture ${r.status}`);
      return r.json();
    };
    const brain = await post("/api/brains", {
      name: "Synthetic graph browser proof",
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
  let canonicalView: unknown;
  page.on("request", (request) => {
    if (
      request.method() === "POST" &&
      request.url().endsWith(`/api/brains/${fixture.brain}/graph/view`)
    )
      canonicalView = request.postDataJSON();
  });
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
  const load = () => loadGraph(page);
  await load();
  await openGraphDrawer(
    page,
    "Eligible entities",
    "Browse eligible entity pages",
  );
  await expect(panel.getByTestId("graph-entity")).toHaveCount(4);
  await page.keyboard.press("Escape");
  await openGraphDrawer(page, "Graph status and maintenance", "Graph status");
  await expect(
    panel.getByText("4 eligible entities · 2 eligible relationships", {
      exact: true,
    }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  const explorer = page.locator("body");
  const canvas = explorer.getByTestId("graph-canvas");
  await expect(canvas).toHaveAttribute("data-ready", "true");
  await expect(canvas.locator("canvas").first()).toBeVisible();
  await expect(explorer.getByTestId("exploration-counts")).toHaveText(
    "4 displayed entities · 2 directed relationships",
  );
  expect(canonicalView).toBeTruthy();
  const graphReads = await page.evaluate(
    async ({ brain, body }) => {
      const me = await (await fetch("/api/auth/me")).json();
      const milliseconds: number[] = [];
      for (let sample = 0; sample < 20; sample++) {
        const started = performance.now();
        const response = await fetch(`/api/brains/${brain}/graph/view`, {
          method: "POST",
          headers: {
            "content-type": "application/json",
            "x-csrf-token": me.csrf_token,
          },
          body: JSON.stringify(body),
        });
        if (!response.ok)
          throw Error(`Canonical graph measurement ${response.status}`);
        await response.json();
        milliseconds.push(Math.round((performance.now() - started) * 10) / 10);
      }
      return milliseconds;
    },
    { brain: fixture.brain, body: canonicalView },
  );
  const graphInspections: number[] = [];
  for (let sample = 0; sample < 20; sample++) {
    const started = performance.now();
    const selection = explorer.getByRole("textbox", {
      name: "Inspect graph entity",
      exact: true,
    });
    await selection.click();
    await selection.fill("Amber graph service");
    await expect(
      page.getByRole("option", { name: /^Amber graph service ·/ }),
    ).toBeVisible();
    await selection.press("ArrowDown");
    await selection.press("Enter");
    await expect(explorer.getByTestId("exploration-node")).toContainText(
      "Amber graph service",
    );
    graphInspections.push(Math.round((performance.now() - started) * 10) / 10);
    await page.keyboard.press("Escape");
    await expect(
      page.getByRole("dialog", { name: "Graph entity", exact: true }),
    ).not.toBeVisible();
  }
  const p95 = (values: number[]) =>
    [...values].sort((a, b) => a - b)[Math.ceil(values.length * 0.95) - 1];
  await mkdir("../.cache/ui", { recursive: true });
  await writeFile(
    "../.cache/ui/graph-repeated-measurements.json",
    JSON.stringify(
      {
        note: "20 local samples against the real four-entity, two-edge graph fixture. Reads measure completed canonical graph/view responses; inspection measures keyboard selection to visible entity details. These are warm fixture observations, not capacity or a pre-redesign comparison.",
        canonical_view: {
          sample_count: graphReads.length,
          p95_ms: p95(graphReads),
          milliseconds: graphReads,
        },
        entity_inspection: {
          sample_count: graphInspections.length,
          p95_ms: p95(graphInspections),
          milliseconds: graphInspections,
        },
      },
      null,
      2,
    ),
  );
  // Use the installed renderer's geometry to make a real pointer selection;
  // keyboard selection below is the independent accessible route.
  await canvas.scrollIntoViewIfNeeded();
  await page.evaluate(
    () =>
      new Promise<void>((done) =>
        requestAnimationFrame(() => requestAnimationFrame(() => done())),
      ),
  );
  const point = await canvas.evaluate((element) => {
    const cy = (
      element as HTMLElement & { _cyreg: { cy: import("cytoscape").Core } }
    )._cyreg.cy;
    const node = cy
      .nodes()
      .filter((n) => n.data("kind") === "source_version")
      .first();
    return { ...node.renderedPosition(), label: node.data("label") as string };
  });
  await canvas.click({ position: { x: point.x, y: point.y } });
  await expect(
    explorer.getByTestId("exploration-node"),
    JSON.stringify(point),
  ).toContainText(point.label);
  await page.keyboard.press("Escape");
  const entity = explorer.getByRole("textbox", {
    name: "Inspect graph entity",
    exact: true,
  });
  await entity.click();
  await entity.fill("Amber graph service");
  await expect(
    page.getByRole("option", { name: /^Amber graph service ·/ }),
  ).toBeVisible();
  await entity.press("ArrowDown");
  await entity.press("Enter");
  await expect(explorer.getByTestId("exploration-node")).toContainText(
    "Amber graph service",
  );
  await explorer
    .getByRole("button", { name: "Explore connections", exact: true })
    .click();
  await expect(explorer.getByTestId("exploration-counts")).toHaveText(
    "2 displayed entities · 1 directed relationships",
  );
  await page.keyboard.press("Escape");
  const relationship = explorer.getByRole("textbox", {
    name: "Inspect graph relationship",
    exact: true,
  });
  await relationship.click();
  await relationship.fill("supported by");
  await relationship.press("ArrowDown");
  await relationship.press("Enter");
  await expect(explorer.getByTestId("exploration-edge")).toContainText(
    "Amber graph service → Amber graph evidence",
  );
  await explorer
    .getByRole("button", { name: "Inspect target entity", exact: true })
    .click();
  await explorer
    .getByRole("button", { name: "Inspect selected evidence", exact: true })
    .click();
  await expect(
    page.getByRole("dialog").getByText(/GRAPH_INERT_EVIDENCE/),
  ).toBeVisible();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Back to graph", exact: true })
    .click();
  await openGraphDrawer(
    page,
    "Explore relationships",
    "Explore from an entity",
  );
  await explorer
    .getByRole("button", { name: "Show eligible overview", exact: true })
    .click();
  await expect(explorer.getByTestId("exploration-counts")).toHaveText(
    "4 displayed entities · 2 directed relationships",
  );
  await openGraphDrawer(
    page,
    "Eligible entities",
    "Browse eligible entity pages",
  );
  const amber = panel
    .getByTestId("graph-entity")
    .filter({ has: page.getByText("Amber graph service", { exact: true }) });
  const source = panel
    .getByTestId("graph-entity")
    .filter({ has: page.getByText("Amber graph evidence", { exact: true }) });
  await amber
    .getByRole("button", { name: "Use as start", exact: true })
    .click();
  await source.getByRole("button", { name: "Use as end", exact: true }).click();
  let paths = 0;
  let views = 0;
  let rebuilds = 0;
  page.on("request", (request) => {
    if (request.method() !== "POST") return;
    if (request.url().endsWith("/graph/path")) paths++;
    if (request.url().endsWith("/graph/view")) views++;
    if (request.url().endsWith("/graph/rebuild")) rebuilds++;
  });
  await openGraphDrawer(page, "Find an evidence path", "Find path");
  await panel
    .getByRole("button", { name: "Find shortest eligible path", exact: true })
    .click();
  await expect(
    panel.getByRole("heading", {
      name: "1 hops in the eligible graph",
      exact: true,
    }),
  ).toBeVisible();
  await expect(panel.getByTestId("graph-path-entity")).toHaveCount(2);
  await expect(
    panel.getByText("supported by · provenance", { exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await openGraphDrawer(
    page,
    "Explore relationships",
    "Explore from an entity",
  );
  await explorer
    .getByRole("button", { name: "Show shortest path on canvas", exact: true })
    .click();
  await expect(explorer.getByTestId("exploration-counts")).toHaveText(
    "2 displayed entities · 1 directed relationships",
  );
  await explorer
    .getByRole("button", { name: "Zoom graph in", exact: true })
    .focus();
  await page.keyboard.press("Enter");
  await explorer
    .getByRole("button", { name: "Fit graph", exact: true })
    .click();
  await openGraphDrawer(
    page,
    "Eligible entities",
    "Browse eligible entity pages",
  );
  await source
    .getByRole("button", { name: "Inspect evidence", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(dialog.getByText(/GRAPH_INERT_EVIDENCE/)).toBeVisible();
  expect(await page.evaluate(() => "graphInjected" in window)).toBe(false);
  await dialog
    .getByRole("button", { name: "Back to graph", exact: true })
    .click();
  const afterExplicit = { paths, views, rebuilds };
  // Exercise ordinary status polling/focus. These reads must not replay a path,
  // a rebuild, or a previously submitted graph view.
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await page.waitForTimeout(3400);
  expect({ paths, views, rebuilds }).toEqual(afterExplicit);
  await openGraphDrawer(
    page,
    "Eligible entities",
    "Browse eligible entity pages",
  );
  const cobalt = panel
    .getByTestId("graph-entity")
    .filter({ has: page.getByText("Cobalt graph service", { exact: true }) });
  await cobalt.getByRole("button", { name: "Use as end", exact: true }).click();
  await openGraphDrawer(page, "Find an evidence path", "Find path");
  await panel
    .getByRole("button", { name: "Find shortest eligible path", exact: true })
    .click();
  await expect(
    panel.getByRole("heading", {
      name: "No path within the selected hop bound",
      exact: true,
    }),
  ).toBeVisible();
  await openGraphDrawer(page, "Graph filters", "Filters");
  await panel
    .getByRole("textbox", { name: "Graph evidence mode", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Accepted claims only", exact: true })
    .click();
  await expect(panel.getByTestId("graph-entity")).toHaveCount(0);
  await expect(panel.getByTestId("graph-canvas")).toHaveCount(0);
  await load();
  await expect(
    panel.getByText("No eligible entities", { exact: true }),
  ).toBeVisible();
  await openGraphDrawer(page, "Graph filters", "Filters");
  await panel
    .getByRole("textbox", { name: "Graph evidence mode", exact: true })
    .click();
  await page
    .getByRole("option", {
      name: "Investigation with qualifications",
      exact: true,
    })
    .click();
  await load();
  await openGraphDrawer(
    page,
    "Eligible entities",
    "Browse eligible entity pages",
  );
  await expect(panel.getByTestId("graph-entity")).toHaveCount(4);
  await page.keyboard.press("Escape");
  await expect(canvas).toHaveAttribute("data-ready", "true");
  // An actual invalid center must replace the graph with an error, not retain
  // a formerly successful canvas. A subsequent explicit overview recovers it.
  await openGraphDrawer(
    page,
    "Explore relationships",
    "Explore from an entity",
  );
  await explorer
    .getByLabel("Exploration center", { exact: true })
    .fill("claim:invalid");
  await explorer
    .getByRole("button", { name: "Explore from center", exact: true })
    .click();
  await expect(
    explorer.getByText(
      "The center must be an eligible entity in this exact graph view.",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(canvas).toHaveCount(0);
  await openGraphDrawer(
    page,
    "Explore relationships",
    "Explore from an entity",
  );
  await explorer
    .getByRole("button", { name: "Show eligible overview", exact: true })
    .click();
  await expect(canvas).toHaveAttribute("data-ready", "true");
  await openGraphDrawer(page, "Find an evidence path", "Find path");
  await panel.getByLabel("Path start", { exact: true }).fill("claim:invalid");
  await panel
    .getByLabel("Path end", { exact: true })
    .fill(`source_version:${fixture.rows[0].version}`);
  await openGraphDrawer(page, "Find an evidence path", "Find path");
  await panel
    .getByRole("button", { name: "Find shortest eligible path", exact: true })
    .click();
  await expect(
    panel.getByText(
      "Both path endpoints must be eligible members of this exact graph view.",
      { exact: true },
    ),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await canvas.scrollIntoViewIfNeeded();
  await page.screenshot({
    path: "../.cache/exploration-proof/desktop.png",
    fullPage: false,
  });
  // Renderer-only stress uses the real response shape with synthetic display
  // records. Actual 500/2000 admission and overflow are covered in PostgreSQL/
  // Neo4j integration tests; no mock establishes canonical eligibility here.
  await page.route("**/graph/explore", async (route) => {
    const response = await route.fetch();
    const graph = await response.json();
    graph.nodes = Array.from({ length: 500 }, (_, i) => ({
      ...graph.nodes[0],
      key: `claim:dense-${i}`,
      evidence: {
        ...graph.nodes[0].evidence,
        label: `Dense synthetic entity ${i}`,
      },
    }));
    graph.edges = Array.from({ length: 2000 }, (_, i) => ({
      ...graph.edges[0],
      id: `dense-edge-${i}`,
      from: `claim:dense-${Math.floor(i / 4)}`,
      to: `claim:dense-${(Math.floor(i / 4) + (i % 4) + 1) % 500}`,
    }));
    graph.view.total_nodes = 500;
    graph.view.total_edges = 2000;
    await route.fulfill({ json: graph });
  });
  const began = Date.now();
  await openGraphDrawer(
    page,
    "Explore relationships",
    "Explore from an entity",
  );
  await explorer
    .getByRole("button", { name: "Show eligible overview", exact: true })
    .click();
  await expect(explorer.getByTestId("exploration-counts")).toHaveText(
    "500 displayed entities · 2000 directed relationships",
  );
  await expect(canvas).toHaveAttribute("data-ready", "true");
  expect(
    await canvas.evaluate((element) => {
      const cy = (
        element as HTMLElement & { _cyreg: { cy: import("cytoscape").Core } }
      )._cyreg.cy;
      return [cy.nodes().length, cy.edges().length];
    }),
  ).toEqual([500, 2000]);
  await entity.fill("Dense synthetic entity 499");
  await entity.press("ArrowDown");
  await entity.press("Enter");
  await expect(explorer.getByTestId("exploration-node")).toContainText(
    "Dense synthetic entity 499",
  );
  console.info(
    `500-node/2000-edge renderer and keyboard selection: ${Date.now() - began} ms`,
  );
  await page.keyboard.press("Escape");
  await canvas.scrollIntoViewIfNeeded();
  await page.screenshot({ path: "../.cache/exploration-proof/dense.png" });
  await page.unroute("**/graph/explore");
  // Clock expiry is not an epoch event. Verify that the response deadline
  // itself clears both the canvas and parent results and disposes the renderer.
  await page.route("**/graph/explore", async (route) => {
    const response = await route.fetch();
    const graph = await response.json();
    graph.expires_at = new Date(Date.now() + 2000).toISOString();
    await route.fulfill({ json: graph });
  });
  await openGraphDrawer(
    page,
    "Explore relationships",
    "Explore from an entity",
  );
  await explorer
    .getByRole("button", { name: "Show eligible overview", exact: true })
    .click();
  await expect(canvas).toHaveAttribute("data-ready", "true");
  const disposed = await canvas.evaluateHandle(
    (element) =>
      (element as HTMLElement & { _cyreg: { cy: import("cytoscape").Core } })
        ._cyreg.cy,
  );
  await expect(canvas).toHaveCount(0);
  await expect(panel.getByTestId("graph-entity")).toHaveCount(0);
  expect(await disposed.evaluate((cy) => cy.destroyed())).toBe(true);
  await disposed.dispose();
  await page.unroute("**/graph/explore");
  expect(errors).toEqual([]);
});
