import { test, expect, type Page } from "@playwright/test";

async function command(
  page: Page,
  path: string,
  body?: unknown,
  method = body === undefined ? "GET" : "POST",
) {
  const me = await (await page.request.get("/api/auth/me")).json();
  const response = await page.request.fetch(path, {
    method,
    data: body,
    headers: { "x-csrf-token": me.csrf_token },
  });
  expect(response.ok(), `${method} ${path}: ${response.status()}`).toBeTruthy();
  return response.status() === 204 ? null : response.json();
}

async function fixture(page: Page) {
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
  const brain = await command(page, "/api/brains", {
    name: `Inline cards ${crypto.randomUUID()}`,
  });
  const base = `/api/brains/${brain.id}`;
  const description = [
    "Read **approved** metadata.",
    "",
    "```json",
    '{"region":"test"}',
    "```",
    "",
    "```toml",
    'region = "test"',
    "```",
    "",
    "```yaml",
    "region: test",
    "```",
    "",
    "```bash",
    "echo test",
    "```",
    "",
    "<script>window.inlineInjected=true</script>",
    "![tracking](https://example.invalid/tracking.png)",
  ].join("\n");
  const definition = {
    key: `inline-${crypto.randomUUID()}`,
    name: "Inline proof",
    transport: "stdio",
    command: "/usr/bin/false",
    arguments: [],
    placements: ["central"],
    credential_aliases: [],
    configuration_schema: { type: "object", additionalProperties: false },
    tools: [
      {
        name: "inspect",
        description,
        inputSchema: {
          type: "object",
          properties: { region: { type: "string" } },
          additionalProperties: false,
        },
      },
    ],
  };
  await command(page, "/api/mcp/definitions", definition);
  const connection = await command(page, `${base}/mcp/connections`, {
    name: "Inline MCP",
    definition_key: definition.key,
    target: "inert",
    placement: "central",
    configuration: {},
  });
  const detail = await command(page, `${base}/mcp/profiles`, {
    name: "Inline group",
    connection_ids: [connection.summary.id],
  });
  return { brain, base, connection, profile: detail.profile, description };
}

test("stable inspector preserves drafts and highlights exact inert metadata", async ({
  page,
  context,
}) => {
  test.setTimeout(120_000);
  const f = await fixture(page);
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto(`/brains/${f.brain.id}/connections`);
  const inspector = page.getByRole("complementary", {
    name: "Connection details",
  });
  const before = await inspector.boundingBox();
  await inspector
    .getByRole("button", { name: "Edit connection", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Edit connection", exact: true }),
  ).not.toBeVisible();
  await expect(
    inspector.getByRole("heading", { name: "Test status", exact: true }),
  ).toBeVisible();
  await expect(
    inspector.getByRole("heading", { name: "Tool access", exact: true }),
  ).toBeVisible();
  const after = await inspector.boundingBox();
  expect(after!.width).toBe(before!.width);
  await inspector
    .getByLabel("Connection name", { exact: true })
    .fill("Cancelled name");
  await inspector.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(
    inspector.getByRole("heading", { name: "Inline MCP", exact: true }),
  ).toBeVisible();
  await inspector
    .getByRole("button", { name: "Edit connection", exact: true })
    .click();
  await inspector.getByText("More configuration", { exact: true }).click();
  await inspector
    .getByLabel("Description", { exact: true })
    .fill("Retained unsaved draft");
  const live = await command(
    page,
    `${f.base}/mcp/connections/${f.connection.summary.id}`,
  );
  await command(
    page,
    `${f.base}/mcp/connections/${f.connection.summary.id}`,
    {
      name: live.summary.name,
      description: "Changed elsewhere",
      definition_key: live.summary.definition_key,
      target: live.target,
      placement: live.summary.placement,
      runner_reference: live.runner_reference,
      credential_alias: live.credential_alias,
      configuration: live.configuration,
      enabled: live.summary.enabled,
      environment_id: live.summary.environment_id,
      base_revision: live.summary.revision,
    },
    "PUT",
  );
  await expect(inspector).toContainText("Your draft is retained");
  await expect(
    inspector.getByLabel("Description", { exact: true }),
  ).toHaveValue("Retained unsaved draft");
  await expect(
    inspector.getByRole("button", { name: "Save connection", exact: true }),
  ).toBeDisabled();
  await inspector
    .getByRole("button", { name: "Reload settings", exact: true })
    .click();
  await inspector.getByText("More configuration", { exact: true }).click();
  await expect(
    inspector.getByLabel("Description", { exact: true }),
  ).toHaveValue("Changed elsewhere");
  await inspector.getByRole("button", { name: "Cancel", exact: true }).click();
  await inspector.getByRole("tab", { name: "Tools", exact: true }).click();
  await inspector.getByText("Full description", { exact: true }).click();
  await expect(inspector.locator(".rc-markdown strong")).toHaveText("approved");
  for (const language of ["json", "toml", "yaml", "bash"]) {
    await expect(
      inspector
        .getByLabel(`${language} code`, { exact: true })
        .locator("span")
        .first(),
    ).toBeVisible();
  }
  await inspector
    .getByLabel("json code", { exact: true })
    .locator("..")
    .getByRole("button", { name: "Copy code", exact: true })
    .click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    '{"region":"test"}\n',
  );
  await expect(
    inspector.locator('img[src="https://example.invalid/tracking.png"]'),
  ).toHaveCount(0);
  expect(
    await page.evaluate(
      () => (window as unknown as { inlineInjected?: boolean }).inlineInjected,
    ),
  ).toBeUndefined();
});

test("card icons edit independent grants and retry only unfinished writes", async ({
  page,
  browser,
}) => {
  test.setTimeout(120_000);
  const f = await fixture(page);
  const username = `inline-reader-${crypto.randomUUID()}`;
  const invite = await command(page, "/api/team/invitations", { username });
  const guest = await browser.newContext();
  const response = await guest.request.post("/api/auth/enroll", {
    data: { token: invite.token, password: crypto.randomUUID() },
  });
  expect(response.ok()).toBeTruthy();
  const member = (await response.json()).user;
  await command(
    page,
    `${f.base}/grants/${member.id}`,
    { role: "reader" },
    "PUT",
  );
  await page.goto(`/brains/${f.brain.id}/connections?tab=profiles`);
  const card = page.getByTestId("mcp-profile");
  await expect(
    card.getByRole("img", { name: "Use for you: Denied", exact: true }),
  ).toHaveClass(/denied/);
  await expect(
    card.getByRole("img", { name: "Manage for you: Allowed", exact: true }),
  ).toHaveClass(/allowed/);
  await expect(
    card.getByRole("img", { name: "Share for you: Allowed", exact: true }),
  ).toHaveClass(/allowed/);
  await card
    .getByRole("button", { name: "Edit Inline group", exact: true })
    .click();
  await card
    .getByLabel("Tool group description", { exact: true })
    .fill("Saved in place");
  const owner = process.env.RECOLLECT_OWNER_USERNAME!;
  await card
    .getByRole("button", {
      name: `Use for ${owner} direct: Denied`,
      exact: true,
    })
    .click();
  await card.getByRole("button", { name: "Add person", exact: true }).click();
  await card.getByLabel("Username", { exact: true }).fill(username);
  await card.getByRole("button", { name: "Add", exact: true }).click();
  const attempts: string[] = [];
  const grantUrl = `**${f.base}/mcp/profiles/${f.profile.id}/grants`;
  let rejectOwner = true;
  await page.route(grantUrl, async (route) => {
    const body = route.request().postDataJSON();
    attempts.push(body.username);
    if (body.username === owner && rejectOwner) {
      rejectOwner = false;
      await route.fulfill({
        status: 503,
        json: { message: "Synthetic grant outage" },
      });
    } else await route.continue();
  });
  await card.getByRole("button", { name: "Save changes", exact: true }).click();
  await expect(card).toContainText("Some changes saved");
  await expect(card).toContainText("Group settings saved");
  await expect(card).toContainText(`${username} permissions saved`);
  await card
    .getByRole("button", { name: "Retry remaining changes", exact: true })
    .click();
  await expect(
    card.getByRole("button", { name: "Edit Inline group", exact: true }),
  ).toBeVisible();
  expect(attempts).toEqual([username, owner, owner]);
  await expect(
    card.getByRole("img", { name: "Use for you: Allowed", exact: true }),
  ).toHaveClass(/allowed/);
  const person = card
    .getByTestId("mcp-effective-member")
    .filter({ hasText: username });
  await expect(
    person.getByRole("img", {
      name: `Use for ${username}: Allowed`,
      exact: true,
    }),
  ).toHaveClass(/allowed/);
  await expect(
    person.getByRole("img", {
      name: `Manage for ${username}: Denied`,
      exact: true,
    }),
  ).toHaveClass(/denied/);
  const nameBox = await person.locator("strong").boundingBox();
  const permissionBox = await person
    .locator(".mcp-permission-icons")
    .boundingBox();
  expect(Math.abs(nameBox!.y - permissionBox!.y)).toBeLessThan(10);
  await page.unroute(grantUrl);
  await guest.close();
});
