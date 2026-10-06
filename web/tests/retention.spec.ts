import { openDetails } from "./desktop-helpers";
import { test, expect } from "@playwright/test";

test("retention, explicit excerpts and erasure survive a lost response", async ({
  page,
}) => {
  test.setTimeout(90_000);
  page.setDefaultTimeout(10_000);
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
  const brain = await page.evaluate(async () => {
    const me = await (await fetch("/api/auth/me")).json();
    const send = async (url: string, body: unknown) => {
      const r = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error("Synthetic fixture failed: " + r.status);
      return r.json();
    };
    const brain = await send("/api/brains", {
      name: "Browser retention proof",
    });
    const source = await send("/api/brains/" + brain.id + "/sources", {
      title: "Synthetic raw browser source",
      media_type: "text/plain",
      retention_class: "raw_session",
      retain_content: true,
      content: "First raw line.\nRetained excerpt line.\nThird raw line.\n",
    });
    await send("/api/brains/" + brain.id + "/claims", {
      base_revision: null,
      operation_id: null,
      content: {
        kind: "claim",
        subject: "Browser erased claim",
        predicate: "has source support",
        value: "Synthetic raw support",
        rationale: "Controlled erasure browser fixture",
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
    return brain.id as string;
  });
  await page.goto("/brains/" + brain + "/settings?tab=privacy");
  await expect(
    page.getByRole("heading", { name: "Privacy", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Edit privacy settings", exact: true })
    .click();
  let dialog = page.getByRole("form", {
    name: "Privacy settings",
    exact: true,
  });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    dialog.getByLabel("Maximum text per event · KiB", { exact: true }),
  ).toBeEditable();
  await page.screenshot({
    path: "../.cache/actionable-privacy-inline.png",
    animations: "disabled",
  });
  await dialog.getByLabel("Raw sessions · days", { exact: true }).fill("14");
  await dialog
    .getByRole("button", { name: "Save changes", exact: true })
    .click();
  await expect(page.getByRole("button", { name: "Edit privacy settings", exact: true })).toBeVisible();
  await expect(dialog.getByLabel("Raw sessions · days", { exact: true })).toHaveValue("14");
  await page
    .getByRole("button", { name: "Edit privacy settings", exact: true })
    .click();
  let storageChanged = false;
  await page.route(`**/api/brains/${brain}/evidence`, async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    body.policy.allow_document_content = false;
    storageChanged = true;
    await route.fulfill({ response, json: body });
  });
  await expect(
    page.getByText(
      "Privacy settings changed elsewhere. Cancel and reopen to edit them.",
    ),
  ).toBeVisible();
  expect(storageChanged).toBe(true);
  await expect(
    page.getByRole("button", { name: "Save changes", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await page.unroute(`**/api/brains/${brain}/evidence`);
  await expect(
    page.getByRole("heading", { name: "Privacy", exact: true }),
  ).toBeVisible();
  await page.goto(`/brains/${brain}/sources`);
  await page
    .getByRole("button", { name: /Synthetic raw browser source/ })
    .click();
  await openDetails(page, "More source actions");
  await page
    .getByRole("button", { name: "Retain supporting excerpt", exact: true })
    .click();
  dialog = page.getByRole("dialog", {
    name: "Retain supporting excerpt",
    exact: true,
  });
  await dialog.getByLabel("Excerpt title").fill("Browser retained excerpt");
  await dialog.getByLabel("Last line", { exact: true }).fill("2");
  await dialog.getByLabel("First line", { exact: true }).fill("2");
  await dialog
    .getByRole("button", { name: "Create excerpt", exact: true })
    .click();
  await expect(
    dialog.getByText("Excerpt retained. It is available as a separate source."),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible();
  const lineage = page.getByRole("dialog", {
    name: "Knowledge lineage",
    exact: true,
  });
  await expect(lineage).toBeVisible();
  await expect
    .poll(() =>
      lineage.evaluate((element) => element.contains(document.activeElement)),
    )
    .toBeTruthy();
  await page.keyboard.press("Escape");
  await expect(lineage).not.toBeVisible();
  await page.getByRole("button", { name: /Browser retained excerpt/ }).click();
  await expect(page.getByTestId("source-content")).toHaveText(
    "Retained excerpt line.\n",
  );
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: /Synthetic raw browser source/ })
    .click();
  await openDetails(page, "More source actions");
  await page.getByRole("button", { name: "Erase source", exact: true }).click();
  dialog = page.getByRole("dialog", {
    name: "Erase controlled memory",
    exact: true,
  });
  await expect(dialog.getByText(/2 source versions/)).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 800 });
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    )
    .toBeTruthy();
  await page.screenshot({ path: "../.cache/ui-retention-1280.png" });
  let first = true;
  await page.route("**/api/brains/" + brain + "/erasures", async (route) => {
    if (route.request().method() === "POST" && first) {
      first = false;
      await route.fetch();
      await route.abort("failed");
    } else await route.continue();
  });
  await dialog
    .getByRole("button", { name: "Confirm erasure", exact: true })
    .click();
  await expect(dialog.getByRole("alert")).toContainText(
    /fetch|network|failed/i,
  );
  await page.waitForTimeout(4500);
  await expect(
    dialog.getByRole("button", { name: "Confirm erasure", exact: true }),
  ).toBeEnabled();
  await dialog
    .getByRole("button", { name: "Confirm erasure", exact: true })
    .click();
  await page.unroute("**/api/brains/" + brain + "/erasures");
  await page.goto(`/brains/${brain}/sources`);
  await expect(
    page.getByRole("button", { name: /Synthetic raw browser source/ }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: /Browser retained excerpt/ }),
  ).toHaveCount(0);
  await page.goto(`/brains/${brain}/activity?tab=removal`);
  const retryCleanup = page.getByRole("button", {
    name: "Retry cleanup",
    exact: true,
  });
  // The worker may have finished central cleanup before this view opens.
  await expect(
    page
      .getByText("Central cleanup complete", { exact: true })
      .or(retryCleanup),
  ).toBeVisible();
  if (await retryCleanup.isVisible()) await retryCleanup.click();
  await expect(
    page.getByText("Central cleanup complete", { exact: true }),
  ).toBeVisible();
  const status = await page.evaluate(
    async (brain) =>
      await (await fetch("/api/brains/" + brain + "/erasures")).json(),
    brain,
  );
  expect(status.total).toBe(1);
  expect(status.items[0].pending_artifacts).toBe(0);
  expect(status.items[0].journaled).toBe(true);
  await page.goto(`/brains/${brain}/memory`);
  await page.getByRole("button", { name: /^Filters/ }).click();
  await page.getByRole("textbox", { name: "Claim view", exact: true }).click();
  await page
    .getByRole("option", { name: "Include historical states", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  await page.getByRole("button", { name: /^Unavailable memory ·/ }).click();
  await page
    .getByRole("button", { name: "Memory history & actions", exact: true })
    .click();
  const erasedClaim = page.getByRole("dialog", {
    name: "Claim and knowledge history",
    exact: true,
  });
  await expect(erasedClaim.getByText(/This revision was erased/)).toBeVisible();
  await openDetails(erasedClaim, "More memory actions");
  await expect(
    erasedClaim.getByRole("button", { name: "Erase claim", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.goto(`/brains/${brain}/settings?tab=privacy`);
  await page
    .getByRole("heading", { name: "Privacy", exact: true })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: "../.cache/ui-retention-desktop.png" });
  expect(errors).toEqual([]);
});
