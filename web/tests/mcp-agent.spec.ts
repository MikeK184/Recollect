import { test, expect } from "@playwright/test";

test("desktop coding-agent guidance binds the Brain and explains direct credentials", async ({
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
  await expect(page.getByLabel("Switch Brain", { exact: true })).toBeVisible();
  const brain = new URL(page.url()).pathname.split("/")[2]!;
  await page.goto(`/brains/${brain}/agents`);
  await page
    .getByRole("button", { name: "Connect coding agent", exact: true })
    .click();
  dialog = page.getByRole("dialog");
  await expect(dialog.getByText(/recollect-agent pair/)).not.toBeVisible();
  await expect(dialog.getByTestId("agent-setup-command")).toContainText(
    `/api/brains/${brain}/mcp/agent`,
  );
  await expect(
    dialog.getByLabel("Store your access token", { exact: true }),
  ).toBeVisible();
  await dialog.getByLabel("Coding host", { exact: true }).click();
  await page.getByRole("option", { name: "Claude Code", exact: true }).click();
  await expect(dialog.getByTestId("agent-setup-command")).toContainText(
    '"Authorization": "Bearer ${RECOLLECT_MCP_TOKEN}"',
  );
  await expect(dialog.getByTestId("agent-credential-command")).toContainText(
    "export RECOLLECT_MCP_TOKEN",
  );
  await expect(
    dialog.getByLabel("Store your access token", { exact: true }),
  ).toHaveCount(0);
  await expect(
    dialog.getByLabel("Session integration", { exact: true }),
  ).toHaveCount(0);
  await page.screenshot({
    path: "../.cache/mcp-tools-desktop.png",
    fullPage: false,
  });
  expect(errors).toEqual([]);
});
