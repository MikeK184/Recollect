import { openDetails } from "./desktop-helpers";
import { test, expect as baseExpect, type Page } from "@playwright/test";
const expect = baseExpect.configure({ timeout: 20_000 });

async function setup(page: Page, name: string) {
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
  const fixture = await page.evaluate(async (name) => {
    const me = await (await fetch("/api/auth/me")).json();
    const post = async (url: string, body: unknown) => {
      const r = await fetch(url, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-csrf-token": me.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error(`Investigation fixture: ${r.status} ${url}`);
      return r.json();
    };
    const brain = await post("/api/brains", { name });
    const base = `/api/brains/${brain.id}`;
    const area = await post(`${base}/evidence/groups`, {
      kind: "area",
      name: "Test systems",
    });
    const environment = await post(`${base}/evidence/groups`, {
      kind: "environment",
      name: "Laboratory",
    });
    const source = await post(`${base}/sources`, {
      title: "Investigation source",
      content:
        "Investigation declared values: 8080 and 9090.\nEXACT_OLD_EVIDENCE <script>window.investigationInjected=true</script>\n",
      retain_content: true,
      media_type: "text/plain",
      group_ids: [area.id, environment.id],
    });
    const missing = await post(`${base}/sources`, {
      title: "Unavailable note",
      source_uri: "https://example.invalid/retained-reference",
      retain_content: false,
      media_type: "text/plain",
    });
    const selection = {
      repository_ids: [],
      area_ids: [area.id],
      environment_id: environment.id,
    };
    const content = (
      subject: string,
      value: string,
      version = source.version.id,
    ) => ({
      kind: "claim",
      subject,
      predicate: "port",
      value,
      rationale: "Synthetic declared evidence.",
      selection,
      manifest_revision_id: null,
      validity: { kind: "unknown", from: null, to: null, precision: "unknown" },
      freshness: "current",
      operational: "declared",
      observed_at: null,
      observation: "",
      supports: [
        { kind: "source_version", id: version, line_from: null, line_to: null },
      ],
    });
    const left = await post(`${base}/claims`, {
      content: content("Investigation Boreal", "8080"),
    });
    const right = await post(`${base}/claims`, {
      content: content("Investigation Boreal", "9090"),
    });
    const original = await post(`${base}/claims`, {
      content: content("Investigation Cobalt", "7070"),
    });
    const accepted = (
      await post(`${base}/claims/${original.claim_id}/review`, {
        base_revision: original.id,
        action: "accept",
        reason: "Independent declared positive control.",
      })
    ).claims[0].revision;
    const changing = await post(`${base}/claims`, {
      content: content("Investigation Orion", "6060"),
    });
    const unavailable = await post(`${base}/claims`, {
      content: content(
        "Missing supported memory",
        "retained assertion",
        missing.version.id,
      ),
    });
    const procedure = await post(`${base}/claims`, {
      content: {
        ...content(
          "Investigation procedure",
          "Inspect the fixture without changing it.",
        ),
        kind: "procedure",
        predicate: "procedure",
        procedure: {
          conditions: "Use the isolated local fixture.",
          steps: ["Read its status.", "Record the observation."],
          expected_outcome: "A recorded declaration.",
          observations: [],
        },
      },
    });
    const handover = await post(`${base}/claims`, {
      content: {
        ...content(
          "Investigation handover",
          "An untested procedure is recorded.",
        ),
        kind: "handover",
        predicate: "handover",
        handover: {
          completed: ["Recorded the procedure."],
          next_steps: ["Verify conditions before any execution."],
          risks: ["No operational proof."],
          contributions: [procedure.id],
        },
      },
    });
    return {
      brain: brain.id,
      base,
      area: area.id,
      environment: environment.id,
      selection,
      source,
      missing,
      left,
      right,
      accepted,
      changing,
      unavailable,
      procedure,
      handover,
    };
  }, name);
  await expect
    .poll(
      async () =>
        (
          await (
            await page.request.get(
              `${fixture.base}/sources/${fixture.source.id}/versions/${fixture.source.version.id}`,
            )
          ).json()
        ).version.processing,
    )
    .toBe("ready");
  await page.goto(`/brains/${fixture.brain}/ask?tab=search`);
  return fixture;
}

async function post(page: Page, url: string, body: unknown, method = "POST") {
  return page.evaluate(
    async ({ url, body, method }) => {
      const me = await (await fetch("/api/auth/me")).json();
      const r = await fetch(url, {
        method,
        headers: {
          "content-type": "application/json",
          "x-csrf-token": me.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error(`Investigation command: ${r.status}`);
      return r.json();
    },
    { url, body, method },
  );
}

test("desktop investigation preserves comparison, history, sources and scoped graph navigation", async ({
  page,
  context,
}) => {
  test.setTimeout(180_000);
  page.setDefaultTimeout(12_000);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const f = await setup(page, "Desktop investigation proof");
  await post(page, `${f.base}/graph/rebuild`, { kind: "knowledge" });
  await expect
    .poll(async () =>
      (
        await (await page.request.get(`${f.base}/graph`)).json()
      ).generations.some((g: { state: string }) => g.state === "ready"),
    )
    .toBe(true);
  const panel = page.getByRole("region", {
    name: "Recall memory",
    exact: true,
  });
  const submit = panel.getByRole("button", { name: "Recall", exact: true });
  await openDetails(panel, "Refine evidence search");
  await panel
    .getByRole("button", { name: "Scope, time and exact lookup" })
    .click();
  await panel.getByLabel("Recall areas", { exact: true }).click();
  await page.getByRole("option", { name: "Test systems", exact: true }).click();
  await page.keyboard.press("Escape");
  await panel.getByLabel("Recall environment", { exact: true }).click();
  await page.getByRole("option", { name: "Laboratory", exact: true }).click();
  await panel
    .getByRole("button", { name: "Scope, time and exact lookup" })
    .click();
  let recalls = 0;
  page.on("request", (r) => {
    if (r.method() === "POST" && r.url().endsWith(`${f.base}/recall`))
      recalls++;
  });
  const read = page.waitForResponse(
    (r) =>
      r.url().endsWith(`${f.base}/recall`) && r.request().method() === "POST",
  );
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Investigation");
  await submit.click();
  const answer = await (await read).json();
  await expect(panel.getByTestId("recall-result")).toHaveCount(
    answer.context.items.length,
  );
  expect(
    answer.context.items.some((i: { id: string }) => i.id === f.left.claim_id),
  ).toBe(true);
  expect(
    answer.context.items.some((i: { id: string }) => i.id === f.right.claim_id),
  ).toBe(true);
  await expect(panel.getByTestId("recall-selection")).toContainText(
    "Test systems",
  );
  await expect(panel.getByTestId("recall-selection")).toContainText(
    "Laboratory",
  );
  await panel.getByRole("tab", { name: /^Disagreements/ }).click();
  await expect(panel.getByTestId("recall-disagreement")).toHaveCount(1);
  await expect(panel.getByTestId("recall-disagreement")).toContainText(
    "Value: 8080",
  );
  await expect(panel.getByTestId("recall-disagreement")).toContainText(
    "Value: 9090",
  );
  await panel.screenshot({
    path: "../.cache/investigation-disagreements.png",
    animations: "disabled",
  });
  await panel.getByRole("tab", { name: /^Sources/ }).click();
  await expect(panel.getByTestId("recall-source")).toHaveCount(1);
  await panel
    .getByRole("button", { name: "Copy attributed context", exact: true })
    .click();
  const copied = JSON.parse(
    await page.evaluate(() => navigator.clipboard.readText()),
  );
  expect(copied).toEqual(answer.context);
  await panel
    .getByRole("button", { name: "Inspect exact source", exact: true })
    .click();
  let dialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(dialog.getByTestId("claim-evidence-text")).toContainText(
    "EXACT_OLD_EVIDENCE",
  );
  expect(
    await page.evaluate(
      () =>
        (window as unknown as { investigationInjected?: boolean })
          .investigationInjected,
    ),
  ).toBeUndefined();
  await page.keyboard.press("Escape");
  await expect(panel.getByRole("tab", { name: /^Sources/ })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  expect(recalls).toBe(1);
  await panel.getByRole("tab", { name: /^Matches/ }).click();
  const cobalt = panel
    .getByTestId("recall-result")
    .filter({ hasText: "Investigation Cobalt" });
  const exploreCall = page.waitForRequest((r) =>
    r.url().endsWith(`${f.base}/graph/explore`),
  );
  await cobalt
    .getByRole("button", { name: "Explore relationships", exact: true })
    .click();
  const explore = (await exploreCall).postDataJSON();
  expect(explore.center).toBe(`claim:${f.accepted.id}`);
  expect(explore.scope.selection).toEqual(f.selection);
  expect(explore.scope.manifest_revision_id).toBeNull();
  expect(explore.scope.mode).toBe("investigation");
  dialog = page.getByRole("dialog", {
    name: "Relationships for recalled evidence",
    exact: true,
  });
  await expect(dialog.getByTestId("graph-canvas")).toBeVisible();
  await dialog
    .getByRole("button", { name: "Shortest path controls", exact: true })
    .click();
  await dialog
    .getByLabel("Investigation path end", { exact: true })
    .fill(`source_version:${f.source.version.id}`);
  const pathRead = page.waitForResponse((r) =>
    r.url().endsWith(`${f.base}/graph/path`),
  );
  await dialog
    .getByRole("button", { name: "Find investigation path", exact: true })
    .click();
  expect((await (await pathRead).json()).status).toBe("path");
  await dialog
    .getByRole("button", { name: "Shortest path controls", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "Show shortest path on canvas", exact: true })
    .click();
  await expect(dialog.getByTestId("exploration-counts")).toHaveText(
    "2 displayed entities · 1 directed relationships",
  );
  await dialog.getByTestId("graph-canvas").scrollIntoViewIfNeeded();
  await dialog.screenshot({
    path: "../.cache/investigation-graph.png",
    animations: "disabled",
  });
  await dialog
    .getByRole("button", { name: "Close recalled graph", exact: true })
    .click();
  expect(recalls).toBe(1);
  for (const name of ["Investigation procedure", "Investigation handover"]) {
    await panel
      .getByTestId("recall-result")
      .filter({ hasText: name })
      .getByRole("button", { name: "Inspect claim and history", exact: true })
      .click();
    dialog = page.getByRole("dialog", {
      name: "Claim and knowledge history",
      exact: true,
    });
    await expect(dialog).toContainText(
      name === "Investigation procedure"
        ? "Use the isolated local fixture."
        : "Verify conditions before any execution.",
    );
    await expect(dialog.getByRole("button", { name: /^Execute/ })).toHaveCount(
      0,
    );
    await page.keyboard.press("Escape");
  }
  await panel.getByLabel("Recall mode", { exact: true }).click();
  await page
    .getByRole("option", { name: "Accepted and current", exact: true })
    .click();
  await submit.click();
  await expect(panel.getByTestId("recall-result")).toHaveCount(1);
  await expect(panel.getByTestId("recall-result")).toContainText(
    "Investigation Cobalt",
  );
  await panel.getByLabel("Recall mode", { exact: true }).click();
  await page
    .getByRole("option", { name: "Investigation", exact: true })
    .click();
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Investigation Orion");
  const frozenRead = page.waitForResponse(
    (r) =>
      r.url().endsWith(`${f.base}/recall`) && r.request().method() === "POST",
  );
  await submit.click();
  const frozen = await (await frozenRead).json();
  const oldStatus = await (
    await page.request.get(`${f.base}/graph?offset=0`)
  ).json();
  // Hold only the metadata observation to exercise the interval before the next
  // remote-change poll. Claim/history and mutation requests remain real.
  const statusRoute = `**${f.base}/graph?*`;
  await page.route(statusRoute, (route) => route.fulfill({ json: oldStatus }));
  await post(
    page,
    `${f.base}/claims/${f.changing.claim_id}`,
    {
      base_revision: f.changing.id,
      content: { ...f.changing.content, value: "6161" },
    },
    "PUT",
  );
  const historyRead = page.waitForRequest((r) =>
    r.url().includes(`/claims/${f.changing.claim_id}?knowledge_at=`),
  );
  await panel
    .getByTestId("recall-result")
    .filter({ hasText: "Investigation Orion" })
    .getByRole("button", { name: "Inspect claim and history", exact: true })
    .click();
  expect(
    new URL((await historyRead).url()).searchParams.get("knowledge_at"),
  ).toBe(frozen.knowledge_at);
  dialog = page.getByRole("dialog", {
    name: "Claim and knowledge history",
    exact: true,
  });
  await expect(dialog.getByTestId("claim-value")).toHaveText("6060");
  await expect(dialog).toContainText("Historical knowledge revision");
  await openDetails(dialog, "Version history");
  await dialog.getByLabel("Knowledge revision", { exact: true }).click();
  await page
    .getByRole("option", { name: "Latest knowledge", exact: true })
    .click();
  await expect(dialog.getByTestId("claim-value")).toHaveText("6161");
  await page.keyboard.press("Escape");
  const countBeforeInvalidation = recalls;
  await page.unroute(statusRoute);
  await expect(
    panel.getByText("Investigation cleared", { exact: true }),
  ).toBeVisible();
  await expect(panel.getByTestId("recall-result")).toHaveCount(0);
  expect(recalls).toBe(countBeforeInvalidation);
  await panel.getByLabel("Recall mode", { exact: true }).click();
  await page
    .getByRole("option", { name: "Qualified history", exact: true })
    .click();
  await panel
    .getByRole("button", { name: "Scope, time and exact lookup" })
    .click();
  await panel
    .getByLabel("Recall knowledge time (UTC)", { exact: true })
    .fill(frozen.knowledge_at.replace(/Z$/, "").slice(0, 19));
  await submit.click();
  await expect(panel.getByTestId("recall-result").first()).toContainText(
    "6060",
  );
  await expect(
    panel
      .getByTestId("recall-result")
      .first()
      .getByRole("button", { name: "Explore relationships", exact: true }),
  ).toBeDisabled();
  await panel
    .getByTestId("recall-result")
    .first()
    .getByRole("button", { name: "Inspect claim and history", exact: true })
    .click();
  dialog = page.getByRole("dialog", {
    name: "Claim and knowledge history",
    exact: true,
  });
  await dialog
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  await expect(
    page
      .getByRole("dialog")
      .getByRole("button", { name: "Confirm review", exact: true }),
  ).toHaveCount(0);
  await page.keyboard.press("Escape");
  await openDetails(dialog, "Version history");
  await dialog.getByLabel("Knowledge revision", { exact: true }).click();
  await page
    .getByRole("option", { name: "Latest knowledge", exact: true })
    .click();
  await expect(dialog.getByTestId("claim-value")).toHaveText("6161");
  await dialog
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  await page
    .getByRole("dialog")
    .getByLabel(/^Review reason/)
    .fill("Optional review of the retained synthetic declaration.");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Confirm review", exact: true })
    .click();
  await expect(
    panel.getByText("Investigation cleared", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(
    (
      await (
        await page.request.get(`${f.base}/claims/${f.changing.claim_id}`)
      ).json()
    ).selected.revision.review,
  ).toBe("accepted");
  expect(
    (await (await page.request.get(`${f.base}/models/usage`)).json()).total,
  ).toBe(0);
  expect(errors).toEqual([]);
});

test("bounded comparison and late recall responses do not expand copied context or restore old selection", async ({
  page,
  context,
}) => {
  test.setTimeout(100_000);
  page.setDefaultTimeout(12_000);
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const f = await setup(page, "Bounded investigation proof");
  await post(page, `${f.base}/sources/${f.source.id}/versions`, {
    title: "Updated investigation source",
    content: "REPLACEMENT_BYTES belong to a later source version.",
    retain_content: true,
    media_type: "text/plain",
    base_version: f.source.version.id,
  });
  const panel = page.getByRole("region", {
    name: "Recall memory",
    exact: true,
  });
  const submit = panel.getByRole("button", { name: "Recall", exact: true });
  await openDetails(panel, "Refine evidence search");
  await panel
    .getByRole("button", { name: "Scope, time and exact lookup" })
    .click();
  // Exercise the partial-bundle UI using a real server response with a bounded
  // agent-style request. Resource tuning is no longer a browser control.
  await page.route(`**${f.base}/recall`, async (route) => {
    const request = route.request().postDataJSON();
    const response = await route.fetch({
      postData: JSON.stringify({ ...request, limit: 1 }),
    });
    await route.fulfill({ response });
  });
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Investigation Boreal");
  const response = page.waitForResponse(
    (r) =>
      r.url().endsWith(`${f.base}/recall`) && r.request().method() === "POST",
  );
  await submit.click();
  const answer = await (await response).json();
  expect(answer.context.items).toHaveLength(1);
  await panel.getByRole("tab", { name: /^Sources/ }).click();
  await panel
    .getByRole("button", { name: "Inspect exact source", exact: true })
    .click();
  await expect(
    page.getByRole("dialog").getByTestId("claim-evidence-text"),
  ).toContainText("EXACT_OLD_EVIDENCE");
  await expect(
    page.getByRole("dialog").getByTestId("claim-evidence-text"),
  ).not.toContainText("REPLACEMENT_BYTES");
  await page.keyboard.press("Escape");
  let calls = 0;
  page.on("request", (r) => {
    if (
      r.url().includes(`${f.base}/claims/`) ||
      r.url().endsWith(`${f.base}/recall`)
    )
      calls++;
  });
  await panel.getByRole("tab", { name: /^Disagreements/ }).click();
  await expect(
    panel.getByText(/related claim is outside this bounded result set/),
  ).toBeVisible();
  expect(calls).toBe(0);
  await panel
    .getByRole("button", { name: "Inspect related claim", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Claim and knowledge history",
    exact: true,
  });
  await expect(dialog.getByTestId("claim-value")).toHaveText(
    answer.context.items[0].text.includes("Value: 8080") ? "9090" : "8080",
  );
  await page.keyboard.press("Escape");
  await panel
    .getByRole("button", { name: "Copy attributed context", exact: true })
    .click();
  expect(
    JSON.parse(await page.evaluate(() => navigator.clipboard.readText())),
  ).toEqual(answer.context);
  await page.evaluate(() =>
    Object.defineProperty(navigator.clipboard, "writeText", {
      configurable: true,
      value: () => Promise.reject(new Error("Fixture clipboard denial")),
    }),
  );
  await panel
    .getByRole("button", { name: "Context copied", exact: true })
    .click();
  await expect(panel.getByText(/Context could not be copied/)).toBeVisible();
  const missingId = answer.context.items[0].claim.conflicting_claim_ids[0];
  const deniedRoute = `**${f.base}/claims/${missingId}?*`;
  await page.route(deniedRoute, (route) =>
    route.fulfill({ status: 403, json: { message: "Claim access changed" } }),
  );
  await panel
    .getByRole("button", { name: "Inspect related claim", exact: true })
    .click();
  await expect(
    panel.getByText("Investigation cleared", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(panel.getByTestId("recall-result")).toHaveCount(0);
  await page.unroute(deniedRoute);

  const routeUrl = `**${f.base}/recall`;
  let release!: () => void;
  let prepared!: () => void;
  let finished!: () => void;
  const hold = new Promise<void>((resolve) => {
    release = resolve;
  });
  const ready = new Promise<void>((resolve) => {
    prepared = resolve;
  });
  const complete = new Promise<void>((resolve) => {
    finished = resolve;
  });
  await page.route(routeUrl, async (route) => {
    const response = await route.fetch();
    prepared();
    await hold;
    await route.fulfill({ response }).catch(() => {
      /* The original request was cancelled by the changed selection. */
    });
    finished();
  });
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Investigation Cobalt");
  await submit.click();
  await ready;
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Investigation Orion");
  release();
  await complete;
  await expect(panel.getByTestId("recall-result")).toHaveCount(0);
  await page.unroute(routeUrl);
  await submit.click();
  await expect(panel.getByTestId("recall-result")).toHaveCount(1);
  await expect(panel.getByTestId("recall-result")).toContainText(
    "Investigation Orion",
  );
  await expect(panel.getByTestId("recall-result")).not.toContainText(
    "Investigation Cobalt",
  );
  expect(
    (await (await page.request.get(`${f.base}/models/usage`)).json()).total,
  ).toBe(0);
  expect(errors).toEqual([]);
});

test("desktop investigation clears nested stale evidence and text context without resubmitting", async ({
  page,
}) => {
  test.setTimeout(150_000);
  page.setDefaultTimeout(12_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const f = await setup(page, "Investigation invalidation proof");
  const panel = page.getByRole("region", {
    name: "Recall memory",
    exact: true,
  });
  const submit = panel.getByRole("button", { name: "Recall", exact: true });
  await openDetails(panel, "Refine evidence search");
  let recalls = 0;
  page.on("request", (r) => {
    if (r.method() === "POST" && r.url().endsWith(`${f.base}/recall`))
      recalls++;
  });
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Missing supported memory");
  await submit.click();
  await panel.getByRole("tab", { name: /^Sources/ }).click();
  await expect(panel.getByTestId("recall-source")).toContainText(
    "reference only",
  );
  await panel
    .getByRole("button", { name: "Inspect exact source", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toContainText(
    "Source text is unavailable here",
  );
  await page.keyboard.press("Escape");
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Investigation source");
  await submit.click();
  await panel
    .getByRole("button", { name: "Inspect evidence 1", exact: true })
    .click();
  let dialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(dialog.getByTestId("claim-evidence-text")).toContainText(
    "EXACT_OLD_EVIDENCE",
  );
  const evidenceUrl = `**${f.base}/claim-evidence/source_version/${f.source.version.id}`;
  await page.route(evidenceUrl, (route) =>
    route.fulfill({
      status: 503,
      json: { message: "Evidence access changed" },
    }),
  );
  await expect(
    dialog.getByText("Evidence access changed", { exact: true }),
  ).toBeVisible();
  await expect(dialog.getByTestId("claim-evidence-text")).toHaveCount(0);
  const errorCount = recalls;
  await page.unroute(evidenceUrl);
  await dialog
    .getByRole("button", { name: "Refresh evidence", exact: true })
    .click();
  await expect(dialog.getByTestId("claim-evidence-text")).toBeVisible();
  await page.keyboard.press("Escape");
  expect(recalls).toBe(errorCount);
  // The canonical API deadline is proven in the platform fixture; use a short
  // advertised deadline here to verify browser clearing independently.
  let expiry: string | null = null;
  await page.route(evidenceUrl, async (route) => {
    const response = await route.fetch();
    const data = await response.json();
    expiry ??= new Date(Date.now() + 1500).toISOString();
    await route.fulfill({ response, json: { ...data, expires_at: expiry } });
  });
  await panel
    .getByRole("button", { name: "Inspect evidence 1", exact: true })
    .click();
  dialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(dialog.getByTestId("claim-evidence-text")).toBeVisible();
  await expect(
    dialog.getByText(/reached its retention deadline/),
  ).toBeVisible();
  await expect(dialog.getByTestId("claim-evidence-text")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await page.unroute(evidenceUrl);
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Investigation Boreal");
  await submit.click();
  await panel
    .getByTestId("recall-result")
    .first()
    .getByRole("button", { name: "Inspect claim and history", exact: true })
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Inspect conflicting evidence", exact: true })
    .first()
    .click();
  await expect(
    page
      .getByRole("dialog", { name: "Conflict evidence", exact: true })
      .getByText(/8080|9090/)
      .first(),
  ).toBeVisible();
  const beforeErase = recalls;
  const target = { kind: "source", id: f.source.id };
  const preview = await post(page, `${f.base}/erasures/preview`, target);
  await post(page, `${f.base}/erasures`, {
    target,
    eligibility_epoch: preview.eligibility_epoch,
  });
  await expect(
    panel.getByText("Investigation cleared", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(panel.getByTestId("recall-result")).toHaveCount(0);
  expect(recalls).toBe(beforeErase);
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("Missing supported memory");
  await submit.click();
  await expect(panel.getByTestId("recall-result")).toHaveCount(1);
  const responseRoute = `**${f.base}/recall`;
  await page.route(responseRoute, async (route) => {
    const response = await route.fetch();
    await route.fulfill({
      response,
      json: {
        ...(await response.json()),
        expires_at: new Date(Date.now() + 1500).toISOString(),
      },
    });
  });
  await submit.click();
  await expect(panel.getByTestId("recall-result")).toHaveCount(1);
  const beforeExpiry = recalls;
  await expect(
    panel.getByText("Investigation cleared", { exact: true }),
  ).toBeVisible();
  await expect(panel.getByTestId("recall-result")).toHaveCount(0);
  expect(recalls).toBe(beforeExpiry);
  await page.unroute(responseRoute);
  await panel
    .getByRole("button", { name: "Scope, time and exact lookup" })
    .click();
  await panel.getByLabel("Text search", { exact: true }).uncheck();
  await panel
    .getByLabel("Search memory", { exact: true })
    .fill("NoSuchSyntheticQuestion");
  await submit.click();
  await expect(
    panel.getByText("No matching evidence", { exact: true }),
  ).toBeVisible();
  await panel.getByLabel("Recall mode", { exact: true }).click();
  await page
    .getByRole("option", { name: "Accepted and current", exact: true })
    .click();
  await submit.click();
  await expect(
    panel.getByText("Insufficient eligible evidence", { exact: true }),
  ).toBeVisible();
  expect(
    (await (await page.request.get(`${f.base}/models/usage`)).json()).total,
  ).toBe(0);
  expect(errors).toEqual([]);
});
