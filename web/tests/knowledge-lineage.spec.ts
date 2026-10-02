import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

// The shared lineage inspector: one canonical definition serving all four
// Knowledge views, dense rows that keep every status dimension readable as
// text, and keyboard reachability with focus return.

const inspector = (page: Page) =>
  page.getByRole("complementary", { name: "Lineage inspector", exact: true });
const switcher = (page: Page) =>
  page.getByRole("group", { name: "Knowledge views", exact: true });

async function reviewScreenshots(page: Page, name: string) {
  for (const width of [1280, 1440, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    await page.evaluate(async () => {
      await document.fonts.ready;
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
    });
    await page.screenshot({ path: `../.cache/ui/${name}-${width}.png` });
  }
}

async function signIn(page: Page) {
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
}

/** One Brain with a retained source and an accepted claim that supports it. */
async function createFixture(page: Page) {
  return page.evaluate(async () => {
    const me = await (await fetch("/api/auth/me")).json();
    const send = async (url: string, body: unknown) => {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!response.ok)
        throw new Error(`Lineage fixture failed (${response.status}): ${url}`);
      return response.json();
    };
    const brain = await send("/api/brains", {
      name: "Lineage inspector proof",
    });
    const base = `/api/brains/${brain.id}`;
    const source = await send(`${base}/sources`, {
      title: "Lineage notes",
      media_type: "text/plain",
      content: "Lineage declares the port 8080.\n",
      retain_content: true,
      group_ids: [],
    });
    const claim = await send(`${base}/claims`, {
      content: {
        kind: "claim",
        subject: "Lineage service",
        predicate: "port",
        value: "8080",
        rationale: "Synthetic lineage fixture.",
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
            line_from: null,
            line_to: null,
          },
        ],
      },
    });
    await send(`${base}/claims/${claim.claim_id}/review`, {
      base_revision: claim.id,
      action: "accept",
      reason: "Fixture acceptance for the lineage proof.",
    });
    return { brain: brain.id, source: source.id, claim: claim.claim_id };
  });
}

async function readyKnowledgeGraph(page: Page, base: string) {
  await page.evaluate(async (base) => {
    const me = await (await fetch("/api/auth/me")).json();
    const response = await fetch(`${base}/graph/rebuild`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": me.csrf_token,
      },
      body: JSON.stringify({ kind: "knowledge" }),
    });
    if (!response.ok)
      throw new Error(`Graph rebuild failed (${response.status})`);
  }, base);
  await expect
    .poll(
      async () =>
        (
          await (await page.request.get(`${base}/graph`)).json()
        ).generations.some((g: { state: string }) => g.state === "ready"),
      { timeout: 90_000 },
    )
    .toBe(true);
}

test("one lineage inspector instance serves all four views without remounting", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await signIn(page);
  const fixture = await createFixture(page);
  await page.goto(`/brains/${fixture.brain}/memory`);
  const region = inspector(page);
  await expect(region).toHaveCount(1);
  // Mark the mounted region; a remount would drop this attribute.
  await region.evaluate((node) =>
    node.setAttribute("data-lineage-probe", "mounted"),
  );
  for (const label of ["Sources", "Graph", "Repositories"]) {
    await switcher(page)
      .getByRole("link", { name: label, exact: true })
      .click();
    await expect(region).toHaveCount(1);
    await expect(region).toHaveAttribute("data-lineage-probe", "mounted");
  }
});

test("selecting a memory shows content, evidence with exact versions and a bounded neighbourhood", async ({
  page,
}) => {
  test.setTimeout(240_000);
  await signIn(page);
  const fixture = await createFixture(page);
  const base = `/api/brains/${fixture.brain}`;
  await readyKnowledgeGraph(page, base);
  await page.goto(`/brains/${fixture.brain}/memory`);
  const region = inspector(page);
  await page
    .getByRole("button", { name: "Lineage service · port", exact: true })
    .click();
  // The first selection opens the canonical lineage inspector directly.
  await expect(
    page.getByRole("dialog", { name: "Knowledge lineage" }),
  ).toBeVisible();
  await expect(region).toContainText("Lineage service");
  await expect(
    region.getByText("Review: accepted", { exact: true }),
  ).toBeVisible();
  await expect(region.getByText(/^Freshness: /)).toBeVisible();
  await expect(region.getByText(/^Operational: /)).toBeVisible();
  await expect(region.getByText(/Knowledge time: learned/)).toBeVisible();
  await expect(region.getByText(/Provenance:/)).toBeVisible();
  // Supporting evidence names the exact source version and location.
  const evidenceButton = region.getByRole("button", {
    name: /Lineage notes · source version/,
  });
  await expect(evidenceButton).toBeVisible();
  await evidenceButton.click();
  const evidenceDialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(evidenceDialog.getByTestId("claim-evidence-text")).toContainText(
    "Lineage declares the port 8080.",
  );
  await page.keyboard.press("Escape");
  await expect(evidenceDialog).not.toBeVisible();
  // The bounded neighbourhood sits inline beneath the evidence.
  await expect(
    region.getByText("Bounded graph neighbourhood", { exact: true }),
  ).toBeVisible();
  await expect(
    region.locator(".lineage-node-button", { hasText: "Lineage notes" }),
  ).toBeVisible();
  await reviewScreenshots(page, "lineage-memory");
  const audit = await new AxeBuilder({ page })
    .include(".knowledge-inspector-region")
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  expect(audit.violations.map(({ id, impact }) => ({ id, impact }))).toEqual(
    [],
  );
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", { name: "Lineage service · port", exact: true }),
  ).toBeFocused();
  await page
    .getByRole("button", { name: "Lineage service · port", exact: true })
    .click();
  await region
    .getByRole("link", { name: "Show in Graph", exact: true })
    .click();
  await expect(page).toHaveURL(new RegExp(`/graph\\?claim=${fixture.claim}$`));
  await expect(
    region.getByText("Review: accepted", { exact: true }),
  ).toBeVisible();
  await region
    .getByRole("link", { name: "Show in Memory", exact: true })
    .click();
  await expect(page).toHaveURL(new RegExp(`/memory\\?claim=${fixture.claim}$`));
  await expect(
    region.getByText("Review: accepted", { exact: true }),
  ).toBeVisible();
});

test("selecting a source shows versions, availability and what it supports with links into derived records", async ({
  page,
}) => {
  test.setTimeout(240_000);
  await signIn(page);
  const fixture = await createFixture(page);
  const base = `/api/brains/${fixture.brain}`;
  await readyKnowledgeGraph(page, base);
  await page.goto(`/brains/${fixture.brain}/sources`);
  const region = inspector(page);
  await page
    .getByRole("button", { name: /Lineage notes/ })
    .first()
    .click();
  await expect(
    page.getByRole("dialog", { name: "Knowledge lineage" }),
  ).toBeVisible();
  // Versions and availability stay explicit.
  await expect(region).toContainText("Lineage notes");
  await expect(region.getByText(/recorded version/)).toBeVisible();
  await expect(region.getByText("Processed", { exact: true })).toBeVisible();
  await expect(region.locator(".lineage-content")).toContainText(
    "Lineage declares the port 8080.",
  );
  // What it currently supports links into each derived record.
  await expect(
    region.getByText("What this source supports", { exact: true }),
  ).toBeVisible();
  const derived = region.locator(".lineage-node-button", {
    hasText: "Lineage service",
  });
  await expect(derived).toBeVisible();
  await expect(region.getByText(/not independent support/i)).toBeVisible();
  await reviewScreenshots(page, "lineage-source");
  await region
    .getByRole("link", { name: "Show in Graph", exact: true })
    .click();
  await region
    .getByRole("button", { name: "Source history & actions", exact: true })
    .click();
  await expect(page).toHaveURL(/\/sources\?.*detail=record/);
  await expect(
    page.getByRole("dialog", { name: "Source evidence", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: /Lineage notes/ })
    .first()
    .click();
  // The link reuses the same inspector as a memory selection.
  await derived.click();
  await page.keyboard.press("Tab");
  await expect
    .poll(() => region.evaluate((el) => el.contains(document.activeElement)))
    .toBe(true);
  await expect(
    region.getByText("Review: accepted", { exact: true }),
  ).toBeVisible();
  await expect(
    region.getByText("Bounded graph neighbourhood", { exact: true }),
  ).toBeVisible();
  await region
    .getByRole("link", { name: "Show in Memory", exact: true })
    .click();
  await expect(page).toHaveURL(/\/memory\?center=claim/);
  await expect(
    region.getByText("Review: accepted", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Request failed", { exact: true }),
  ).not.toBeVisible();
});

test("dense rows keep the three status dimensions readable as text and hide machine identifiers", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await signIn(page);
  const fixture = await createFixture(page);
  await page.goto(`/brains/${fixture.brain}/memory`);
  const row = page.locator(".memory-record").first();
  // One composite status mark whose meaning is also its text.
  await expect(row.locator(".memory-record-status")).toHaveCount(1);
  await expect(
    row.locator(".memory-record-status .mantine-Badge-root"),
  ).toHaveCount(1);
  const statusText = (
    await row.locator(".memory-record-status").allInnerTexts()
  ).join(" ");
  expect(statusText).toContain("Review:");
  expect(statusText).toContain("Freshness:");
  expect(statusText).toContain("Operational:");
  // Machine identifiers stay out of list rows.
  const rows = (await page.locator(".memory-record").allInnerTexts()).join(
    "\n",
  );
  expect(rows).not.toContain(fixture.claim);
  await page.goto(`/brains/${fixture.brain}/sources`);
  const sourceRow = page.locator(".source-row").first();
  await expect(sourceRow.locator(".source-record-line")).toHaveCount(2);
  await expect(sourceRow.locator(".mantine-Badge-root")).toHaveCount(2);
  const sourceRows = (await page.locator(".source-row").allInnerTexts()).join(
    "\n",
  );
  expect(sourceRows).not.toContain(fixture.source);
});

test("keyboard users can open a memory and focus returns to its row", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await signIn(page);
  const fixture = await createFixture(page);
  await page.goto(`/brains/${fixture.brain}/memory`);
  const rowButton = page.getByRole("button", {
    name: "Lineage service · port",
    exact: true,
  });
  await rowButton.focus();
  await expect(rowButton).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("dialog", { name: "Knowledge lineage" }),
  ).toBeVisible();
  const lineage = inspector(page);
  await lineage
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  const review = page.getByRole("dialog", {
    name: "Review and corrections",
    exact: true,
  });
  await review
    .getByRole("button", { name: "Edit reviewed content", exact: true })
    .click();
  const editor = page.getByRole("dialog", {
    name: "Edit reviewed content",
    exact: true,
  });
  await editor
    .getByRole("textbox", { name: "Claim value", exact: true })
    .fill("discard this draft");
  await editor.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(review).toBeVisible();
  await review
    .getByRole("button", { name: "Edit reviewed content", exact: true })
    .click();
  await expect(
    editor.getByRole("textbox", { name: "Claim value", exact: true }),
  ).toHaveValue("8080");
  await page.keyboard.press("Escape");
  await expect(review).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(review).not.toBeVisible();
  await page.keyboard.press("Escape");
  await expect(rowButton).toBeFocused();
});

test("lineage correction loads scope, saves reviewed content and refreshes canonical memory", async ({
  page,
}) => {
  await signIn(page);
  const fixture = await createFixture(page);
  const base = `/api/brains/${fixture.brain}`;
  const session = await (await page.request.get("/api/auth/me")).json();
  const source = await page.request.post(`${base}/sources`, {
    headers: { "x-csrf-token": session.csrf_token },
    data: { title: "Corrected lineage evidence", media_type: "text/plain", content: "Lineage declares port 9090.\n", retain_content: true },
  });
  expect(source.ok()).toBe(true);
  const updatedEvidence = await source.json();
  await page.goto(`/brains/${fixture.brain}/memory?claim=${fixture.claim}`);
  await inspector(page)
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  const review = page.getByRole("dialog", {
    name: "Review and corrections",
    exact: true,
  });
  await review
    .getByRole("button", { name: "Edit reviewed content", exact: true })
    .click();
  const editor = page.getByRole("dialog", {
    name: "Edit reviewed content",
    exact: true,
  });
  await editor.getByRole("textbox", { name: "Claim value", exact: true }).fill("9090");
  await editor.getByRole("button", { name: "Remove support", exact: true }).click();
  await editor.getByLabel("Find evidence", { exact: true }).fill("Corrected lineage evidence");
  await editor.getByRole("button", { name: "Use evidence", exact: true }).click();
  await editor
    .getByRole("button", { name: "Use this content", exact: true })
    .click();
  await review
    .getByRole("textbox", { name: "Review reason", exact: true })
    .fill("Correct the port from replacement evidence.");
  await review
    .getByRole("button", { name: "Confirm review", exact: true })
    .click();
  await expect(
    review.getByRole("heading", { name: "Review recorded", exact: true }),
  ).toBeVisible();
  const current = (
    await (await page.request.get(`${base}/claims/${fixture.claim}`)).json()
  ).selected;
  expect(current.revision.content.supports[0].id).toBe(updatedEvidence.version.id);
  expect(current.revision.review).toBe("accepted");
  expect(current.revision.content.value).toBe("9090");
  expect(current.revision.content.supports).toHaveLength(1);
});
