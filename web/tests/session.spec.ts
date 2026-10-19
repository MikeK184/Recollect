import { test, expect, type Page } from "@playwright/test";
import type { QueryClient } from "@tanstack/react-query";
import { revalidateSession } from "../src/sessionCheck";

const ended = "Your session ended — sign in again";

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

test("concurrent 401s share one session check and a failed check keeps the session", async () => {
  const stored = new Map<string, unknown>();
  const cache = {
    setQueryData: (key: unknown[], value: unknown) =>
      stored.set(String(key), value),
  } as unknown as QueryClient;
  let checks = 0;
  let finish!: (value: null) => void;
  const pending = () => {
    checks += 1;
    return new Promise<null>((resolve) => (finish = resolve));
  };
  const first = revalidateSession(cache, pending);
  const second = revalidateSession(cache, pending);
  expect(second).toBe(first);
  expect(checks).toBe(1);
  finish(null);
  await first;
  expect(stored.get("session")).toBeNull();
  // The next 401 starts a new check; one that cannot reach /me leaves the
  // session untouched instead of ending it.
  stored.set("session", { user: "kept" });
  await revalidateSession(cache, () => Promise.reject(new Error("offline")));
  expect(stored.get("session")).toEqual({ user: "kept" });
  expect(checks).toBe(1);
});

test("a 401 on one request keeps a valid session signed in", async ({
  page,
}) => {
  await ownerLogin(page);
  let checks = 0;
  page.on("request", (request) => {
    if (new URL(request.url()).pathname === "/api/auth/me") checks += 1;
  });
  await page.route("**/api/team", (route) =>
    route.fulfill({
      status: 401,
      contentType: "application/json",
      body: JSON.stringify({
        code: "unauthorized",
        message: "Sign in to continue.",
      }),
    }),
  );
  const rejected = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === "/api/team" &&
      response.status() === 401,
  );
  await page.getByRole("link", { name: "Team", exact: true }).click();
  await rejected;
  // The 401 is checked against /api/auth/me, which still accepts the session.
  await expect.poll(() => checks).toBeGreaterThan(0);
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Welcome back" })).toHaveCount(
    0,
  );
  await expect(page.getByText(ended)).toHaveCount(0);
  await page.unroute("**/api/team");
  await page.reload();
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
});

test("a session the server revoked leads to sign-in with a notice", async ({
  page,
}) => {
  await ownerLogin(page);
  // Revoke this very session on the server without telling the app.
  const me = await page.request.get("/api/auth/me");
  const { csrf_token } = await me.json();
  const logout = await page.request.post("/api/auth/logout", {
    headers: { "X-CSRF-Token": csrf_token },
  });
  expect(logout.ok()).toBe(true);
  await page.getByRole("link", { name: "Team", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
  await expect(page.getByText(ended)).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign out" })).toHaveCount(0);
  // Signing in again clears the notice; the next explicit sign-out adds none.
  await page
    .getByLabel(/^Password/)
    .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  await expect(page.getByText(ended)).toHaveCount(0);
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(
    page.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
  await expect(page.getByText(ended)).toHaveCount(0);
});

test("an explicit sign-out shows no notice", async ({ page }) => {
  await ownerLogin(page);
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(
    page.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
  await expect(page.getByText(ended)).toHaveCount(0);
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Welcome back" }),
  ).toBeVisible();
  await expect(page.getByText(ended)).toHaveCount(0);
});
