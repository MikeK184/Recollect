import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import {
  brainNavigation,
  brainTiers,
  defaultKnowledgeView,
  knowledgeSections,
  resolveKnowledgeView,
} from "../src/app/navigation";

// The tiered sidebar and the Knowledge surface shell: four ordered tiers with
// visible group headings, and one surface hosting Memory, Sources, Graph and
// Repositories behind one switcher and one inspector region. Ask's own behavior
// is deliberately not exercised beyond its landing route.

const tierLinks: Record<(typeof brainTiers)[number], readonly string[]> = {
  Ask: ["Ask"],
  Knowledge: ["Memory", "Sources", "Graph", "Repositories"],
  Wiring: ["Agents", "Connections", "Settings"],
  Assurance: ["Activity"],
} as const;

const views = [
  ["memory", "Memory"],
  ["sources", "Sources"],
  ["graph", "Graph"],
  ["repositories", "Repositories"],
] as const;

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

test("the sidebar renders four ordered tiers with visible group headings", async ({
  page,
}) => {
  await signIn(page);
  const brain = await createBrain(page);
  await page.goto(`/brains/${brain}/memory`);
  expect([...brainTiers]).toEqual(["Ask", "Knowledge", "Wiring", "Assurance"]);
  await expect(brainNav(page).getByRole("group")).toHaveCount(4);
  expect(await brainNav(page).locator(".nav-label").allTextContents()).toEqual([
    ...brainTiers,
  ]);
  for (const tier of brainTiers) {
    const group = brainNav(page).getByRole("group", {
      name: tier,
      exact: true,
    });
    await expect(group.locator(".nav-label")).toBeVisible();
    expect(await group.getByRole("link").allTextContents()).toEqual([
      ...tierLinks[tier],
    ]);
  }
  // Every destination is a direct link now; the collapsed group is gone.
  await expect(brainNav(page).getByRole("link")).toHaveCount(
    brainNavigation.length,
  );
  await expect(brainNav(page).locator("details")).toHaveCount(0);
  await expect(page.getByText("Workspace & settings")).toHaveCount(0);
  // Selection state and the document title keep resolving per section.
  await expect(
    brainNav(page).getByRole("link", { name: "Memory", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await expect(page).toHaveTitle(/Memory · Recollect$/);
  await expectAccessible(page, ".workspace-nav");
  for (const label of ["Settings", "Activity"]) {
    await brainNav(page)
      .getByRole("link", { name: label, exact: true })
      .click();
    await expect(page).toHaveURL(
      new RegExp(
        `/brains/${brain}/${label === "Settings" ? "settings" : "activity"}$`,
      ),
    );
    await expect(
      brainNav(page).getByRole("link", { name: label, exact: true }),
    ).toHaveAttribute("aria-current", "page");
    await expect(
      page.getByRole("heading", { level: 1, name: label, exact: true }),
    ).toBeVisible();
    await expect(surface(page)).toHaveCount(0);
  }
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
    await expect(
      switcher(page).getByRole("link", { name: label, exact: true }),
    ).toHaveAttribute("aria-current", "page");
    await expect(
      brainNav(page).getByRole("link", { name: label, exact: true }),
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
    await expect(
      switcher(page).getByRole("link", { name: label, exact: true }),
    ).toHaveAttribute("aria-current", "page");
  }
  // The switcher walks real history, so Back and forward land on each view.
  await page.goto(`/brains/${brain}/memory`);
  for (const [, label] of views.slice(1))
    await switcher(page)
      .getByRole("link", { name: label, exact: true })
      .click();
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/repositories$`));
  for (const view of ["graph", "sources", "memory"]) {
    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/brains/${brain}/${view}$`));
    await expect(surface(page)).toHaveAttribute("data-knowledge-view", view);
  }
  for (const view of ["sources", "graph", "repositories"]) {
    await page.goForward();
    await expect(page).toHaveURL(new RegExp(`/brains/${brain}/${view}$`));
    await expect(surface(page)).toHaveAttribute("data-knowledge-view", view);
  }
  // Keyboard users reach the switcher and it performs a real route change.
  await page.goto(`/brains/${brain}/memory`);
  const graphLink = switcher(page).getByRole("link", {
    name: "Graph",
    exact: true,
  });
  await graphLink.focus();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/graph$`));
  await expect(surface(page)).toHaveAttribute("data-knowledge-view", "graph");
});

test("one switcher and one inspector region serve all four views without remounting", async ({
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
  for (const [view, label] of views.slice(1)) {
    await switcher(page)
      .getByRole("link", { name: label, exact: true })
      .click();
    await expect(surface(page)).toHaveCount(1);
    await expect(switcher(page)).toHaveCount(1);
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
  await page.goto(`/brains/${brain}/memory?tab=claim`);
  await expect(
    page.getByRole("tab", { name: "Claims", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await switcher(page)
    .getByRole("link", { name: "Sources", exact: true })
    .click();
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/sources$`));
  expect(new URL(page.url()).search).toBe("");
  await expectAccessible(page, ".knowledge-switcher");
  await expectAccessible(page, ".knowledge-inspector");
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
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/ask$`));
  await expect(surface(page)).toHaveCount(0);
});

test("the Ask landing and the global destinations keep their current behavior", async ({
  page,
}) => {
  await signIn(page);
  const brain = await createBrain(page);
  await page.goto(`/brains/${brain}`);
  await expect(page).toHaveURL(new RegExp(`/brains/${brain}/ask$`));
  await expect(
    page.getByRole("heading", { level: 1, name: /Ask your Brain/ }),
  ).toBeVisible();
  await expect(surface(page)).toHaveCount(0);
  await expect(inspector(page)).toHaveCount(0);
  await expect(
    brainNav(page).getByRole("link", { name: "Ask", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await expect(page).toHaveTitle(/Ask · Recollect$/);
  // Team, Brains and Devices stay global destinations outside the tiers.
  await page.goto("/");
  await expect(brainNav(page)).toHaveCount(0);
  const global = page.getByRole("navigation", {
    name: "Workspace navigation",
    exact: true,
  });
  await expect(global.getByRole("link")).toHaveCount(3);
  expect(
    await global
      .getByRole("link")
      .evaluateAll((nodes) =>
        nodes.map((node) => (node as HTMLAnchorElement).getAttribute("href")),
      ),
  ).toEqual(["/", "/team", "/devices"]);
  for (const [href, label] of [
    ["/team", "Team"],
    ["/devices", "Devices"],
  ]) {
    await global.locator(`a[href="${href}"]`).click();
    await expect(
      page.getByRole("heading", { level: 1, name: label, exact: true }),
    ).toBeVisible();
    await expect(surface(page)).toHaveCount(0);
  }
});
