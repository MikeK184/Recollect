#!/usr/bin/env node
// Renders the README's animated SVG illustrations in Light and Ink from the
// design tokens (web/src/design/tokens.css) and the original wordmark. GitHub
// shows SVGs as images, so motion is SMIL and CSS only, packets hide under
// reduced motion, and every frame reads with motion off. Run from the
// repository root: node scripts/render-readme-art.mjs
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const out = path.join(root, 'docs/assets/readme');

// --- Tokens: OKLCH roles from the token sheet, converted to sRGB hex. ---------
const css = await readFile(path.join(root, 'web/src/design/tokens.css'), 'utf8');
function block(selector) {
  const start = css.indexOf(selector);
  const body = css.slice(css.indexOf('{', start) + 1, css.indexOf('}', start));
  return Object.fromEntries([...body.matchAll(/--([\w-]+):\s*([^;]+);/g)].map(match => [match[1], match[2].trim()]));
}
function hex(value, roles) {
  const reference = value.match(/^var\(--([\w-]+)\)$/);
  if (reference) return hex(roles[reference[1]], roles);
  if (value.startsWith('#')) return value;
  const lch = value.match(/^oklch\(([\d.]+) ([\d.]+) ([\d.]+)\)$/);
  if (!lch) throw new Error(`Unsupported colour ${value}`);
  const [L, C, H] = lch.slice(1).map(Number);
  const a = C * Math.cos((H * Math.PI) / 180);
  const b = C * Math.sin((H * Math.PI) / 180);
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const linear = [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s, -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s];
  return '#' + linear.map(x => {
    const c = Math.min(1, Math.max(0, x));
    const encoded = c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
    return Math.round(encoded * 255).toString(16).padStart(2, '0');
  }).join('');
}
const names = ['background', 'card', 'muted', 'accent', 'border', 'border-strong', 'foreground', 'foreground-secondary', 'muted-foreground', 'faint-foreground', 'brand', 'brand-soft', 'brand-text', 'primary', 'primary-foreground', 'destructive', 'terminal', 'terminal-bar', 'terminal-border', 'terminal-foreground', 'terminal-muted', 'ansi-green', 'ansi-yellow', 'ansi-magenta', 'ansi-cyan', 'ansi-blue', 'kind-claim', 'brand-ink', 'brand-jade', 'brand-sage', 'brand-amber', 'brand-paper'];
const light = block(':root,');
const dark = { ...light, ...block('[data-theme="dark"]') };
const themes = {
  light: Object.fromEntries(names.map(name => [name, hex(light[name], light)])),
  ink: Object.fromEntries(names.map(name => [name, hex(dark[name], dark)])),
};

// --- Wordmark: the outlined Newsreader path from the portable logo. ----------
const logo = await readFile(path.join(root, 'web/public/brand/recollect-logo.svg'), 'utf8');
const wordmark = logo.match(/<\/g><path fill="#101b2a" d="([^"]+)"/)[1];

// --- Drawing helpers --------------------------------------------------------
const SANS = "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif";
const MONO = "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace";
const esc = text => text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const r3 = n => Math.round(n * 10000) / 10000;

// Opacity windows on a cycle of T seconds: [[start, end], ...].
function windows(list, T, fade = 0.18, on = 1, off = 0) {
  const times = [0];
  const values = [off];
  for (const [start, end] of list) {
    const s = Math.max(start, 0.01);
    times.push(s, s + fade, end - fade, end);
    values.push(off, on, on, off);
  }
  times.push(T);
  values.push(off);
  return `values="${values.join(';')}" keyTimes="${times.map(t => r3(t / T)).join(';')}" dur="${T}s" repeatCount="indefinite"`;
}
const fadeIn = (list, T) => `<animate attributeName="opacity" ${windows(list, T)}/>`;
function packet(d, start, end, T, reverse = false) {
  const s = r3(Math.max(start, 0.01) / T);
  const e = r3(end / T);
  return `<circle class="pk" r="5" fill="var(--brand)" filter="url(#pk)" opacity="0">
    <animateMotion path="${d}" dur="${T}s" repeatCount="indefinite" calcMode="spline" keyPoints="${reverse ? '1;1;0;0' : '0;0;1;1'}" keyTimes="0;${s};${e};1" keySplines="0 0 1 1;0.33 0 0.2 1;0 0 1 1"/>
    <animate attributeName="opacity" values="0;0;1;1;0;0" keyTimes="0;${s};${r3(s + 0.01)};${r3(e - 0.01)};${e};1" dur="${T}s" repeatCount="indefinite"/>
  </circle>`;
}
const edge = (d, flow = [], T = 1) => `<path class="edge" d="${d}"/>${flow.length ? `<path class="flow" d="${d}" opacity="0">${fadeIn(flow, T)}</path>` : ''}`;
const highlight = (x, y, w, h, rx, list, T) => `<rect class="hl" x="${x}" y="${y}" width="${w}" height="${h}" rx="${rx}" opacity="0">${fadeIn(list, T)}</rect>`;
const tile = (x, y, size = 36) => `<rect x="${x}" y="${y}" width="${size}" height="${size}" rx="${size / 4}" fill="var(--muted)"/>`;
const icons = {
  terminal: '<polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/>',
  server: '<rect width="20" height="8" x="2" y="2" rx="2"/><rect width="20" height="8" x="2" y="14" rx="2"/><line x1="6" x2="6.01" y1="6" y2="6"/><line x1="6" x2="6.01" y1="18" y2="18"/>',
  laptop: '<path d="M20 16V7a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v9m16 0H4m16 0 1.28 2.55a1 1 0 0 1-.9 1.45H3.62a1 1 0 0 1-.9-1.45L4 16"/>',
  shield: '<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/>',
  notebook: '<path d="M2 6h4"/><path d="M2 10h4"/><path d="M2 14h4"/><path d="M2 18h4"/><rect width="16" height="20" x="4" y="2" rx="2"/><path d="M9.5 8h5"/><path d="M9.5 12H16"/><path d="M9.5 16H14"/>',
  file: '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>',
  unplug: '<path d="m19 5 3-3"/><path d="m2 22 3-3"/><path d="M6.3 20.3a2.4 2.4 0 0 0 3.4 0L12 18l-6-6-2.3 2.3a2.4 2.4 0 0 0 0 3.4Z"/><path d="M7.5 13.5 10 11"/><path d="M10.5 16.5 13 14"/><path d="m12 6 6 6 2.3-2.3a2.4 2.4 0 0 0 0-3.4l-2.6-2.6a2.4 2.4 0 0 0-3.4 0Z"/>',
  check: '<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>',
};
const icon = (name, x, y, size = 18, color = 'var(--muted-foreground)') => `<g transform="translate(${x} ${y}) scale(${size / 24})" fill="none" stroke="${color}" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">${icons[name]}</g>`;
function mark(x, y, size, theme) {
  const ink = theme === 'ink' ? 'var(--foreground)' : 'var(--brand-ink)';
  const jade = theme === 'ink' ? 'var(--brand)' : 'var(--brand-jade)';
  return `<g transform="translate(${x} ${y}) scale(${size / 64})" fill="none"><g stroke-linecap="round" stroke-width="4.5">
    <path class="draw d1" pathLength="100" d="M29 8C17.4 9.4 8.5 19.3 8.5 31.2" stroke="${ink}"/>
    <path class="draw d2" pathLength="100" d="M40.5 10.2C51.5 14.6 57.6 27 53.4 38.4" stroke="${jade}"/>
    <path class="draw d3" pathLength="100" d="M46.5 48.6C36.7 56.9 21.4 54.9 13.8 44.6" stroke="${ink}"/>
    <path class="draw d4" pathLength="100" d="M22.2 23.8C29 17.5 40.5 21 41.2 30.3C41.9 38.9 31.2 43 25.2 36.9" stroke="${jade}"/></g>
    <path class="dot" d="m31.4 27 4.7 4.6-4.7 4.6-4.6-4.6Z" fill="${ink}"/>
    <circle class="dot" cx="33.5" cy="8" r="2.2" fill="var(--brand-sage)"/><circle class="dot" cx="51.1" cy="43.2" r="2.2" fill="var(--brand-amber)"/><circle class="dot" cx="10" cy="36.9" r="2.2" fill="var(--brand-sage)"/></g>`;
}
const paperMark = (x, y, size) => `<rect x="${x}" y="${y}" width="${size}" height="${size}" rx="${size / 4}" fill="var(--brand-paper)"/>${mark(x + size * 0.12, y + size * 0.12, size * 0.76, 'light').replace(/class="(draw d\d|dot)"/g, '')}`;

function svg(width, height, title, body, theme, extraStyle = '') {
  const colors = themes[theme];
  // Colours are written as literal hex values: presentation attributes do not
  // reliably resolve CSS variables when GitHub shows an SVG as an image.
  const resolve = text => text.replace(/var\(--([\w-]+)\)/g, (_, name) => {
    if (!colors[name]) throw new Error(`Unknown colour role ${name}`);
    return colors[name];
  });
  return resolve(`<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-label="${esc(title)}">
<title>${esc(title)}</title>
<style>
  text { font-family: ${SANS}; fill: var(--foreground); }
  .mono { font-family: ${MONO}; }
  .muted { fill: var(--muted-foreground); }
  .second { fill: var(--foreground-secondary); }
  .label { font-family: ${MONO}; font-size: 11.5px; letter-spacing: 0.08em; fill: var(--muted-foreground); }
  .card { fill: var(--card); stroke: var(--border-strong); stroke-width: 1; }
  .edge { fill: none; stroke: var(--border-strong); stroke-width: 1.6; }
  .flow { fill: none; stroke: var(--brand); stroke-width: 1.6; stroke-dasharray: 4 6; animation: flow 0.8s linear infinite; }
  .hl { fill: none; stroke: var(--brand); stroke-width: 1.6; filter: url(#glow); }
  .soft { fill: var(--brand-soft); }
  @keyframes flow { to { stroke-dashoffset: -10; } }
  @media (prefers-reduced-motion: reduce) { .pk, .flow { display: none; } }
  ${extraStyle}
</style>
<defs>
  <filter id="glow" x="-20%" y="-30%" width="140%" height="160%"><feDropShadow dx="0" dy="3" stdDeviation="6" flood-color="${colors.brand}" flood-opacity="${theme === 'ink' ? 0.35 : 0.22}"/></filter>
  <filter id="pk" x="-200%" y="-200%" width="500%" height="500%"><feDropShadow dx="0" dy="0" stdDeviation="2.5" flood-color="${colors.brand}" flood-opacity="0.8"/></filter>
</defs>
<rect x="0.5" y="0.5" width="${width - 1}" height="${height - 1}" rx="20" fill="var(--background)" stroke="var(--border)"/>
${body}
</svg>
`);
}

// --- Hero -------------------------------------------------------------------
function hero(theme) {
  const body = `
  ${paperMark(48, 46, 44)}
  <g transform="translate(38.6 38.2) scale(0.8)"><path fill="var(--foreground)" d="${wordmark}"/></g>
  <circle cx="257" cy="76.6" r="3.6" fill="var(--brand-text)"/>
  <text x="48" y="158" font-size="36" font-weight="700" letter-spacing="-1.1">Every coding agent starts</text>
  <text x="48" y="202" font-size="36" font-weight="700" letter-spacing="-1.1">where the last one stopped.</text>
  <text class="second" x="48" y="244" font-size="16.5">Shared memory, dynamic skills and governed MCP tools</text>
  <text class="second" x="48" y="268" font-size="16.5">for Codex, Claude Code and OpenCode. Self-hosted.</text>
  <circle cx="790" cy="152" r="112" fill="var(--brand-soft)" opacity="0.55"/>
  <circle class="ring" cx="790" cy="152" r="112" fill="none" stroke="var(--brand)" stroke-opacity="0.35"/>
  ${mark(700, 62, 180, theme)}`;
  const style = `
  .draw { stroke-dasharray: 100; stroke-dashoffset: 100; animation: draw 9s cubic-bezier(0.16, 1, 0.3, 1) infinite; }
  .d2 { animation-delay: 0.15s; } .d3 { animation-delay: 0.3s; } .d4 { animation-delay: 0.45s; }
  .dot { opacity: 0; animation: dot 9s ease-out infinite; animation-delay: 1s; transform-box: fill-box; transform-origin: center; }
  .ring { transform-box: fill-box; transform-origin: center; animation: ring 9s ease-out infinite; opacity: 0; }
  @keyframes draw { 0% { stroke-dashoffset: 100; } 16%, 86% { stroke-dashoffset: 0; } 98%, 100% { stroke-dashoffset: 100; } }
  @keyframes dot { 0% { opacity: 0; transform: scale(0.4); } 8%, 76% { opacity: 1; transform: scale(1); } 86%, 100% { opacity: 0; transform: scale(0.4); } }
  @keyframes ring { 0%, 18% { opacity: 0; transform: scale(0.94); } 26% { opacity: 1; } 44%, 100% { opacity: 0; transform: scale(1.08); } }
  @media (prefers-reduced-motion: reduce) { .draw { animation: none; stroke-dashoffset: 0; } .dot { animation: none; opacity: 1; } .ring { display: none; } }`;
  return svg(960, 300, 'Recollect: every coding agent starts where the last one stopped. Shared memory, dynamic skills and governed MCP tools for Codex, Claude Code and OpenCode.', body, theme, style);
}

// --- System overview ----------------------------------------------------------
function system(theme) {
  const T = 14;
  const agent = (x, y, name, sub, glow) => `<rect class="card" x="${x}" y="${y}" width="200" height="72" rx="14"/>${tile(x + 16, y + 18)}${icon('terminal', x + 25, y + 27)}<text x="${x + 64}" y="${y + 33}" font-size="16" font-weight="600">${name}</text><text class="mono muted" x="${x + 64}" y="${y + 53}" font-size="12.5">${sub}</text>${highlight(x, y, 200, 72, 14, glow, T)}`;
  const target = (x, y, name, sub, glyph, glow) => `<rect class="card" x="${x}" y="${y}" width="210" height="72" rx="14"/>${tile(x + 16, y + 18)}${icon(glyph, x + 25, y + 27)}<text x="${x + 64}" y="${y + 33}" font-size="16" font-weight="600">${name}</text><text class="mono muted" x="${x + 64}" y="${y + 53}" font-size="12.5">${sub}</text>${highlight(x, y, 210, 72, 14, glow, T)}`;
  const a = ['M240 96 C310 96 310 196 380 196', 'M240 236 C310 236 310 236 380 236', 'M240 376 C310 376 310 276 380 276'];
  const t = ['M580 196 C645 196 645 96 710 96', 'M580 236 C645 236 645 236 710 236', 'M580 276 C645 276 645 376 710 376'];
  const row = (y, glyph, name, valueA, valueB, list) => `<rect class="soft" x="392" y="${y}" width="176" height="52" rx="10" opacity="0">${fadeIn(list, T)}</rect>${icon(glyph, 406, y + 17)}<text class="second" x="434" y="${y + 31}" font-size="14.5" font-weight="500">${name}</text>${valueB
    ? `<text class="mono" x="554" y="${y + 31}" font-size="14" text-anchor="end">${valueA}<animate attributeName="opacity" values="1;1;0;0;1" keyTimes="0;${r3(3.3 / T)};${r3(3.4 / T)};${r3(13.8 / T)};1" dur="${T}s" repeatCount="indefinite"/></text><text class="mono" x="554" y="${y + 31}" font-size="14" text-anchor="end" opacity="0">${valueB}<animate attributeName="opacity" values="0;0;1;1;0" keyTimes="0;${r3(3.3 / T)};${r3(3.4 / T)};${r3(13.8 / T)};1" dur="${T}s" repeatCount="indefinite"/></text>`
    : `<text class="mono" x="554" y="${y + 31}" font-size="14" text-anchor="end">${valueA}</text>`}`;
  const steps = [
    ['01  CAPTURE', 'Claude Code finishes a fix. The plugin captures the permitted session events.', 0, 2.3],
    ['02  LEARN', 'Recollect checks what the evidence supports and learns the decision, with its source.', 2.3, 4.5],
    ['03  RECALL', 'Next morning, Codex gets that decision as cited context before its first prompt.', 4.5, 6.3],
    ['04  SKILLS', 'A skill published once in the Brain syncs into every host’s native skills folder.', 6.3, 8.3],
    ['05  TOOLS', 'OpenCode calls an approved MCP tool; it runs on a private runner inside your network.', 8.3, 13.6],
  ];
  const caption = steps.map(([label, text, s, e]) => `<g opacity="0">${fadeIn([[s, e]], T)}<text class="label" x="40" y="482" fill="var(--brand-text)" style="fill:var(--brand-text)">${label}</text><text class="second" x="150" y="482" font-size="14.5">${esc(text)}</text></g>`).join('');
  const body = `
  <rect x="704" y="330" width="222" height="114" rx="18" fill="none" stroke="var(--border-strong)" stroke-dasharray="3 5"/>
  <text class="label" x="815" y="436" text-anchor="middle" font-size="10.5">YOUR NETWORK</text>
  ${edge(a[0], [[4.5, 6.0], [6.3, 7.8]], T)}${edge(a[1], [[0.2, 1.8], [6.3, 7.8]], T)}${edge(a[2], [[6.3, 7.8], [8.3, 9.6], [12.0, 13.3]], T)}
  ${edge(t[0])}${edge(t[1])}${edge(t[2], [[9.6, 11.9]], T)}
  ${agent(40, 60, 'Codex', 'plugin', [[4.6, 6.2], [7.3, 8.1]])}
  ${agent(40, 200, 'Claude Code', 'plugin', [[0, 2.2], [7.4, 8.1]])}
  ${agent(40, 340, 'OpenCode', 'direct MCP', [[7.5, 8.1], [8.3, 13.6]])}
  <rect class="card" x="380" y="110" width="200" height="252" rx="16"/>
  ${paperMark(396, 126, 34)}<text x="442" y="141" font-size="16" font-weight="600">Brain</text><text class="mono muted" x="442" y="159" font-size="12.5">platform-team</text>
  <line x1="380" x2="580" y1="176" y2="176" stroke="var(--border)"/>
  ${row(184, 'notebook', 'Memory', '412', '413', [[1.3, 6.2]])}
  ${row(240, 'file', 'Skills', '6', '', [[6.2, 8.2]])}
  ${row(296, 'unplug', 'Tools', '9', '', [[9.2, 12.0]])}
  ${highlight(380, 110, 200, 252, 16, [[2.3, 4.5], [9.2, 9.9]], T)}
  ${target(710, 60, 'Recollect service', 'Context7 · Exa', 'server', [])}
  ${target(710, 200, 'Caller’s device', 'local stdio', 'laptop', [])}
  ${target(710, 340, 'Private runner', 'office-ubuntu', 'shield', [[10.4, 12.0]])}
  ${packet(a[1], 0.3, 1.5, T)}
  ${packet(a[0], 4.6, 5.8, T, true)}
  ${packet(a[0], 6.4, 7.5, T, true)}${packet(a[1], 6.5, 7.6, T, true)}${packet(a[2], 6.6, 7.7, T, true)}
  ${packet(a[2], 8.4, 9.4, T)}${packet(t[2], 9.6, 10.6, T)}${packet(t[2], 11.0, 11.9, T, true)}${packet(a[2], 12.0, 13.0, T, true)}
  <line x1="40" x2="920" y1="456" y2="456" stroke="var(--border)"/>
  ${caption}`;
  return svg(960, 500, 'How Recollect works: Codex, Claude Code and OpenCode share one Brain. Sessions are captured and learned, memory returns as cited context, skills sync to every host, and approved MCP calls run where you placed them.', body, theme);
}

// --- Memory loop --------------------------------------------------------------
function loop(theme) {
  const T = 10;
  const stages = [
    ['CAPTURE', 'Keep the evidence', ['Permitted prompts, tool', 'calls and imports enter', 'a durable queue.'], 'capture policy'],
    ['LEARN', 'Learn what holds', ['Claims and decisions', 'pass a support check and', 'keep their sources.'], 'support-checked'],
    ['RECALL', 'Recall with citations', ['Exact, keyword, semantic', 'and graph recall before', 'every prompt.'], '8 KiB per prompt'],
    ['ACT', 'Act with the right tools', ['Skills load on demand;', 'approved MCP calls run', 'where you placed them.'], 'governed tools'],
  ];
  const w = 212;
  const body = stages.map(([label, title, lines, meta], index) => {
    const x = 24 + index * (w + 20);
    const s = 0.3 + index * 2.2;
    const e = s + 2.2;
    const link = index < 3 ? `${edge(`M${x + w + 2} 112 L${x + w + 18} 112`, [[e - 0.4, e + 0.6]], T)}${packet(`M${x + w + 2} 112 L${x + w + 18} 112`, e - 0.3, e + 0.4, T)}` : '';
    return `<g><rect class="card" x="${x}" y="24" width="${w}" height="176" rx="14"/>
      <text class="label" x="${x + 18}" y="52">${label}</text>
      <g opacity="0">${fadeIn([[e - 0.2, T - 0.4]], T)}${icon('check', x + w - 34, 38, 18, 'var(--brand)')}</g>
      <text x="${x + 18}" y="82" font-size="15.5" font-weight="600">${title}</text>
      ${lines.map((line, row) => `<text class="second" x="${x + 18}" y="${106 + row * 19}" font-size="13.5">${line}</text>`).join('')}
      <rect x="${x + 18}" y="164" width="${meta.length * 7.6 + 18}" height="22" rx="11" fill="var(--muted)"/>
      <text class="mono muted" x="${x + 27}" y="179" font-size="11.5">${meta}</text>
      ${highlight(x, 24, w, 176, 14, [[s, e]], T)}</g>${link}`;
  }).join('');
  return svg(960, 224, 'The memory loop: capture, learn, recall and act on every prompt, each step keeping its evidence.', body, theme);
}

// --- Dynamic skills -----------------------------------------------------------
function skills(theme) {
  const T = 9;
  const fan = ['M356 162 C398 162 398 70 440 70', 'M356 162 C398 162 398 162 440 162', 'M356 162 C398 162 398 254 440 254'];
  const hosts = [['Claude Code', '.claude/skills/vault-inspection', 'reload_skills'], ['Codex', '.agents/skills/vault-inspection', 'file watch'], ['OpenCode', '.opencode/skills/vault-inspection', 'ctx.skill.reload()']];
  const hostRows = hosts.map(([name, location, reload], index) => {
    const y = 34 + index * 92;
    const arrive = 2.9 + index * 0.15;
    return `<rect class="card" x="440" y="${y}" width="496" height="72" rx="14"/>${tile(456, y + 18)}${icon('terminal', 465, y + 27)}
      <text x="504" y="${y + 32}" font-size="15.5" font-weight="600">${name}</text><text class="mono muted" x="504" y="${y + 52}" font-size="12.5">${location}</text>
      <rect x="826" y="${y + 14}" width="94" height="24" rx="12" fill="var(--muted)"/><rect class="soft" x="826" y="${y + 14}" width="94" height="24" rx="12" opacity="0">${fadeIn([[arrive, T - 0.3]], T)}</rect>
      <text class="mono second" x="873" y="${y + 30}" font-size="12" text-anchor="middle">r3<animate attributeName="opacity" values="1;1;0;0;1" keyTimes="0;${r3(arrive / T)};${r3((arrive + 0.05) / T)};${r3((T - 0.3) / T)};1" dur="${T}s" repeatCount="indefinite"/></text>
      <text class="mono" x="873" y="${y + 30}" font-size="12" text-anchor="middle" style="fill:var(--brand-text)" opacity="0">r4 loaded<animate attributeName="opacity" values="0;0;1;1;0" keyTimes="0;${r3(arrive / T)};${r3((arrive + 0.05) / T)};${r3((T - 0.3) / T)};1" dur="${T}s" repeatCount="indefinite"/></text>
      <text class="mono" x="920" y="${y + 58}" font-size="11" text-anchor="end" style="fill:var(--faint-foreground)">${reload}</text>
      ${highlight(440, y, 496, 72, 14, index === 0 ? [[1.9 + index * 0.15, 3.2 + index * 0.15], [4.6, 7.6]] : [[1.9 + index * 0.15, 3.2 + index * 0.15]], T)}`;
  }).join('');
  const body = `
  <rect class="card" x="24" y="34" width="332" height="256" rx="14"/>
  ${tile(40, 50)}${icon('file', 49, 59)}<text x="88" y="66" font-size="15.5" font-weight="600">vault-inspection</text><text class="muted" x="88" y="85" font-size="12.5">Agents → Skills · Brain-wide</text>
  <rect x="296" y="56" width="44" height="24" rx="12" fill="var(--muted)"/>
  <text class="mono" x="318" y="72" font-size="12" text-anchor="middle">r3<animate attributeName="opacity" values="1;1;0;0;1" keyTimes="0;${r3(1.0 / T)};${r3(1.05 / T)};${r3((T - 0.3) / T)};1" dur="${T}s" repeatCount="indefinite"/></text>
  <text class="mono" x="318" y="72" font-size="12" text-anchor="middle" opacity="0">r4<animate attributeName="opacity" values="0;0;1;1;0" keyTimes="0;${r3(1.0 / T)};${r3(1.05 / T)};${r3((T - 0.3) / T)};1" dur="${T}s" repeatCount="indefinite"/></text>
  <rect x="40" y="102" width="300" height="140" rx="10" fill="var(--muted)"/>
  ${['---', 'name: vault-inspection', 'description: Inspect Vault with', '  attributable evidence.', '---', 'Read repository instructions first.'].map((line, index) => `<text class="mono" x="54" y="${126 + index * 19}" font-size="12" style="fill:${line === '---' ? 'var(--faint-foreground)' : 'var(--foreground-secondary)'}">${esc(line)}</text>`).join('')}
  <text class="muted" x="40" y="271" font-size="12.5">Markdown and text only · no scripts</text>
  <rect x="260" y="254" width="80" height="26" rx="8" fill="var(--primary)"/><text x="300" y="271.5" font-size="12.5" font-weight="500" text-anchor="middle" style="fill:var(--primary-foreground)">Publish</text>
  ${highlight(24, 34, 332, 256, 14, [[0.2, 1.7]], T)}
  ${fan.map(d => edge(d, [[1.8, 3.2]], T)).join('')}
  ${hostRows}
  ${fan.map((d, index) => packet(d, 1.9 + index * 0.15, 2.9 + index * 0.15, T)).join('')}
  <text class="mono muted" x="440" y="318" font-size="12">skills.list → metadata · skills.fetch → the body, on demand</text>`;
  return svg(960, 340, 'Dynamic skills: a skill published once in the Brain syncs as a native skill into Claude Code, Codex and OpenCode, which reload it.', body, theme);
}

// --- Governed tools -----------------------------------------------------------
function tools(theme) {
  const T = 10;
  const fan = ['M612 162 C651 162 651 70 690 70', 'M612 162 C651 162 651 162 690 162', 'M612 162 C651 162 651 254 690 254'];
  const link = 'M286 162 L330 162';
  const checks = [['Use grant', 'Alex · Office files group', 1.5], ['Environment', 'Development', 2.0], ['Approved tool', 'read_file · pinned schema', 2.5]];
  const targets = [['Recollect service', 'Context7 · Exa', 'server'], ['Caller’s device', 'local stdio', 'laptop'], ['Private runner', 'office-ubuntu · outbound', 'shield']];
  const body = `
  <rect x="24" y="64" width="262" height="196" rx="14" fill="var(--terminal)" stroke="var(--terminal-border)"/>
  <path d="M24 78 a14 14 0 0 1 14 -14 h234 a14 14 0 0 1 14 14 v18 h-262 z" fill="var(--terminal-bar)"/>
  <text class="mono" x="40" y="86" font-size="12" style="fill:var(--terminal-foreground)">opencode</text><text class="mono" x="270" y="86" font-size="12" text-anchor="end" style="fill:var(--terminal-muted)">mcp.call</text>
  ${[['tool', 'filesystem.read_file'], ['caller', 'alex · OpenCode'], ['env', 'Development']].map(([key, value], index) => `<text class="mono" x="40" y="${126 + index * 26}" font-size="12.5" style="fill:var(--terminal-muted)">${key}</text><text class="mono" x="104" y="${126 + index * 26}" font-size="12.5" style="fill:var(--terminal-foreground)">${value}</text>`).join('')}
  <text class="mono" x="40" y="230" font-size="12.5" style="fill:var(--terminal-muted)">result</text>
  <text class="mono" x="104" y="230" font-size="12.5" style="fill:var(--terminal-muted)">waiting…<animate attributeName="opacity" values="1;1;0;0;1" keyTimes="0;${r3(6.1 / T)};${r3(6.15 / T)};${r3((T - 0.3) / T)};1" dur="${T}s" repeatCount="indefinite"/></text>
  <text class="mono" x="104" y="230" font-size="12.5" style="fill:var(--ansi-green)" opacity="0">3 files · captured<animate attributeName="opacity" values="0;0;1;1;0" keyTimes="0;${r3(6.1 / T)};${r3(6.15 / T)};${r3((T - 0.3) / T)};1" dur="${T}s" repeatCount="indefinite"/></text>
  ${highlight(24, 64, 262, 196, 14, [[0.1, 1.3], [5.4, 6.6]], T)}
  ${edge(link, [[0.2, 1.3], [5.2, 6.2]], T)}
  <rect class="card" x="330" y="50" width="282" height="224" rx="14"/>
  <text class="label" x="348" y="78">RECOLLECT CHECKS</text>
  ${checks.map(([name, sub, at], index) => { const y = 96 + index * 58; return `<line x1="348" x2="594" y1="${y}" y2="${y}" stroke="var(--border)"/><text x="348" y="${y + 24}" font-size="14.5" font-weight="600">${name}</text><text class="muted" x="348" y="${y + 43}" font-size="12.5">${sub}</text><g opacity="0">${fadeIn([[at, T - 0.3]], T)}${icon('check', 570, y + 17, 20, 'var(--brand)')}</g>`; }).join('')}
  ${highlight(330, 50, 282, 224, 14, [[1.2, 2.9]], T)}
  ${fan.map((d, index) => edge(d, index === 2 ? [[2.9, 6.0]] : [], T)).join('')}
  ${targets.map(([name, sub, glyph], index) => { const y = 34 + index * 92; return `<rect class="card" x="690" y="${y}" width="246" height="72" rx="14"/>${tile(706, y + 18)}${icon(glyph, 715, y + 27)}<text x="754" y="${y + 32}" font-size="15.5" font-weight="600">${name}</text><text class="mono muted" x="754" y="${y + 52}" font-size="12">${sub}</text>${index === 2 ? highlight(690, y, 246, 72, 14, [[3.8, 5.4]], T) : ''}`; }).join('')}
  <rect x="682" y="210" width="262" height="120" rx="18" fill="none" stroke="var(--border-strong)" stroke-dasharray="3 5"/>
  <text class="label" x="813" y="322" text-anchor="middle" font-size="10.5">YOUR NETWORK</text>
  ${packet(link, 0.3, 1.2, T)}${packet(fan[2], 3.0, 4.0, T)}${packet(fan[2], 4.5, 5.3, T, true)}${packet(link, 5.4, 6.1, T, true)}`;
  return svg(960, 344, 'Governed MCP tools: OpenCode requests a tool, Recollect checks the Use grant, environment and approved tool, then dispatches the call to a private runner inside your network and returns the result.', body, theme);
}

await mkdir(out, { recursive: true });
const drawings = { hero, system, loop, skills, tools };
for (const [name, draw] of Object.entries(drawings)) {
  for (const theme of ['light', 'ink']) await writeFile(path.join(out, `${name}-${theme}.svg`), draw(theme));
}
console.log(`Rendered ${Object.keys(drawings).length * 2} README illustrations into docs/assets/readme/.`);
