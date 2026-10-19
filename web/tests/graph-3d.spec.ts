import { test, expect, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:net";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { palette } from "../src/design/tokens";

let vite: ChildProcess;
let origin: string;
const root = fileURLToPath(new URL("../", import.meta.url));

test.beforeAll(async () => {
  const server = createServer();
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string") throw Error("No fixture port");
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
    { cwd: root, stdio: "ignore" },
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

async function fixture(page: Page, large = false, fallback = false) {
  const rendererSource = await (
    await fetch(`${origin}/src/GraphRenderer.tsx`)
  ).text();
  const appSource = await (await fetch(`${origin}/src/main.tsx`)).text();
  const reactPath = rendererSource.match(
    /from "([^"\n]*\/react\.js[^"\n]*)"/,
  )![1];
  const mantinePath = rendererSource.match(
    /from "([^"\n]*@mantine_core[^"\n]*)"/,
  )![1];
  const domPath = appSource.match(
    /from "([^"\n]*react-dom_client[^"\n]*)"/,
  )![1];
  await page.route("**/__graph-3d-fixture__*", (route) =>
    route.fulfill({
      contentType: "text/html",
      body: `<!doctype html><html><head><title>Qualified 3D fixture</title></head><body><div id="root"></div><script type="module">import RefreshRuntime from "/@react-refresh"; RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module">
import React from '${reactPath}';
import ReactDOM from '${domPath}';
const {createRoot}=ReactDOM;
import { MantineProvider } from '${mantinePath}';
import '/node_modules/@mantine/core/styles.css';
import '/src/styles.css';
import '/src/features/graph/graph.css';
import { theme, cssVariablesResolver } from '/src/design/theme.ts';
import { GraphRenderer } from '/src/GraphRenderer.tsx';
const h=React.createElement;
const count=${large ? 500 : 4};
const nodes=Array.from({length:count},(_,i)=>Object.freeze({key:'qualified:'+i,evidence:Object.freeze({label:${large ? "true" : "false"}?'Service evidence '+(i+1):i===0?'Vault service':i===1?'Deployment source':i===2?'Isolated evidence':'<img src=x onerror="window.graphInjected=true">',kind:i%2?'source_version':'claim'})}));
const edges=${large ? "Array.from({length:2000},(_,i)=>({id:'edge:'+i,from:'qualified:'+(i%500),to:'qualified:'+((i*17+13)%500),family:'knowledge',relation:'supported_by'}))" : "[{id:'forward',from:'qualified:0',to:'qualified:1',family:'knowledge',relation:'supported_by'},{id:'parallel',from:'qualified:0',to:'qualified:1',family:'knowledge',relation:'declared_in'},{id:'reverse',from:'qualified:1',to:'qualified:0',family:'knowledge',relation:'refers_to'},{id:'loop',from:'qualified:0',to:'qualified:0',family:'knowledge',relation:'reconciles'}]"};
const groups=${large ? "Array.from({length:8},(_,i)=>({id:'topic:'+i,label:'Qualified topic '+(i+1),nodeKeys:nodes.filter((_,n)=>n%8===i).map(n=>n.key)}))" : "[{id:'topic:a',label:'Platform identity',nodeKeys:['qualified:0','qualified:1']},{id:'topic:b',label:'Delivery',nodeKeys:['qualified:1','qualified:3']}]"};
window.fixtureInputs=JSON.stringify({nodes,edges,groups});
function Fixture(){const [choice,choose]=React.useState(null); const [hidden,hide]=React.useState(false); const [cleared,clear]=React.useState(false); const [mounted,mount]=React.useState(true); window.fixtureChoose=choose; window.fixtureClear=()=>{clear(true);choose(null)};window.fixtureMount=mount;window.fixtureHide=hide; return h('main',null,h('h1',null,'Qualified evidence graph'),h('div',{style:{height:'760px',display:hidden?'none':'flex'}},mounted&&h(GraphRenderer,{nodes:cleared?[]:nodes,edges:cleared?[]:edges,displayGroups:cleared?[]:groups,choice,onChoose:choose,pathNodes:cleared?[]:['qualified:0','qualified:1'],pathEdges:cleared?[]:['forward'],inspector:h('aside',{'aria-label':'Selected evidence'},choice?choice.kind+':'+choice.id:'No selection')})),h('output',{'data-testid':'chosen'},choice?choice.kind+':'+choice.id:'No selection'));}
window.fixtureData=()=>JSON.stringify({nodes,edges,groups});
createRoot(document.getElementById('root')).render(h(MantineProvider,{theme,cssVariablesResolver},h(Fixture)));
</script></body></html>`,
    }),
  );
  page.on("pageerror", (error) =>
    console.log("Fixture page error:", error.message),
  );
  await page.goto(`${origin}/__graph-3d-fixture__`);
  const canvas = page.getByTestId("graph-canvas");
  if (fallback) return canvas;
  await expect(canvas).toHaveAttribute("data-dimension", "3d");
  await expect(canvas).toHaveAttribute("data-ready", "true");
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  return canvas;
}

// Resolve the fixture's renderer ref only to locate actual projected pixels and
// read rendered geometry. Input still travels through real browser pointer
// events and ray picking; production code exposes no test/global scene API.
async function sceneProbe(page: Page) {
  return page.getByTestId("graph-canvas").evaluate((element: any) => {
    let fiber =
      element[
        Object.keys(element).find((key) => key.startsWith("__reactFiber"))!
      ];
    let api: any;
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
    if (!api) throw Error("Fixture renderer ref unavailable");
    const nodes: any[] = [],
      links: any[] = [],
      arrows: string[] = [];
    api.scene().traverse((object: any) => {
      const data = object.__data;
      if (object.__graphObjType === "node")
        nodes.push({
          id: data.id,
          position: [data.x, data.y, data.z],
          point: api.graph2ScreenCoords(data.x, data.y, data.z),
          color: object.children[0].material.color.getHexString(),
        });
      if (object.__linkThreeObjType === "arrow") arrows.push(data.id);
      else if (data?.from && data?.__curve) {
        const point = data.__curve.getPoint(0.5);
        links.push({
          id: data.id,
          from: data.from,
          to: data.to,
          curve: [point.x, point.y, point.z],
          point: api.graph2ScreenCoords(point.x, point.y, point.z),
          color: object.material.color.getHexString(),
          radius:
            object.geometry.parameters.radius ??
            object.geometry.parameters.radiusTop,
        });
      }
    });
    return {
      nodes,
      links,
      arrows,
      camera: api.camera().position.toArray(),
      target: api.controls().target.toArray(),
      frame: api.renderer().info.render.frame,
      rect: element.getBoundingClientRect().toJSON(),
    };
  });
}

test("initially offscreen 3D waits for visibility when measurement mounts its scene", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const state = window as typeof window & {
      graphInitiallyOffscreen?: boolean;
      graphMeasure?: () => void;
      graphReveal?: () => void;
    };
    const NativeResizeObserver = window.ResizeObserver;
    window.ResizeObserver = class extends NativeResizeObserver {
      constructor(callback: ResizeObserverCallback) {
        let measured = false;
        super((entries, observer) => {
          if (
            !measured &&
            entries.some((entry) =>
              entry.target.matches(".graph-spatial-viewport"),
            )
          ) {
            state.graphMeasure = () => {
              measured = true;
              callback(entries, observer);
            };
          } else callback(entries, observer);
        });
      }
    };
    const NativeIntersectionObserver = window.IntersectionObserver;
    window.IntersectionObserver = class extends NativeIntersectionObserver {
      private callback: IntersectionObserverCallback;
      constructor(
        callback: IntersectionObserverCallback,
        options?: IntersectionObserverInit,
      ) {
        super(callback, options);
        this.callback = callback;
      }
      observe(target: Element) {
        if (!target.matches(".graph-spatial-viewport"))
          return super.observe(target);
        state.graphInitiallyOffscreen = !target.querySelector("canvas");
        const notify = (visible: boolean) =>
          this.callback(
            [{ target, isIntersecting: visible } as IntersectionObserverEntry],
            this,
          );
        notify(false);
        state.graphReveal = () => notify(true);
      }
    };
  });
  const canvas = await fixture(page, false, true);
  await expect(canvas).toHaveAttribute("data-dimension", "3d");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any).graphInitiallyOffscreen === true &&
          typeof (window as any).graphMeasure === "function",
      ),
    )
    .toBe(true);
  await expect(canvas.locator("canvas")).toHaveCount(0);
  await page.evaluate(() => (window as any).graphMeasure());
  await expect(canvas).toHaveAttribute("data-ready", "true");
  const frames = () =>
    page.evaluate(
      () =>
        new Promise<void>((resolve) => {
          let remaining = 8;
          const next = () =>
            --remaining ? requestAnimationFrame(next) : resolve();
          requestAnimationFrame(next);
        }),
    );
  await frames();
  const hiddenFrame = (await sceneProbe(page)).frame;
  await frames();
  expect((await sceneProbe(page)).frame).toBe(hiddenFrame);
  await page.evaluate(() => (window as any).graphReveal());
  await expect
    .poll(async () => (await sceneProbe(page)).frame)
    .toBeGreaterThan(hiddenFrame);
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  expect((await sceneProbe(page)).nodes).toHaveLength(4);
});

test("qualified 3D preserves identities, parallel/reverse/loop edges, isolates and safe labels with keyboard inspection and 2D fallback", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.emulateMedia({ reducedMotion: "reduce" });
  const canvas = await fixture(page);
  await expect(canvas).toHaveAttribute("data-nodes", "4");
  await expect(canvas).toHaveAttribute("data-edges", "4");
  await expect(canvas).toHaveAttribute("data-path-edges", "1");
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
  const geometry = await sceneProbe(page);
  expect(geometry.nodes.map((n) => n.id).sort()).toEqual([
    "qualified:0",
    "qualified:1",
    "qualified:2",
    "qualified:3",
  ]);
  expect(geometry.arrows.sort()).toEqual([
    "forward",
    "loop",
    "parallel",
    "reverse",
  ]);
  for (const id of ["qualified:0", "qualified:1"])
    expect(geometry.nodes.find((node) => node.id === id)?.color).toBe(
      palette.sageDark.slice(1),
    );
  expect(geometry.nodes.find((node) => node.id === "qualified:2")?.color).toBe(
    palette.amber.slice(1),
  );
  for (const link of geometry.links) {
    expect(link.color).toBe(
      (link.id === "forward" ? palette.sageDark : palette.lineDark).slice(1),
    );
    expect(link.radius).toBeCloseTo(link.id === "forward" ? 0.9 : 0.3);
  }
  expect(
    new Set(
      geometry.links
        .filter((link) => link.id !== "loop")
        .map((link) => JSON.stringify(link.curve)),
    ).size,
  ).toBe(3);
  const isolated = geometry.nodes.find((node) => node.id === "qualified:2")!;
  await page.mouse.move(
    geometry.rect.x + isolated.point.x,
    geometry.rect.y + isolated.point.y,
  );
  await expect(page.getByRole("tooltip")).toContainText("Isolated evidence");
  await page.mouse.click(
    geometry.rect.x + isolated.point.x,
    geometry.rect.y + isolated.point.y,
  );
  await expect(page.getByTestId("chosen")).toHaveText("node:qualified:2");
  const loop = geometry.links.find((link) => link.id === "loop")!;
  await page.mouse.move(
    geometry.rect.x + loop.point.x,
    geometry.rect.y + loop.point.y,
  );
  await expect(page.getByRole("tooltip")).toContainText("reconciles");
  await page.mouse.click(
    geometry.rect.x + loop.point.x,
    geometry.rect.y + loop.point.y,
  );
  await expect(page.getByTestId("chosen")).toHaveText("edge:loop");
  expect(await page.evaluate(() => (window as any).scene)).toBeUndefined();
  const finder = page.getByRole("textbox", {
    name: "Inspect graph entity",
    exact: true,
  });
  await finder.fill("Isolated evidence");
  await finder.press("ArrowDown");
  await finder.press("Enter");
  await expect(page.getByTestId("chosen")).toHaveText("node:qualified:2");
  const relationship = page.getByRole("textbox", {
    name: "Inspect graph relationship",
    exact: true,
  });
  await relationship.fill("reconciles");
  await relationship.press("ArrowDown");
  await relationship.press("Enter");
  await expect(page.getByTestId("chosen")).toHaveText("edge:loop");
  await page.setViewportSize({ width: 1120, height: 900 });
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  await finder.fill("<img");
  await page.getByRole("option", { name: /<img/ }).click();
  expect(
    await page.evaluate(() => (window as any).graphInjected),
  ).toBeUndefined();
  expect(
    await page.evaluate(
      () => (window as any).fixtureData() === (window as any).fixtureInputs,
    ),
  ).toBe(true);
  await finder.fill("Vault service");
  await page.getByRole("option", { name: /^Vault service ·/ }).click();
  await mkdir("../.cache/ui", { recursive: true });
  await page.screenshot({ path: "../.cache/ui/graph-3d-qualified-laptop.png" });
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.screenshot({ path: "../.cache/ui/graph-3d-qualified-wide.png" });
  await page.mouse.move(0, 0);
  await page.evaluate(() => (window as any).fixtureHide(true));
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
  const hiddenFrame = (await sceneProbe(page)).frame;
  await page.evaluate(
    () =>
      new Promise((resolve) => {
        let left = 8;
        const tick = () =>
          --left ? requestAnimationFrame(tick) : resolve(null);
        requestAnimationFrame(tick);
      }),
  );
  expect((await sceneProbe(page)).frame).toBe(hiddenFrame);
  await page.evaluate(() => (window as any).fixtureHide(false));
  await expect
    .poll(async () => (await sceneProbe(page)).frame)
    .toBeGreaterThan(hiddenFrame);
  const beforeSwitchState = await sceneProbe(page);
  const beforeSwitch = await canvas.screenshot();
  await page.getByText("2D", { exact: true }).click();
  await expect(page.getByTestId("graph-canvas")).not.toHaveAttribute(
    "data-dimension",
    "3d",
  );
  await expect(
    page.getByRole("textbox", { name: "Inspect graph entity", exact: true }),
  ).toContainText("");
  await page.getByText("3D", { exact: true }).click();
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-ready",
    "true",
  );
  await expect(canvas).toHaveAttribute("data-layout-ready", "true");
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
  await writeFile("../.cache/ui/graph-3d-before-switch.png", beforeSwitch);
  const afterSwitch = await canvas.screenshot();
  await writeFile("../.cache/ui/graph-3d-after-switch.png", afterSwitch);
  const afterSwitchState = await sceneProbe(page);
  expect(afterSwitchState.camera).toEqual(beforeSwitchState.camera);
  expect(afterSwitchState.target).toEqual(beforeSwitchState.target);
  expect(
    afterSwitchState.nodes.map((node) => ({
      id: node.id,
      position: node.position,
    })),
  ).toEqual(
    beforeSwitchState.nodes.map((node) => ({
      id: node.id,
      position: node.position,
    })),
  );
  await page
    .locator('[data-dimension="3d"] canvas')
    .evaluate((el: HTMLCanvasElement) => {
      const gl = el.getContext("webgl2");
      if (!gl) throw Error("Missing real WebGL2 context");
      gl.getExtension("WEBGL_lose_context")?.loseContext();
    });
  await expect(
    page.getByText("3D is unavailable.", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByRole("textbox", { name: "Inspect graph entity", exact: true }),
  ).toBeVisible();
  await page.evaluate(() => (window as any).fixtureClear());
  await expect(
    page.getByText("Platform identity", { exact: false }),
  ).not.toBeVisible();
  await expect(page.getByTestId("chosen")).toHaveText("No selection");
  expect(errors).toEqual([]);
});

test("WebGL creation failure offers functional 2D without repeated retries", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const get = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (
      this: HTMLCanvasElement,
      type: string,
      ...args: any[]
    ) {
      if (
        type === "webgl" ||
        type === "webgl2" ||
        type === "experimental-webgl"
      )
        return null;
      return get.apply(this, [type, ...args] as any);
    } as any;
  });
  await fixture(page, false, true);
  await expect(
    page.getByText("3D is unavailable.", { exact: false }),
  ).toBeVisible();
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-ready",
    "true",
  );
  await page
    .getByRole("textbox", { name: "Inspect graph entity", exact: true })
    .fill("Vault");
  await page.getByRole("option", { name: /Vault service/ }).click();
  await expect(page.getByTestId("chosen")).toHaveText("node:qualified:0");
});

test("500-node 2000-edge WebGL measurement and repeated view lifecycle", async ({
  page,
}, info) => {
  test.setTimeout(180_000);
  const started = Date.now();
  const canvas = await fixture(page, true);
  const usable = Date.now() - started;
  const diagnostics = await page
    .locator('[data-dimension="3d"] canvas')
    .evaluate((el: HTMLCanvasElement) => {
      const gl = el.getContext("webgl2")!;
      const ext = gl.getExtension("WEBGL_debug_renderer_info");
      return {
        renderer: ext
          ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL)
          : gl.getParameter(gl.RENDERER),
        vendor: ext
          ? gl.getParameter(ext.UNMASKED_VENDOR_WEBGL)
          : gl.getParameter(gl.VENDOR),
        userAgent: navigator.userAgent,
      };
    });
  const frames = await page.evaluate(
    () =>
      new Promise<number[]>((resolve) => {
        const values: number[] = [];
        let prior = performance.now();
        function frame(now: number) {
          values.push(now - prior);
          prior = now;
          if (values.length < 90) requestAnimationFrame(frame);
          else resolve(values.slice(1));
        }
        requestAnimationFrame(frame);
      }),
  );
  const interaction: number[] = [];
  const finderInteraction: number[] = [];
  const heaps: number[] = [];
  const cdp = await page.context().newCDPSession(page);
  const documentHandle = (
    await cdp.send("Runtime.evaluate", { expression: "document" })
  ).result.objectId!;
  const listenerCounts: number[] = [];
  await cdp.send("HeapProfiler.enable");
  await cdp.send("Performance.enable");
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  for (let i = 0; i < 20; i++) {
    const delay = await page.evaluate(async (i) => {
      const start = performance.now();
      (window as any).fixtureChoose({ kind: "node", id: "qualified:" + i });
      await new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      );
      return performance.now() - start;
    }, i);
    interaction.push(delay);
  }
  for (let i = 0; i < 10; i++) {
    await page
      .getByRole("textbox", { name: "Inspect graph entity", exact: true })
      .fill(`Service evidence ${i + 1}`);
    await page.evaluate(() => {
      const option = document.querySelector('[role="option"]');
      (window as any).finderStart = 0;
      option?.addEventListener(
        "pointerdown",
        () => {
          (window as any).finderStart = performance.now();
        },
        { once: true },
      );
      const observer = new MutationObserver(() => {
        if ((window as any).finderStart)
          requestAnimationFrame(() => {
            (window as any).finderDelay =
              performance.now() - (window as any).finderStart;
            observer.disconnect();
          });
      });
      observer.observe(document.querySelector('[data-testid="chosen"]')!, {
        subtree: true,
        childList: true,
        characterData: true,
      });
    });
    await page
      .getByRole("option", { name: new RegExp(`^Service evidence ${i + 1} ·`) })
      .click();
    await expect(page.getByTestId("chosen")).toHaveText(`node:qualified:${i}`);
    finderInteraction.push(
      await page.evaluate(() => (window as any).finderDelay ?? 0),
    );
  }
  // Warm both lazy modules before measuring retained heap, so one-time 2D
  // module parsing is not mistaken for retention across view lifecycles.
  await page.getByText("2D", { exact: true }).click();
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-ready",
    "true",
  );
  await page.getByText("3D", { exact: true }).click();
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-layout-ready",
    "true",
  );
  for (let i = 0; i < 12; i++) {
    if (i < 6) {
      await page.evaluate(() => (window as any).fixtureMount(false));
      await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
      await page.evaluate(() => (window as any).fixtureMount(true));
    } else {
      await page.getByText("2D", { exact: true }).click();
      await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
        "data-ready",
        "true",
      );
      await page.getByText("3D", { exact: true }).click();
    }
    await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
      "data-layout-ready",
      "true",
    );
    listenerCounts.push(
      (
        await cdp.send("DOMDebugger.getEventListeners", {
          objectId: documentHandle,
        })
      ).listeners.filter((listener) => listener.type === "keydown").length,
    );
    await cdp.send("HeapProfiler.collectGarbage");
    const metrics = await cdp.send("Performance.getMetrics");
    heaps.push(metrics.metrics.find((m) => m.name === "JSHeapUsedSize")!.value);
  }
  await cdp.send("HeapProfiler.disable");
  await cdp.send("Performance.disable");
  await cdp.detach();
  const p95 = (values: number[]) =>
    [...values].sort((a, b) => a - b)[Math.ceil(values.length * 0.95) - 1];
  const result = {
    note: "Isolated browser renderer fixture, no API/model work. Actual WebGL renderer recorded below. Browser requestAnimationFrame cadence is not GPU render time. 2-frame React selection paint latency includes active rendering; finder timing uses pointerdown to inspector mutation plus next frame.",
    ...diagnostics,
    first_usable_ms: usable,
    frame_ms: { p95: p95(frames), samples: frames },
    interaction_ms: {
      p95: p95(interaction),
      target_ms: 100,
      passed: p95(interaction) < 100,
      samples: interaction,
    },
    finder_inspection_ms: {
      p95: p95(finderInteraction),
      samples: finderInteraction,
    },
    heap_bytes_after_forced_gc: heaps,
    document_keydown_listeners: listenerCounts,
    viewport: page.viewportSize(),
    browser: info.project.use.channel,
  };
  await mkdir("../.cache/ui", { recursive: true });
  await writeFile(
    "../.cache/ui/graph-3d-performance.json",
    JSON.stringify(result, null, 2),
  );
  await page.screenshot({ path: "../.cache/ui/graph-3d-wide.png" });
  await page.setViewportSize({ width: 1120, height: 900 });
  await page.screenshot({ path: "../.cache/ui/graph-3d-laptop.png" });
  expect(errors).toEqual([]);
  expect(p95(interaction)).toBeLessThan(100);
  expect(p95(finderInteraction)).toBeLessThan(100);
  expect(
    Math.max(...heaps.slice(3)) - Math.max(...heaps.slice(0, 3)),
  ).toBeLessThan(8 * 1024 * 1024);
  expect(new Set(listenerCounts).size).toBe(1);
  await expect(canvas).toHaveAttribute("data-nodes", "500");
  await expect(canvas).toHaveAttribute("data-edges", "2000");
});
