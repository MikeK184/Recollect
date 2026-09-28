import { test, expect } from "@playwright/test";

test("author procedures, compose handovers and preserve contribution inspection", async ({
  page,
}) => {
  test.setTimeout(180_000);
  page.setDefaultTimeout(15_000);
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
  await expect(
    page.getByRole("button", { name: "Create Brain", exact: true }),
  ).toBeVisible();
  const fixture = await page.evaluate(async () => {
    const me = await (await fetch("/api/auth/me")).json();
    const send = async (path: string, body: unknown) => {
      const response = await fetch(path, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!response.ok)
        throw new Error("Procedure fixture failed: " + response.status);
      return response.json();
    };
    const brain = await send("/api/brains", {
      name: "Procedures and handovers proof",
    });
    const source = await send("/api/brains/" + brain.id + "/sources", {
      title: "Synthetic procedure evidence",
      media_type: "text/plain",
      retain_content: true,
      content:
        "Amber.port = 8080\nThe local synthetic status check succeeded.\n",
    });
    return { brain: brain.id, source: source.id };
  });
  await page.goto("/brains/" + fixture.brain + "/memory");
  await page.getByRole("button", { name: "Add memory", exact: true }).click();
  await page.getByRole("button", { name: "Structured entry", exact: true }).click();
  let dialog = page.getByRole("dialog");
  await dialog
    .getByRole("textbox", { name: "Memory kind", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Procedure / runbook", exact: true })
    .click();
  await dialog
    .getByRole("textbox", { name: "Subject", exact: true })
    .fill("Inspect synthetic Amber");
  await dialog
    .getByRole("textbox", { name: "Property or relationship", exact: true })
    .fill("procedure");
  await dialog
    .getByRole("textbox", { name: "Claim value", exact: true })
    .fill("Inspect the local fixture port.");
  await dialog
    .getByRole("textbox", { name: "Procedure conditions", exact: true })
    .fill("Use the isolated test fixture.");
  await dialog
    .getByRole("textbox", { name: "Ordered procedure steps", exact: true })
    .fill("Inspect the status.\nRecord the port.");
  await dialog
    .getByRole("textbox", { name: "Expected outcome", exact: true })
    .fill("The port is 8080.");
  await expect(
    dialog.getByText("Untested: no procedure observations recorded.", {
      exact: true,
    }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Use evidence", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "Add procedure observation", exact: true })
    .click();
  await dialog
    .getByRole("textbox", { name: "Tested conditions 1", exact: true })
    .fill("Isolated local test.");
  await dialog
    .getByRole("textbox", { name: "Observed result 1", exact: true })
    .fill("The status check returned port 8080.");
  await dialog
    .getByRole("textbox", { name: "Observation evidence 1", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Synthetic procedure evidence", exact: true })
    .click();
  await page.keyboard.press("Escape");
  await dialog
    .getByRole("button", { name: "Save proposal", exact: true })
    .click();
  await expect(
    dialog.getByText("Observed success", { exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText("Review: proposed", { exact: true }),
  ).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 800 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: "../.cache/ui-procedure-1280.png" });
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.getByRole("button", { name: "Add memory", exact: true }).click();
  await page.getByRole("button", { name: "Structured entry", exact: true }).click();
  dialog = page.getByRole("dialog");
  await dialog
    .getByRole("textbox", { name: "Memory kind", exact: true })
    .click();
  await page.getByRole("option", { name: "Handover", exact: true }).click();
  await dialog
    .getByRole("textbox", { name: "Subject", exact: true })
    .fill("Amber investigation handover");
  await dialog
    .getByRole("textbox", { name: "Claim value", exact: true })
    .fill("The linked local procedure records a successful synthetic check.");
  await dialog
    .getByRole("checkbox", {
      name: "Inspect synthetic Amber · procedure",
      exact: true,
    })
    .check();
  await expect(
    dialog.getByRole("button", { name: "Remove contribution", exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("textbox", { name: "Completed work", exact: true })
    .fill("Recorded the local test.");
  await dialog
    .getByRole("textbox", { name: "Next steps", exact: true })
    .fill("Review the observation.");
  await dialog
    .getByRole("textbox", { name: "Risks and open questions", exact: true })
    .fill("Customer runtime has not been checked.");
  await dialog
    .getByRole("button", { name: "Save proposal", exact: true })
    .click();
  await expect(
    dialog.getByText("Exact contributions", { exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Inspect synthetic Amber", exact: true })
    .click();
  await expect(
    dialog.getByText("Observed success", { exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(
    dialog.getByText("Exact contributions", { exact: true }),
  ).toBeVisible();
  await page.screenshot({ path: "../.cache/ui-handover-desktop.png" });
  await page.keyboard.press("Escape");
  await page.getByRole("tab", { name: "Handovers", exact: true }).click();
  await page
    .getByRole("button", { name: "Generate handover", exact: true })
    .click();
  dialog = page.getByRole("dialog", { name: "Generate handover", exact: true });
  await dialog
    .getByRole("textbox", { name: "Handover title", exact: true })
    .fill("Generated synthetic Amber handover");
  await dialog
    .getByRole("checkbox", {
      name: "Inspect synthetic Amber · procedure",
      exact: true,
    })
    .check();
  await dialog
    .getByRole("button", { name: "Queue handover", exact: true })
    .click();
  await expect(dialog.getByRole("alert")).toContainText(
    "has not allowed this provider",
  );
  if (process.env.RECOLLECT_TEST_OPENAI === "1") {
    await page.evaluate(async (brain) => {
      const base = "/api/brains/" + brain;
      const settings = await (await fetch(base + "/models/policy")).json();
      const me = await (await fetch("/api/auth/me")).json();
      const response = await fetch(base + "/models/policy", {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify({
          base_change: settings.current.change_id,
          policy: {
            ...settings.current.policy,
            enabled: true,
            autonomous_memory: false,
            purposes: ["synthesis"],
            content_classes: ["claim", "query"],
          },
        }),
      });
      if (!response.ok) throw new Error("Synthetic synthesis policy failed.");
    }, fixture.brain);
    await dialog
      .getByRole("button", { name: "Queue handover", exact: true })
      .click();
    await expect(
      page.getByRole("button", {
        name: "Inspect generated handover",
        exact: true,
      }),
    ).toBeVisible({ timeout: 90000 });
    await page
      .getByRole("button", { name: "Inspect generated handover", exact: true })
      .click();
    dialog = page.getByRole("dialog");
    await expect(
      dialog.getByText(/Model derivation: gpt-5.6-luna · handover-1/),
    ).toBeVisible();
    await expect(
      dialog.getByText("Review: proposed", { exact: true }),
    ).toBeVisible();
    const proof = await page.evaluate(async (brain) => {
      const base = "/api/brains/" + brain;
      const runs = await (await fetch(base + "/handovers")).json();
      const claim = await (
        await fetch(base + "/claims/" + runs.items[0].claim_id)
      ).json();
      const usage = await (await fetch(base + "/models/usage")).json();
      return {
        origin: claim.selected.revision.origin,
        reviewer: claim.selected.revision.reviewer_id,
        runs,
        usage,
      };
    }, fixture.brain);
    expect(proof.origin).toBe("model_synthesized");
    expect(proof.reviewer).toBeNull();
    expect(proof.usage.total).toBe(1);
    expect(proof.usage.requests[0].purpose).toBe("synthesis");
    expect(proof.runs.items[0].state).toBe("succeeded");
    await page.setViewportSize({ width: 1280, height: 800 });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page.screenshot({
      path: "../.cache/ui-generated-handover-1280.png",
    });
  }
  expect(errors).toEqual([]);
});
