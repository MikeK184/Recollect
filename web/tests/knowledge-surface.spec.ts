import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import {
  brainNavigation,
  brainTiers,
  defaultKnowledgeView,
  knowledgeSections,
  resolveKnowledgeView,
} from "../src/app/navigation";

// The sidebar and the Knowledge surface shell under the October 3 graph-led
// workspace amendment: the Brain tier (Dashboard, Ask, Graph, Explore) and the
// Manage disclosure (Agents, Connections, Settings). One surface hosts Memory,
// Sources and Repositories behind Explore's one switcher, and Graph as its own
// sidebar route, with one inspector region throughout. Ask's own behavior is
// deliberately not exercised beyond its landing route.

const tierLinks: Record<(typeof brainTiers)[number], readonly string[]> = {
  Brain: ["Dashboard", "Ask", "Graph", "Explore"],
  Manage: ["Agents", "Connections", "Settings"],
} as const;

const views = [
  ["memory", "Memory"],
  ["sources", "Sources"],
  ["graph", "Graph"],
  ["repositories", "Repositories"],
] as const;
// Legacy Knowledge routes select Explore in the sidebar; Graph selects itself.
const sidebarLabel = (view: string) => (view === "graph" ? "Graph" : "Explore");
const switched = views.filter(([view]) => view !== "graph");

const brainNav = (page: Page) =>
  page.getByRole("navigation", { name: "Brain navigation", exact: true });
const surface = (page: Page) => page.locator(".knowledge-surface");
const switcher = (page: Page) =>
  page.getByRole("group", { name: "Knowledge views", exact: true });
const inspector = (page: Page) =>
  page.getByRole("complementary", { name: "Lineage inspector", exact: true });

async function expectAccessible(page: Page, context: string) {
  const audit = await new AxeBuilder({ page })
    .include(context)
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  expect(
    audit.violations.map(({ id, impact, nodes }) => ({
      id,
      impact,
      context,
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

async function createBrain(page: Page) {
  return page.evaluate(async () => {
    const session = await (await fetch("/api/auth/me")).json();
    const response = await fetch("/api/brains", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": session.csrf_token,
      },
      body: JSON.stringify({
        name: "Knowledge surface fixture",
        description: "Synthetic fixture for the tiered sidebar and surface.",
      }),
    });
    if (!response.ok)
      throw new Error(`Knowledge surface fixture failed: ${response.status}`);
    return (await response.json()).id as string;
  });
}

test("the sidebar renders the Brain tier and the Manage disclosure in order", async ({
  page,
}) => {
  await signIn(page);
  const brain = await createBrain(page);
  await page.goto(`/brains/${brain}/memory`);
  expect([...brainTiers]).toEqual(["Brain", "Manage"]);
  await expect(brainNav(page).getByRole("group")).toHaveCount(2);
  for (const tier of brainTiers) {
    const group = brainNav(page).getByRole("group", {
      name: tier,
      exact: true,
    });
    await expect(group).toHaveCount(1);
    expect(await group.getByRole("link").allTextContents()).toEqual([
      ...tierLinks[tier],
    ]);
  }
  // Manage is one visible, open disclosure; every destination is a direct link.
  const manage = brainNav(page).locator("details");
  await expect(manage).toHaveCount(1);
  await expect(manage).toHaveAttribute("open", "");
  await expect(manage.locator("summary")).toHaveText("Manage");
  await expect(manage.locator("summary")).toBeVisible();
  await expect(brainNav(page).getByRole("link")).toHaveCount(
    brainNavigation.filter((item) => !item.hidden).length,
  );
  // A legacy Knowledge route selects Explore; the document title still names
  // the section.
  await expect(
    brainNav(page).getByRole("link", { name: "Explore", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await expect(page).toHaveTitle(/Memory · Recollect$/);
  await expectAccessible(page, 'nav[aria-label="Brain navigation"]');
  await brainNav(page)
    .getByRole("link", { name: "Settings", exact: true })
    .click();
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/settings$`));
  await expect(
    brainNav(page).getByRole("link", { name: "Settings", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await expect(
    page.getByRole("heading", { level: 1, name: "Settings", exact: true }),
  ).toBeVisible();
  await expect(surface(page)).toHaveCount(0);
  // Dashboard exposes Activity, which is no peer sidebar destination.
  await brainNav(page)
    .getByRole("link", { name: "Dashboard", exact: true })
    .click();
  await page.getByRole("link", { name: "Activity details", exact: true }).click();
  await expect(page).toHaveURL(
    new RegExp(`/brains/${brain}/activity\\?tab=attention$`),
  );
  await expect(
    page.getByRole("heading", { level: 1, name: "Activity", exact: true }),
  ).toBeVisible();
  await expect(surface(page)).toHaveCount(0);
});

test("each Knowledge route deep-links, reloads and survives Back and forward into the right view", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await signIn(page);
  const brain = await createBrain(page);
  for (const [view, label] of views) {
    await page.goto(`/brains/${brain}/${view}`);
    await expect(page).toHaveURL(new RegExp(`/brains/${brain}/${view}$`));
    await expect(surface(page)).toHaveAttribute("data-knowledge-view", view);
    await expect(
      surface(page).getByRole("heading", {
        level: 1,
        name: label,
        exact: true,
      }),
    ).toBeVisible();
    // Graph's canvas has no view switcher stacked above it.
    if (view === "graph") await expect(switcher(page)).toHaveCount(0);
    else
      await expect(
        switcher(page).getByRole("link", { name: label, exact: true }),
      ).toHaveAttribute("aria-current", "page");
    await expect(
      brainNav(page).getByRole("link", {
        name: sidebarLabel(view),
        exact: true,
      }),
    ).toHaveAttribute("aria-current", "page");
    await expect(inspector(page)).toHaveCount(1);
    await page.reload();
    await expect(page).toHaveURL(new RegExp(`/brains/${brain}/${view}$`));
    await expect(surface(page)).toHaveAttribute("data-knowledge-view", view);
    await expect(
      surface(page).getByRole("heading", {
        level: 1,
        name: label,
        exact: true,
      }),
    ).toBeVisible();
    if (view !== "graph")
      await expect(
        switcher(page).getByRole("link", { name: label, exact: true }),
      ).toHaveAttribute("aria-current", "page");
  }
  // The switcher and the sidebar walk real history, so Back and forward land
  // on each view.
  await page.goto(`/brains/${brain}/memory`);
  for (const [, label] of switched.slice(1))
    await switcher(page)
      .getByRole("link", { name: label, exact: true })
      .click();
  await brainNav(page).getByRole("link", { name: "Graph", exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/graph$`));
  for (const view of ["repositories", "sources", "memory"]) {
    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/brains/${brain}/${view}$`));
    await expect(surface(page)).toHaveAttribute("data-knowledge-view", view);
  }
  for (const view of ["sources", "repositories", "graph"]) {
    await page.goForward();
    await expect(page).toHaveURL(new RegExp(`/brains/${brain}/${view}$`));
    await expect(surface(page)).toHaveAttribute("data-knowledge-view", view);
  }
  // Keyboard users reach the switcher and it performs a real route change.
  await page.goto(`/brains/${brain}/memory`);
  const sourcesLink = switcher(page).getByRole("link", {
    name: "Sources",
    exact: true,
  });
  await sourcesLink.focus();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/sources$`));
  await expect(surface(page)).toHaveAttribute("data-knowledge-view", "sources");
});

test("one switcher and one inspector region serve the Knowledge views without remounting", async ({
  page,
}) => {
  await signIn(page);
  const brain = await createBrain(page);
  await page.goto(`/brains/${brain}/memory`);
  // Mark the mounted shell; a remount would drop these attributes.
  await surface(page).evaluate((node) =>
    node.setAttribute("data-shell-probe", "mounted"),
  );
  await inspector(page).evaluate((node) =>
    node.setAttribute("data-inspector-probe", "mounted"),
  );
  // Sources and Repositories switch in Explore; Graph is a sidebar route on the
  // same surface, visited last because its canvas has no switcher.
  for (const [view, label] of [...switched.slice(1), views[2]]) {
    if (view === "graph")
      await brainNav(page)
        .getByRole("link", { name: label, exact: true })
        .click();
    else
      await switcher(page)
        .getByRole("link", { name: label, exact: true })
        .click();
    await expect(surface(page)).toHaveCount(1);
    await expect(switcher(page)).toHaveCount(view === "graph" ? 0 : 1);
    await expect(inspector(page)).toHaveCount(1);
    await expect(surface(page)).toHaveAttribute("data-shell-probe", "mounted");
    await expect(inspector(page)).toHaveAttribute(
      "data-inspector-probe",
      "mounted",
    );
    await expect(surface(page)).toHaveAttribute("data-knowledge-view", view);
    await expect(inspector(page)).toHaveAttribute("data-knowledge-view", view);
  }
  // A view's own validated URL state is never substituted into another view.
  // The older `tab=claim` link resolves to Memory's kind filter.
  await page.goto(`/brains/${brain}/memory?tab=claim`);
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/memory\\?kind=claim$`));
  await switcher(page)
    .getByRole("link", { name: "Sources", exact: true })
    .click();
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/sources$`));
  expect(new URL(page.url()).search).toBe("");
  await expectAccessible(page, ".knowledge-switcher");
  await expectAccessible(page, ".knowledge-inspector-region");
});

test("an unknown view value falls back to the documented default", async ({
  page,
}) => {
  // One validation point for the surface's view value.
  expect([...knowledgeSections]).toEqual([
    "memory",
    "sources",
    "graph",
    "repositories",
  ]);
  for (const view of knowledgeSections)
    expect(resolveKnowledgeView(view)).toBe(view);
  for (const unknown of ["nope", "", "MEMORY", "memories", "ask"])
    expect(resolveKnowledgeView(unknown)).toBe(defaultKnowledgeView);
  expect(defaultKnowledgeView).toBe("memory");
  await signIn(page);
  const brain = await createBrain(page);
  // A stray view value in the query cannot hijack the path-selected view.
  await page.goto(`/brains/${brain}/memory?view=nope`);
  await expect(surface(page)).toHaveAttribute(
    "data-knowledge-view",
    defaultKnowledgeView,
  );
  // An unknown Brain section keeps its documented landing instead of a blank surface.
  await page.goto(`/brains/${brain}/not-a-view`);
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/dashboard$`));
  await expect(surface(page)).toHaveCount(0);
});

test("the Dashboard landing and the global destinations resolve", async ({
  page,
}) => {
  await signIn(page);
  const brain = await createBrain(page);
  await page.goto(`/brains/${brain}`);
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/dashboard$`));
  await expect(
    page.getByRole("heading", { level: 1, name: /Dashboard/ }),
  ).toBeVisible();
  await expect(surface(page)).toHaveCount(0);
  await expect(inspector(page)).toHaveCount(0);
  await expect(
    brainNav(page).getByRole("link", { name: "Dashboard", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await expect(page).toHaveTitle(/Dashboard · Recollect$/);
  // Brains, My agents and the owner's Connectors stay global workspace
  // destinations outside the tiers; Team is the owner's persistent bottom item.
  await page.goto("/");
  await expect(brainNav(page)).toHaveCount(0);
  const global = page.getByRole("navigation", {
    name: "Workspace navigation",
    exact: true,
  });
  const installation = page.getByRole("navigation", {
    name: "Installation navigation",
    exact: true,
  });
  await expect(global.getByRole("link")).toHaveCount(3);
  const hrefs = (links: import("@playwright/test").Locator) =>
    links.evaluateAll((nodes) =>
      nodes.map((node) => (node as HTMLAnchorElement).getAttribute("href")),
    );
  expect(await hrefs(global.getByRole("link"))).toEqual([
    "/",
    "/agents",
    "/connectors",
  ]);
  expect(await hrefs(installation.getByRole("link"))).toEqual(["/team"]);
  for (const [nav, href, label] of [
    [installation, "/team", "Team"],
    [global, "/agents", "Agents"],
    [global, "/connectors", "Connectors"],
  ] as const) {
    await nav.locator(`a[href="${href}"]`).click();
    await expect(
      page.getByRole("heading", { level: 1, name: label, exact: true }),
    ).toBeVisible();
    await expect(surface(page)).toHaveCount(0);
  }
});
