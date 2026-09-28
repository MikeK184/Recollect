import { test, expect, type Page } from "@playwright/test";
import { spawn, execFile, type ChildProcess } from "node:child_process";
import { randomUUID } from "node:crypto";
import { resolve } from "node:path";

test("opaque pairing codes survive numeric and exponent-shaped URL parsing", async ({
  page,
}) => {
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
  for (const code of ["12345678", "01234567", "1234E567"]) {
    // Deterministic wire fixture isolates URL preservation; the native test below
    // exercises actual pairing approval, OS-store delivery and revocation.
    await page.route(`**/api/devices/pairings/${code}`, (route) =>
      route.fulfill({
        json: {
          user_code: code,
          name: "Opaque-code fixture",
          state: "pending",
          expires_at: new Date(Date.now() + 60_000).toISOString(),
        },
      }),
    );
    await page.goto(`/devices?code=${code}`);
    await expect(page.getByText(code, { exact: true })).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Approve device", exact: true }),
    ).toBeVisible();
    expect(new URL(page.url()).searchParams.get("code")).toBe(code);
  }
});

test("browser approves a native companion, persists its OS credential and revokes it", async ({
  page,
}) => {
  test.setTimeout(75_000);
  const binary = resolve("../target/debug/recollect-agent");
  const env = {
    PATH: process.env.PATH,
    HOME: process.env.HOME,
    TMPDIR: process.env.TMPDIR,
    RECOLLECT_URL: process.env.RECOLLECT_UI_TEST_ORIGIN!,
    RECOLLECT_DEVICE_PROFILE: `ui-test-${randomUUID()}`,
  };
  const run = (args: string[]) =>
    new Promise<{ ok: boolean; stdout: string; stderr: string }>((done) => {
      execFile(
        binary,
        args,
        { env, timeout: 15_000 },
        (error, stdout, stderr) => done({ ok: !error, stdout, stderr }),
      );
    });
  let companion: ChildProcess | undefined;
  async function pair(name: string, browser: Page) {
    let output = "";
    let errors = "";
    companion = spawn(binary, ["pair", name], {
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    companion.stdout!.on("data", (chunk) => {
      output += chunk.toString();
    });
    companion.stderr!.on("data", (chunk) => {
      errors += chunk.toString();
    });
    const finished = new Promise<number | null>((done) => {
      companion!.once("exit", done);
      companion!.once("error", () => done(-1));
    });
    await expect
      .poll(() => output.includes("Open ") || errors.length > 0, {
        timeout: 15_000,
      })
      .toBe(true);
    expect(errors).toBe("");
    const link = output.match(/Open (\S+)/)?.[1];
    expect(link).toBeTruthy();
    await browser.goto(link!);
    await expect(
      browser.getByRole("heading", {
        name: /^(Welcome back|Approve a companion)$/,
      }),
    ).toBeVisible();
    if (
      await browser.getByRole("heading", { name: "Welcome back" }).isVisible()
    ) {
      await browser
        .getByLabel(/^Username/)
        .fill(process.env.RECOLLECT_OWNER_USERNAME!);
      await browser
        .getByLabel(/^Password/)
        .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
      await browser
        .getByRole("button", { name: "Sign in", exact: true })
        .click();
    }
    await expect(
      browser.getByRole("heading", { name: "Approve a companion" }),
    ).toBeVisible();
    await expect(
      browser.getByText(new URL(link!).searchParams.get("code")!, {
        exact: true,
      }),
    ).toBeVisible();
    await browser.screenshot({
      path: "../.cache/ui/device-approval.png",
      fullPage: true,
      animations: "disabled",
    });
    await browser
      .getByRole("button", { name: "Approve device", exact: true })
      .click();
    await expect(browser.getByText("Your companion is connected.")).toBeVisible(
      { timeout: 20_000 },
    );
    expect(await finished, errors).toBe(0);
    expect(output).toContain("Credential saved in the OS store.");
    companion = undefined;
    return browser
      .getByTestId("device-card")
      .filter({ has: browser.getByText(name, { exact: true }) });
  }
  try {
    const card = await pair("Development laptop", page);
    // Each command is a fresh process; success depends on a real persisted OS-store entry.
    const identity = await run(["whoami"]);
    expect(identity.ok, identity.stderr).toBe(true);
    const actor = JSON.parse(identity.stdout);
    expect(actor.username).toBe(process.env.RECOLLECT_OWNER_USERNAME);
    expect(typeof actor.device_id).toBe("string");
    const brains = await run(["brains"]);
    expect(brains.ok, brains.stderr).toBe(true);
    expect(Array.isArray(JSON.parse(brains.stdout))).toBe(true);
    const duplicate = await run(["pair", "Duplicate"]);
    expect(duplicate.ok).toBe(false);
    expect(duplicate.stderr).toContain("already paired");
    await page.getByRole("link", { name: "Devices", exact: true }).click();
    await expect(card.getByText("Active", { exact: true })).toBeVisible();
    await page.screenshot({
      path: "../.cache/ui/devices.png",
      fullPage: true,
      animations: "disabled",
    });
    await page.setViewportSize({ width: 1280, height: 800 });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
    ).toBe(false);
    await page.screenshot({
      path: "../.cache/ui/devices-1280.png",
      fullPage: true,
      animations: "disabled",
    });
    await card.getByRole("button", { name: "Revoke", exact: true }).click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Revoke device", exact: true })
      .click();
    await expect(card.getByText("Revoked", { exact: true })).toBeVisible();
    const rejected = await run(["whoami"]);
    expect(rejected.ok).toBe(false);
    expect(rejected.stderr).toContain("Device access is unavailable");
    const unpair = await run(["unpair"]);
    expect(unpair.ok, unpair.stderr).toBe(true);
    const missing = await run(["whoami"]);
    expect(missing.ok).toBe(false);
    expect(missing.stderr).toContain("No paired device");
    // Pairing the same profile again is valid after deletion, and unpair revokes an active device.
    const replacement = await pair("Replacement laptop", page);
    const activeUnpair = await run(["unpair"]);
    expect(activeUnpair.ok, activeUnpair.stderr).toBe(true);
    await expect(
      replacement.getByText("Revoked", { exact: true }),
    ).toBeVisible();
    expect((await run(["whoami"])).stderr).toContain("No paired device");
  } finally {
    if (companion && companion.exitCode === null) companion.kill("SIGTERM");
    // This UUID profile belongs only to this test; never enumerate or alter other credentials.
    await run(["unpair"]);
    const cleanup = await run(["forget"]);
    expect(cleanup.ok, cleanup.stderr).toBe(true);
  }
});
