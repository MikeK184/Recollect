import { readFile, readdir, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { pages } from '../catalog.mjs';

const output = fileURLToPath(new URL('../.vitepress/dist/', import.meta.url));
const base = process.env.SITE_BASE || '/Recollect/';
const origin = 'https://site.invalid';
async function walk(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const target = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`Symlink in static artifact: ${target}`);
    if (entry.isDirectory()) files.push(...await walk(target));
    else files.push(target);
  }
  return files;
}
const files = await walk(output);
const html = new Map(await Promise.all(files.filter(file => file.endsWith('.html')).map(async file => [path.relative(output, file).replaceAll('\\', '/'), await readFile(file, 'utf8')])));
for (const required of ['index.html', 'docs/index.html', '404.html', ...pages.map(page => `docs/${page.slug}.html`)]) {
  if (!html.has(required)) throw new Error(`Missing website route: ${required}`);
}
for (const page of pages) {
  const content = html.get(`docs/${page.slug}.html`);
  if (!content.includes('<h1') || !content.includes(`https://github.com/MikeK184/Recollect/edit/main/${page.source}`)) {
    throw new Error(`Incomplete guide rendering or incorrect canonical edit link: ${page.slug}`);
  }
}
let references = 0;
for (const [file, content] of html) {
  for (const match of content.matchAll(/(?:href|src)="([^"]+)"/g)) {
    const target = match[1].replaceAll('&amp;', '&');
    if (/^(data:|mailto:|javascript:)/.test(target)) continue;
    const url = new URL(target, `${origin}${base}${file}`);
    if (url.origin !== origin) continue;
    if (!url.pathname.startsWith(base)) throw new Error(`Link escapes project base in ${file}: ${target}`);
    let relative = decodeURIComponent(url.pathname.slice(base.length));
    if (!relative || relative.endsWith('/')) relative += 'index.html';
    const resolved = path.join(output, relative);
    if (!resolved.startsWith(output)) throw new Error(`Unsafe artifact target: ${target}`);
    try { await stat(resolved); } catch { throw new Error(`Broken reference in ${file}: ${target}`); }
    if (url.hash && html.has(relative)) {
      const id = decodeURIComponent(url.hash.slice(1));
      const ids = new Set([...html.get(relative).matchAll(/id="([^"]+)"/g)].map(item => item[1]));
      if (!ids.has(id)) throw new Error(`Broken anchor in ${file}: ${target}`);
    }
    references++;
  }
}
for (const required of ['brand/recollect-logo.svg', 'brand/recollect-symbol.svg', 'fonts/newsreader-variable.ttf', 'licenses/newsreader-OFL.txt', ...['agent-workflow', 'environment-access', 'private-runner'].map(name => `visuals/recollect-${name}-v3.png`)]) {
  await stat(path.join(output, required));
}
if (files.some(file => /(?:^|\/)(?:\.env|runtime|credentials|node_modules)(?:\/|$)/.test(path.relative(output, file)))) throw new Error('Unexpected private/runtime artifact');
console.log(`Site check passed: ${html.size} HTML routes, ${references} local references and anchors, ${files.length} static files.`);
