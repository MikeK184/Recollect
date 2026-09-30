import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { openDetails } from "./desktop-helpers";

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
async function command(
  page: Page,
  path: string,
  body?: unknown,
  method = body === undefined ? "GET" : "POST",
) {
  const session = await (await page.request.get("/api/auth/me")).json();
  const response = await page.request.fetch(path, {
    method,
    data: body,
    headers: { "x-csrf-token": session.csrf_token },
  });
  expect(response.ok(), `${method} ${path}: ${response.status()}`).toBeTruthy();
  return response.json();
}

test("new Brain has managed defaults, compact navigation and simple agent setup", async ({
  page,
}) => {
  await signIn(page);
  await page.getByRole("button", { name: "Create Brain", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByLabel(/^Name/)
    .fill("Managed browser fixture");
  await expect(
    page.getByText("Autonomous memory included", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Create Brain", exact: true })
    .click();
  await expect(page).toHaveURL(/\/brains\/[^/]+\/ask/);
  const brain = new URL(page.url()).pathname.split("/")[2];
  const data = await command(page, `/api/brains/${brain}/automation`);
  expect(data.models.current.policy.automatic_embedding).toBe(true);
  expect(data.capture.policy.enabled).toBe(true);
  const nav = page.getByRole("navigation", { name: "Brain navigation" });
  await expect(nav.getByRole("link")).toHaveCount(9);
  await expect(
    nav.getByRole("link", { name: "Settings", exact: true }),
  ).toBeVisible();
  await page.goto(`/brains/${brain}/settings?tab=ai`);
  await expect(
    page.getByRole("heading", { name: "Autonomous memory", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Edit model policy", exact: true }),
  ).toHaveCount(0);
  await expect(page.getByLabel("Daily token allowance")).toHaveCount(0);
  await page.screenshot({
    path: "../.cache/ui/managed-settings.png",
    fullPage: true,
  });
  await page.goto(`/brains/${brain}/settings?tab=privacy`);
  await expect(
    page.getByRole("button", { name: "Edit retention", exact: true }),
  ).toHaveCount(0);
  await page.goto(`/brains/${brain}/connections`);
  await page
    .getByRole("button", { name: "Connect Codex", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Connect Codex",
    exact: true,
  });
  await expect(dialog.getByTestId("agent-setup-command")).toContainText(
    `/api/brains/${brain}/mcp/agent`,
  );
  await expect(dialog.getByTestId("agent-setup-command")).toContainText(
    /bearer_token_env_var|http_headers_helper/,
  );
  await expect(dialog.getByText(/recollect-agent pair/)).not.toBeVisible();
  await expect(
    dialog.getByLabel("Session integration", { exact: true }),
  ).toHaveCount(0);
  await page.screenshot({
    path: "../.cache/ui/managed-agent-setup.png",
    fullPage: true,
  });
});

test("existing Brain has one Ask activation action and a plain note contribution", async ({
  page,
}) => {
  await signIn(page);
  const brain = await command(page, "/api/brains", {
    name: "Existing managed setup fixture",
  });
  const base = `/api/brains/${brain.id}`;
  await page.goto(`/brains/${brain.id}/ask`);
  await page
    .getByRole("button", { name: "Enable autonomous memory", exact: true })
    .click();
  await expect
    .poll(
      async () =>
        (await command(page, `${base}/automation`)).models.current.policy
          .enabled,
    )
    .toBe(true);
  // The UI proof does not send retained notes to a paid provider.
  const settings = await command(page, `${base}/models/policy`);
  await command(
    page,
    `${base}/models/policy`,
    {
      base_change: settings.current.change_id,
      policy: {
        ...settings.current.policy,
        enabled: false,
        automatic_learning: false,
        automatic_embedding: false,
      },
    },
    "PUT",
  );
  await page.goto(`/brains/${brain.id}/ask?tab=search`);
  await expect(page.getByLabel("Search memory", { exact: true })).toBeVisible();
  await expect(
    page.getByLabel("Recall mode", { exact: true }).first(),
  ).not.toBeVisible();
  await expect(page.getByLabel("Maximum results", { exact: true })).toHaveCount(
    0,
  );
  await page.goto(`/brains/${brain.id}/memory`);
  await page.getByRole("button", { name: "Add memory", exact: true }).click();
  const dialog = page.getByRole("dialog", {
    name: "Add a memory",
    exact: true,
  });
  await expect(dialog.getByLabel("Subject", { exact: true })).toHaveCount(0);
  await dialog
    .getByLabel(/^What should this Brain remember/)
    .fill("MANAGED_NOTE_EVIDENCE: Amber uses port 8080.");
  await page.screenshot({
    path: "../.cache/ui/managed-note.png",
    fullPage: true,
  });
  await dialog.getByRole("button", { name: "Save note", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await page.goto(`/brains/${brain.id}/sources`);
  await expect(
    page.getByRole("button", { name: /MANAGED_NOTE_EVIDENCE/ }),
  ).toBeVisible();
});

test("empty connector catalogue has a working owner registration path", async ({
  page,
}) => {
  await signIn(page);
  const brain = await command(page, "/api/brains", {
    name: "Connector setup fixture",
  });
  await page.goto(`/brains/${brain.id}/connections?tab=connections`);
  // This suite owns its database and does not pre-import the CLI fixture.
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Add an MCP connector",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await dialog.getByText("Import manifest", { exact: true }).click();
  await openDetails(dialog, "Paste or inspect connector JSON");
  await dialog
    .getByLabel("Connector JSON", { exact: true })
    .fill(
      readFileSync(
        "../crates/server/tests/fixtures/mcp-catalogue.json",
        "utf8",
      ),
    );
  await dialog
    .getByRole("button", { name: "Register connector", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  await page
    .getByRole("button", { name: "Use registered connector", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Add MCP connection", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByRole("tab", { name: "Tool groups", exact: true }).click();
  await expect(page.getByText(/Profiles are tool groups/)).toBeVisible();
});
