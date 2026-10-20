import { test, expect, type Page } from "@playwright/test";
import { randomUUID } from "node:crypto";
import { spawn, type ChildProcess } from "node:child_process";
import { createServer } from "node:net";
import { once } from "node:events";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import type { components } from "../src/api-schema";

type Request = components["schemas"]["OrganizationRequest"];
type View = components["schemas"]["OrganizationView"];
type Command = components["schemas"]["TopicCommand"];
type AliasCommand = components["schemas"]["AliasCommand"];
let vite: ChildProcess, origin: string;
test.beforeAll(async () => {
  const server = createServer();
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string")
    throw Error("Missing fixture port");
  const port = address.port;
  await new Promise<void>((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  origin = `http://127.0.0.1:${port}`;
  vite = spawn(
    process.execPath,
    [
      "node_modules/vite/bin/vite.js",
      "--host",
      "127.0.0.1",
      "--port",
      String(port),
      "--strictPort",
    ],
    { cwd: fileURLToPath(new URL("../", import.meta.url)), stdio: "ignore" },
  );
  vite.unref();
  await expect
    .poll(
      async () => {
        try {
          return (await fetch(origin)).ok;
        } catch {
          return false;
        }
      },
      { timeout: 20_000 },
    )
    .toBe(true);
});
test.afterAll(async () => {
  if (vite?.exitCode === null && vite?.signalCode === null) {
    const exited = once(vite, "exit");
    vite.kill("SIGTERM");
    await Promise.race([
      exited,
      new Promise((resolve) => setTimeout(resolve, 2000)),
    ]);
  }
});

async function fixture(page: Page, role = "admin") {
  const brain = randomUUID(),
    topic = randomUUID(),
    secondTopic = randomUUID(),
    entity = randomUUID(),
    otherEntity = randomUUID(),
    environment = randomUUID(),
    fieldEntity = randomUUID(),
    versions = [randomUUID(), randomUUID()],
    sources = [randomUUID(), randomUUID()],
    inputs = [randomUUID(), randomUUID()];
  const now = new Date().toISOString();
  const brainData = {
    id: brain,
    name: "Platform engineering",
    description: "Shared source evidence",
    role,
    archived: false,
    owner_id: randomUUID(),
    created_at: now,
    updated_at: now,
  };
  const state = {
    denied: false,
    error: false,
    epoch: 1,
    organizationEpoch: null as number | null,
    partial: false,
    expires: null as string | null,
    change: randomUUID(),
    name: "Platform identity",
    pinned: false,
    excluded: false,
    split: false,
    requests: [] as Request[],
    writes: [] as { body: Command; key: string | undefined }[],
    delay: null as null | Promise<void>,
    contextOnly: false,
    representativeOnly: false,
    fieldEnabled: false,
    fieldError: false,
    unmapped: false,
    sourceOnly: false,
    manualOverlay: false,
    duplicateSourceTitles: false,
    sourcePageError: false,
    sourcePageExpires: null as string | null,
    sourceRequests: [] as components["schemas"]["OrganizationSourcesRequest"][],
    producer: "ready" as "ready" | "unconfigured" | "blocked",
    aliasStale: false,
    aliasExpires: null as string | null,
    aliasWrites: [] as {
      id: string;
      body: AliasCommand;
      key: string | undefined;
    }[],
    aliasStates: [0, 1, 2].map(() => ({
      disposition: "pending",
      change: null as string | null,
    })),
  };
  const aliasIds = [randomUUID(), randomUUID(), randomUUID()];
  const aliasVersions = [randomUUID(), randomUUID(), randomUUID()];
  const aliasSourceIds = [randomUUID(), randomUUID(), randomUUID()];
  const aliasTexts = [
    "app is also known as service.",
    "app is not known as service.",
    "app was known as service.",
  ];
  const mentionIds = [randomUUID(), randomUUID(), randomUUID()];
  const node = (index: number) => ({
    key: `source_version:${versions[index]}`,
    evidence: {
      id: sources[index],
      revision_id: versions[index],
      kind: "source_version",
      label: state.duplicateSourceTitles
        ? "Shared architecture guide"
        : index
          ? "Development handover"
          : "Production deployment",
      text: "",
      recorded_at: now,
      score: 1,
      channels: ["graph"],
      selection: { repository_ids: [], area_ids: [], environment_id: null },
      qualifications: [],
      claim: null,
      provenance: [
        {
          kind: "source_version",
          id: versions[index],
          source_id: sources[index],
          availability: "retained",
          repository_id: null,
          snapshot_id: null,
          revision: null,
        },
      ],
    },
  });
  function view(request: Request): View {
    const narrowed = request.scope.selection.environment_id === environment;
    const eligible = narrowed ? [node(0)] : [node(0), node(1)];
    const evidence =
      request.center?.kind === "topic" && request.center.id === secondTopic
        ? [node(0)]
        : eligible;
    const selected = state.excluded
      ? evidence.filter((item) => item.key !== `source_version:${versions[1]}`)
      : evidence;
    const name = narrowed ? "Production identity" : state.name;
    const memberships = selected.flatMap((item, index) => [
      {
        topic_id: topic,
        entity_id: entity,
        input_id: inputs[index],
        source_key: item.key,
        reason: "shared_entity",
        weight: 1,
      },
      ...(index === 0
        ? [
            {
              topic_id: secondTopic,
              entity_id: entity,
              input_id: inputs[index],
              source_key: item.key,
              reason: "shared_entity",
              weight: 1,
            },
          ]
        : []),
    ]);
    const actualMemberships =
      request.center?.kind === "topic"
        ? memberships.filter((member) => member.topic_id === request.center!.id)
        : memberships;
    const exactMentions = evidence.map((item, index) => ({
      id: mentionIds[index],
      entity_id: entity,
      input_id: inputs[index],
      source_key: item.key,
      byte_start: 0,
      byte_end: 5,
      quote: "Vault",
    }));
    exactMentions.push({
      id: mentionIds[2],
      entity_id: otherEntity,
      input_id: inputs[0],
      source_key: evidence[0].key,
      byte_start: 11,
      byte_end: 25,
      quote: "Azure identity",
    });
    const allCards = [
      {
        id: topic,
        kind: "topic",
        topic_kind: state.manualOverlay
          ? ("manual_overlay" as const)
          : undefined,
        label: name,
        source_count: selected.length,
        entity_count: state.sourceOnly ? 0 : 2,
        pinned: state.pinned,
        change_id: state.change,
      },
      {
        id: secondTopic,
        kind: "topic",
        label: state.split ? "Separate topic" : "Delivery",
        source_count: 1,
        entity_count: state.sourceOnly ? 0 : 1,
        pinned: false,
        change_id: randomUUID(),
      },
      {
        id: entity,
        kind: "service",
        label: "Vault service",
        source_count: evidence.length,
        entity_count: 1,
        pinned: false,
        change_id: null,
      },
      {
        id: otherEntity,
        kind: "technology",
        label: "Azure identity",
        source_count: 1,
        entity_count: 1,
        pinned: false,
        change_id: null,
      },
    ];
    const topics = allCards.filter(
      (card) =>
        card.kind === "topic" &&
        (!request.center ||
          request.center.kind !== "topic" ||
          request.center.id === card.id),
    );
    return {
      brain_id: brain,
      scope: request.scope,
      epoch: state.organizationEpoch ?? state.epoch,
      expires_at: state.expires,
      state: state.partial ? "partial" : "ready",
      producer_status: state.producer,
      community_status: state.sourceOnly
        ? { state: "ready", report_id: null, vector_features: "partial" }
        : { state: "unconfigured", report_id: null },
      topics: state.contextOnly || state.unmapped ? [] : topics,
      entities:
        state.contextOnly || state.sourceOnly || state.unmapped
          ? []
          : allCards.filter((card) => card.kind !== "topic"),
      context_cards: state.contextOnly ? allCards : [],
      evidence: state.unmapped
        ? []
        : state.sourceOnly
          ? [node(state.excluded ? 1 : 0)]
          : selected,
      mentions:
        state.sourceOnly || state.representativeOnly || state.unmapped
          ? []
          : exactMentions,
      representative_mentions:
        state.sourceOnly || state.unmapped
          ? []
          : [exactMentions[0], exactMentions[exactMentions.length - 1]],
      memberships: state.sourceOnly || state.unmapped ? [] : actualMemberships,
      source_memberships:
        state.sourceOnly && request.center?.kind !== "entity"
          ? [
              {
                topic_id: topic,
                input_id: inputs[state.excluded ? 1 : 0],
                source_key: node(state.excluded ? 1 : 0).key,
                reason: "retained_vector",
                weight: 1,
              },
            ]
          : [],
      representative_sources:
        state.sourceOnly && request.center?.kind !== "entity"
          ? [
              {
                topic_id: topic,
                input_id: inputs[state.excluded ? 1 : 0],
                source_key: node(state.excluded ? 1 : 0).key,
                provenance_kind: "source_document",
              },
            ]
          : [],
      source_next_cursor:
        state.sourceOnly && !state.excluded && request.center?.kind !== "entity"
          ? {
              brain_id: brain,
              after_source: null,
              through_source: sources[1],
              after_topic: topic,
              after_input: inputs[0],
              epoch: state.organizationEpoch ?? state.epoch,
              scope: request.scope,
              topic_id:
                request.center?.kind === "topic" ? request.center.id : null,
              expires_at: null,
            }
          : null,
      connections:
        state.sourceOnly || state.unmapped
          ? []
          : [
              {
                id: randomUUID(),
                from: entity,
                to: otherEntity,
                relation: "uses",
                interpretation: "source_described_navigation",
                source_key: evidence[0].key,
                byte_start: 0,
                byte_end: 26,
                quote: "Vault uses Azure identity.",
              },
            ],
      coverage: {
        examined_sources: narrowed ? 1 : 3,
        qualified_sources: narrowed || request.center ? evidence.length : 3,
        mapped_sources: state.unmapped ? 0 : selected.length,
        unassigned_sources: state.unmapped
          ? 3
          : narrowed || request.center
            ? 0
            : 1,
        counts_are_lower_bounds: state.partial,
      },
      next_cursor:
        state.partial && !request.cursor
          ? {
              after_source: null,
              card_offset: 25,
              row_cursor: { phase: "mentions", after_id: null },
              epoch: state.organizationEpoch ?? state.epoch,
              scope: request.scope,
              center: request.center,
              expires_at: null,
            }
          : null,
    };
  }
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname,
      method = route.request().method();
    const json = (body: unknown, status = 200) =>
      route.fulfill({
        status,
        contentType: "application/json",
        body: JSON.stringify(body),
      });
    if (path === "/api/auth/me")
      return json({
        user: {
          id: randomUUID(),
          username: "Graph owner",
          installation_owner: true,
        },
        csrf_token: "synthetic-csrf",
      });
    if (path === "/api/brains") return json([brainData]);
    if (path === `/api/brains/${brain}`)
      return state.denied
        ? json({ message: "Brain access revoked" }, 403)
        : json(brainData);
    if (path === "/api/status") return json({ ready: true });
    if (path === `/api/brains/${brain}/graph`)
      return state.denied
        ? json({ message: "Graph access revoked" }, 403)
        : json({
            memory_epoch: state.epoch,
            link_epoch: 1,
            organization_epoch: state.organizationEpoch ?? state.epoch,
            generations: [],
            jobs: [],
            nodes: 0,
            edges: 0,
          });
    if (path === `/api/brains/${brain}/organization/fields/view`) {
      if (state.denied || state.fieldError)
        return json(
          { message: "Canonical field qualification failed" },
          state.denied ? 403 : 500,
        );
      const request = route.request().postDataJSON();
      const evidence = state.fieldEnabled ? [node(0), node(1)] : [];
      return json({
        brain_id: brain,
        scope: request.scope,
        epoch: state.organizationEpoch ?? state.epoch,
        expires_at: state.expires,
        entities: state.fieldEnabled
          ? [
              {
                id: fieldEntity,
                kind: "environment",
                reference_id: environment,
                label: "PROD",
                label_pointer: "/name",
                contributor_count: evidence.length,
              },
            ]
          : [],
        origins: evidence.map((n, i) => ({
          id: inputs[i],
          entity_id: fieldEntity,
          input_id: inputs[i],
          origin_kind: "source_context",
          logical_id: sources[i],
          revision_id: versions[i],
          field_pointer: "/environments/0",
          value: environment,
          evidence_key: n.key,
        })),
        evidence,
        coverage: {
          examined_inputs: 3,
          qualified_inputs: 3,
          assigned_inputs: evidence.length,
          unassigned_inputs: 3 - evidence.length,
          counts_are_lower_bounds: false,
          refresh_state: "ready",
        },
        next_cursor: null,
      });
    }
    if (path === `/api/brains/${brain}/workspace`)
      return json({
        areas: [],
        repositories: [],
        environments: [],
        checkouts: [],
        workspaces: [],
        tasks: [],
        checkout_total: 0,
        checkout_offset: 0,
        task_total: 0,
        task_offset: 0,
      });
    if (path === `/api/brains/${brain}/evidence`)
      return json({
        groups: [
          {
            id: environment,
            brain_id: brain,
            kind: "environment",
            name: "Production",
            description: "",
            source_count: 1,
          },
        ],
        sources: [],
        total: 0,
        offset: 0,
      });
    if (path === `/api/brains/${brain}/organization/view`) {
      const request = route.request().postDataJSON() as Request;
      state.requests.push(request);
      const result = view(request);
      if (state.delay) await state.delay;
      if (state.denied)
        return json({ message: "Organization access revoked" }, 403);
      if (state.error)
        return json({ message: "Organization refresh failed" }, 503);
      return json(result).catch(() => {});
    }
    if (path === `/api/brains/${brain}/organization/sources/view`) {
      const request = route
        .request()
        .postDataJSON() as components["schemas"]["OrganizationSourcesRequest"];
      state.sourceRequests.push(request);
      if (state.sourcePageError || state.denied)
        return json({ message: "Source memberships changed" }, 409);
      return json({
        brain_id: brain,
        scope: request.scope,
        epoch: state.organizationEpoch ?? state.epoch,
        expires_at: state.sourcePageExpires,
        state: "partial",
        topics: [],
        evidence: [node(1)],
        source_memberships: [
          {
            topic_id: topic,
            input_id: inputs[1],
            source_key: node(1).key,
            reason: "retained_vector",
            weight: 0.8,
          },
        ],
        representative_sources: [],
        coverage: {
          examined_sources: 3,
          qualified_sources: 3,
          mapped_sources: 2,
          unassigned_sources: 1,
          counts_are_lower_bounds: true,
        },
        community_status: {
          state: "ready",
          report_id: null,
          vector_features: "partial",
        },
        next_cursor: null,
      });
    }
    if (path === `/api/brains/${brain}/organization/aliases/view`) {
      const request = route
        .request()
        .postDataJSON() as components["schemas"]["AliasRequest"];
      if (state.denied)
        return json({ message: "Alias evidence access revoked" }, 403);
      return json({
        brain_id: brain,
        scope: request.scope,
        epoch: state.organizationEpoch ?? state.epoch,
        expires_at: state.aliasExpires,
        candidates: aliasIds.map((id, index) => ({
          id,
          input_id: inputs[0],
          from: {
            id: entity,
            kind: "service",
            label: "app",
            source_count: 1,
            entity_count: 1,
            pinned: false,
            change_id: null,
          },
          to: {
            id: otherEntity,
            kind: "service",
            label: "service",
            source_count: 1,
            entity_count: 1,
            pinned: false,
            change_id: null,
          },
          source_key: `source_version:${aliasVersions[index]}`,
          byte_start: 0,
          byte_end: aliasTexts[index].length,
          quote: aliasTexts[index],
          disposition: state.aliasStates[index].disposition,
          change_id: state.aliasStates[index].change,
          acceptance_state:
            index === 0 ? "explicit_positive_statement" : "unsupported_context",
        })),
        evidence: aliasVersions.map((version, index) => ({
          ...node(0),
          key: `source_version:${version}`,
          evidence: {
            ...node(0).evidence,
            id: aliasSourceIds[index],
            revision_id: version,
            label: `Alias declaration ${index + 1}`,
            provenance: [
              {
                ...node(0).evidence.provenance[0],
                id: version,
                source_id: aliasSourceIds[index],
              },
            ],
          },
        })),
        next_cursor: null,
      });
    }
    if (
      path.startsWith(`/api/brains/${brain}/organization/aliases/`) &&
      method === "PUT"
    ) {
      const id = path.split("/").at(-1)!;
      const index = aliasIds.indexOf(id);
      const body = route.request().postDataJSON() as AliasCommand;
      state.aliasWrites.push({
        id,
        body,
        key: route.request().headers()["idempotency-key"],
      });
      if (
        state.aliasStale ||
        body.base_change_id !== state.aliasStates[index].change
      )
        return json({ message: "Organization inputs changed" }, 409);
      if (body.action === "accept" && index !== 0)
        return json({ message: "Unsupported alias evidence" }, 409);
      state.aliasStates[index] = {
        change: randomUUID(),
        disposition:
          body.action === "accept"
            ? "accepted"
            : body.action === "reject"
              ? "rejected"
              : "pending",
      };
      state.epoch++;
      return json({
        id,
        change_id: state.aliasStates[index].change,
        disposition: state.aliasStates[index].disposition,
      });
    }
    if (
      path.startsWith(`/api/brains/${brain}/organization/topics/`) &&
      method === "PUT"
    ) {
      const body = route.request().postDataJSON() as Command;
      state.writes.push({
        body,
        key: route.request().headers()["idempotency-key"],
      });
      if (body.base_change_id !== state.change)
        return json({ message: "Organization inputs changed" }, 409);
      if (
        body.change.action === "rename" ||
        body.change.action === "rename_source"
      )
        state.name = body.change.label;
      if (body.change.action === "pin") state.pinned = body.change.pinned;
      if (
        body.change.action === "exclude" ||
        body.change.action === "exclude_sources"
      )
        state.excluded = body.change.excluded;
      if (
        body.change.action === "split" ||
        body.change.action === "split_sources"
      )
        state.split = true;
      state.change = randomUUID();
      state.epoch++;
      return json({
        id: topic,
        change_id: state.change,
        split_topic_id:
          body.change.action === "split" ||
          body.change.action === "split_sources"
            ? secondTopic
            : null,
      });
    }
    if (
      path.startsWith(`/api/brains/${brain}/claim-evidence/source_version/`)
    ) {
      const version = path.split("/").at(-1)!;
      const index = versions.indexOf(version);
      const aliasIndex = aliasVersions.indexOf(version);
      return state.denied
        ? json({ message: "Evidence access revoked" }, 403)
        : json({
            evidence: {
              kind: "source_version",
              id: version,
              label:
                aliasIndex >= 0
                  ? `Alias declaration ${aliasIndex + 1}`
                  : index
                    ? "Development handover"
                    : "Production deployment",
              source_id:
                aliasIndex >= 0 ? aliasSourceIds[aliasIndex] : sources[index],
              created_at: now,
              availability: "retained",
              revision: null,
              reference: null,
              observed_at: null,
            },
            text:
              aliasIndex >= 0
                ? aliasTexts[aliasIndex]
                : "Vault uses Azure identity. Exact retained source text.",
            data: { source_version_id: version },
            expires_at: state.expires,
          });
    }
    return json({ message: "Unneeded fixture route" }, 503);
  });
  await page.goto(`${origin}/brains/${brain}/graph`);
  await expect(
    page.getByRole("region", { name: "Automatic knowledge organization" }),
  ).toBeVisible();
  await expect(page.getByTestId("organization-topic")).toHaveCount(2);
  return {
    brain,
    topic,
    secondTopic,
    entity,
    environment,
    versions,
    inputs,
    state,
  };
}

test("default topic cards expand actual evidence in3D and2D with overlapping memberships and exactsource inspector", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const f = await fixture(page);
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  await expect(
    page.getByText("3 eligible sources · 2 mapped · 1 unassigned in this read"),
  ).toBeVisible();
  await page
    .getByRole("button", {
      name: "Expand topic Platform identity",
      exact: true,
    })
    .click();
  await expect
    .poll(() => f.state.requests.at(-1)?.center)
    .toEqual({ kind: "topic", id: f.topic });
  const canvas = page.getByTestId("graph-canvas");
  await expect(canvas).toHaveAttribute("data-ready", "true");
  await expect(canvas).toHaveAttribute("data-nodes", "2");
  await expect(canvas).toHaveAttribute("data-edges", "0");
  await page
    .getByRole("textbox", { name: "Inspect graph entity", exact: true })
    .fill("Production deployment");
  await page.getByRole("option", { name: /^Production deployment ·/ }).click();
  const inspector = page.getByRole("complementary", {
    name: "Selected organization evidence",
  });
  await expect(inspector).toContainText("Exact UTF-8 bytes 0–5");
  await expect(inspector).toContainText("Vault");
  await inspector
    .getByRole("button", { name: "Open exact source", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(dialog.getByTestId("claim-evidence-text")).toHaveText(
    "Vault uses Azure identity. Exact retained source text.",
  );
  await dialog
    .getByRole("button", { name: "Close evidence", exact: true })
    .click();
  await expect(dialog).not.toBeVisible();
  await canvas.scrollIntoViewIfNeeded();
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  await expect(
    page.getByRole("region", { name: "Source-described entity relationships" }),
  ).toContainText("do not establish accepted operational truth");
  await mkdir("../.cache/ui", { recursive: true });
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/organization-expanded-wide.png",
    fullPage: true,
  });
  // ADR 0025: 1440×900 is the smallest supported layout (16-inch laptops).
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/organization-expanded-laptop.png",
    fullPage: true,
  });
  await page.getByText("2D", { exact: true }).click();
  await expect(page.getByLabel("Loaded topic memberships")).toContainText(
    "Platform identity · 2",
  );
  await expect(
    page.getByRole("textbox", { name: "Inspect graph entity", exact: true }),
  ).toHaveValue(/Production deployment/);
  await page.getByRole("button", { name: "All topics", exact: true }).click();
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  await expect(page.getByTestId("organization-topic")).toHaveCount(2);
  await expect(
    page.getByText("Reading current topics and source evidence…", {
      exact: true,
    }),
  ).toHaveCount(0);
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/organization-overview-laptop.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/organization-overview-wide.png",
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "Expand entity Vault service", exact: true })
    .click();
  await expect
    .poll(() => f.state.requests.at(-1)?.center)
    .toEqual({ kind: "entity", id: f.entity });
  expect(f.state.requests.every((request) => request.limit === 25)).toBe(true);
  expect(errors).toEqual([]);
});

test("scope changes, late responses, epoch errors and failed refresh clear semanticcards, graph and inspector", async ({
  page,
}) => {
  const f = await fixture(page);
  await page
    .getByRole("button", {
      name: "Expand topic Platform identity",
      exact: true,
    })
    .click();
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-ready",
    "true",
  );
  await page.getByRole("button", { name: "Scope", exact: true }).click();
  const scope = page.getByRole("dialog", {
    name: "Organization scope",
    exact: true,
  });
  await scope
    .getByRole("textbox", { name: "Organization environment", exact: true })
    .click();
  await page.getByRole("option", { name: "Production", exact: true }).click();
  await scope
    .getByRole("button", { name: "Apply organization scope", exact: true })
    .click();
  await expect(page.getByTestId("organization-topic").first()).toContainText(
    "Production identity",
  );
  await expect(
    page.getByText("Development handover", { exact: true }),
  ).toHaveCount(0);
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  let release!: () => void;
  f.state.delay = new Promise((resolve) => {
    release = resolve;
  });
  await page
    .getByRole("button", {
      name: "Expand topic Production identity",
      exact: true,
    })
    .click();
  await expect(
    page.getByText("Reading current topics and source evidence…"),
  ).toBeVisible();
  await page.getByRole("button", { name: "All topics", exact: true }).click();
  f.state.delay = null;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await expect(page.getByTestId("organization-topic")).toHaveCount(2);
  release();
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  f.state.error = true;
  f.state.epoch += 1;
  await expect(
    page.getByText("Organization refresh failed", { exact: false }),
  ).toBeVisible();
  await expect(page.getByTestId("organization-topic")).toHaveCount(0);
  await expect(page.getByTestId("organization-entity")).toHaveCount(0);
  await expect(
    page.getByRole("complementary", { name: "Selected organization evidence" }),
  ).toHaveCount(0);
  f.state.error = false;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await expect(page.getByTestId("organization-topic")).toHaveCount(2);
  f.state.denied = true;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await expect(page.getByTestId("organization-topic")).toHaveCount(0);
  await expect(
    page.getByText("Production identity", { exact: true }),
  ).toHaveCount(0);
});

test("topic commands carry currentversion and idempotency forrename, pin, exclude andsplit; readerhas noeditor", async ({
  page,
}) => {
  const f = await fixture(page);
  const organize = () =>
    page.getByRole("button", {
      name: `Organize topic ${f.state.name}`,
      exact: true,
    });
  await organize().click();
  let editor = page.getByRole("dialog", {
    name: "Organize automatic topic",
    exact: true,
  });
  await editor
    .getByRole("textbox", { name: "Name supported by", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Production deployment", exact: true })
    .click();
  await expect(
    editor.getByRole("button", { name: "Save topic name", exact: true }),
  ).toBeEnabled();
  await editor
    .getByRole("textbox", { name: "Topic name", exact: true })
    .fill("a".repeat(256));
  await expect(
    editor.getByRole("button", { name: "Save topic name", exact: true }),
  ).toBeEnabled();
  await editor
    .getByRole("textbox", { name: "Topic name", exact: true })
    .fill("Identity platform");
  await editor
    .getByRole("button", { name: "Save topic name", exact: true })
    .click();
  await expect(page.getByTestId("organization-topic").first()).toContainText(
    "Identity platform",
  );
  expect(f.state.writes[0].body.change).toEqual({
    action: "rename",
    label: "Identity platform",
    basis_input: f.inputs[0],
  });
  await organize().click();
  editor = page.getByRole("dialog", {
    name: "Organize automatic topic",
    exact: true,
  });
  await editor.getByRole("button", { name: "Pin topic", exact: true }).click();
  await expect(page.getByTestId("organization-topic").first()).toContainText(
    "Pinned",
  );
  await organize().click();
  editor = page.getByRole("dialog", {
    name: "Organize automatic topic",
    exact: true,
  });
  await editor
    .getByRole("checkbox", { name: "Development handover", exact: true })
    .check();
  await editor
    .getByRole("button", { name: "Exclude selected sources", exact: true })
    .click();
  await expect.poll(() => f.state.writes.length).toBe(3);
  await expect(page.getByTestId("organization-topic")).toHaveCount(2);
  await organize().click();
  editor = page.getByRole("dialog", {
    name: "Organize automatic topic",
    exact: true,
  });
  await editor
    .getByRole("checkbox", { name: "Production deployment", exact: true })
    .check();
  await editor
    .getByRole("button", { name: "Split selected into topic", exact: true })
    .click();
  await expect.poll(() => f.state.writes.length).toBe(4);
  expect(
    f.state.writes.every((write) => !!write.key && !!write.body.base_change_id),
  ).toBe(true);
  expect(
    new Set(f.state.writes.map((write) => write.body.base_change_id)).size,
  ).toBe(4);
  expect(f.state.writes[2].body.change).toEqual({
    action: "exclude",
    input_ids: [f.inputs[1]],
    excluded: true,
  });
  const reader = await page.context().newPage();
  await fixture(reader, "reader");
  await expect(
    reader.getByRole("button", { name: /^Organize topic / }),
  ).toHaveCount(0);
  await reader.close();
});

test("partial pagination and retention deadline remain honest and clear all derived labels", async ({
  page,
}) => {
  const f = await fixture(page);
  f.state.partial = true;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await expect(
    page.getByText("Counts are lower bounds", { exact: false }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Next organization page", exact: true })
    .click();
  await expect
    .poll(() => f.state.requests.at(-1)?.cursor?.card_offset)
    .toBe(25);
  f.state.expires = new Date(Date.now() + 800).toISOString();
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await expect(
    page.getByText("This organization reached its retention deadline.", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(page.getByTestId("organization-topic")).toHaveCount(0);
  await expect(page.getByTestId("organization-entity")).toHaveCount(0);
});

test("detail context cards and representative anchors remain navigable outside rankedcardpages", async ({
  page,
}) => {
  const f = await fixture(page);
  await page
    .getByRole("button", {
      name: "Expand topic Platform identity",
      exact: true,
    })
    .click();
  f.state.contextOnly = true;
  f.state.representativeOnly = true;
  f.state.partial = true;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await expect(
    page.getByText("Additional source context", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByText("No published topics yet", { exact: true }),
  ).toHaveCount(0);
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-nodes",
    "2",
  );
  await page
    .getByRole("textbox", { name: "Inspect graph entity", exact: true })
    .fill("Production deployment");
  await page.getByRole("option", { name: /^Production deployment ·/ }).click();
  const inspector = page.getByRole("complementary", {
    name: "Selected organization evidence",
  });
  await expect(inspector).toContainText("Vault service");
  await expect(inspector).toContainText("Azure identity");
  await expect(
    page.getByRole("region", { name: "Source-described entity relationships" }),
  ).toContainText("Vault service → Azure identity");
  await expect(page.getByLabel("Loaded topic memberships")).toContainText(
    "Platform identity",
  );
  await page.locator(".organization-context summary").click();
  await expect(
    page
      .getByRole("button", { name: "Organize context topic", exact: true })
      .first(),
  ).toBeVisible();
});

test("alias evidence preserves exact sources, human versioned decisions and unsupported contexts", async ({
  page,
}) => {
  const f = await fixture(page);
  await page
    .getByRole("button", { name: "Review aliases", exact: true })
    .click();
  const review = page.getByRole("region", {
    name: "Alias evidence review",
    exact: true,
  });
  const candidates = review.getByTestId("alias-candidate");
  await expect(candidates).toHaveCount(3);
  const positive = candidates.nth(0),
    negative = candidates.nth(1),
    history = candidates.nth(2);
  await expect(positive).toContainText("Exact UTF-8 bytes 0–29");
  await expect(negative).toContainText("app is not known as service.");
  await expect(history).toContainText("app was known as service.");
  await expect(
    negative.getByRole("button", {
      name: "Accept alias navigation",
      exact: true,
    }),
  ).toBeDisabled();
  await expect(
    history.getByRole("button", {
      name: "Accept alias navigation",
      exact: true,
    }),
  ).toBeDisabled();
  await positive
    .getByRole("button", { name: "Inspect exact alias source", exact: true })
    .click();
  const evidence = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(evidence.getByTestId("claim-evidence-text")).toHaveText(
    "app is also known as service.",
  );
  await evidence
    .getByRole("button", { name: "Close evidence", exact: true })
    .click();
  await expect(evidence).not.toBeVisible();
  await positive
    .getByRole("button", { name: "Accept alias navigation", exact: true })
    .click();
  await expect(positive).toContainText("accepted");
  expect(f.state.aliasWrites[0].body).toEqual({
    base_change_id: null,
    action: "accept",
  });
  expect(f.state.aliasWrites[0].key).toBeTruthy();
  const acceptedVersion = f.state.aliasStates[0].change;
  await positive
    .getByRole("button", { name: "Revoke alias decision", exact: true })
    .click();
  await expect(positive).toContainText("pending");
  expect(f.state.aliasWrites[1].body).toEqual({
    base_change_id: acceptedVersion,
    action: "revoke",
  });
  await negative
    .getByRole("button", { name: "Reject alias", exact: true })
    .click();
  await expect(negative).toContainText("rejected");
  await mkdir("../.cache/ui", { recursive: true });
  await page.screenshot({
    animations: "disabled",
    path: "../.cache/ui/organization-alias-review.png",
  });
  f.state.aliasStale = true;
  await positive
    .getByRole("button", { name: "Accept alias navigation", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "Organization inputs changed",
  );
  await expect(candidates).toHaveCount(0);
  await expect(page.getByTestId("organization-topic")).toHaveCount(0);
});

test("alias readers inspect without commands and alias deadlines clear every derived result", async ({
  page,
}) => {
  const f = await fixture(page, "reader");
  f.state.aliasExpires = new Date(Date.now() + 3000).toISOString();
  await page
    .getByRole("button", { name: "Review aliases", exact: true })
    .click();
  const review = page.getByRole("region", {
    name: "Alias evidence review",
    exact: true,
  });
  await expect(review.getByTestId("alias-candidate")).toHaveCount(3);
  await expect(
    review.getByRole("button", {
      name: "Accept alias navigation",
      exact: true,
    }),
  ).toHaveCount(0);
  await expect(
    review.getByRole("button", { name: "Reject alias", exact: true }),
  ).toHaveCount(0);
  await expect(
    review.getByRole("button", { name: "Revoke alias decision", exact: true }),
  ).toHaveCount(0);
  await expect(
    review
      .getByRole("button", { name: "Inspect exact alias source", exact: true })
      .first(),
  ).toBeEnabled();
  await expect(page.getByRole("alert")).toContainText(
    "Alias evidence reached its retention deadline",
  );
  await expect(page.getByTestId("alias-candidate")).toHaveCount(0);
  await expect(page.getByTestId("organization-topic")).toHaveCount(0);
  expect(f.state.aliasWrites).toHaveLength(0);
});

test("organization epoch heartbeat replaces another writer's topic and alias changes without memory epoch changes", async ({
  page,
}) => {
  const f = await fixture(page);
  f.state.organizationEpoch = f.state.epoch;
  await page
    .getByRole("button", {
      name: "Expand topic Platform identity",
      exact: true,
    })
    .click();
  await page
    .getByRole("textbox", { name: "Inspect graph entity", exact: true })
    .fill("Production deployment");
  await page.getByRole("option", { name: /^Production deployment ·/ }).click();
  const inspector = page.getByRole("complementary", {
    name: "Selected organization evidence",
    exact: true,
  });
  await expect(inspector).toContainText("Production deployment");
  f.state.name = "Identity renamed by another writer";
  f.state.organizationEpoch = 2;
  await expect(
    page.getByRole("heading", {
      name: "Identity renamed by another writer",
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.getByLabel("Loaded topic memberships")).toContainText(
    "Identity renamed by another writer",
  );
  await expect(inspector).not.toContainText("Production deployment");
  expect(f.state.epoch).toBe(1);
  expect(f.state.writes).toHaveLength(0);
  await page.getByRole("button", { name: "All topics", exact: true }).click();
  await page
    .getByRole("button", { name: "Review aliases", exact: true })
    .click();
  const candidate = page
    .getByRole("region", { name: "Alias evidence review", exact: true })
    .getByTestId("alias-candidate")
    .first();
  await expect(candidate).toContainText("pending");
  const version = randomUUID();
  f.state.aliasStates[0] = { disposition: "accepted", change: version };
  f.state.organizationEpoch = 3;
  await expect(candidate).toContainText("accepted");
  await expect(candidate).toContainText(version);
  expect(f.state.epoch).toBe(1);
  expect(f.state.aliasWrites).toHaveLength(0);
});

test("canonical field pointers remain distinct from source spans and failures clear every view", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const f = await fixture(page);
  f.state.fieldEnabled = true;
  f.state.unmapped = true;
  f.state.producer = "unconfigured";
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await expect(page.getByLabel("Entity producer readiness")).toContainText(
    "Entity extraction awaits a qualified compatible producer",
  );
  await expect(page.getByLabel("Native community readiness")).toContainText(
    "Native communities await a qualified recipe",
  );
  await expect(page.getByTestId("organization-entity")).toHaveCount(0);
  await expect(page.getByTestId("organization-field-entity")).toHaveCount(1);
  await mkdir("../.cache/ui", { recursive: true });
  await page.screenshot({
    path: "../.cache/ui/organization-fields-overview.png",
    fullPage: true,
    animations: "disabled",
  });
  await page
    .getByRole("button", {
      name: "Explore canonical reference PROD",
      exact: true,
    })
    .click();
  const canvas = page.getByTestId("graph-canvas");
  await expect(canvas).toHaveAttribute("data-ready", "true");
  await page
    .getByRole("button", {
      name: "Inspect canonical evidence Production deployment",
      exact: true,
    })
    .click();
  const inspector = page.getByRole("complementary", {
    name: "Canonical field inspector",
  });
  await expect(inspector).toContainText("/environments/0");
  await expect(inspector).toContainText(f.environment);
  await expect(inspector).toContainText(f.versions[0]);
  await expect(inspector).not.toContainText("UTF-8 bytes");
  await inspector
    .getByRole("button", { name: "Open canonical evidence", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(dialog.getByTestId("claim-evidence-text")).toHaveText(
    "Vault uses Azure identity. Exact retained source text.",
  );
  await dialog
    .getByRole("button", { name: "Close evidence", exact: true })
    .click();
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  await mkdir("../.cache/ui", { recursive: true });
  // ADR 0025 review sizes; below 1440 the inspector is a modal sheet.
  for (const width of [1728, 1440]) {
    await page.setViewportSize({ width, height: 960 });
    await canvas.scrollIntoViewIfNeeded();
    await expect(canvas).toHaveAttribute("data-layout-ready", "true");
    await page.screenshot({
      animations: "disabled",
      path: `../.cache/ui/organization-fields-${width}.png`,
    });
  }
  f.state.fieldError = true;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "Canonical field qualification failed",
  );
  await expect(page.getByTestId("organization-field-entity")).toHaveCount(0);
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  await expect(inspector).toHaveCount(0);
  expect(f.state.writes).toHaveLength(0);
  expect(f.state.aliasWrites).toHaveLength(0);
  expect(errors).toEqual([]);
});

test("source-only native groups page exact contributors and use source rename/exclude/split without entity anchors", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const f = await fixture(page);
  f.state.sourceOnly = true;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await expect(page.getByTestId("organization-entity")).toHaveCount(0);
  await expect(page.getByText("Source group", { exact: true })).toBeVisible();
  await page
    .getByRole("button", {
      name: "Expand topic Platform identity",
      exact: true,
    })
    .click();
  const canvas = page.getByTestId("graph-canvas");
  await expect(canvas).toHaveAttribute("data-nodes", "1");
  await page
    .getByRole("button", { name: "Next source membership page", exact: true })
    .click();
  await expect(canvas).toHaveAttribute("data-nodes", "2");
  expect(f.state.sourceRequests[0].topic_id).toBe(f.topic);
  await expect(
    page.getByText("Source membership coverage is partial.", { exact: false }),
  ).toBeVisible();
  await page
    .getByRole("textbox", { name: "Inspect graph entity", exact: true })
    .fill("Development handover");
  await page.getByRole("option", { name: /^Development handover ·/ }).click();
  const inspector = page.getByRole("complementary", {
    name: "Selected organization evidence",
  });
  await expect(inspector).toContainText("Source member of Platform identity");
  await expect(inspector).not.toContainText("Exact UTF-8 bytes");
  await inspector
    .getByRole("button", { name: "Open exact source", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(dialog.getByTestId("claim-evidence-text")).toContainText(
    "Exact retained source text",
  );
  await dialog
    .getByRole("button", { name: "Close evidence", exact: true })
    .click();
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  await mkdir("../.cache/ui", { recursive: true });
  // ADR 0025 review sizes; below 1440 the inspector is a modal sheet.
  for (const width of [1728, 1440]) {
    await page.setViewportSize({ width, height: 960 });
    await canvas.scrollIntoViewIfNeeded();
    await expect(canvas).toHaveAttribute("data-layout-ready", "true");
    await page.screenshot({
      path: `../.cache/ui/organization-source-groups-${width}.png`,
      animations: "disabled",
    });
  }
  await page.getByText("2D", { exact: true }).click();
  await expect(page.getByLabel("Loaded topic memberships")).toContainText(
    "Platform identity · 2",
  );
  await expect(
    page.getByRole("textbox", { name: "Inspect graph entity", exact: true }),
  ).toHaveValue(/Development handover/);
  const edit = () =>
    page.getByRole("button", {
      name: `Organize topic ${f.state.name}`,
      exact: true,
    });
  await edit().click();
  let editor = page.getByRole("dialog", {
    name: "Organize automatic topic",
    exact: true,
  });
  await editor
    .getByRole("textbox", { name: "Name supported by", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Production deployment", exact: true })
    .click();
  await expect(
    editor.getByRole("button", { name: "Save topic name", exact: true }),
  ).toBeEnabled();
  await editor
    .getByRole("textbox", { name: "Topic name", exact: true })
    .fill("é".repeat(120) + "a");
  await expect(
    editor.getByRole("button", { name: "Save topic name", exact: true }),
  ).toBeDisabled();
  await editor
    .getByRole("textbox", { name: "Topic name", exact: true })
    .fill("a".repeat(240));
  await expect(
    editor.getByRole("button", { name: "Save topic name", exact: true }),
  ).toBeEnabled();
  await editor
    .getByRole("textbox", { name: "Topic name", exact: true })
    .fill("Platform sources");
  await editor
    .getByRole("button", { name: "Save topic name", exact: true })
    .click();
  await expect.poll(() => f.state.writes.length).toBe(1);
  expect(f.state.writes[0].body.change).toEqual({
    action: "rename_source",
    label: "Platform sources",
    basis_input: f.inputs[0],
  });
  await edit().click();
  editor = page.getByRole("dialog", {
    name: "Organize automatic topic",
    exact: true,
  });
  await editor
    .getByRole("checkbox", { name: "Production deployment", exact: true })
    .check();
  await editor
    .getByRole("button", { name: "Exclude selected sources", exact: true })
    .click();
  await expect.poll(() => f.state.writes.length).toBe(2);
  expect(f.state.writes[1].body.change).toEqual({
    action: "exclude_sources",
    input_ids: [f.inputs[0]],
    excluded: true,
  });
  await edit().click();
  editor = page.getByRole("dialog", {
    name: "Organize automatic topic",
    exact: true,
  });
  await editor
    .getByRole("checkbox", { name: "Development handover", exact: true })
    .check();
  await editor
    .getByRole("button", { name: "Split selected into topic", exact: true })
    .click();
  await expect.poll(() => f.state.writes.length).toBe(3);
  expect(f.state.writes[2].body.change).toEqual({
    action: "split_sources",
    input_ids: [f.inputs[1]],
  });
  expect(f.state.writes.every((w) => !!w.key && !!w.body.base_change_id)).toBe(
    true,
  );
  expect(errors).toEqual([]);
});

test("manual overlays keep their identity and versioned source editor distinct in overview and context", async ({
  page,
}) => {
  const f = await fixture(page);
  f.state.sourceOnly = true;
  f.state.manualOverlay = true;
  f.state.duplicateSourceTitles = true;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  const topic = page.locator(`[data-topic-id="${f.topic}"]`);
  await expect(topic.getByText("Manual group", { exact: true })).toBeVisible();
  await expect(topic.getByText("Source group", { exact: true })).toHaveCount(0);
  await topic
    .getByRole("button", {
      name: "Expand topic Platform identity",
      exact: true,
    })
    .click();
  await page
    .getByRole("button", { name: "Next source membership page", exact: true })
    .click();
  await topic
    .getByRole("button", {
      name: "Organize topic Platform identity",
      exact: true,
    })
    .click();
  const editor = page.getByRole("dialog", {
    name: "Organize manual group",
    exact: true,
  });
  await expect(editor).toContainText("Changes apply to this manual group.");
  await expect(editor).not.toContainText("automatic topic");
  const choices = f.versions.map(
    (version) => `Shared architecture guide · version ${version.slice(0, 8)}`,
  );
  for (const choice of choices)
    await expect(
      editor.getByRole("checkbox", { name: choice, exact: true }),
    ).toBeVisible();
  await editor
    .getByRole("textbox", { name: "Name supported by", exact: true })
    .click();
  await page.getByRole("option", { name: choices[1], exact: true }).click();
  await editor
    .getByRole("textbox", { name: "Topic name", exact: true })
    .fill("Reviewed architecture");
  const priorChange = f.state.change;
  await editor
    .getByRole("button", { name: "Save topic name", exact: true })
    .click();
  await expect.poll(() => f.state.writes.length).toBe(1);
  expect(f.state.writes[0].body).toEqual({
    base_change_id: priorChange,
    change: {
      action: "rename_source",
      label: "Reviewed architecture",
      basis_input: f.inputs[1],
    },
  });
  expect(f.state.writes[0].key).toBeTruthy();
  await expect(topic).toContainText("Reviewed architecture");
  await expect(topic.getByText("Manual group", { exact: true })).toBeVisible();
  f.state.contextOnly = true;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await page.getByText("Additional source context", { exact: false }).click();
  await expect(page.getByTestId("organization-topic")).toHaveCount(0);
  await expect(page.getByText("Manual group", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "Organize context topic", exact: true })
    .first()
    .click();
  await expect(editor).toBeVisible();
  await expect(editor).toContainText("Changes apply to this manual group.");
  await mkdir("../.cache/ui", { recursive: true });
  // ADR 0025 review sizes; below 1440 the inspector is a modal sheet.
  for (const width of [1728, 1440]) {
    await page.setViewportSize({ width, height: 960 });
    await page.screenshot({
      path: `../.cache/ui/organization-manual-overlay-${width}.png`,
      animations: "disabled",
    });
  }
});

test("source page error, expiry and entity center cannot retain old source groups or inspector", async ({
  page,
}) => {
  const f = await fixture(page);
  f.state.sourceOnly = true;
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await page
    .getByRole("button", {
      name: "Expand topic Platform identity",
      exact: true,
    })
    .click();
  f.state.sourcePageError = true;
  await page
    .getByRole("button", { name: "Next source membership page", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "Source memberships changed",
  );
  await expect(page.getByTestId("organization-topic")).toHaveCount(0);
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  await expect(
    page.getByRole("complementary", { name: "Selected organization evidence" }),
  ).toHaveCount(0);
  f.state.sourcePageError = false;
  f.state.sourcePageExpires = new Date(Date.now() + 3000).toISOString();
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Next source membership page", exact: true })
    .click();
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-nodes",
    "2",
  );
  await expect(page.getByRole("alert")).toContainText("retention deadline", {
    timeout: 5000,
  });
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  f.state.sourcePageExpires = null;
  f.state.sourceOnly = false;
  await page.getByRole("button", { name: "All topics", exact: true }).click();
  await page
    .getByRole("button", { name: "Refresh organization", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Expand entity Vault service", exact: true })
    .click();
  await expect(
    page.getByRole("group", { name: "Source membership pages" }),
  ).toHaveCount(0);
});
