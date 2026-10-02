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
  return new URL(page.url()).pathname.split("/")[2];
}

test("delete brain: admin-only control, preview confirmation, distinct errors and absence", async ({
  page,
  browser,
}) => {
  test.setTimeout(150_000);
  page.setDefaultTimeout(15_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));

  await ownerLogin(page);

  // Disposable Brain with one source so the preview shows real counts.
  const name = `delete-proof-${Date.now()}`;
  const brain = await createBrain(page, name);
  await page.evaluate(async (id) => {
    const me = await (await fetch("/api/auth/me")).json();
    const r = await fetch(`/api/brains/${id}/sources`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": me.csrf_token,
      },
      body: JSON.stringify({
        title: "Deletion proof source",
        media_type: "text/plain",
        retention_class: "raw_session",
        retain_content: true,
        content: "Proof line for deletion.\n",
      }),
    });
    if (!r.ok) throw new Error("source fixture failed: " + r.status);
  }, brain);

  await expect
    .poll(
      async () => {
        const status = await (
          await page.request.get(`/api/brains/${brain}/processing`)
        ).json();
        return status.pending_jobs;
      },
      { timeout: 60_000 },
    )
    .toBe(0);

  // Settings General: the delete control exists for an administrator and is a
  // separate control from archive.
  await page.goto(`/brains/${brain}/settings`);
  await expect(
    page.getByRole("heading", { name: "Delete this Brain", exact: true }),
  ).toBeVisible();
  const deleteControl = page.getByRole("button", {
    name: "Delete Brain",
    exact: true,
  });
  await expect(deleteControl).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Archive Brain", exact: true }),
  ).toBeVisible();

  // Opening the flow previews first: dependent counts and the backup-window
  // limitation are shown, and confirmation stays disabled until the exact
  // Brain name is typed.
  await deleteControl.click();
  const dialog = page.getByRole("dialog", {
    name: "Delete this Brain",
    exact: true,
  });
  await expect(dialog.getByText(/dependent record/)).toBeVisible();
  await expect(dialog.getByText("Sources", { exact: true })).toBeVisible();
  await expect(
    dialog.getByText(/managed backups can remain for up to \d+ days/),
  ).toBeVisible();
  const confirm = dialog.getByRole("button", {
    name: "Delete Brain",
    exact: true,
  });
  await expect(confirm).toBeDisabled();

  // A mismatched confirmation string is rejected with its distinct message.
  // The button only enables on an exact match, so the mismatch is forced at
  // the wire level to prove the UI surfaces the server's 400 correctly.
  let mangle: "confirmation" | "closure" | null = "confirmation";
  await page.route(`**/api/brains/${brain}`, async (route) => {
    const request = route.request();
    if (request.method() !== "DELETE" || !mangle) return route.continue();
    const body = JSON.parse(request.postData()!);
    expect(typeof body.closure).toBe("string");
    if (mangle === "confirmation") {
      body.confirmation = "definitely-not-the-name";
      mangle = "closure";
    } else {
      body.closure = (BigInt(body.closure) ^ 1n).toString();
      mangle = null;
    }
    await route.fulfill({
      response: await route.fetch({ postData: JSON.stringify(body) }),
    });
  });
  await dialog.getByLabel("Exact Brain name").fill(name);
  await confirm.click();
  await expect(dialog.getByText(/did not match the Brain name/)).toBeVisible();
  expect(
    await page.evaluate(
      async (id) => (await fetch(`/api/brains/${id}`)).status,
      brain,
    ),
  ).toBe(200);

  // A genuinely stale counter is a different server failure and causes the
  // UI to fetch a new preview before another explicit confirmation.
  const refreshed = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/brains/${brain}/deletions/preview`) &&
      response.ok(),
  );
  await confirm.click();
  await refreshed;
  await expect(dialog.getByText(/changed since the preview/)).toBeVisible();
  await expect(confirm).toBeEnabled();

  // Exact name: the DELETE is accepted and the resulting state is shown.
  let deletionId = "";
  page.on("response", async (res) => {
    if (
      res.request().method() === "DELETE" &&
      res.url().includes(`/api/brains/${brain}`) &&
      res.ok()
    )
      deletionId = ((await res.json()).request as { id: string }).id;
  });
  await confirm.click();
  await expect(page.getByText(/Deletion (in progress|complete)/)).toBeVisible();

  // The absent Brain is left for the Brains list, and its cache is cleared.
  await expect(
    page.getByRole("heading", { name: "Your Brains", exact: true }),
  ).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText(name, { exact: true })).toHaveCount(0);
  expect(
    await page.evaluate(
      async (id) => (await fetch(`/api/brains/${id}`)).status,
      brain,
    ),
  ).toBe(404);
  for (const section of ["ask", "sources"]) {
    await page.goto(`/brains/${brain}/${section}`);
    await expect(
      page.getByText("This resource is unavailable or you do not have access."),
    ).toBeVisible();
  }
  // The deletion status endpoint reports the committed request accurately.
  const status = await page.evaluate(
    async ({ id, rid }) => {
      const r = await fetch(`/api/brains/${id}/deletions/${rid}`);
      return { ok: r.ok, body: r.ok ? await r.json() : null };
    },
    { id: brain, rid: deletionId },
  );
  expect(status.ok).toBe(true);
  expect(status.body.disposition).toBe("deleted");
  expect(["pending", "complete"]).toContain(status.body.state);
  await expect
    .poll(
      async () => {
        const current = await (
          await page.request.get(`/api/brains/${brain}/deletions/${deletionId}`)
        ).json();
        return current.state;
      },
      { timeout: 60_000 },
    )
    .toBe("complete");

  // A non-administrator never sees the delete control.
  const username = `deletereader-${randomUUID().slice(0, 8)}`;
  const second = await createBrain(page, `admin-only-${Date.now()}`);
  await page.goto("/team");
  await page.getByRole("button", { name: "Add teammate" }).click();
  let invite = page.getByRole("dialog");
  await invite.getByLabel(/^Teammate username/).fill(username);
  await invite.getByRole("button", { name: "Create invitation" }).click();
  const invitation = await invite.getByLabel("Invitation link").inputValue();
  await invite.getByRole("button", { name: "Done", exact: true }).click();
  const context = await browser.newContext();
  const member = await context.newPage();
  member.on("pageerror", (e) => errors.push(e.message));
  await member.goto(invitation);
  await member.getByLabel(/^Choose a password/).fill(randomUUID());
  await member.getByRole("button", { name: "Accept invitation" }).click();
  await expect(
    member.getByRole("heading", { name: "Room for your first idea" }),
  ).toBeVisible();
  await page.goto(`/brains/${second}/settings?tab=access`);
  const access = page.getByRole("region", {
    name: "Brain access",
    exact: true,
  });
  await access.getByLabel(/^Account username/).fill(username);
  await access.getByRole("button", { name: "Save direct grant" }).click();
  await expect(
    access.getByText("Effective access is now reader."),
  ).toBeVisible();
  await member.goto(`/brains/${second}/settings`);
  await expect(
    member.getByRole("heading", { name: "Settings", level: 1, exact: true }),
  ).toBeVisible();
  await expect(
    member.getByRole("button", { name: "Delete Brain", exact: true }),
  ).toHaveCount(0);
  await expect(
    member.getByRole("button", { name: "Edit Brain", exact: true }),
  ).toHaveCount(0);
  await context.close();

  expect(errors).toEqual([]);
});
