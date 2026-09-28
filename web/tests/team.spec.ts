import { test, expect, type Page } from "@playwright/test";
import { randomUUID } from "node:crypto";

async function ownerLogin(page: Page) {
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
async function createBrain(page: Page, name: string) {
  await page.goto("/");
  await page.getByRole("button", { name: "Create Brain", exact: true }).click();
  await page.getByRole("dialog").getByLabel(/^Name/).fill(name);
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Create Brain", exact: true })
    .click();
  await expect(page.getByLabel("Switch Brain", { exact: true })).toBeVisible();
  const brain = new URL(page.url()).pathname.split("/")[2];
  await page.goto(`/brains/${brain}/settings?tab=access`);
}

test("invite a teammate and revoke shared Brain access through the browser", async ({
  page,
  browser,
}) => {
  await ownerLogin(page);
  const username = `teammate-${randomUUID().slice(0, 8)}`;
  await page.goto("/team");
  await page.getByRole("button", { name: "Add teammate" }).click();
  let dialog = page.getByRole("dialog");
  await dialog.getByLabel(/^Teammate username/).fill(username);
  await dialog.getByRole("button", { name: "Create invitation" }).click();
  const invitation = await dialog.getByLabel("Invitation link").inputValue();
  await dialog.getByRole("button", { name: "Done", exact: true }).click();
  const context = await browser.newContext();
  const member = await context.newPage();
  await member.goto(invitation);
  await expect(
    member.getByRole("heading", { name: "Join your team" }),
  ).toBeVisible();
  expect(new URL(member.url()).hash).toBe("");
  await member.getByLabel(/^Choose a password/).fill(randomUUID());
  await member.getByRole("button", { name: "Accept invitation" }).click();
  await expect(
    member.getByRole("heading", { name: "Room for your first idea" }),
  ).toBeVisible();
  await expect(
    member.getByRole("link", { name: "Team", exact: true }),
  ).toHaveCount(0);
  await createBrain(page, "Shared team evidence");
  const brainUrl = page.url();
  dialog = page.getByRole("region", { name: "Brain access", exact: true });
  await dialog.getByLabel(/^Account username/).fill(username);
  await dialog.getByRole("button", { name: "Save direct grant" }).click();
  await expect(
    dialog.getByText("Effective access is now reader."),
  ).toBeVisible();
  await expect(dialog.getByText(username, { exact: true })).toBeVisible();
  await page.screenshot({
    path: "../.cache/ui/access.png",
    animations: "disabled",
  });
  await member.goto(brainUrl);
  await expect(
    member.getByRole("heading", { name: "Settings", level: 1, exact: true }),
  ).toBeVisible();
  await expect(
    member.getByRole("button", { name: "Edit Brain", exact: true }),
  ).toHaveCount(0);
  const memberCard = dialog
    .locator(".mantine-Card-root")
    .filter({ has: page.getByText(username, { exact: true }) });
  await memberCard.getByRole("button", { name: "Remove direct grant" }).click();
  await expect(dialog.getByText("Effective access is now none.")).toBeVisible();
  await expect(
    member.getByRole("heading", { name: "Settings", level: 1, exact: true }),
  ).toHaveCount(0, { timeout: 10000 });
  await expect(
    member.getByText("This resource is unavailable or you do not have access."),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await page.goto("/team");
  const account = page
    .locator(".mantine-Card-root")
    .filter({ has: page.getByText(username, { exact: true }) })
    .filter({
      has: page.getByRole("button", { name: "Disable", exact: true }),
    });
  await account.getByRole("button", { name: "Disable", exact: true }).click();
  await expect(page.getByText("account status · disabled")).toBeVisible();
  await member.reload();
  await expect(
    member.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
  await page.screenshot({ path: "../.cache/ui/team.png", fullPage: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Your Brains", exact: true }),
  ).toBeVisible();
  await page.goto("/team");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
  ).toBe(false);
  await page.screenshot({
    path: "../.cache/ui/team-1280.png",
    fullPage: true,
  });
  await context.close();
});

test("organization sign-in obtains a Brain role from a real signed group claim", async ({
  page,
}) => {
  test.skip(
    !process.env.RECOLLECT_OIDC_ISSUER,
    "Run ./scripts/test-oidc.sh ui for the isolated provider",
  );
  await ownerLogin(page);
  await page.goto("/team");
  await page.getByRole("button", { name: "Add teammate" }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel(/^Teammate username/).fill("organization-browser");
  await dialog.getByLabel("Sign-in method").click();
  await page.getByRole("option", { name: "Organization identity" }).click();
  await dialog
    .getByLabel(/^Provider subject/)
    .fill("Cg0wLTM4NS0yODA4OS0wEgRtb2Nr");
  await dialog.getByRole("button", { name: "Enroll identity" }).click();
  await expect(
    page.getByText("organization-browser", { exact: true }),
  ).toBeVisible();
  await createBrain(page, "Organization evidence");
  const access = page.getByRole("region", {
    name: "Brain access",
    exact: true,
  });
  await access.getByLabel(/^Group name or ID/).fill("authors");
  await access.getByRole("button", { name: "Save group mapping" }).click();
  await expect(access.getByText("Group mapping updated.")).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Sign out" }).click();
  const pairingResponse = await page.request.post("/api/devices/pairings", {
    data: { name: "Organization browser companion" },
  });
  expect(pairingResponse.ok()).toBe(true);
  const pairing = await pairingResponse.json();
  await page.goto(pairing.verification_url);
  await page
    .getByRole("link", { name: "Sign in with your organization" })
    .click();
  await expect(
    page.getByRole("heading", { name: "Approve a companion", exact: true }),
  ).toBeVisible();
  expect(new URL(page.url()).searchParams.get("code")).toBe(pairing.user_code);
  await page.getByRole("button", { name: "Decline", exact: true }).click();
  await expect(
    page.getByText(
      "This pairing was declined or cancelled. Start a new request to try again.",
    ),
  ).toBeVisible();
  await page.goto("/");
  await expect(
    page.getByText("organization-browser", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("heading", { name: "Organization evidence", exact: true })
    .click();
  await page
    .getByRole("navigation", { name: "Brain navigation" })
    .getByRole("link", { name: "Settings", exact: true })
    .click();
  await page.getByRole("tab", { name: "Access", exact: true }).click();
  await expect(page.getByText("Your role:")).toContainText("reader");
  await expect(page.getByRole("button", { name: "Manage access" })).toHaveCount(
    0,
  );
  await page.screenshot({
    path: "../.cache/ui/organization.png",
    fullPage: true,
  });
});
