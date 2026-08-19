import { test, expect } from "@playwright/test";

test("desktop coding-agent guidance fixes the Brain and explains optional credentials", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await page
    .getByLabel(/^Username/)
    .fill(process.env.RECOLLECT_OWNER_USERNAME!);
  await page
    .getByLabel(/^Password/)
    .fill(process.env.RECOLLECT_OWNER_PASSWORD!);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await page.getByRole("button", { name: "Create Brain", exact: true }).click();
  let dialog = page.getByRole("dialog");
  await dialog.getByLabel(/^Name/).fill("Coding host connection");
  await dialog
    .getByRole("button", { name: "Create Brain", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Coding host connection", exact: true }),
  ).toBeVisible();
  const brain = new URL(page.url()).pathname.split("/").at(-1)!;
  await page
    .getByRole("button", { name: "Connect coding agent", exact: true })
    .click();
  dialog = page.getByRole("dialog");
  await expect(dialog.getByTestId("agent-setup-command")).toContainText(
    `mcp-config codex --brain ${brain}`,
  );
  await dialog.getByLabel("Coding host", { exact: true }).click();
  await page.getByRole("option", { name: "Claude Code", exact: true }).click();
  await expect(dialog.getByTestId("agent-setup-command")).toContainText(
    `mcp-config claude --brain ${brain}`,
  );
  await dialog.getByLabel("Session integration", { exact: true }).click();
  await page
    .getByRole("option", {
      name: "Tools with automatic session capture",
      exact: true,
    })
    .click();
  await expect(dialog.getByTestId("agent-setup-command")).toContainText(
    `capture setup claude . --brain ${brain}`,
  );
  await expect(
    dialog.getByText(/Vault is optional for each managed MCP connection/),
  ).toBeVisible();
  await expect(
    dialog.getByText(/Creating settings alone does not prove a connection/),
  ).toBeVisible();
  await page.screenshot({
    path: "../.cache/mcp-tools-desktop.png",
    fullPage: false,
  });
  expect(errors).toEqual([]);
});
