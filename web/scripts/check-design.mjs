import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import ts from "typescript";

// DESIGN_CHECK_ROOT points the check at a seeded copy (check-design.test.mjs).
const root = process.env.DESIGN_CHECK_ROOT
  ? path.resolve(process.env.DESIGN_CHECK_ROOT)
  : fileURLToPath(new URL("../", import.meta.url));
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
async function read(file) {
  try {
    return await readFile(path.join(root, file));
  } catch {
    failures.push(`Missing ${file}`);
    return null;
  }
}

// Colour roles of the Tailwind `@theme inline` map, from the version 2 tokens
// pinned in docs/foundation/design-system.md. Colour utilities must name one;
// the theme clears `--color-*`, so the default palette, white and black are out.
const colours = new Set([
  ...`background foreground foreground-secondary muted-foreground
  faint-foreground card card-foreground popover popover-foreground elevated
  primary primary-hover primary-foreground secondary secondary-foreground muted
  accent accent-foreground border border-strong input ring overlay thumb brand
  brand-hover brand-foreground brand-soft brand-text success destructive
  destructive-foreground destructive-soft destructive-text warning
  warning-foreground warning-soft warning-text info info-soft sidebar
  sidebar-foreground sidebar-primary sidebar-primary-foreground sidebar-accent
  sidebar-accent-foreground sidebar-border sidebar-ring terminal terminal-bar
  terminal-border terminal-foreground terminal-muted brand-ink brand-jade
  brand-sage brand-amber brand-paper`.split(/\s+/),
  ...[
    "claim",
    "decision",
    "procedure",
    "handover",
    "source",
    "entity",
    "tool",
  ].flatMap((kind) => [`kind-${kind}`, `kind-${kind}-soft`]),
  ...[1, 2, 3, 4, 5].map((series) => `chart-${series}`),
  ...["red", "green", "yellow", "blue", "magenta", "cyan"].map(
    (name) => `ansi-${name}`,
  ),
  "transparent",
  "current",
  "inherit",
]);
const colourUtility =
  /^(text-shadow|inset-shadow|drop-shadow|inset-ring|ring-offset|border-[xytrblse]|bg|text|border|outline|ring|shadow|divide|accent|caret|fill|stroke|decoration|placeholder|from|via|to)-(.+)$/;
// Tailwind values under those prefixes that set no colour; the theme's own
// --text-*/--shadow-* scales are added from its @theme blocks below.
const nonColour = {
  bg: /^(?:none|fixed|local|scroll|auto|cover|contain|center|top|bottom|left|right|(?:top|bottom|left|right)-(?:top|bottom|left|right)|repeat(?:-[xy]|-round|-space)?|no-repeat|(?:clip|origin|blend|size|position|linear|radial|conic|gradient)(?:-.+)?)$/,
  text: /^(?:xs|sm|base|lg|\d?xl|left|center|right|justify|start|end|wrap|nowrap|balance|pretty|ellipsis|clip)$/,
  // `border-b`, `border-x` set a side's width, not a colour.
  border:
    /^(?:[xytrblse]|solid|dashed|dotted|double|hidden|none|collapse|separate|spacing(?:-.+)?)$/,
  divide: /^(?:[xy](?:-\d+|-reverse)?|solid|dashed|dotted|double|none)$/,
  outline: /^(?:none|hidden|solid|dashed|dotted|double|offset-.+)$/,
  ring: /^inset$/,
  shadow: /^(?:2xs|xs|sm|md|lg|xl|2xl|none|inner)$/,
  "inset-shadow": /^(?:2xs|xs|sm|none)$/,
  "drop-shadow": /^(?:xs|sm|md|lg|xl|2xl|none)$/,
  "text-shadow": /^(?:2xs|xs|sm|md|lg|none)$/,
  accent: /^auto$/,
  fill: /^none$/,
  stroke: /^none$/,
  decoration:
    /^(?:solid|double|dotted|dashed|wavy|auto|from-font|clone|slice)$/,
};
const themeScales = new Map();
const rawColour =
  /#[\da-f]{3,8}(?![\da-z])|\b(?:rgba?|hsla?|oklch|oklab|lab|lch|hwb|color)\(/i;
const namedColour = new RegExp(
  `(?<![\\w-])(?:${`aliceblue antiquewhite aqua aquamarine azure beige bisque black
  blanchedalmond blue blueviolet brown burlywood cadetblue chartreuse chocolate
  coral cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod
  darkgray darkgreen darkgrey darkkhaki darkmagenta darkolivegreen darkorange
  darkorchid darkred darksalmon darkseagreen darkslateblue darkslategray
  darkslategrey darkturquoise darkviolet deeppink deepskyblue dimgray dimgrey
  dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold
  goldenrod gray green greenyellow grey honeydew hotpink indianred indigo ivory
  khaki lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral
  lightcyan lightgoldenrodyellow lightgray lightgreen lightgrey lightpink
  lightsalmon lightseagreen lightskyblue lightslategray lightslategrey
  lightsteelblue lightyellow lime limegreen linen magenta maroon
  mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen
  mediumslateblue mediumspringgreen mediumturquoise mediumvioletred
  midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive
  olivedrab orange orangered orchid palegoldenrod palegreen paleturquoise
  palevioletred papayawhip peachpuff peru pink plum powderblue purple
  rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown seagreen
  seashell sienna silver skyblue slateblue slategray slategrey snow springgreen
  steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke
  yellow yellowgreen`
    .trim()
    .split(/\s+/)
    .join("|")})(?![\\w-])`,
  "i",
);
// A colour value names a colour once token references and urls are removed.
function literalColour(value) {
  const rest = value.replace(/var\(--[\w-]+\)|url\([^)]*\)/gi, "");
  return rawColour.test(rest) || namedColour.test(rest);
}
function classFailures(classes) {
  const found = [];
  for (const token of classes.split(/\s+/).filter(Boolean)) {
    // Drop variants (hover:, data-[state=open]:) and the important marker.
    let depth = 0;
    let start = 0;
    for (let index = 0; index < token.length; index += 1) {
      if ("[(".includes(token[index])) depth += 1;
      else if ("])".includes(token[index])) depth -= 1;
      else if (token[index] === ":" && depth === 0) start = index + 1;
    }
    const utility = token.slice(start).replace(/^!|!$/g, "").replace(/^-/, "");
    const bracket = utility.search(/[[(]/);
    if (bracket >= 0) {
      const value = utility.slice(bracket);
      if (/^[[(]color:/i.test(value) || literalColour(value))
        found.push(`arbitrary colour value ${token}`);
      continue;
    }
    const match = utility.match(colourUtility);
    if (!match) continue;
    const prefix = match[1].startsWith("border-") ? "border" : match[1];
    const value = match[2].replace(/\/[^/]*$/, "");
    if (
      !colours.has(value) &&
      !/^(?:\d+(?:\.\d+)?%?|px)$/.test(value) &&
      !nonColour[prefix]?.test(value) &&
      !themeScales.get(prefix)?.has(value)
    )
      found.push(`${token} names no token role`);
  }
  return found;
}

const sources = (await files(path.join(root, "src"))).filter(
  (file) => /\.(?:css|tsx?)$/.test(file) && !file.endsWith(".d.ts"),
);
const css = new Map();
for (const file of sources.filter((file) => file.endsWith(".css")))
  css.set(
    file,
    (await readFile(file, "utf8")).replace(/\/\*[\s\S]*?\*\//g, ""),
  );
for (const rules of css.values())
  for (const block of rules.matchAll(/@theme\b[^{]*\{/g)) {
    let depth = 1;
    let end = block.index + block[0].length;
    while (depth && end < rules.length) {
      if (rules[end] === "{") depth += 1;
      if (rules[end] === "}") depth -= 1;
      end += 1;
    }
    const theme = rules.slice(block.index, end);
    for (const [, name] of theme.matchAll(/--([\w-]+)\s*:/g)) {
      const prefix = [
        "text-shadow",
        "inset-shadow",
        "drop-shadow",
        "shadow",
        "text",
      ].find((scale) => name.startsWith(`${scale}-`));
      if (!prefix) continue;
      if (!themeScales.has(prefix)) themeScales.set(prefix, new Set());
      themeScales.get(prefix).add(name.slice(prefix.length + 1).split("--")[0]);
    }
  }

const designLayer = `${path.sep}src${path.sep}design${path.sep}`;
const cssColourProperty =
  /^(?:--[\w-]+|color|background(?:-color|-image)?|border(?:-[\w-]+)?|outline(?:-color)?|fill|stroke|box-shadow|text-shadow|caret-color|accent-color|text-decoration(?:-color)?|column-rule(?:-color)?|stop-color|flood-color|lighting-color|scrollbar-color)$/i;
for (const [file, rules] of css) {
  const relative = path.relative(root, file);
  for (const [, classes] of rules.matchAll(/@apply\s+([^;]+);/g))
    for (const failure of classFailures(classes))
      failures.push(`${relative}: ${failure}`);
  if (file.includes(designLayer)) continue;
  if (rawColour.test(rules))
    failures.push(
      `${relative}: raw colour literal; use a token role from src/design`,
    );
  for (const [, property, value] of rules.matchAll(
    /([\w-]+)\s*:\s*([^;{}]+)(?=[;}])/g,
  ))
    if (
      cssColourProperty.test(property) &&
      namedColour.test(value.replace(/var\(--[\w-]+\)|url\([^)]*\)/gi, ""))
    )
      failures.push(
        `${relative}: named colour in ${property}; use a token role`,
      );
  if (
    [...rules.matchAll(/font-family\s*:\s*([^;]+);/gi)].some(
      (match) => !match[1].trim().startsWith("var(--"),
    )
  )
    failures.push(`${relative}: use a shared font variable`);
}

const classAttribute = /^(?:class|className|classNames|\w+ClassNames?)$/;
const classHelpers = new Set([
  "cn",
  "clsx",
  "cva",
  "cx",
  "twMerge",
  "twJoin",
  "tv",
]);
const styleColourProperty =
  /^(?:--[\w-]+|color|background\w*|border\w*|outline\w*|fill|stroke|\w*Shadow|\w*Color|textDecoration|columnRule)$/;
for (const file of sources.filter((file) => !file.endsWith(".css"))) {
  const content = await readFile(file, "utf8");
  const relative = path.relative(root, file);
  const designFile = file.includes(designLayer);
  const ast = ts.createSourceFile(file, content, ts.ScriptTarget.Latest, true);
  const nameOf = (node) =>
    node.name?.getText(ast).replaceAll('"', "").replaceAll("'", "") ?? "";
  const line = (node) =>
    ast.getLineAndCharacterOfPosition(node.getStart(ast)).line + 1;
  function visit(node) {
    const text =
      ts.isStringLiteralLike(node) ||
      ts.isTemplateHead(node) ||
      ts.isTemplateMiddle(node) ||
      ts.isTemplateTail(node)
        ? node.text
        : null;
    if (text !== null) {
      // Classify a literal by where it sits: class lists, style properties and
      // inline styles are declarations; any other string (an example hex value
      // in a code sample, say) is content and is not checked.
      let classList = false;
      let styleValue = false;
      let inlineStyle = false;
      let styleProperty = null;
      for (
        let parent = node.parent;
        parent && !ts.isSourceFile(parent);
        parent = parent.parent
      ) {
        const name =
          ts.isPropertyAssignment(parent) ||
          ts.isJsxAttribute(parent) ||
          ts.isVariableDeclaration(parent)
            ? nameOf(parent)
            : "";
        if (classAttribute.test(name)) classList = true;
        if (
          ts.isCallExpression(parent) &&
          classHelpers.has(parent.expression.getText(ast))
        )
          classList = true;
        if (
          (ts.isPropertyAssignment(parent) || ts.isJsxAttribute(parent)) &&
          /^(c|bg|style|styles|.*color.*|background.*|border.*|fill|stroke|.*shadow.*|outline.*)$/i.test(
            name,
          )
        )
          styleValue = true;
        if (ts.isVariableDeclaration(parent) && /colou?r/i.test(name))
          styleValue = true;
        if (ts.isPropertyAssignment(parent) && styleProperty === null)
          styleProperty = name;
        if (
          (ts.isJsxAttribute(parent) || ts.isPropertyAssignment(parent)) &&
          /^(style|styles)$/.test(name)
        )
          inlineStyle = true;
        if (ts.isStatement(parent)) break;
      }
      if (classList)
        for (const failure of classFailures(text))
          failures.push(`${relative}:${line(node)}: ${failure}`);
      if (
        !designFile &&
        inlineStyle &&
        styleColourProperty.test(styleProperty ?? "") &&
        literalColour(text)
      )
        failures.push(
          `${relative}:${line(node)}: inline style colour; use a token utility`,
        );
      else if (!designFile && styleValue && rawColour.test(text))
        failures.push(
          `${relative}:${line(node)}: raw colour literal; move it to src/design`,
        );
    }
    if (
      !designFile &&
      ts.isPropertyAssignment(node) &&
      /^(fontFamily|font-family)$/.test(nameOf(node)) &&
      ts.isStringLiteralLike(node.initializer) &&
      !node.initializer.text.startsWith("var(--")
    )
      failures.push(`${relative}: use fonts.* or a shared font variable`);
    ts.forEachChild(node, visit);
  }
  visit(ast);
}

// Version 2 families, each pinned from google/fonts `ofl/<directory>/` with its
// notice at public/licenses/<directory>-OFL.txt.
const fontFamilies = {
  monasans: "Mona Sans",
  jetbrainsmono: "JetBrains Mono",
  newsreader: "Newsreader",
};
const manifestBytes = await read("public/fonts/manifest.json");
const manifest = manifestBytes ? JSON.parse(manifestBytes) : { files: [] };
const shipped = new Set();
for (const font of manifest.files) {
  const directory = font.source?.match(/\/ofl\/([a-z\d]+)\//)?.[1];
  if (directory) shipped.add(directory);
  else failures.push(`Font has no google/fonts ofl source: ${font.file}`);
  const bytes = await read(path.join("public/fonts", font.file));
  if (
    bytes &&
    (bytes.length !== font.bytes ||
      createHash("sha256").update(bytes).digest("hex") !== font.sha256)
  )
    failures.push(`Font integrity mismatch: ${font.file}`);
}
for (const directory of Object.keys(fontFamilies))
  if (!shipped.has(directory))
    failures.push(
      `Missing font manifest entry: ${fontFamilies[directory]} (ofl/${directory})`,
    );
for (const directory of new Set([...Object.keys(fontFamilies), ...shipped])) {
  const license = await read(
    path.join("public/licenses", `${directory}-OFL.txt`),
  );
  if (
    license &&
    (!license.includes("SIL OPEN FONT LICENSE") ||
      (fontFamilies[directory] &&
        !license.includes(`The ${fontFamilies[directory]} Project Authors`)))
  )
    failures.push(`Missing OFL notice: ${directory}`);
}
for (const name of [
  "recollect-symbol.svg",
  "recollect-symbol-mono.svg",
  "recollect-logo.svg",
]) {
  const svg = (await read(path.join("public/brand", name)))?.toString() ?? "";
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
  `Design checks passed: token colours and utilities, shared fonts, ${manifest.files.length} pinned fonts and licenses, 3 self-contained SVG assets.`,
);
