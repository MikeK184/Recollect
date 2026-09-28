import { test, expect as baseExpect } from "@playwright/test";
const expect = baseExpect.configure({ timeout: 20_000 });

test("desktop graph recall preserves witnesses, explicit queries and scope/expiry clearing", async ({
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
      if (!r.ok) throw new Error(`Recall graph fixture ${r.status}`);
      return r.json();
    };
    const brain = await post("/api/brains", { name: "Desktop fusion proof" });
    const base = `/api/brains/${brain.id}`;
    const rows = [];
    for (const name of ["Amber", "Cobalt"]) {
      const source = await post(`${base}/sources`, {
        title: `${name} source`,
        media_type: "text/plain",
        retain_content: true,
        content: `${name} uses port 8080.\nFUSION_INERT <script>window.fusionInjected=true</script>\n`,
      });
      const claim = await post(`${base}/claims`, {
        content: {
          kind: "claim",
          subject: `${name} service`,
          predicate: "port",
          value: "8080",
          rationale: "Synthetic declared evidence.",
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
    return { brain: brain.id, base, rows };
  });
  for (const row of fixture.rows) {
    await expect
      .poll(
        async () =>
          (
            await (
              await page.request.get(
                `${fixture.base}/sources/${row.source}/versions/${row.version}`,
              )
            ).json()
          ).version.processing,
      )
      .toBe("ready");
  }
  await page.evaluate(async ({ base }) => {
    const me = await (await fetch("/api/auth/me")).json();
    const response = await fetch(`${base}/graph/rebuild`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-csrf-token": me.csrf_token,
      },
      body: JSON.stringify({ kind: "knowledge" }),
    });
    if (!response.ok) throw new Error(`Graph rebuild ${response.status}`);
  }, fixture);
  await expect
    .poll(async () =>
      (
        await (await page.request.get(`${fixture.base}/graph`)).json()
      ).generations.some((g: { state: string }) => g.state === "ready"),
    )
    .toBe(true);
  await page.goto(`/brains/${fixture.brain}/ask?tab=search`);
  const panel = page.getByRole("region", {
    name: "Recall memory",
    exact: true,
  });
  await panel
    .getByRole("button", { name: "Scope, time and exact lookup" })
    .click();
  await panel.getByLabel("Graph relationships", { exact: true }).check();
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Amber service");
  let queries = 0;
  page.on("request", (request) => {
    if (
      request.method() === "POST" &&
      request.url().endsWith(`${fixture.base}/recall`)
    )
      queries++;
  });
  const submit = panel.getByRole("button", { name: "Recall", exact: true });
  await submit.click();
  await expect(panel.getByTestId("recall-result")).toHaveCount(2);
  await expect(panel.getByTestId("recall-graph-witness")).toHaveCount(1);
  await expect(panel.getByTestId("recall-graph-witness")).toContainText(
    "Amber service → Amber source",
  );
  await expect(panel.getByTestId("recall-graph-status")).toContainText(
    "1 anchors · 1 connected candidates",
  );
  await expect(
    panel.getByText(/Source coverage: 1 resolved groups/),
  ).toBeVisible();
  expect(queries).toBe(1);
  const connected = panel
    .getByTestId("recall-result")
    .filter({ has: page.getByTestId("recall-graph-witness") });
  await connected
    .getByRole("button", { name: "Inspect evidence 1", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(dialog.getByTestId("claim-evidence-text")).toContainText(
    "FUSION_INERT",
  );
  expect(
    await page.evaluate(
      () => (window as unknown as { fusionInjected?: boolean }).fusionInjected,
    ),
  ).toBeUndefined();
  await page.keyboard.press("Escape");
  await panel
    .getByRole("button", { name: "Scope, time and exact lookup" })
    .click();
  await expect(
    panel.getByLabel("Recall graph", { exact: true }),
  ).not.toBeVisible();
  await panel.screenshot({
    path: "../.cache/recall-fusion-desktop.png",
    animations: "disabled",
  });
  await panel
    .getByRole("button", { name: "Scope, time and exact lookup" })
    .click();
  await panel.getByLabel("Recall graph hops", { exact: true }).fill("1");
  await expect(panel.getByTestId("recall-result")).toHaveCount(0);
  expect(queries).toBe(1);
  await submit.click();
  await expect(panel.getByTestId("recall-result")).toHaveCount(2);
  await panel
    .getByLabel("Prefer source coverage before adding depth", { exact: true })
    .uncheck();
  await expect(panel.getByTestId("recall-result")).toHaveCount(0);
  expect(queries).toBe(2);
  await panel.getByLabel("Text search", { exact: true }).uncheck();
  await panel
    .getByLabel("Exact identities and literals", { exact: true })
    .uncheck();
  await expect(submit).toBeDisabled();
  await expect(
    panel.getByText("Graph search needs current query anchors"),
  ).toBeVisible();
  await panel
    .getByLabel("Exact identities and literals", { exact: true })
    .check();
  await page.route(`**${fixture.base}/recall`, async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    body.graph.expires_at = new Date(Date.now() + 2000).toISOString();
    await route.fulfill({ response, json: body });
  });
  await submit.click();
  await expect(panel.getByTestId("recall-result")).toHaveCount(2);
  await expect(panel.getByTestId("recall-result")).toHaveCount(0, {
    timeout: 5000,
  });
  expect(queries).toBe(3);
  await page.unroute(`**${fixture.base}/recall`);
  await page.route(`**${fixture.base}/recall`, (route) =>
    route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({
        code: "graph_unavailable",
        message: "Synthetic unavailable graph",
      }),
    }),
  );
  await submit.click();
  await expect(panel.getByText("Recall failed")).toBeVisible();
  await expect(panel.getByTestId("recall-result")).toHaveCount(0);
  await page.unroute(`**${fixture.base}/recall`);
  await submit.click();
  await expect(panel.getByTestId("recall-result")).toHaveCount(2);
  expect(queries).toBe(5);
  await page.evaluate(async ({ base, rows }) => {
    const me = await (await fetch("/api/auth/me")).json();
    const response = await fetch(`${base}/claims/${rows[0].claim}/review`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-csrf-token": me.csrf_token,
      },
      body: JSON.stringify({
        base_revision: rows[0].revision,
        action: "reject",
        reason: "Synthetic current correction",
      }),
    });
    if (!response.ok) throw new Error(`Review ${response.status}`);
  }, fixture);
  await expect(panel.getByTestId("recall-result")).toHaveCount(0);
  expect(queries).toBe(5);
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Cobalt service");
  await submit.click();
  await expect(panel.getByTestId("recall-result")).toHaveCount(2);
  await expect(panel.getByTestId("recall-graph-witness")).toContainText(
    "Cobalt service → Cobalt source",
  );
  expect(errors).toEqual([]);
});
