import { test, expect, type Page } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import { openDetails } from "./desktop-helpers";
import AxeBuilder from "@axe-core/playwright";

async function expectAccessible(page: Page) {
  const audit = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  expect(
    audit.violations.map(({ id, impact, nodes }) => ({
      id,
      impact,
      targets: nodes.map((node) => node.target),
    })),
  ).toEqual([]);
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
    page.getByRole("heading", { name: "Your Brains", exact: true }),
  ).toBeVisible();
}

async function createBrains(page: Page) {
  return page.evaluate(async () => {
    const session = await (await fetch("/api/auth/me")).json();
    async function send(url: string, body: unknown) {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": session.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!response.ok)
        throw new Error(`Desktop fixture failed: ${response.status}`);
      return response.json();
    }
    const first = await send("/api/brains", {
      name: "Desktop evidence — a deliberately long Brain name for readable navigation",
      description: "Synthetic desktop acceptance fixtures.",
    });
    const second = await send("/api/brains", { name: "Other desktop Brain" });
    await send(`/api/brains/${first.id}/sources`, {
      title: "FIRST_BRAIN_PRIVATE_SOURCE",
      media_type: "text/plain",
      content: "Synthetic protected evidence for navigation isolation.",
      retain_content: true,
      source_uri: null,
      group_ids: [],
    });
    await send(`/api/brains/${second.id}/sources`, {
      title: "SECOND_BRAIN_VISIBLE_SOURCE",
      media_type: "text/plain",
      content: "Permitted control for the second Brain.",
      retain_content: true,
      source_uri: null,
      group_ids: [],
    });
    return { first: first.id as string, second: second.id as string };
  });
}

const destinations = [
  ["Ask", "ask"],
  ["Memory", "memory"],
  ["Sources", "sources"],
  ["Graph", "graph"],
  ["Repositories", "repositories"],
  ["Agents", "agents"],
  ["Connections", "connections"],
  ["Activity", "activity"],
  ["Settings", "settings"],
] as const;

test("desktop destinations are real routes with browser history and isolated feature consumers", async ({
  page,
}) => {
  test.setTimeout(90_000);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await signIn(page);
  const fixture = await createBrains(page);
  await page.goto(`/brains/${fixture.first}`);
  await expect(page).toHaveURL(new RegExp(`/brains/${fixture.first}/ask$`));
  const timings: {
    destination: string;
    elapsed_ms: number;
    requests: string[];
  }[] = [];
  for (const [label, suffix] of destinations) {
    const requests: string[] = [];
    const track = (request: import("@playwright/test").Request) => {
      const url = new URL(request.url());
      if (url.pathname.startsWith("/api/"))
        requests.push(`${request.method()} ${url.pathname}`);
    };
    page.on("request", track);
    const start = performance.now();
    await page
      .getByRole("navigation", { name: "Brain navigation" })
      .getByRole("link", { name: label, exact: true })
      .click();
    await expect(page).toHaveURL(
      new RegExp(`/brains/${fixture.first}/${suffix}$`),
    );
    await expect(
      page.getByRole("heading", {
        level: 1,
        name: label === "Ask" ? "Ask your Brain" : label,
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      page
        .getByRole("navigation", { name: "Brain navigation" })
        .getByRole("link", { name: label, exact: true }),
    ).toHaveAttribute("aria-current", "page");
    timings.push({
      destination: suffix,
      elapsed_ms: Math.round(performance.now() - start),
      requests,
    });
    page.off("request", track);
    await expectAccessible(page);
  }
  await page.goBack();
  await expect(
    page.getByRole("heading", { level: 1, name: "Activity", exact: true }),
  ).toBeVisible();
  await page.goForward();
  await expect(
    page.getByRole("heading", { level: 1, name: "Settings", exact: true }),
  ).toBeVisible();
  await page.goto(`/brains/${fixture.first}/settings?tab=privacy`);
  await expect(
    page.getByRole("tab", { name: "Retention & privacy", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.reload();
  await expect(
    page.getByRole("tab", { name: "Retention & privacy", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.goto(`/brains/${fixture.first}/settings?tab=unknown-tab`);
  await expect(
    page.getByRole("tab", { name: "General", exact: true }),
  ).toHaveAttribute("aria-selected", "true");

  await page.goto(`/brains/${fixture.first}/unknown-section`);
  await expect(page).toHaveURL(new RegExp(`/brains/${fixture.first}/ask$`));

  for (const [path, title] of [
    ["/", "Your Brains"],
    ["/team", "Team"],
    ["/devices", "Devices"],
  ]) {
    await page.goto(path);
    await expect(
      page.getByRole("heading", { level: 1, name: title, exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("navigation", { name: "Workspace navigation" }),
    ).toBeVisible();
    await expect(
      page.getByRole("navigation", { name: "Brain navigation" }),
    ).toHaveCount(0);
    await expectAccessible(page);
  }

  // A source page can read its catalogue and shared authority. It must not mount
  // a hidden graph, private tasks, capture, model usage, MCP or jobs dashboard.
  const sourceRequests: string[] = [];
  const trackSources = (request: import("@playwright/test").Request) => {
    const url = new URL(request.url());
    if (url.pathname.startsWith(`/api/brains/${fixture.first}/`))
      sourceRequests.push(url.pathname);
  };
  page.on("request", trackSources);
  await page.goto(`/brains/${fixture.first}/sources`);
  await expect(
    page.getByRole("button", { name: /FIRST_BRAIN_PRIVATE_SOURCE/ }),
  ).toBeVisible();
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await page.waitForTimeout(4200);
  page.off("request", trackSources);
  expect(sourceRequests.some((path) => path.endsWith("/evidence"))).toBe(true);
  expect(
    sourceRequests.filter((path) =>
      /\/(graph|mcp|models|capture|tasks|jobs)(\/|$)/.test(path),
    ),
  ).toEqual([]);
  await mkdir("../.cache/ui", { recursive: true });
  await writeFile(
    "../.cache/ui/desktop-navigation-measurements.json",
    JSON.stringify(
      {
        note: "Single isolated fixture run, elapsed navigation observations rather than a capacity or p95 claim.",
        timings,
        sourceRequests,
      },
      null,
      2,
    ),
  );
  expect(errors).toEqual([]);
});

test("desktop assets, keyboard navigation and layout remain consistent at supported widths", async ({
  page,
}) => {
  const external: string[] = [];
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (
      url.protocol.startsWith("http") &&
      url.origin !== new URL(process.env.RECOLLECT_UI_TEST_ORIGIN!).origin
    )
      external.push(url.origin);
  });
  await signIn(page);
  const fixture = await createBrains(page);
  await page.goto(`/brains/${fixture.first}/sources`);
  await expect(
    page.getByRole("button", { name: /FIRST_BRAIN_PRIVATE_SOURCE/ }),
  ).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
  const fonts = await page.evaluate(() => ({
    title: getComputedStyle(document.querySelector("h1")!).fontFamily,
    body: getComputedStyle(document.body).fontFamily,
    loaded: [...document.fonts]
      .filter((font) => font.status === "loaded")
      .map((font) => font.family),
  }));
  expect(fonts.title).toContain("Newsreader");
  expect(fonts.body).toContain("Manrope");
  expect(fonts.loaded.join(" ")).toContain("Newsreader");
  expect(fonts.loaded.join(" ")).toContain("Manrope");
  const logo = page
    .getByRole("link", { name: "Recollect home", exact: true })
    .locator("img");
  await expect(logo).toBeVisible();
  await expect(logo).toHaveAttribute("src", /\.svg$/);
  const logoResponse = await page.request.get(
    (await logo.getAttribute("src")) as string,
  );
  expect(logoResponse.headers()["content-type"]).toContain("image/svg+xml");
  expect(await logoResponse.text()).toContain("<svg");
  const navIcons = page
    .getByRole("navigation", { name: "Brain navigation" })
    .locator("svg");
  await expect(navIcons).toHaveCount(9);
  for (const icon of await navIcons.all())
    await expect(icon).toHaveAttribute("width", "18");
  for (const width of [1280, 1440, 1920]) {
    await page.setViewportSize({ width, height: width === 1280 ? 800 : 1080 });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page.screenshot({
      path: `../.cache/ui/desktop-sources-${width}.png`,
      animations: "disabled",
    });
  }
  await page.emulateMedia({ reducedMotion: "reduce" });
  const filterButton = page.getByRole("button", {
    name: "Filters",
    exact: true,
  });
  await filterButton.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("dialog", { name: "Filter sources", exact: true }),
  ).toBeVisible();
  const filters = page.getByRole("dialog", {
    name: "Filter sources",
    exact: true,
  });
  await expectAccessible(page);
  for (let index = 0; index < 12; index++) {
    await page.keyboard.press("Tab");
    await expect
      .poll(() =>
        filters.evaluate((node) => node.contains(document.activeElement)),
      )
      .toBe(true);
  }
  await page.keyboard.press("Escape");
  await expect(filterButton).toBeFocused();
  const memory = page
    .getByRole("navigation", { name: "Brain navigation" })
    .getByRole("link", { name: "Memory", exact: true });
  await memory.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "Memory", level: 1, exact: true }),
  ).toBeVisible();
  // Text resize is a repeatable reflow check. Native browser zoom is separately
  // inspected in CUA; this is not reported as an equivalent browser-zoom test.
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "200%";
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await expect(
    page.getByRole("button", { name: "Add memory", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "../.cache/ui/desktop-memory-text-200.png",
    animations: "disabled",
  });
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "";
  });
  expect(external).toEqual([]);
});

test("an in-flight source read cannot populate another Brain after switching", async ({
  page,
}) => {
  await signIn(page);
  const fixture = await createBrains(page);
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  let received!: () => void;
  const readStarted = new Promise<void>((resolve) => {
    received = resolve;
  });
  await page.route(
    `**/api/brains/${fixture.first}/evidence*`,
    async (route) => {
      const response = await route.fetch();
      received();
      await gate;
      await route.fulfill({ response }).catch(() => {}); // Navigation may abort it.
    },
  );
  await page.goto(`/brains/${fixture.first}/sources`);
  await readStarted;
  await page
    .getByLabel("Switch Brain", { exact: true })
    .selectOption(fixture.second);
  await expect(page).toHaveURL(new RegExp(`/brains/${fixture.second}/ask$`));
  release();
  await page
    .getByRole("navigation", { name: "Brain navigation" })
    .getByRole("link", { name: "Sources", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: /SECOND_BRAIN_VISIBLE_SOURCE/ }),
  ).toBeVisible();
  await expect(
    page.getByText("FIRST_BRAIN_PRIVATE_SOURCE", { exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
  await expect(
    page.getByText("SECOND_BRAIN_VISIBLE_SOURCE", { exact: true }),
  ).toHaveCount(0);
  await page.goBack();
  await expect(
    page.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
});

test("exact source and claim links survive reload/history and reject foreign Brain identities", async ({
  page,
}) => {
  test.setTimeout(60_000);
  await signIn(page);
  const f = await page.evaluate(async () => {
    const session = await (await fetch("/api/auth/me")).json();
    const send = async (path: string, body: unknown, method = "POST") => {
      const response = await fetch(path, {
        method,
        headers: {
          "content-type": "application/json",
          "x-csrf-token": session.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!response.ok) throw new Error(`Deep-link fixture ${response.status}`);
      return response.json();
    };
    const brain = await send("/api/brains", { name: "Exact desktop history" });
    const other = await send("/api/brains", {
      name: "Foreign desktop history",
    });
    const base = `/api/brains/${brain.id}`;
    const source = await send(`${base}/sources`, {
      title: "Original desktop source",
      media_type: "text/plain",
      content: "EXACT_OLD_DESKTOP_BYTES\n",
      retain_content: true,
    });
    await send(`${base}/sources/${source.id}/versions`, {
      title: "Updated desktop source",
      media_type: "text/plain",
      content: "CURRENT_DESKTOP_BYTES\n",
      retain_content: true,
      base_version: source.version.id,
    });
    const foreign = await send(`/api/brains/${other.id}/sources`, {
      title: "FOREIGN_DESKTOP_CANARY",
      media_type: "text/plain",
      content: "FOREIGN_DESKTOP_CANARY\n",
      retain_content: true,
    });
    const content = {
      kind: "claim",
      subject: "Exact desktop memory",
      predicate: "configuration",
      value: "ORIGINAL_DESKTOP_MEMORY",
      rationale: "Synthetic history.",
      selection: { repository_ids: [], area_ids: [], environment_id: null },
      manifest_revision_id: null,
      validity: { kind: "unknown", from: null, to: null, precision: "unknown" },
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
    };
    const claim = await send(`${base}/claims`, { content });
    await send(
      `${base}/claims/${claim.claim_id}`,
      {
        base_revision: claim.id,
        content: { ...content, value: "CURRENT_DESKTOP_MEMORY" },
      },
      "PUT",
    );
    return { brain: brain.id, source, foreign, claim };
  });
  const sources = `/brains/${f.brain}/sources`;
  const exactSource = `${sources}?source=${f.source.id}&version=${f.source.version.id}`;
  await page.goto(sources);
  await page.goto(exactSource);
  const sourceContent = page.getByTestId("source-content");
  await expect(sourceContent).toHaveText("EXACT_OLD_DESKTOP_BYTES\n");
  await page.reload();
  await expect(sourceContent).toHaveText("EXACT_OLD_DESKTOP_BYTES\n");
  await page.goBack();
  await expect(sourceContent).toHaveCount(0);
  await page.goForward();
  await expect(sourceContent).toHaveText("EXACT_OLD_DESKTOP_BYTES\n");
  await page.goto(
    `${sources}?source=${f.foreign.id}&version=${f.foreign.version.id}`,
  );
  await expect(page.getByRole("alert")).toContainText(
    /unavailable|access|found/i,
  );
  await expect(
    page.getByText("FOREIGN_DESKTOP_CANARY", { exact: false }),
  ).toHaveCount(0);
  await expect(sourceContent).toHaveCount(0);

  const params = new URLSearchParams({
    claim: f.claim.claim_id,
    revision: f.claim.id,
    knowledge: f.claim.recorded_at,
  });
  await page.goto(`/brains/${f.brain}/memory?${params}`);
  const claim = page.getByRole("dialog", {
    name: "Claim and knowledge history",
    exact: true,
  });
  await expect(claim.getByTestId("claim-value")).toHaveText(
    "ORIGINAL_DESKTOP_MEMORY",
  );
  await page.reload();
  await expect(claim.getByTestId("claim-value")).toHaveText(
    "ORIGINAL_DESKTOP_MEMORY",
  );
  await openDetails(claim, "Version history");
  await claim.getByLabel("Knowledge revision", { exact: true }).click();
  await page
    .getByRole("option", { name: "Latest knowledge", exact: true })
    .click();
  await expect(claim.getByTestId("claim-value")).toHaveText(
    "CURRENT_DESKTOP_MEMORY",
  );
  await page.goBack();
  await expect(claim.getByTestId("claim-value")).toHaveText(
    "ORIGINAL_DESKTOP_MEMORY",
  );
  await page.goto(
    `/brains/${f.brain}/memory?claim=${f.claim.claim_id}&revision=${f.foreign.version.id}&knowledge=${encodeURIComponent(f.claim.recorded_at)}`,
  );
  await expect(page.getByRole("alert")).toContainText(/revision|match/i);
  await expect(page.getByTestId("claim-value")).toHaveCount(0);
});

test("non-sensitive filters restore from direct links without guessing repository inputs", async ({
  page,
}) => {
  await signIn(page);
  const f = await createBrains(page);
  const groups = await page.evaluate(async (brain) => {
    const session = await (await fetch("/api/auth/me")).json();
    const result: Record<string, string> = {};
    for (const kind of ["collection", "environment"]) {
      const response = await fetch(`/api/brains/${brain}/evidence/groups`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-csrf-token": session.csrf_token,
        },
        body: JSON.stringify({ kind, name: `Desktop ${kind}` }),
      });
      if (!response.ok) throw new Error(`Filter fixture ${response.status}`);
      result[kind] = (await response.json()).id;
    }
    return result;
  }, f.first);
  await page.goto(`/brains/${f.first}/sources?collection=${groups.collection}`);
  await page.reload();
  await page.getByRole("button", { name: /^Filters/ }).click();
  await expect(
    page
      .getByRole("dialog", { name: "Filter sources", exact: true })
      .getByLabel("Collection", { exact: true }),
  ).toHaveValue("Desktop collection");
  await page.getByRole("button", { name: "Show sources", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "No matching sources", exact: true }),
  ).toBeVisible();
  await page.goto(`/brains/${f.first}/memory?mode=strict_accepted`);
  await page.reload();
  await page.getByRole("button", { name: /^Filters/ }).click();
  await expect(
    page
      .getByRole("dialog", { name: "Filter memory", exact: true })
      .getByLabel("Claim view", { exact: true }),
  ).toHaveValue("Strict accepted");
  await page.keyboard.press("Escape");
  const actions: string[] = [];
  page.on("request", (request) => {
    if (
      request.method() === "POST" &&
      /\/(graph\/(rebuild|analytics)|answer-requests|models\/check)$/.test(
        new URL(request.url()).pathname,
      )
    )
      actions.push(request.url());
  });
  await page.goto(
    `/brains/${f.first}/graph?kind=combined&environment=${groups.environment}`,
  );
  await page.reload();
  await expect(
    page.getByRole("radio", { name: "Combined", exact: true }),
  ).toBeChecked();
  await expect(
    page.getByRole("heading", {
      name: "Choose an exact graph view",
      exact: true,
    }),
  ).toBeVisible();
  expect(actions).toEqual([]);
});

test("repeated local navigation and canonical recall observations record sample counts and request volume", async ({
  page,
}) => {
  test.setTimeout(150_000);
  await signIn(page);
  const fixture = await createBrains(page);
  await page.goto(`/brains/${fixture.first}/settings`);
  const measurements: Record<
    string,
    { milliseconds: number[]; requests: number[] }
  > = {};
  for (let sample = 0; sample < 20; sample++) {
    for (const [label, suffix] of destinations) {
      let requests = 0;
      const track = (request: import("@playwright/test").Request) => {
        if (new URL(request.url()).pathname.startsWith("/api/")) requests++;
      };
      page.on("request", track);
      const start = performance.now();
      await page
        .getByRole("navigation", { name: "Brain navigation" })
        .getByRole("link", { name: label, exact: true })
        .click();
      await expect(
        page.getByRole("heading", {
          level: 1,
          name: label === "Ask" ? "Ask your Brain" : label,
          exact: true,
        }),
      ).toBeVisible();
      const elapsed = performance.now() - start;
      page.off("request", track);
      const metric = (measurements[suffix] ??= {
        milliseconds: [],
        requests: [],
      });
      metric.milliseconds.push(Math.round(elapsed * 10) / 10);
      metric.requests.push(requests);
    }
  }
  const recall = await page.evaluate(async (brain) => {
    const session = await (await fetch("/api/auth/me")).json();
    const headers = {
      "content-type": "application/json",
      "x-csrf-token": session.csrf_token,
    };
    const send = async (suffix: string, body: unknown) => {
      const response = await fetch(`/api/brains/${brain}/${suffix}`, {
        method: "POST",
        headers,
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        const failure = await response.json();
        throw Error(
          `Measurement fixture ${suffix}: ${response.status} ${failure.code}`,
        );
      }
      return response.json();
    };
    const source = await send("sources", {
      title: "Desktop measurement evidence",
      media_type: "text/plain",
      content: "DesktopMeasurement declares port 8181.\n",
      retain_content: true,
    });
    await send("claims", {
      content: {
        kind: "claim",
        subject: "DesktopMeasurement",
        predicate: "port",
        value: "8181",
        rationale: "Synthetic bounded measurement.",
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
    const milliseconds: number[] = [];
    for (let sample = 0; sample < 20; sample++) {
      const start = performance.now();
      const result = await send("recall", {
        query: "DesktopMeasurement",
        channels: ["exact", "lexical"],
      });
      if (!result.context.items.length)
        throw Error("Measurement must have useful canonical evidence");
      milliseconds.push(Math.round((performance.now() - start) * 10) / 10);
    }
    return milliseconds;
  }, fixture.first);
  const p95 = (values: number[]) =>
    [...values].sort((left, right) => left - right)[
      Math.ceil(values.length * 0.95) - 1
    ];
  await mkdir("../.cache/ui", { recursive: true });
  await writeFile(
    "../.cache/ui/desktop-repeated-measurements.json",
    JSON.stringify(
      {
        note: "20 local samples per route from sidebar click to visible route heading; API request counts stop at that boundary and do not claim settled backend data. No pre-redesign baseline, capacity or provider-answer performance claim. Recall measures completed canonical exact/lexical requests with useful synthetic evidence and no provider calls.",
        navigation: Object.fromEntries(
          Object.entries(measurements).map(([name, metric]) => [
            name,
            {
              sample_count: metric.milliseconds.length,
              p95_ms: p95(metric.milliseconds),
              ...metric,
            },
          ]),
        ),
        recall: {
          sample_count: recall.length,
          p95_ms: p95(recall),
          milliseconds: recall,
        },
      },
      null,
      2,
    ),
  );
});

test("authentication, font fallback and archived or unavailable Brain states remain accessible", async ({
  page,
}) => {
  test.setTimeout(90_000);
  let blockedFonts = 0;
  await page.route("**/fonts/**", async (route) => {
    blockedFonts++;
    await route.abort("failed");
  });
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Welcome back", exact: true }),
  ).toBeVisible();
  await expectAccessible(page);
  await signIn(page);
  const f = await createBrains(page);
  await page.goto(`/brains/${f.first}/sources`);
  await expect(
    page.getByRole("button", { name: /FIRST_BRAIN_PRIVATE_SOURCE/ }),
  ).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
  expect(blockedFonts).toBeGreaterThan(0);
  expect(
    await page.evaluate(
      () =>
        [...document.fonts].filter(
          (font) =>
            font.family.includes("Newsreader") && font.status === "loaded",
        ).length,
    ),
  ).toBe(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "../.cache/ui/desktop-font-fallback.png",
    animations: "disabled",
  });
  await expectAccessible(page);
  await page.evaluate(async (brain) => {
    const session = await (await fetch("/api/auth/me")).json();
    const response = await fetch(`/api/brains/${brain}`, {
      method: "PATCH",
      headers: {
        "content-type": "application/json",
        "x-csrf-token": session.csrf_token,
      },
      body: JSON.stringify({ archived: true }),
    });
    if (!response.ok) throw Error(`Archive fixture ${response.status}`);
  }, f.first);
  await page.goto(`/brains/${f.first}/ask`);
  await expect(
    page.getByText("This Brain is archived", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Ask Brain", exact: true }),
  ).toBeDisabled();
  await expectAccessible(page);
  await page.goto(`/brains/${crypto.randomUUID()}/sources`);
  await expect(page.getByRole("alert").first()).toBeVisible();
  await expect(
    page.getByText("FIRST_BRAIN_PRIVATE_SOURCE", { exact: false }),
  ).toHaveCount(0);
  await expectAccessible(page);
});
