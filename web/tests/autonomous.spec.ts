import { openDetails } from "./desktop-helpers";
import { test, expect } from "@playwright/test";

test("Brain learns, revises and refreshes its handover without individual review", async ({
  page,
}) => {
  test.skip(
    process.env.RECOLLECT_TEST_OPENAI !== "1" ||
      process.env.RECOLLECT_TEST_MODEL_WORKER !== "1",
    "Opt in to the native worker and bounded synthetic OpenAI calls.",
  );
  test.setTimeout(180_000);
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
      const r = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify(body),
      });
      if (!r.ok)
        throw new Error("Synthetic autonomous fixture failed: " + r.status);
      return r.json();
    };
    const brain = await send("/api/brains", {
      name: "Autonomous memory proof",
    });
    const source = await send("/api/brains/" + brain.id + "/sources", {
      title: "Synthetic Amber configuration",
      media_type: "text/plain",
      retain_content: true,
      content: "Amber.port = 8080\n",
    });
    return { brain: brain.id, source: source.id, version: source.version.id };
  });
  const base = "/api/brains/" + fixture.brain;
  await page.goto("/brains/" + fixture.brain + "/settings?tab=ai");
  await openDetails(page, "Advanced model controls");
  await page
    .getByRole("button", { name: "Edit model policy", exact: true })
    .click();
  const policy = page.getByRole("dialog", {
    name: "Model policy",
    exact: true,
  });
  await expect(
    policy.getByLabel("Maintain memory autonomously", { exact: true }),
  ).toBeChecked();
  await policy.getByLabel("Allow model transmission", { exact: true }).check();
  await policy
    .getByLabel("Daily token allowance", { exact: true })
    .fill("20000");
  await policy
    .getByRole("button", { name: "Save model policy", exact: true })
    .click();
  await expect(policy).not.toBeVisible();
  const learning = async (version: string) => {
    await expect
      .poll(
        () =>
          page.evaluate(
            async ({ base, version }) => {
              const r = await (await fetch(base + "/learning")).json();
              return r.items.some(
                (v: { source_version_id: string; state: string }) =>
                  v.source_version_id === version &&
                  !["queued", "running"].includes(v.state),
              );
            },
            { base, version },
          ),
        { timeout: 60_000, intervals: [500, 1000] },
      )
      .toBe(true);
    return page.evaluate(
      async ({ base, version }) => {
        const runs = await (await fetch(base + "/learning")).json();
        return runs.items.find(
          (v: { source_version_id: string }) => v.source_version_id === version,
        );
      },
      { base, version },
    );
  };
  const learned = await learning(fixture.version);
  expect(learned.state, JSON.stringify(learned)).toBe("succeeded");
  expect(learned.accepted).toBeGreaterThan(0);
  const claim = await page.evaluate(
    async ({ base, ids }) => {
      for (const id of ids) {
        const d = await (await fetch(base + "/claims/" + id)).json();
        if (d.selected.revision.content.predicate === "port") return d.selected;
      }
      throw new Error("The real model did not extract the literal port.");
    },
    { base, ids: learned.claim_ids },
  );
  expect(claim.revision.review).toBe("accepted");
  expect(claim.revision.reviewer_id).toBeNull();
  expect(claim.eligibility.strict_accepted).toBe(true);
  expect(claim.eligibility.strict_operational).toBe(false);
  await page.evaluate(
    async ({ base, id }) => {
      const me = await (await fetch("/api/auth/me")).json();
      const r = await fetch(base + "/handovers", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify({
          title: "Amber configuration handover",
          contributions: [id],
          operation_id: null,
        }),
      });
      if (!r.ok)
        throw new Error("Synthetic handover request failed: " + r.status);
    },
    { base, id: claim.revision.id },
  );
  await expect
    .poll(
      () =>
        page.evaluate(async (base) => {
          const r = await (await fetch(base + "/handovers")).json();
          return r.items.some(
            (v: { state: string }) => !["queued", "running"].includes(v.state),
          );
        }, base),
      { timeout: 60_000, intervals: [500, 1000] },
    )
    .toBe(true);
  const original = await page.evaluate(
    async (base) => (await (await fetch(base + "/handovers")).json()).items[0],
    base,
  );
  expect(original.state, JSON.stringify(original)).toBe("succeeded");
  const next = await page.evaluate(
    async ({ base, fixture }) => {
      const me = await (await fetch("/api/auth/me")).json();
      const r = await fetch(base + "/sources/" + fixture.source + "/versions", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": me.csrf_token,
        },
        body: JSON.stringify({
          base_version: fixture.version,
          title: "Updated synthetic Amber configuration",
          media_type: "text/plain",
          retain_content: true,
          content:
            "Amber.port = 9090\nThis replaces the previous port 8080 declaration for the same current configuration.\n",
        }),
      });
      if (!r.ok) throw new Error("Synthetic source change failed: " + r.status);
      return r.json();
    },
    { base, fixture },
  );
  const revised = await learning(next.version.id);
  expect(revised.state, JSON.stringify(revised)).toBe("succeeded");
  expect(revised.revised).toBeGreaterThan(0);
  const current = await page.evaluate(
    async ({ base, id }) =>
      (await (await fetch(base + "/claims/" + id)).json()).selected,
    { base, id: claim.revision.claim_id },
  );
  expect(current.revision.content.value).toBe("9090");
  expect(current.revision.reviewer_id).toBeNull();
  expect(current.eligibility.strict_accepted).toBe(true);
  await expect
    .poll(
      () =>
        page.evaluate(
          async ({ base, id }) => {
            const r = await (await fetch(base + "/handovers")).json();
            return r.items.some(
              (v: { claim_id: string; automatic: boolean; state: string }) =>
                v.claim_id === id &&
                v.automatic &&
                !["queued", "running"].includes(v.state),
            );
          },
          { base, id: original.claim_id },
        ),
      { timeout: 60_000, intervals: [500, 1000] },
    )
    .toBe(true);
  const refresh = await page.evaluate(
    async ({ base, id }) => {
      const r = await (await fetch(base + "/handovers")).json();
      return r.items.find(
        (v: { claim_id: string; automatic: boolean }) =>
          v.claim_id === id && v.automatic,
      );
    },
    { base, id: original.claim_id },
  );
  expect(refresh.state, JSON.stringify(refresh)).toBe("succeeded");
  const result = await page.evaluate(
    async ({ base, id }) =>
      (await (await fetch(base + "/claims/" + id)).json()).selected,
    { base, id: original.claim_id },
  );
  expect(result.eligibility.strict_accepted).toBe(true);
  expect(result.revision.reviewer_id).toBeNull();
  expect(result.revision.content.handover.contributions).toContain(
    current.revision.id,
  );
  await page.reload();
  await openDetails(page, "Advanced model controls");
  await expect(page.getByText(/Autonomous memory is configured/)).toBeVisible();
  await page.goto(`/brains/${fixture.brain}/activity?tab=models`);
  await page
    .getByRole("heading", { name: "Model usage and learning", exact: true })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: "../.cache/ui-autonomous-desktop.png" });
  await page.goto(`/brains/${fixture.brain}/memory?tab=handover`);
  await page
    .getByRole("button", { name: "Inspect generated handover", exact: true })
    .first()
    .click();
  await expect(
    page.getByRole("dialog", { name: "Claim and knowledge history" }),
  ).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 800 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth + 2,
    ),
  ).toBe(true);
  await page.screenshot({ path: "../.cache/ui-autonomous-1280.png" });
  expect(errors).toEqual([]);
  const usage = await page.evaluate(
    async (base) => await (await fetch(base + "/models/usage")).json(),
    base,
  );
  expect(
    usage.requests.filter((r: { state: string }) => r.state === "succeeded"),
  ).toHaveLength(4);
});
