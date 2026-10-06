import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { test, expect } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import { randomUUID } from "node:crypto";

test("owner manages a Brain, recovers a request error and signs out on desktop at 1280 pixels", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await mkdir("../.cache/ui", { recursive: true });
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
  await page.screenshot({ path: "../.cache/ui/login.png", fullPage: true });
  await page
    .getByLabel(/^Username/)
    .fill(process.env.RECOLLECT_OWNER_USERNAME!);
  await page.getByLabel(/^Password/).fill(randomUUID());
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByText("Something needs attention")).toBeVisible();
  await page
    .getByLabel(/^Password/)
    .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Room for your first idea" }),
  ).toBeVisible();
  await expect(page.getByText("All services connected")).toBeVisible();
  await page.screenshot({ path: "../.cache/ui/empty.png", fullPage: true });
  await page.getByRole("button", { name: "Create Brain", exact: true }).click();
  const form = page.getByRole("dialog");
  await form.getByLabel(/^Name/).fill("Platform engineering");
  await form
    .getByLabel("Description")
    .fill("Decisions, systems and the evidence behind them.");
  await form.getByRole("button", { name: "Create Brain", exact: true }).click();
  await expect(page.getByLabel("Switch Brain", { exact: true })).toBeVisible();
  const brain = new URL(page.url()).pathname.split("/")[2];
  await page.goto(`/brains/${brain}/activity?tab=processing`);
  const processing = page.getByRole("region", {
    name: "Background processing",
  });
  await expect(processing.getByText("Queued", { exact: true })).toBeVisible();
  const refreshJob = processing
    .locator(".job-row")
    .filter({
      has: page.getByText("Update Brain details", { exact: true }),
    })
    .first(); // Managed creation also queues refreshes for its policy changes.
  await refreshJob.getByRole("button", { name: "Cancel job" }).click();
  await expect(
    refreshJob.getByRole("button", { name: "Retry job" }),
  ).toBeVisible();
  await refreshJob.getByRole("button", { name: "Retry job" }).click();
  await expect(
    refreshJob.getByRole("button", { name: "Cancel job" }),
  ).toBeVisible();
  await promisify(execFile)("../target/debug/recollect-server", [
    "worker-once",
  ]);
  await expect(
    processing.getByText("Up to date", { exact: true }),
  ).toBeVisible();
  await page.goto(`/brains/${brain}/settings`);
  await page.getByRole("button", { name: "Edit Brain" }).click();
  const editor = page.getByRole("region", {
    name: "Edit Brain",
    exact: true,
  });
  await editor.getByLabel(/^Name/).fill("Platform knowledge");
  await editor.getByRole("button", { name: "Save changes" }).click();
  await expect(
    page.getByLabel("Switch Brain", { exact: true }).locator("option:checked"),
  ).toHaveText("Platform knowledge");
  await page
    .getByRole("button", { name: "Archive Brain", exact: true })
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Archive Brain", exact: true })
    .click();
  await expect(page.getByText("This Brain is archived")).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "Reopen Brain", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Reopen Brain", exact: true })
    .click();
  await expect(page.getByText("This Brain is archived")).toHaveCount(0);
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.screenshot({ path: "../.cache/ui/brain.png", fullPage: true });
  await page.getByRole("link", { name: "All Brains" }).click();
  await expect(
    page.getByRole("heading", { name: "Platform knowledge" }),
  ).toBeVisible();
  await page.screenshot({ path: "../.cache/ui/brains.png", fullPage: true });
  await page.route("**/api/brains", (route) => route.abort());
  await page.reload();
  await expect(page.getByText("Something needs attention")).toBeVisible();
  await page.unroute("**/api/brains");
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("heading", { name: "Platform knowledge" }),
  ).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.screenshot({ path: "../.cache/ui/1280.png", fullPage: true });
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > window.innerWidth,
  );
  expect(overflow).toBe(false);
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(
    page.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});
