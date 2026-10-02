import { test, expect, type Page } from "@playwright/test";
import { randomUUID } from "node:crypto";
import type { components } from "../src/api-schema";
import { openDetails } from "./desktop-helpers";

type Device = components["schemas"]["Device"];
type BrainRecord = components["schemas"]["Brain"];

const hour = 3_600_000;
const day = 86_400_000;
const offset = (milliseconds: number) =>
  new Date(Date.now() + milliseconds).toISOString();

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

/** Authorized reads and writes only, through the browser's own session. */
async function api<T>(
  page: Page,
  path: string,
  method = "GET",
  data?: unknown,
): Promise<T> {
  const session = (await (await page.request.get("/api/auth/me")).json()) as {
    csrf_token: string;
    user: { id: string };
  };
  const response = await page.request.fetch(path, {
    method,
    data,
    headers: { "x-csrf-token": session.csrf_token },
  });
  if (!response.ok())
    throw new Error(
      `List-hygiene fixture ${method} ${path}: ${response.status()}`,
    );
  return (await response.json()) as T;
}

/** Records the id handed to the clipboard without asking for clipboard access. */
async function recordCopies(page: Page) {
  await page.addInitScript(() => {
    const store = window as unknown as { __copied: string[] };
    store.__copied = [];
    const clipboard = {
      writeText: (value: string) => {
        store.__copied.push(String(value));
        return Promise.resolve();
      },
    };
    Object.defineProperty(Navigator.prototype, "clipboard", {
      configurable: true,
      get: () => clipboard,
    });
  });
  return async () =>
    page.evaluate(
      () => (window as unknown as { __copied?: string[] }).__copied ?? [],
    );
}

const rowText = (page: Page, testId: string) =>
  page.getByTestId(testId).allInnerTexts();

/** Mantine renders both an input and a listbox per Select, so address the
 * input by role rather than by its accessible name alone. */
async function pickView(page: Page, label: RegExp) {
  const input = page.getByRole("textbox", { name: "Brain view" });
  await input.click();
  await page.getByRole("option", { name: label }).click();
}

test("device list orders active records first and collapses revoked or expired history", async ({
  page,
}) => {
  test.setTimeout(90_000);
  page.setDefaultTimeout(10_000);
  const copies = await recordCopies(page);
  const ids = {
    revoked: randomUUID(),
    expired: randomUUID(),
    waiting: randomUUID(),
    active: randomUUID(),
  };
  // Deliberately unsorted on the wire so the rendered order is the list's own.
  const records: Device[] = [
    {
      id: ids.revoked,
      name: "Retired workstation",
      claimed: true,
      created_at: offset(-30 * day),
      expires_at: offset(-day),
      last_used_at: offset(-2 * day),
      revoked_at: offset(-2 * day),
    },
    {
      id: ids.expired,
      name: "Lapsed laptop",
      claimed: true,
      created_at: offset(-20 * day),
      expires_at: offset(-hour),
      last_used_at: null,
    },
    {
      id: ids.waiting,
      name: "Approved desktop",
      claimed: false,
      created_at: offset(-hour),
      expires_at: offset(hour),
      last_used_at: null,
    },
    {
      id: ids.active,
      name: "Development laptop",
      claimed: true,
      created_at: offset(-7 * day),
      expires_at: offset(7 * day),
      last_used_at: offset(-hour),
    },
  ];
  let fixture = records;
  await signIn(page);
  await page.route("**/api/devices", (route) =>
    route.request().method() === "GET"
      ? route.fulfill({ json: fixture })
      : route.fallback(),
  );
  await page.goto("/devices");
  // History is collapsed: only the two records that can still act are listed,
  // active first.
  await expect(page.getByTestId("device-card")).toHaveCount(2);
  expect(await rowText(page, "device-row-name")).toEqual([
    "Development laptop",
    "Approved desktop",
  ]);
  await expect(
    page.getByRole("heading", { name: "No devices paired yet" }),
  ).toHaveCount(0);
  await expect(page.getByText(/records? are collapsed/)).toBeVisible();

  // The filter states its own meaning as text and is a native checkbox, so a
  // keyboard user can focus it and toggle it.
  const filter = page.getByRole("checkbox", {
    name: /Show revoked and expired devices/,
  });
  await expect(filter).not.toBeChecked();
  // Reach the history filter through the actual keyboard tab order.
  for (let i = 0; i < 40; i++) {
    if (await filter.evaluate((el) => el === document.activeElement)) break;
    await page.keyboard.press("Tab");
  }
  await expect(filter).toBeFocused();
  await page.keyboard.press("Space");
  await expect(filter).toBeChecked();
  await expect(page.getByTestId("device-card")).toHaveCount(4);
  expect(await rowText(page, "device-row-name")).toEqual([
    "Development laptop",
    "Approved desktop",
    "Lapsed laptop",
    "Retired workstation",
  ]);
  // Every state reads as text beside its mark, so no distinction is color-only.
  for (const [name, status] of [
    ["Development laptop", "Active"],
    ["Approved desktop", "Waiting for companion"],
    ["Lapsed laptop", "Expired"],
    ["Retired workstation", "Revoked"],
  ]) {
    await expect(
      page
        .getByTestId("device-card")
        .filter({ has: page.getByText(name, { exact: true }) })
        .getByText(status, { exact: true }),
    ).toBeVisible();
  }

  // Machine identifiers never occupy a row; the id is a copyable field.
  const rows = (await rowText(page, "device-card")).join("\n");
  for (const id of Object.values(ids)) {
    expect(rows).not.toContain(id);
    expect(rows).not.toContain(id.slice(0, 13));
  }
  const active = page
    .getByTestId("device-card")
    .filter({ has: page.getByText("Development laptop", { exact: true }) });
  await openDetails(active, "Device identifier");
  await active.getByRole("button", { name: "Copy device ID" }).click();
  await expect(active.getByRole("button", { name: "Copied" })).toBeVisible();
  expect(await copies()).toContain(ids.active);

  // Filtered-empty and genuinely-empty stay distinct from a populated list.
  fixture = records.slice(0, 2);
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "No device is connected right now" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "No devices paired yet" }),
  ).toHaveCount(0);
  await expect(page.getByText(/records? are collapsed/)).toBeVisible();
  fixture = [];
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "No devices paired yet" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "No device is connected right now" }),
  ).toHaveCount(0);
  await expect(page.getByText(/records? are collapsed/)).toHaveCount(0);
});

test("brain list keeps archived history behind its filter and separates the three empty states", async ({
  page,
}) => {
  test.setTimeout(120_000);
  page.setDefaultTimeout(10_000);
  const copies = await recordCopies(page);
  const suffix = randomUUID().slice(0, 8);
  await signIn(page);
  const me = (await api<{ user: { id: string } }>(page, "/api/auth/me")).user
    .id;
  const live = await api<BrainRecord>(page, "/api/brains", "POST", {
    name: `Hygiene live ${suffix}`,
    description: "Errands and notes",
  });
  const retired = await api<BrainRecord>(page, "/api/brains", "POST", {
    name: `Hygiene retired ${suffix}`,
    description: "Roadmap and decisions",
  });
  await api(page, `/api/brains/${retired.id}`, "PATCH", { archived: true });
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Your Brains", exact: true }),
  ).toBeVisible();

  // Active-first: the archived record is absent from the active list entirely,
  // and no listed card carries the archived mark.
  await expect(
    page.getByTestId("brain-row-name").filter({ hasText: live.name }),
  ).toBeVisible();
  const activeNames = await rowText(page, "brain-row-name");
  expect(activeNames).toContain(live.name);
  expect(activeNames).not.toContain(retired.name);
  const listed = page.getByTestId("brain-card");
  await expect(listed.filter({ hasText: /\bArchived\b/ })).toHaveCount(0);

  // Historical records are reachable only through the explicit filter, which
  // states its own meaning as text.
  await expect(
    page.getByText(/archived Brains? hidden from this list/),
  ).toBeVisible();
  await pickView(page, /^Archived Brains \(history\)$/);
  await expect(page.getByText(/^Reading history:/)).toBeVisible();
  const archivedNames = await rowText(page, "brain-row-name");
  expect(archivedNames).toContain(retired.name);
  expect(archivedNames).not.toContain(live.name);
  await expect(
    page
      .getByTestId("brain-card")
      .filter({ has: page.getByText(retired.name, { exact: true }) })
      .getByText("Archived", { exact: true }),
  ).toBeVisible();

  // A search with no matches is neither an empty Brain nor a filtered-empty view.
  await page.getByLabel("Search Brains").fill(`nothing-matches-${suffix}`);
  await expect(
    page.getByRole("heading", { name: /^No Brains match/ }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "No archived Brains" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Clear search", exact: true }).click();
  await expect(
    page
      .getByTestId("brain-card")
      .filter({ has: page.getByText(retired.name, { exact: true }) }),
  ).toBeVisible();

  // The id stays out of the row and is offered as a copyable field instead.
  await pickView(page, /^Active Brains$/);
  const rows = (await rowText(page, "brain-card")).join("\n");
  for (const id of [live.id, retired.id]) {
    expect(rows).not.toContain(id);
    expect(rows).not.toContain(id.slice(0, 13));
  }
  const card = page
    .getByTestId("brain-card")
    .filter({ has: page.getByText(live.name, { exact: true }) });
  await openDetails(card, "Brain identifier");
  await card.getByRole("button", { name: "Copy Brain ID" }).click();
  await expect(card.getByRole("button", { name: "Copied" })).toBeVisible();
  expect(await copies()).toContain(live.id);

  // The remaining two empty flavors, on controlled authorized reads.
  let stub: BrainRecord[] | null = null;
  await page.route("**/api/brains", (route) =>
    route.request().method() === "GET" && stub
      ? route.fulfill({ json: stub })
      : route.fallback(),
  );
  stub = [{ ...live, archived: false, owner_id: me }];
  await page.reload();
  await pickView(page, /^Shared with you$/);
  await expect(
    page.getByRole("heading", { name: "No Brains shared with you" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /^No Brains match/ }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "Room for your first idea" }),
  ).toHaveCount(0);

  stub = [];
  await page.reload();
  await pickView(page, /^Active Brains$/);
  await expect(
    page.getByRole("heading", { name: "Room for your first idea" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Create your first Brain", exact: true }),
  ).toBeVisible();
});
