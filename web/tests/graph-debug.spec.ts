import { test, expect } from "@playwright/test";

// Temporary debug probe for the entity search keyboard flow. Delete after use.
test("debug entity select enter", async ({ page }) => {
  test.setTimeout(180_000);
  page.setDefaultTimeout(12_000);
  await page.goto("/");
  await page.getByLabel(/^Username/).fill(process.env.RECOLLECT_OWNER_USERNAME!);
  await page.getByLabel(/^Password/).fill(process.env.RECOLLECT_OWNER_PASSWORD!);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Create Brain", exact: true }),
  ).toBeVisible();
  const fixture = await page.evaluate(async () => {
    const me = await (await fetch("/api/auth/me")).json();
    const post = async (path: string, body: unknown) => {
      const r = await fetch(path, {
        method: "POST",
        headers: { "content-type": "application/json", "x-csrf-token": me.csrf_token },
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error(`fixture ${r.status}`);
      return r.json();
    };
    const brain = await post("/api/brains", { name: "Debug graph select" });
    const source = await post(`/api/brains/${brain.id}/sources`, {
      title: "Amber graph evidence",
      media_type: "text/plain",
      retain_content: true,
      content: "Amber port 8080.\n",
    });
    await post(`/api/brains/${brain.id}/claims`, {
      content: {
        kind: "claim",
        subject: "Amber graph service",
        predicate: "port",
        value: "8080",
        rationale: "Debug.",
        selection: { repository_ids: [], area_ids: [], environment_id: null },
        manifest_revision_id: null,
        validity: { kind: "unknown", from: null, to: null, precision: "unknown" },
        freshness: "current",
        operational: "declared",
        observed_at: null,
        observation: "",
        supports: [{ kind: "source_version", id: source.version.id, line_from: 1, line_to: 1 }],
      },
    });
    return { brain: brain.id, source: source.id, version: source.version.id };
  });
  await expect
    .poll(async () =>
      (
        await (
          await page.request.get(
            `/api/brains/${fixture.brain}/sources/${fixture.source}/versions/${fixture.version}`,
          )
        ).json()
      ).version.processing,
    )
    .toBe("ready");
  await page.goto(`/brains/${fixture.brain}/graph`);
  const panel = page.locator("body");
  // Rebuild a ready generation.
  const drawer = page.getByRole("dialog", { name: "Graph status and maintenance", exact: true });
  await page.getByRole("button", { name: "Graph status", exact: true }).click();
  await expect(drawer).toBeVisible();
  await panel.getByRole("button", { name: "Rebuild graph", exact: true }).click();
  await expect(panel.getByText("Graph rebuild queued. Progress appears below.")).toBeVisible();
  await expect
    .poll(async () =>
      (await (await page.request.get(`/api/brains/${fixture.brain}/graph`)).json())
        .generations.some((g: { state: string }) => g.state === "ready"),
    )
    .toBe(true);
  await page.keyboard.press("Escape");
  // Load the view.
  const filters = page.getByRole("dialog", { name: "Graph filters", exact: true });
  await page.getByRole("button", { name: "Filters", exact: true }).click();
  await expect(filters).toBeVisible();
  await filters.getByRole("button", { name: "Apply graph filters", exact: true }).click();
  const canvas = page.getByTestId("graph-canvas");
  await expect(canvas, "canvas ready").toHaveAttribute(
    "data-ready",
    "true",
    { timeout: 60_000 },
  );

  // Mirror graph.spec.ts pre-loop state: pointer selection on the canvas.
  const explorer = page.locator("body");
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
  await expect(explorer.getByTestId("exploration-node")).toContainText(point.label);
  await page.keyboard.press("Escape");

  await page.evaluate(() => {
    window.__trace = [] as string[];
    const t0 = performance.now();
    const ts = () => Math.round(performance.now() - t0);
    const log = (msg: string) => window.__trace.push(`${ts()} ${msg}`);
    document.addEventListener(
      "keydown",
      (e) => {
        if (e.code === "Enter")
          log(`enter on ${(e.target as HTMLElement).getAttribute("aria-label")}`);
      },
      true,
    );
    const origClick = Element.prototype.click;
    Element.prototype.click = function (this: Element) {
      log(
        `click <${this.tagName.toLowerCase()}> id=${this.id} opt=${this.hasAttribute("data-combobox-option")} txt=${(this.textContent ?? "").slice(0, 30)}`,
      );
      return origClick.call(this);
    };
    const origQSA = Document.prototype.querySelectorAll;
    Document.prototype.querySelectorAll = function (this: Document, selector: string) {
      const result = origQSA.call(this, selector);
      if (selector.includes("data-combobox-option"))
        log(`qsa ${selector} -> ${result.length}`);
      return result;
    };
    document.addEventListener(
      "focusin",
      (e) => {
        const el = e.target as HTMLElement;
        if (el.tagName === "INPUT")
          log(`focus ${el.getAttribute("aria-label") ?? "search?"}`);
      },
      true,
    );
    document.addEventListener(
      "focusout",
      (e) => {
        const el = e.target as HTMLElement;
        if (el.tagName === "INPUT")
          log(`blur ${el.getAttribute("aria-label") ?? "search?"}`);
      },
      true,
    );
    const target = document.querySelector(
      'input[aria-label="Inspect graph entity"]',
    ) as HTMLElement | null;
    if (target) {
      new MutationObserver(() => {
        log(`expanded=${target.getAttribute("data-expanded") ?? "off"}`);
      }).observe(target, { attributes: true, attributeFilter: ["data-expanded"] });
    }
  });
  const selection = page.getByRole("textbox", { name: "Inspect graph entity", exact: true });
  for (let sample = 0; sample < 20; sample++) {
    await selection.click();
    await selection.fill("Amber graph service");
    await expect(
      page.getByRole("option", { name: /^Amber graph service ·/ }),
    ).toBeVisible();
    await selection.press("ArrowDown");
    await page.evaluate(() => {
      const input = document.activeElement as HTMLElement | null;
      if (input) (input as unknown as Record<string, number>).__marker = 42;
    });
    const preEnter = await page.evaluate(() => {
      const input = document.activeElement as HTMLElement | null;
      const listId = input?.getAttribute("aria-controls");
      const items = listId
        ? [...document.querySelectorAll(`#${CSS.escape(listId)} [data-combobox-option]`)]
        : [];
      return {
        active: input?.tagName ?? "none",
        isSelectInput: input?.getAttribute("aria-label") === "Inspect graph entity",
        expanded: input?.getAttribute("data-expanded") ?? "absent",
        listId,
        options: items.length,
        selectedIdx: items.findIndex((o) => o.hasAttribute("aria-selected")),
      };
    });
    console.info(`sample ${sample} preEnter=`, JSON.stringify(preEnter));
    await selection.press("Enter");
    try {
      await expect(explorer.getByTestId("exploration-node"), `sample ${sample}`).toContainText(
        "Amber graph service",
        { timeout: 3000 },
      );
    } catch (error) {
      const trace = await page.evaluate(() => window.__trace.splice(-14));
      const post = await page.evaluate((listId) => {
        const list = document.getElementById(listId);
        const input = document.querySelector(
          'input[aria-label="Inspect graph entity"]',
        ) as HTMLElement | null;
        return {
          listExists: !!list,
          optionsInList: list
            ? list.querySelectorAll("[data-combobox-option]").length
            : -1,
          active: (document.activeElement as HTMLElement)?.tagName ?? "none",
          inputConnected: input?.isConnected ?? false,
          inputControls: input?.getAttribute("aria-controls") ?? "none",
          remounted: input?.getAttribute("aria-controls") !== listId,
          markerLost:
            (document.activeElement as unknown as Record<string, number> | null)
              ?.__marker !== 42,
        };
      }, preEnter.listId!);
      console.info(
        `sample ${sample} FAILED`,
        "trace=",
        JSON.stringify(trace),
        "post=",
        JSON.stringify(post),
      );
      throw error;
    }
    await page.keyboard.press("Escape");
    await expect(
      page.getByRole("dialog", { name: "Graph entity", exact: true }),
    ).not.toBeVisible();
  }
});
