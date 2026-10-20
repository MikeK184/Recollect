// Seeds one violation of each kind into a clean copy and runs check-design.mjs
// against it: node --test scripts/check-design.test.mjs (from web/).
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("check-design.mjs", import.meta.url));
const fonts = {
  monasans: ["mona-sans-variable.ttf", "Mona Sans"],
  jetbrainsmono: ["jetbrains-mono-variable.ttf", "JetBrains Mono"],
  newsreader: ["newsreader-variable.ttf", "Newsreader"],
};
const svg = '<svg viewBox="0 0 8 8"><title>Recollect</title></svg>';

function cleanTree() {
  const tree = {
    "src/design/tokens.css":
      ":root { --background: oklch(0.975 0.004 250); --brand: #3d6b59; }",
    "src/design/theme.css": `@theme inline {
  --color-*: initial;
  --color-background: var(--background);
  --text-title-1: 1.75rem;
  --text-title-1--line-height: 2.125rem;
  --shadow-glow: 0 0 0 4px var(--brand-soft);
  @keyframes ping { to { opacity: 0; } }
  --text-mono-label: 0.6875rem;
}`,
    "src/features/page.css":
      ".page { color: var(--foreground); border: 1px solid var(--border); }\n.page-code { @apply bg-card text-foreground; }",
    "src/features/Page.tsx": `import { cn } from "../lib/utils";
const variants = cva("inline-flex rounded-lg text-sm/6", {
  variants: { tone: { brand: "bg-brand text-brand-foreground hover:bg-brand-hover" } },
});
// Content, not a style declaration: a code sample showing a raw colour.
const sample = "body { color: #ff0000; background: oklch(0.5 0.1 20); }";
export function Page({ active }: { active: boolean }) {
  return (
    <section
      className={cn(
        "bg-card text-foreground border border-b border-x-2 border-border/60 shadow-glow text-title-1",
        "data-[state=open]:bg-accent [&_svg:not([class*='text-'])]:text-muted-foreground",
        "ring-2 ring-ring/50 ring-offset-2 ring-offset-background outline-hidden",
        "bg-linear-to-r from-brand/0 to-brand from-10% divide-y divide-border",
        "text-left text-balance bg-(--brand) transition-[color,box-shadow]",
        active && "bg-kind-claim-soft text-kind-claim hover:border-border-strong",
      )}
      style={{ color: "var(--brand-text)", border: "1px solid var(--border)" }}
    >
      <pre className="font-mono text-mono-label">{sample}</pre>
    </section>
  );
}`,
    "public/brand/recollect-symbol.svg": svg,
    "public/brand/recollect-symbol-mono.svg": svg,
    "public/brand/recollect-logo.svg": svg,
  };
  const files = [];
  for (const [directory, [file, family]] of Object.entries(fonts)) {
    const bytes = `${family} bytes`;
    tree[`public/fonts/${file}`] = bytes;
    tree[`public/licenses/${directory}-OFL.txt`] =
      `Copyright 2020 The ${family} Project Authors\n\nSIL OPEN FONT LICENSE Version 1.1\n`;
    files.push({
      file,
      source: `https://raw.githubusercontent.com/google/fonts/0/ofl/${directory}/${file}`,
      sha256: createHash("sha256").update(bytes).digest("hex"),
      bytes: Buffer.byteLength(bytes),
    });
  }
  tree["public/fonts/manifest.json"] = JSON.stringify({ files });
  return tree;
}

async function check(change) {
  const tree = cleanTree();
  change(tree);
  const root = await mkdtemp(path.join(tmpdir(), "check-design-"));
  try {
    for (const [file, content] of Object.entries(tree)) {
      if (content === null) continue;
      await mkdir(path.dirname(path.join(root, file)), { recursive: true });
      await writeFile(path.join(root, file), content);
    }
    const run = spawnSync(process.execPath, [script], {
      env: { ...process.env, DESIGN_CHECK_ROOT: root },
      encoding: "utf8",
    });
    return { status: run.status, output: run.stdout + run.stderr };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}
const addTsx = (tree, body) => {
  tree["src/features/Seeded.tsx"] = body;
};

test("the clean tree passes", async () => {
  const { status, output } = await check(() => {});
  assert.equal(status, 0, output);
});

const seeded = {
  "arbitrary hex in a class name": [
    (tree) => addTsx(tree, '<div className="bg-[#fff]" />'),
    "arbitrary colour value bg-[#fff]",
  ],
  "arbitrary rgb in a class name": [
    (tree) => addTsx(tree, '<div className="border-[rgb(0,0,0)]" />'),
    "arbitrary colour value",
  ],
  "arbitrary hsl in a class name": [
    (tree) => addTsx(tree, 'const c = cn("hover:fill-[hsl(1_2%_3%)]");'),
    "arbitrary colour value",
  ],
  "arbitrary oklch in a class name": [
    (tree) => addTsx(tree, '<i className="text-[oklch(0.5_0.1_167)]" />'),
    "arbitrary colour value",
  ],
  "arbitrary colour type hint": [
    (tree) => addTsx(tree, '<i className="bg-[color:var(--brand)]" />'),
    "arbitrary colour value",
  ],
  "default palette utility": [
    (tree) => addTsx(tree, '<i className="bg-red-500" />'),
    "bg-red-500 names no token role",
  ],
  "bare white utility": [
    (tree) => addTsx(tree, '<i className="hover:bg-white/80" />'),
    "hover:bg-white/80 names no token role",
  ],
  "bare black utility": [
    (tree) => addTsx(tree, 'const titleClassName = "text-black";'),
    "text-black names no token role",
  ],
  "default palette in @apply": [
    (tree) => {
      tree["src/features/seeded.css"] = ".x { @apply border-slate-200; }";
    },
    "border-slate-200 names no token role",
  ],
  "inline style named colour": [
    (tree) => addTsx(tree, '<i style={{ color: "white" }} />'),
    "inline style colour",
  ],
  "inline style oklch": [
    (tree) =>
      addTsx(tree, '<i style={{ backgroundColor: "oklch(0.5 0.1 167)" }} />'),
    "inline style colour",
  ],
  "raw oklch in feature CSS": [
    (tree) => {
      tree["src/features/seeded.css"] = ".x { color: oklch(0.5 0.1 167); }";
    },
    "raw colour literal",
  ],
  "named colour in feature CSS": [
    (tree) => {
      tree["src/features/seeded.css"] = ".x { background: black; }";
    },
    "named colour in background",
  ],
  "raw hex in a colour property": [
    (tree) => addTsx(tree, 'const accentColor = "#3d6b59";'),
    "raw colour literal",
  ],
  "missing Mona Sans manifest entry": [
    (tree) => {
      const manifest = JSON.parse(tree["public/fonts/manifest.json"]);
      manifest.files = manifest.files.filter(
        (font) => !font.source.includes("/monasans/"),
      );
      tree["public/fonts/manifest.json"] = JSON.stringify(manifest);
    },
    "Missing font manifest entry: Mona Sans",
  ],
  "missing JetBrains Mono notice": [
    (tree) => {
      tree["public/licenses/jetbrainsmono-OFL.txt"] = null;
    },
    "Missing public/licenses/jetbrainsmono-OFL.txt",
  ],
  "notice for the wrong family": [
    (tree) => {
      tree["public/licenses/newsreader-OFL.txt"] =
        "Copyright 2018 The Manrope Project Authors\nSIL OPEN FONT LICENSE\n";
    },
    "Missing OFL notice: newsreader",
  ],
  "font bytes changed": [
    (tree) => {
      tree["public/fonts/newsreader-variable.ttf"] = "changed";
    },
    "Font integrity mismatch: newsreader-variable.ttf",
  ],
  "untitled SVG": [
    (tree) => {
      tree["public/brand/recollect-logo.svg"] = '<svg viewBox="0 0 8 8"/>';
    },
    "SVG must be self-contained and titled: recollect-logo.svg",
  ],
};
for (const [name, [change, message]] of Object.entries(seeded))
  test(`fails on ${name}`, async () => {
    const { status, output } = await check(change);
    assert.equal(status, 1, output);
    assert.match(
      output,
      new RegExp(message.replace(/[[\]().*+?^$|\\/]/g, "\\$&")),
    );
  });
