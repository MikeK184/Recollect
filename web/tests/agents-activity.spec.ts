import { test, expect } from "@playwright/test";
import { randomUUID } from "node:crypto";

test("agent inspector keeps the roster visible, expands individual evidence and clears denied data", async ({
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
  const csrf = (await (await page.request.get("/api/auth/me")).json())
    .csrf_token;
  const created = await page.request.post("/api/brains", {
    data: { name: "Compact agent activity proof" },
    headers: { "x-csrf-token": csrf, "Idempotency-Key": randomUUID() },
  });
  expect(created.ok()).toBe(true);
  const brain = (await created.json()).id;
  const device = randomUUID(),
    eventId = randomUUID(),
    source = randomUUID(),
    version = randomUUID();
  let allowed = true;
  const respond = (body: unknown) => ({
    status: allowed ? 200 : 403,
    contentType: "application/json",
    body: JSON.stringify(
      allowed
        ? body
        : { message: "Activity access denied", error: "forbidden" },
    ),
  });
  await page.route(`**/api/brains/${brain}/agents*`, (route) =>
    route.fulfill(
      respond({
        groups: [
          {
            user_name: "Activity owner",
            agents: [
              {
                device_id: device,
                name: "Clean agent",
                host_kind: "codex",
                integration: "plugin",
                active: true,
                can_revoke: false,
                last_used_on_brain_at: new Date().toISOString(),
              },
            ],
          },
        ],
        hidden_count: 0,
      }),
    ),
  );
  const now = new Date().toISOString();
  await page.route(`**/api/brains/${brain}/capture/events*`, (route) =>
    route.fulfill(
      respond({
        total: 1,
        offset: 0,
        items: [
          {
            user_name: "Activity owner",
            agent_name: "Clean agent",
            device_id: device,
            host: "codex",
            host_version: "fixture",
            source_available: true,
            selection: {
              repository_ids: [],
              area_ids: [],
              environment_id: null,
            },
            processing: "ready",
            event: {
              captured_at: now,
              kind: "prompt",
              outcome: "reported",
              coverage: ["partial_host_coverage"],
            },
            receipt: {
              event_id: eventId,
              received_at: now,
              state: "accepted",
              source_id: source,
              source_version_id: version,
            },
          },
        ],
      }),
    ),
  );
  await page.route(
    `**/api/brains/${brain}/sources/${source}/versions/${version}`,
    (route) =>
      route.fulfill(
        respond({
          version: {
            title: "Synthetic retained activity",
            created_at: now,
            retention_class: "raw_session",
            expires_at: null,
          },
          content: "Synthetic retained activity evidence",
        }),
      ),
  );
  await page.goto(`/brains/${brain}/agents`);
  await page.getByRole("button", { name: "Clean agent", exact: true }).click();
  const inspector = page.getByRole("complementary", {
    name: "Agent activity",
    exact: true,
  });
  await expect(inspector).toBeVisible();
  await expect(page.getByTestId("brain-agent-roster")).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    inspector.getByText("Credential enabled", { exact: true }),
  ).toBeVisible();
  await expect(
    inspector.getByText("Partial host coverage", { exact: true }),
  ).toBeVisible();
  const event = inspector.getByTestId("agent-activity-event");
  await expect(event).toHaveCount(1);
  await expect(
    inspector.getByRole("button", { name: "Open evidence" }),
  ).not.toBeVisible();
  await event.locator("summary").click();
  await expect(
    inspector.getByText("Processing: ready", { exact: true }),
  ).toBeVisible();
  await expect(
    inspector.getByText("Activity owner", { exact: true }),
  ).toHaveCount(1);
  await inspector.getByRole("button", { name: "Open evidence" }).click();
  const evidence = page.getByRole("dialog", {
    name: "Captured source evidence",
    exact: true,
  });
  await expect(evidence.getByTestId("capture-source-content")).toContainText(
    "Synthetic retained activity evidence",
  );
  allowed = false;
  await expect(
    page.getByRole("dialog", { name: "Captured source evidence", exact: true }),
  ).toHaveCount(0, { timeout: 12000 });
  await expect(page.getByTestId("agent-activity-event")).toHaveCount(0);
  await expect(
    inspector.getByText("Activity access denied", { exact: true }),
  ).toBeVisible();
  await inspector.getByRole("button", { name: "Close agent activity" }).click();
  await expect(inspector).toHaveCount(0);
});
