import { test, expect } from "@playwright/test";

test("review, correct, resolve conflicts and surface stale reviewer decisions", async ({
  page,
}) => {
  test.setTimeout(90_000);
  page.setDefaultTimeout(10_000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
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
  const fixture = await page.evaluate(async () => {
    const me = await (await fetch("/api/auth/me")).json();
    const send = async (url: string, body: unknown) => {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!response.ok)
        throw new Error(`Fixture request failed: ${response.status}`);
      return response.json();
    };
    const brain = await send("/api/brains", { name: "Browser review proof" });
    const base = `/api/brains/${brain.id}`;
    const source = await send(`${base}/sources`, {
      title: "Synthetic review source",
      media_type: "text/plain",
      content: "Synthetic review values.\n",
      retain_content: true,
      source_uri: null,
      group_ids: [],
      observed_at: null,
    });
    const proposal = (subject: string, value: string) => ({
      base_revision: null,
      operation_id: null,
      content: {
        kind: "claim",
        subject,
        predicate: "configuration",
        value,
        rationale: "Synthetic browser proof.",
        selection: { repository_ids: [], area_ids: [], environment_id: null },
        manifest_revision_id: null,
        validity: {
          kind: "interval",
          from: "2026-01-01T00:00:00Z",
          to: "2026-02-01T00:00:00Z",
          precision: "second",
        },
        freshness: "current",
        operational: "declared",
        observed_at: null,
        observation: "",
        supports: [
          {
            kind: "source_version",
            id: source.version.id,
            line_from: 1,
            line_to: 1,
          },
        ],
      },
    });
    const single = await send(
      `${base}/claims`,
      proposal("Review service", "original value"),
    );
    for (const value of ["first alternative", "second alternative"])
      await send(`${base}/claims`, proposal("Conflicting service", value));
    for (const value of ["early value", "later value"])
      await send(`${base}/claims`, proposal("Conditional service", value));
    return { brain: brain.id, single: single.claim_id };
  });
  await page.goto(`/brains/${fixture.brain}`);
  await page
    .getByRole("button", {
      name: "Review service · configuration",
      exact: true,
    })
    .click();
  await page
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  let dialog = page.getByRole("dialog");
  await dialog
    .getByLabel(/^Review reason/)
    .fill("I checked the retained synthetic source.");
  let first = true;
  await page.route(`**/claims/${fixture.single}/review`, async (route) => {
    if (route.request().method() === "POST" && first) {
      first = false;
      await route.fetch();
      await route.abort("failed");
    } else await route.continue();
  });
  await dialog
    .getByRole("button", { name: "Confirm review", exact: true })
    .click();
  await expect(
    dialog.getByText("Review failed", { exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText(
      "This claim changed while review was open. Retry an interrupted request, or return to the claim to inspect its latest revision.",
      { exact: true },
    ),
  ).toBeVisible({ timeout: 7000 });
  await dialog
    .getByRole("button", { name: "Confirm review", exact: true })
    .click();
  await expect(
    dialog.getByRole("heading", { name: "Review recorded", exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText("Eligible for strict accepted context", { exact: true }),
  ).toBeVisible();
  await page.unroute(`**/claims/${fixture.single}/review`);
  await dialog
    .getByRole("button", { name: "Return to claim", exact: true })
    .click();
  await expect(
    dialog.getByText("Review: accepted", { exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "Edit reviewed content", exact: true })
    .click();
  await dialog
    .getByRole("textbox", { name: "Claim value", exact: true })
    .fill("corrected value");
  await dialog
    .getByRole("button", { name: "Use this content", exact: true })
    .click();
  await dialog
    .getByLabel(/^Review reason/)
    .fill(
      "The original value was incorrect; this is the supported replacement.",
    );
  await dialog
    .getByRole("button", { name: "Confirm review", exact: true })
    .click();
  await expect(
    dialog.getByRole("heading", { name: "Review recorded", exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText("corrected value", { exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "../.cache/ui-review-desktop.png",
    fullPage: false,
  });
  await dialog
    .getByRole("button", { name: "Return to claim", exact: true })
    .click();
  await expect(dialog.getByTestId("claim-value")).toHaveText("corrected value");
  await page.keyboard.press("Escape");

  await page
    .getByRole("button", {
      name: "Conflicting service · configuration",
      exact: true,
    })
    .first()
    .click();
  await page
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  dialog = page.getByRole("dialog");
  await dialog
    .getByRole("button", { name: "Inspect conflicting evidence", exact: true })
    .click();
  await expect(
    dialog.getByRole("button", { name: /Synthetic review source · retained/ }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Back to review", exact: true })
    .click();
  await dialog
    .getByRole("checkbox", { name: /Include .* in resolution/ })
    .check();
  await dialog
    .getByLabel(/^Review reason/)
    .fill("The selected value has the applicable evidence.");
  await dialog
    .getByRole("button", { name: "Confirm conflict resolution", exact: true })
    .click();
  await expect(
    dialog.getByRole("heading", { name: "Review recorded", exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText("Eligible for strict accepted context", { exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Return to claim", exact: true })
    .click();
  await page.keyboard.press("Escape");

  await page
    .getByRole("button", {
      name: "Conditional service · configuration",
      exact: true,
    })
    .first()
    .click();
  await page
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  await dialog
    .getByRole("checkbox", { name: /Include .* in resolution/ })
    .check();
  await dialog
    .getByRole("textbox", { name: "Conflict resolution", exact: true })
    .click();
  await page
    .getByRole("option", {
      name: "Keep both with disjoint applicability",
      exact: true,
    })
    .click();
  for (const [value, from, to] of [
    ["early value", "2026-01-01T00:00", "2026-01-15T00:00"],
    ["later value", "2026-01-15T00:00", "2026-02-01T00:00"],
  ]) {
    await dialog
      .getByRole("button", {
        name: `Adjust applicability for ${value}`,
        exact: true,
      })
      .click();
    await dialog
      .getByLabel("Valid from (UTC; blank means unknown)", { exact: true })
      .fill(from);
    await dialog
      .getByLabel("Valid until (UTC; exclusive, blank means unknown)", {
        exact: true,
      })
      .fill(to);
    await dialog
      .getByRole("button", { name: "Use this content", exact: true })
      .click();
  }
  await dialog
    .getByLabel(/^Review reason/)
    .fill("These values apply in separate, observed periods.");
  await dialog
    .getByRole("button", { name: "Confirm conflict resolution", exact: true })
    .click();
  await expect(
    dialog.getByRole("heading", { name: "Review recorded", exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText("Eligible for strict accepted context", { exact: true }),
  ).toHaveCount(2);
  await dialog
    .getByRole("button", { name: "Return to claim", exact: true })
    .click();
  await page.keyboard.press("Escape");

  await page
    .getByRole("button", {
      name: "Review service · configuration",
      exact: true,
    })
    .click();
  await page
    .getByRole("button", { name: "Review and corrections", exact: true })
    .click();
  await dialog
    .getByRole("textbox", { name: "Review action", exact: true })
    .click();
  await page.getByRole("option", { name: "Withdraw", exact: true }).click();
  await dialog
    .getByLabel(/^Review reason/)
    .fill("Attempt made with an older review revision.");
  await page.evaluate(async ({ brain, single }) => {
    const me = await (await fetch("/api/auth/me")).json();
    const endpoint = `/api/brains/${brain}/claims/${single}`;
    const current = await (await fetch(endpoint)).json();
    const response = await fetch(`${endpoint}/review`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": me.csrf_token,
      },
      body: JSON.stringify({
        base_revision: current.current_revision,
        action: "reject",
        reason: "A competing reviewer rejected this assertion.",
        content: null,
        revalidation_basis: null,
      }),
    });
    if (!response.ok) throw new Error("Competing review failed.");
  }, fixture);
  await dialog
    .getByRole("button", { name: "Confirm review", exact: true })
    .click();
  await expect(
    dialog.getByText("This claim changed. Reload before reviewing it.", {
      exact: true,
    }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});
