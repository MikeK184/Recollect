import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import ts from "typescript";

const root = fileURLToPath(new URL("../", import.meta.url));
const failures = [];
async function files(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  return (
    await Promise.all(
      entries.map((entry) =>
        entry.isDirectory()
          ? files(path.join(directory, entry.name))
          : [path.join(directory, entry.name)],
      ),
    )
  ).flat();
}
const color = /(?:#[a-f\d]{3,8}\b|\brgba?\(|\bhsla?\()/i;
for (const file of await files(path.join(root, "src"))) {
  if (
    !/\.(?:css|tsx?)$/.test(file) ||
    file.endsWith(".d.ts") ||
    file.includes(`${path.sep}design${path.sep}`)
  )
    continue;
  const content = await readFile(file, "utf8");
  const relative = path.relative(root, file);
  if (file.endsWith(".css")) {
    const rules = content.replace(/\/\*[\s\S]*?\*\//g, "");
    if (color.test(rules))
      failures.push(`${relative}: use semantic CSS color variables`);
    if (
      [...rules.matchAll(/font-family\s*:\s*([^;]+);/gi)].some(
        (match) => !match[1].trim().startsWith("var(--"),
      )
    )
      failures.push(`${relative}: use a shared font variable`);
  } else {
    const ast = ts.createSourceFile(
      file,
      content,
      ts.ScriptTarget.Latest,
      true,
    );
    function visit(node) {
      // Scope TS literals to actual style properties/attributes. An example
      // source body containing a hex value is content, not a theme declaration.
      let isStyleValue = false;
      for (
        let parent = node.parent;
        parent && !ts.isSourceFile(parent);
        parent = parent.parent
      ) {
        if (
          (ts.isPropertyAssignment(parent) || ts.isJsxAttribute(parent)) &&
          /^(c|bg|style|styles|.*color.*|background.*|border.*|fill|stroke|.*shadow.*|outline.*)$/i.test(
            parent.name.getText(ast).replaceAll('"', "").replaceAll("'", ""),
          )
        )
          isStyleValue = true;
        if (ts.isStatement(parent)) break;
      }
      if (isStyleValue && ts.isStringLiteralLike(node) && color.test(node.text))
        failures.push(
          `${relative}:${ast.getLineAndCharacterOfPosition(node.pos).line + 1}: move color to design/tokens.ts`,
        );
      if (
        ts.isPropertyAssignment(node) &&
        /^(fontFamily|font-family)$/.test(
          node.name.getText(ast).replaceAll('"', "").replaceAll("'", ""),
        ) &&
        ts.isStringLiteralLike(node.initializer) &&
        !node.initializer.text.startsWith("var(--")
      )
        failures.push(`${relative}: use fonts.* or a shared font variable`);
      ts.forEachChild(node, visit);
    }
    visit(ast);
  }
}
const manifest = JSON.parse(
  await readFile(path.join(root, "public/fonts/manifest.json"), "utf8"),
);
for (const font of manifest.files) {
  const bytes = await readFile(path.join(root, "public/fonts", font.file));
  if (
    bytes.length !== font.bytes ||
    createHash("sha256").update(bytes).digest("hex") !== font.sha256
  )
    failures.push(`Font integrity mismatch: ${font.file}`);
}
for (const name of ["newsreader", "manrope", "dmmono"]) {
  const license = await readFile(
    path.join(root, "public/licenses", `${name}-OFL.txt`),
    "utf8",
  );
  if (!license.includes("SIL OPEN FONT LICENSE"))
    failures.push(`Missing OFL notice: ${name}`);
}
for (const name of [
  "recollect-symbol.svg",
  "recollect-symbol-mono.svg",
  "recollect-logo.svg",
]) {
  const svg = await readFile(path.join(root, "public/brand", name), "utf8");
  if (
    !svg.includes("<title") ||
    !svg.includes("viewBox=") ||
    /<script|<foreignObject|(?:href|src)=["'](?:https?:|data:|\/\/)/i.test(svg)
  )
    failures.push(`SVG must be self-contained and titled: ${name}`);
}
if (failures.length) {
  console.error(failures.join("\n"));
  process.exit(1);
}
console.log(
  `Design checks passed: shared colors/fonts, ${manifest.files.length} pinned fonts and licenses, 3 self-contained SVG assets.`,
);
