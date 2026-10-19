import { chromium, expect as baseExpect } from "@playwright/test";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

// Invoked only by the owned Rust fixture. It supplies an ephemeral authenticated
// session and true production-adapter publications; this script mocks no route
// and never serializes cookies, CSRF tokens, passwords, or request headers.
const expect = baseExpect.configure({ timeout: 20_000 });
const fixturePath = process.env.RECOLLECT_ORGANIZATION_NATIVE_FIXTURE;
if (!fixturePath) throw Error("Missing owned native organization fixture path");
const fixture = JSON.parse(await readFile(fixturePath, "utf8"));
const origin = new URL(fixture.origin);
if (
  origin.protocol !== "http:" ||
  !["127.0.0.1", "localhost"].includes(origin.hostname)
)
  throw Error("Native organization proof requires an owned loopback origin");
const cookie = fixture.session_cookie;
if (!cookie?.name || !cookie?.value || !fixture.brain_id)
  throw Error("Native organization fixture has no ephemeral session or Brain");
const output = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../.cache/ui",
);
await mkdir(output, { recursive: true });
const sources = fixture.sources;
if (!Array.isArray(sources) || sources.length < 3)
  throw Error("Native fixture needs canonical sources");
const keyFor = (source) => `source_version:${source.version_id}`;
const sourceByKey = new Map(sources.map((source) => [keyFor(source), source]));
const related = fixture.related_titles.map((title) => {
  const source = sources.find((source) => source.title === title);
  if (!source) throw Error("Related native source missing");
  return source;
});
const escaped = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const report = {
  fixture:
    "Owned native production mapping API to actual App; loopback model gateway, no paid provider or browser route mocks",
  origin: origin.origin,
  brain_id: fixture.brain_id,
  api: [],
  renderer: {},
  checks: [],
  screenshots: [],
};
let browser, page, deadline;
async function capture(name, fullPage = false) {
  const path = resolve(output, `organization-native-${name}.png`);
  await page.screenshot({ path, fullPage, animations: "disabled" });
  report.screenshots.push(path);
}
async function renderedGeometry() {
  return page.getByTestId("graph-canvas").evaluate((element) => {
    let fiber =
        element[
          Object.keys(element).find((key) => key.startsWith("__reactFiber"))
        ],
      api;
    while (fiber && !api) {
      let hook = fiber.memoizedState;
      while (hook) {
        const candidate = hook.memoizedState?.current;
        if (typeof candidate?.graph2ScreenCoords === "function") {
          api = candidate;
          break;
        }
        hook = hook.next;
      }
      fiber = fiber.return;
    }
    if (!api) throw Error("Native rendered scene is unavailable");
    const nodes = [],
      camera = api.camera();
    api.scene().traverse((object) => {
      if (object.__graphObjType !== "node") return;
      const mesh = object.children[0],
        position = mesh.getWorldPosition(mesh.position.clone());
      const radius =
        mesh.geometry.parameters.radius *
        Math.abs(mesh.getWorldScale(mesh.scale.clone()).x);
      const point = api.graph2ScreenCoords(position.x, position.y, position.z);
      const basis = camera.matrixWorld.elements;
      const edge = api.graph2ScreenCoords(
        position.x + basis[0] * radius,
        position.y + basis[1] * radius,
        position.z + basis[2] * radius,
      );
      nodes.push({
        id: object.__data.id,
        x: point.x,
        y: point.y,
        radius: Math.hypot(edge.x - point.x, edge.y - point.y),
      });
    });
    return {
      width: element.clientWidth,
      height: element.clientHeight,
      camera: camera.position.toArray(),
      target: api.controls().target.toArray(),
      nodes,
    };
  });
}
function validateMentions(view) {
  for (const mention of [...view.representative_mentions, ...view.mentions]) {
    const source = sourceByKey.get(mention.source_key);
    if (!source) continue;
    const bytes = Buffer.from(source.exact_text, "utf8");
    expect(
      bytes.subarray(mention.byte_start, mention.byte_end).toString("utf8"),
    ).toBe(mention.quote);
  }
}
async function run() {
  browser = await chromium.launch({ channel: "chrome", headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 960 },
    reducedMotion: "reduce",
  });
  await context.addCookies([
    {
      name: cookie.name,
      value: cookie.value,
      url: origin.origin,
      httpOnly: true,
      sameSite: "Strict",
    },
  ]);
  page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const started = new WeakMap();
  page.on("request", (request) => {
    started.set(request, performance.now());
    if (
      request.method() === "PUT" &&
      new URL(request.url()).pathname.includes("/organization/topics/")
    ) {
      report.topic_commands ??= [];
      report.topic_commands.push({
        path: new URL(request.url()).pathname,
        command: request.postDataJSON(),
      });
    }
  });
  const base = `/api/brains/${fixture.brain_id}/organization`;
  const observed = new WeakMap();
  page.on("response", (response) => {
    const request = response.request(), url = new URL(response.url());
    if (!url.pathname.startsWith(base)) return;
    // This observer measures headers without waiting on response.finished().
    // Navigation can leave that CDP promise pending even after the separately
    // awaited API JSON/visible UI assertions succeed. Completion is optional
    // event telemetry; it must never hold the functional proof open.
    const row = {
      method: request.method(), path: url.pathname, status: response.status(),
      headers_elapsed_ms: Math.round((performance.now() - started.get(request)) * 10) / 10,
      transport_state: "headers_received",
    };
    observed.set(request, row);
    report.api.push(row);
  });
  page.on("requestfinished", (request) => {
    const row = observed.get(request);
    if (!row) return;
    row.transport_state = "finished";
    row.completed_elapsed_ms = Math.round((performance.now() - started.get(request)) * 10) / 10;
  });
  page.on("requestfailed", (request) => {
    const row = observed.get(request);
    if (row) row.transport_state = "failed_or_navigation_cancelled";
  });
  const organizationResponse = (predicate) =>
    page.waitForResponse(
      (response) => {
        const request = response.request();
        return (
          new URL(response.url()).pathname === `${base}/view` &&
          request.method() === "POST" &&
          (!predicate || predicate(response, request.postDataJSON()))
        );
      },
      { timeout: 35_000 },
    );
  const initialRead = organizationResponse((response) => response.ok());
  const initialFieldsRead = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `${base}/fields/view` &&
      response.ok(),
  );
  await page.goto(`${origin.origin}/brains/${fixture.brain_id}/graph`);
  const initial = await (await initialRead).json();
  validateMentions(initial);
  await expect(
    page.getByRole("region", {
      name: "Automatic knowledge organization",
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.getByTestId("organization-topic").first()).toBeVisible();
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  const topic = initial.topics.find(
    (topic) => topic.source_count >= related.length && topic.entity_count === 1,
  );
  if (!topic)
    throw Error(
      "Actual automatic publication has no topic joining the related documents",
    );
  report.checks.push(
    "Actual production publications expose topic/entity cards without loading a global graph",
  );
  report.overview = {
    topics: initial.topics.length,
    entities: initial.entities.length,
    coverage: initial.coverage,
  };
  for (const [width, name] of [
    [1440, "overview-wide"],
    [1120, "overview-laptop"],
  ]) {
    await page.setViewportSize({ width, height: 960 });
    await capture(name);
  }
  await page.setViewportSize({ width: 1440, height: 960 });
  const initialFields = await (await initialFieldsRead).json();
  expect(initialFields.coverage.refresh_state).toBe("ready");
  expect(initial.producer_status).toBe("ready");
  expect(initial.community_status.state).toBe(
    fixture.source_groups_expected ? "ready" : "unconfigured",
  );
  if (fixture.source_groups_expected) {
    expect(fixture.source_groups_expected.synthetic_quality).toBe(true);
    expect(fixture.source_groups_expected.quality_measured).toBe(false);
    expect(fixture.source_groups_expected.embedding_http_calls).toBe(0);
  }
  const field = initialFields.entities.find(
    (entity) => entity.reference_id === fixture.environment.id,
  );
  expect(field.kind).toBe("environment");
  expect(field.label_pointer).toBe("/name");
  const expandedFieldsRead = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `${base}/fields/view` &&
      response.ok() &&
      response.request().postDataJSON()?.entity_id === field.id,
  );
  await page
    .getByRole("button", {
      name: `Explore canonical reference ${fixture.environment.name}`,
      exact: true,
    })
    .click();
  const expandedFields = await (await expandedFieldsRead).json();
  expect(
    expandedFields.evidence.map((node) => node.evidence.label).sort(),
  ).toEqual([...fixture.environment.expected_titles].sort());
  for (const source of sources.filter((source) =>
    fixture.environment.expected_titles.includes(source.title),
  )) {
    const actualOrigin = expandedFields.origins.find(
      (item) => item.evidence_key === keyFor(source),
    );
    expect(actualOrigin.origin_kind).toBe("source_context");
    expect(actualOrigin.field_pointer).toBe("/environments/0");
    expect(actualOrigin.value).toBe(fixture.environment.id);
    expect(actualOrigin.revision_id).toBe(source.version_id);
    expect(actualOrigin.byte_start).toBeUndefined();
    await page
      .getByRole("button", {
        name: `Inspect canonical evidence ${source.title}`,
        exact: true,
      })
      .click();
    const inspector = page.getByRole("complementary", {
      name: "Canonical field inspector",
      exact: true,
    });
    await expect(inspector).toContainText(actualOrigin.field_pointer);
    await expect(inspector).toContainText(fixture.environment.id);
    await expect(inspector).toContainText(source.version_id);
    await expect(inspector).not.toContainText("UTF-8 bytes");
    await inspector
      .getByRole("button", { name: "Open canonical evidence", exact: true })
      .click();
    const dialog = page.getByRole("dialog", {
      name: "Supporting evidence",
      exact: true,
    });
    await expect(dialog.getByTestId("claim-evidence-text")).toHaveText(
      source.exact_text,
    );
    await dialog
      .getByRole("button", { name: "Close evidence", exact: true })
      .click();
  }
  const fieldCanvas = page.getByTestId("graph-canvas");
  await expect(fieldCanvas).toHaveAttribute("data-layout-ready", "true");
  await expect(fieldCanvas).toHaveAttribute(
    "data-nodes",
    String(expandedFields.evidence.length),
  );
  for (const [width, name] of [
    [1440, "fields-wide"],
    [1120, "fields-laptop"],
  ]) {
    await page.setViewportSize({ width, height: 960 });
    await fieldCanvas.scrollIntoViewIfNeeded();
    await expect(fieldCanvas).toHaveAttribute("data-layout-ready", "true");
    await capture(name);
  }
  report.checks.push(
    "Actual typed canonical field API preserves original environment UUID and source revision/pointer, opens both exact PROD sources, and adds no fake source span",
  );
  report.fields = {
    entities: initialFields.entities.length,
    coverage: initialFields.coverage,
    producer_status: initial.producer_status,
    community_status: initial.community_status,
  };
  await page
    .getByRole("button", { name: "All canonical references", exact: true })
    .click();
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  await page.setViewportSize({ width: 1440, height: 960 });
  const expandStarted = performance.now();
  const expandedRead = organizationResponse(
    (response, request) =>
      response.ok() &&
      request.center?.kind === "topic" &&
      request.center.id === topic.id,
  );
  const [expandedResponse] = await Promise.all([
    expandedRead,
    page
      .locator(`[data-topic-id="${topic.id}"]`)
      .getByRole("button", { name: `Expand topic ${topic.label}`, exact: true })
      .click(),
  ]);
  const expanded = await expandedResponse.json();
  const responseAt = performance.now();
  validateMentions(expanded);
  for (const source of related)
    expect(expanded.evidence.map((node) => node.key)).toContain(keyFor(source));
  expect(new Set(expanded.evidence.map((node) => node.key)).size).toBe(
    expanded.evidence.length,
  );
  const canvas = page.getByTestId("graph-canvas");
  await canvas.scrollIntoViewIfNeeded();
  await expect(canvas).toHaveAttribute("data-dimension", "3d");
  await expect(canvas).toHaveAttribute("data-ready", "true");
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  report.renderer.expand_to_response_ms = Math.round(
    responseAt - expandStarted,
  );
  report.renderer.response_to_first_fit_ms = Math.round(
    performance.now() - responseAt,
  );
  report.renderer.note =
    "Production lazy import and first settled fit after the actual bounded API read; API latency is recorded separately. This is not a GPU frame-time benchmark.";
  report.renderer.webgl = await canvas.locator("canvas").evaluate((element) => {
    const gl = element.getContext("webgl2");
    if (!gl) throw Error("Native browser proof has no WebGL2 renderer");
    const debug = gl.getExtension("WEBGL_debug_renderer_info");
    return debug
      ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL)
      : gl.getParameter(gl.RENDERER);
  });
  for (const source of related) {
    const finder = page.getByRole("textbox", {
      name: "Inspect graph entity",
      exact: true,
    });
    await finder.fill(source.title);
    await page
      .getByRole("option", { name: new RegExp(`^${escaped(source.title)} ·`) })
      .click();
    const inspector = page.getByRole("complementary", {
      name: "Selected organization evidence",
      exact: true,
    });
    await expect(inspector).toContainText(source.title);
    await expect(inspector).toContainText(keyFor(source));
    const mention = [
      ...expanded.representative_mentions,
      ...expanded.mentions,
    ].find((mention) => mention.source_key === keyFor(source));
    if (!mention)
      throw Error("Related native source has no qualified exact mention");
    await expect(inspector).toContainText(
      `Exact UTF-8 bytes ${mention.byte_start}–${mention.byte_end}`,
    );
    await expect(inspector).toContainText(mention.quote);
    await inspector
      .getByRole("button", { name: "Open exact source", exact: true })
      .click();
    const evidence = page.getByRole("dialog", {
      name: "Supporting evidence",
      exact: true,
    });
    await expect(evidence.getByTestId("claim-evidence-text")).toHaveText(
      source.exact_text,
    );
    await evidence
      .getByRole("button", { name: "Close evidence", exact: true })
      .click();
    await expect(evidence).not.toBeVisible();
  }
  report.checks.push(
    "One actual automatic topic contains both related canonical sources with exact UTF-8 spans and source dialogs",
  );
  await canvas.scrollIntoViewIfNeeded();
  const nodeCount = String(expanded.evidence.length);
  await expect(canvas).toHaveAttribute("data-nodes", nodeCount);
  await expect(canvas).toHaveAttribute("data-edges", "0");
  for (const [width, name] of [
    [1440, "expanded-wide"],
    [1120, "expanded-laptop"],
  ]) {
    await page.setViewportSize({ width, height: 960 });
    await canvas.scrollIntoViewIfNeeded();
    await expect(canvas).toHaveAttribute("data-layout-ready", "true");
    await page.evaluate(
      () =>
        new Promise((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(resolve)),
        ),
    );
    const geometry = await renderedGeometry();
    expect(geometry.nodes).toHaveLength(expanded.evidence.length);
    for (const node of geometry.nodes) {
      expect([node.x, node.y, node.radius].every(Number.isFinite)).toBe(true);
      expect(node.radius).toBeGreaterThan(1);
      expect(node.radius).toBeLessThan(
        Math.min(geometry.width, geometry.height) * 0.2,
      );
      expect(node.x).toBeGreaterThanOrEqual(node.radius);
      expect(node.x).toBeLessThanOrEqual(geometry.width - node.radius);
      expect(node.y).toBeGreaterThanOrEqual(node.radius);
      expect(node.y).toBeLessThanOrEqual(geometry.height - node.radius);
    }
    if (geometry.nodes.length === 2) {
      const [a, b] = geometry.nodes;
      expect(Math.hypot(a.x - b.x, a.y - b.y)).toBeGreaterThan(
        a.radius + b.radius + 8,
      );
    }
    (report.renderer.geometry ??= []).push({
      viewport_width: width,
      ...geometry,
    });
    await capture(name);
    await capture(`${name}-viewport`, false);
  }
  const selectedTitle = related.at(-1).title;
  await page.getByText("2D", { exact: true }).click();
  await expect(canvas).toHaveAttribute("data-ready", "true");
  await expect(canvas).toHaveAttribute("data-nodes", nodeCount);
  await expect(
    page.getByRole("textbox", { name: "Inspect graph entity", exact: true }),
  ).toHaveValue(new RegExp(escaped(selectedTitle)));
  await page.getByText("3D", { exact: true }).click();
  await expect(canvas).toHaveAttribute("data-dimension", "3d");
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  await expect(canvas).toHaveAttribute("data-nodes", nodeCount);
  await expect(
    page.getByRole("complementary", {
      name: "Selected organization evidence",
      exact: true,
    }),
  ).toContainText(keyFor(related.at(-1)));
  report.checks.push(
    "2D and 3D retain the actual bounded source identities and selected source",
  );
  await page.getByRole("button", { name: "All topics", exact: true }).click();
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  await page.setViewportSize({ width: 1440, height: 960 });
  if (fixture.source_groups_expected) {
    const expected = fixture.source_groups_expected;
    const topic = initial.topics.find((card) => card.id === expected.topic_id);
    if (!topic)
      throw Error(
        "Controlled native source topic missing from actual overview",
      );
    const nativeRead = organizationResponse(
      (response, request) => response.ok() && request.center?.id === topic.id,
    );
    const [nativeResponse] = await Promise.all([
      nativeRead,
      page
        .locator(`[data-topic-id="${topic.id}"]`)
        .getByRole("button", {
          name: `Expand topic ${topic.label}`,
          exact: true,
        })
        .click(),
    ]);
    const actual = await nativeResponse.json();
    const contributors = actual.source_memberships.filter(
      (member) => member.topic_id === topic.id,
    );
    expect(
      [...new Set(contributors.map((row) => row.source_key))].sort(),
    ).toEqual([...expected.source_keys].sort());
    expect(
      [...new Set(contributors.map((row) => row.input_id))].sort(),
    ).toEqual([...expected.input_ids].sort());
    const nativeCanvas = page.getByTestId("graph-canvas");
    await expect(nativeCanvas).toHaveAttribute(
      "data-nodes",
      String(expected.source_keys.length),
    );
    // Offscreen views pause their finite layout until visible.
    await nativeCanvas.scrollIntoViewIfNeeded();
    await expect(nativeCanvas).toHaveAttribute("data-layout-ready", "true");
    for (const title of expected.source_titles) {
      const source = sources.find((source) => source.title === title);
      await page
        .getByRole("textbox", { name: "Inspect graph entity", exact: true })
        .fill(title);
      await page
        .getByRole("option", { name: new RegExp(`^${escaped(title)} ·`) })
        .click();
      const inspector = page.getByRole("complementary", {
        name: "Selected organization evidence",
        exact: true,
      });
      await expect(inspector).toContainText(keyFor(source));
      await expect(inspector).toContainText(`Source member of ${topic.label}`);
      await inspector
        .getByRole("button", { name: "Open exact source", exact: true })
        .click();
      const evidence = page.getByRole("dialog", {
        name: "Supporting evidence",
        exact: true,
      });
      await expect(evidence.getByTestId("claim-evidence-text")).toHaveText(
        source.exact_text,
      );
      await evidence
        .getByRole("button", { name: "Close evidence", exact: true })
        .click();
    }
    for (const [width, name] of [
      [1440, "source-groups-wide"],
      [1120, "source-groups-laptop"],
    ]) {
      await page.setViewportSize({ width, height: 960 });
      await nativeCanvas.scrollIntoViewIfNeeded();
      await expect(nativeCanvas).toHaveAttribute("data-layout-ready", "true");
      await capture(name);
    }
    await page.getByText("2D", { exact: true }).click();
    await expect(nativeCanvas).toHaveAttribute(
      "aria-label",
      new RegExp(
        `Interactive graph: ${expected.source_keys.length} of ${expected.source_keys.length} loaded nodes`,
      ),
    );
    await expect(
      page.getByRole("textbox", { name: "Inspect graph entity", exact: true }),
    ).toHaveValue(new RegExp(escaped(expected.source_titles.at(-1))));
    await page.getByText("3D", { exact: true }).click();
    await expect(nativeCanvas).toHaveAttribute("data-dimension", "3d");
    await expect(nativeCanvas).toHaveAttribute("data-layout-ready", "true");
    const command = (action) =>
      page
        .waitForResponse(
          (response) =>
            new URL(response.url()).pathname.startsWith(`${base}/topics/`) &&
            response.request().method() === "PUT",
        )
        .then(async (response) => {
          expect(response.request().postDataJSON()?.change?.action).toBe(
            action,
          );
          if (!response.ok()) {
            const body = await response.json().catch(() => ({}));
            throw Error(
              `Native ${action} rejected (${response.status()}, ${body.error?.code ?? "unknown"})`,
            );
          }
          return response;
        });
    await page
      .getByRole("button", {
        name: `Organize topic ${topic.label}`,
        exact: true,
      })
      .click();
    let editor = page.getByRole("dialog", {
      name: "Organize automatic topic",
      exact: true,
    });
    const renamed = expected.source_titles[0],
      splitTitle = expected.source_titles.at(-1);
    await editor
      .getByRole("textbox", { name: "Name supported by", exact: true })
      .click();
    await page.getByRole("option", { name: renamed, exact: true }).click();
    await editor
      .getByRole("textbox", { name: "Topic name", exact: true })
      .fill(renamed);
    await expect(
      editor.getByRole("button", { name: "Save topic name", exact: true }),
    ).toBeEnabled();
    for (const [width, name] of [
      [1440, "source-editor-wide"],
      [1120, "source-editor-laptop"],
    ]) {
      await page.setViewportSize({ width, height: 960 });
      await capture(name);
    }
    const [renamedResponse] = await Promise.all([
      command("rename_source"),
      editor
        .getByRole("button", { name: "Save topic name", exact: true })
        .click(),
    ]);
    expect(renamedResponse.ok()).toBe(true);
    await page
      .getByRole("button", { name: `Organize topic ${renamed}`, exact: true })
      .click();
    editor = page.getByRole("dialog", {
      name: "Organize automatic topic",
      exact: true,
    });
    await editor
      .getByRole("checkbox", { name: splitTitle, exact: true })
      .check();
    const splitViewRead = organizationResponse(
      (response, request) =>
        response.ok() &&
        request.center?.kind === "topic" &&
        request.center.id !== topic.id,
    );
    const [splitResponse, splitViewResponse] = await Promise.all([
      command("split_sources"),
      splitViewRead,
      editor
        .getByRole("button", { name: "Split selected into topic", exact: true })
        .click(),
    ]);
    const split = await splitResponse.json();
    expect(split.split_topic_id).toBeTruthy();
    const splitView = await splitViewResponse.json(),
      splitTopic = splitView.topics.find(
        (card) => card.id === split.split_topic_id,
      );
    if (!splitTopic) throw Error("Native source split topic is not navigable");
    await page
      .getByRole("button", {
        name: `Organize topic ${splitTopic.label}`,
        exact: true,
      })
      .click();
    editor = page.getByRole("dialog", {
      name: "Organize automatic topic",
      exact: true,
    });
    await editor
      .getByRole("checkbox", { name: splitTitle, exact: true })
      .check();
    const [excludedResponse] = await Promise.all([
      command("exclude_sources"),
      editor
        .getByRole("button", { name: "Exclude selected sources", exact: true })
        .click(),
    ]);
    expect(excludedResponse.ok()).toBe(true);
    report.source_groups = {
      topic_id: topic.id,
      source_keys: expected.source_keys,
      report_id: expected.report_id,
      recipe_id: expected.recipe_id,
      semantic_profile_id: expected.semantic_profile_id,
      split_topic_id: split.split_topic_id,
      synthetic_quality: true,
      quality_measured: false,
      embedding_http_calls: 0,
    };
    report.checks.push(
      "Actual native source contributors preserve canonical identities/inspectors in2D/3D and versioned source rename/split/exclude; finite seeded fixture vectors prove no real grouping quality",
    );
    await page.getByRole("button", { name: "All topics", exact: true }).click();
    await expect(nativeCanvas).toHaveCount(0);
    await page.setViewportSize({ width: 1440, height: 960 });
  }
  const aliasRead = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `${base}/aliases/view` &&
      response.ok(),
  );
  await page
    .getByRole("button", { name: "Review aliases", exact: true })
    .click();
  const aliases = await (await aliasRead).json();
  const alias = aliases.candidates.find(
    (candidate) =>
      candidate.from.label === fixture.alias.from_label &&
      candidate.to.label === fixture.alias.to_label,
  );
  if (!alias)
    throw Error("Actual production publication has no expected alias evidence");
  expect(alias.quote).toBe(fixture.alias.quote);
  expect(alias.from.id).not.toBe(alias.to.id);
  expect(alias.acceptance_state).toBe("explicit_positive_statement");
  const aliasSource = sources.find(
    (source) => source.title === fixture.alias.source_title,
  );
  expect(alias.source_key).toBe(keyFor(aliasSource));
  expect(
    Buffer.from(aliasSource.exact_text)
      .subarray(alias.byte_start, alias.byte_end)
      .toString(),
  ).toBe(alias.quote);
  const review = page.getByRole("region", {
    name: "Alias evidence review",
    exact: true,
  });
  const candidate = review
    .getByTestId("alias-candidate")
    .filter({ hasText: alias.id });
  await expect(candidate).toContainText("pending");
  await candidate
    .getByRole("button", { name: "Inspect exact alias source", exact: true })
    .click();
  const aliasEvidence = page.getByRole("dialog", {
    name: "Supporting evidence",
    exact: true,
  });
  await expect(aliasEvidence.getByTestId("claim-evidence-text")).toHaveText(
    aliasSource.exact_text,
  );
  await aliasEvidence
    .getByRole("button", { name: "Close evidence", exact: true })
    .click();
  await expect(aliasEvidence).not.toBeVisible();
  const decisionResponse = () =>
    page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === `${base}/aliases/${alias.id}` &&
        response.request().method() === "PUT",
    );
  const acceptedRead = decisionResponse();
  await candidate
    .getByRole("button", { name: "Accept alias navigation", exact: true })
    .click();
  const acceptedResponse = await acceptedRead;
  expect(acceptedResponse.ok()).toBe(true);
  const accepted = await acceptedResponse.json();
  expect(accepted.disposition).toBe("accepted");
  expect(acceptedResponse.request().postDataJSON()).toEqual({
    base_change_id: alias.change_id ?? null,
    action: "accept",
  });
  await expect(candidate).toContainText("accepted");
  await capture("alias-accepted", false);
  const revokedRead = decisionResponse();
  await candidate
    .getByRole("button", { name: "Revoke alias decision", exact: true })
    .click();
  const revokedResponse = await revokedRead;
  expect(revokedResponse.ok()).toBe(true);
  expect((await revokedResponse.json()).disposition).toBe("pending");
  expect(revokedResponse.request().postDataJSON()).toEqual({
    base_change_id: accepted.change_id,
    action: "revoke",
  });
  await expect(candidate).toContainText("pending");
  await expect(candidate).toContainText(alias.from.id);
  await expect(candidate).toContainText(alias.to.id);
  report.checks.push(
    "Actual human alias accept/revoke uses current versions and retains both original entity IDs",
  );
  await page
    .getByRole("dialog", { name: "Review alias evidence", exact: true })
    .getByRole("button", { name: "Close dialog", exact: true })
    .click();
  if (fixture.environment) {
    await page.getByRole("button", { name: "Scope", exact: true }).click();
    const scope = page.getByRole("dialog", {
      name: "Organization scope",
      exact: true,
    });
    await scope
      .getByRole("textbox", { name: "Organization environment", exact: true })
      .click();
    await page
      .getByRole("option", { name: fixture.environment.name, exact: true })
      .click();
    const scopedRead = organizationResponse(
      (response, request) =>
        response.ok() &&
        request.scope.selection.environment_id === fixture.environment.id,
    );
    await scope
      .getByRole("button", { name: "Apply organization scope", exact: true })
      .click();
    const scoped = await (await scopedRead).json();
    validateMentions(scoped);
    if (fixture.source_groups_expected) {
      expect(scoped.coverage.qualified_sources).toBe(
        fixture.source_groups_expected.prod_qualified_sources,
      );
      expect(scoped.evidence.map((node) => node.key).sort()).toEqual(
        [...fixture.source_groups_expected.prod_source_keys].sort(),
      );
      report.source_groups.scoped_prod_qualified_sources =
        scoped.coverage.qualified_sources;
    }
    expect(scoped.evidence.map((node) => node.evidence.label).sort()).toEqual(
      [...fixture.environment.expected_titles].sort(),
    );
    for (const source of sources.filter(
      (source) => !fixture.environment.expected_titles.includes(source.title),
    ))
      await expect(page.getByText(source.title, { exact: true })).toHaveCount(
        0,
      );
    await capture("prod-scope");
    report.checks.push(
      "Actual environment qualification removes the DEV source and preserves exactly the PROD sources",
    );
  }
  const unknown = randomUUID();
  const rejectedRead = organizationResponse(
    (response, request) =>
      response.status() >= 400 &&
      request.scope.selection.environment_id === unknown,
  );
  await page.goto(
    `${origin.origin}/brains/${fixture.brain_id}/graph?tab=organization&environment=${unknown}`,
  );
  report.invalid_scope_status = (await rejectedRead).status();
  await expect(page.getByRole("alert")).toBeVisible();
  for (const testId of [
    "organization-topic",
    "organization-entity",
    "organization-field-entity",
    "graph-canvas",
    "alias-candidate",
  ])
    await expect(page.getByTestId(testId)).toHaveCount(0);
  await expect(
    page.getByRole("complementary", {
      name: "Selected organization evidence",
      exact: true,
    }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("complementary", {
      name: "Canonical field inspector",
      exact: true,
    }),
  ).toHaveCount(0);
  for (const source of sources)
    await expect(page.getByText(source.title, { exact: true })).toHaveCount(0);
  await capture("invalid-scope");
  report.checks.push(
    "An actual invalid-scope API error clears every previous card, graph, alias and selected source",
  );
  expect(errors).toEqual([]);
  report.api_timing_scope = "response_headers; request completion recorded only when observed, no pending body-reader waits";
  report.page_errors = errors;
  report.status = "passed";
}
try {
  await Promise.race([
    run(),
    new Promise((_, reject) => {
      deadline = setTimeout(
        () =>
          reject(Error("Native organization UI proof exceeded 120 seconds")),
        120_000,
      );
    }),
  ]);
  await writeFile(
    resolve(output, "organization-native-proof.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(
    `Native organization UI proof passed: ${report.checks.length} checks; evidence in .cache/ui/organization-native-proof.json`,
  );
} catch (error) {
  report.status = "failed";
  report.reason =
    error instanceof Error
      ? error.message
      : "Native organization UI proof failed";
  if (page && !page.isClosed()) await capture("failure", false).catch(() => {});
  await writeFile(
    resolve(output, "organization-native-proof.json"),
    JSON.stringify(report, null, 2),
  );
  console.error(report.reason);
  process.exitCode = 1;
} finally {
  clearTimeout(deadline);
  await browser?.close();
}
