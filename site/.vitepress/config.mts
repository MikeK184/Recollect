import { defineConfig } from 'vitepress';
import { groups } from '../catalog.mjs';

// The site is served from the root of its custom domain. SITE_BASE remains for
// previews under a project path, such as a fork's GitHub Pages URL.
const base = process.env.SITE_BASE || '/';
if (!/^\/[a-zA-Z0-9/_-]*\/$/.test(base) && base !== '/') throw new Error('SITE_BASE must be an absolute directory path');
const origin = 'https://recollecthub.com';
const github = 'https://github.com/MikeK184/Recollect';

// Mirrors VitePress's own appearance preference into data-theme before first
// paint, so the shared Light and Ink tokens (web/src/design/tokens.css) apply.
const themeInit = `;(() => {
  let preference = null;
  try { preference = localStorage.getItem('vitepress-theme-appearance'); } catch {}
  const dark = preference === 'dark' || (preference !== 'light' && matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.setAttribute('data-theme', dark ? 'dark' : 'light');
})()`;

export default defineConfig({
  title: 'Recollect',
  description: 'Shared memory, skills and governed MCP tools for coding agents. Self-hosted, with private execution inside your network.',
  lang: 'en',
  base,
  srcDir: 'content',
  // The theme imports the product's token stylesheet from web/src/design.
  vite: { publicDir: '../public', server: { fs: { allow: ['..'] } } },
  appearance: true,
  cleanUrls: false,
  head: [
    ['script', { id: 'recollect-theme' }, themeInit],
    ['link', { rel: 'icon', type: 'image/svg+xml', href: `${base}brand/recollect-symbol.svg` }],
    ['meta', { name: 'theme-color', content: '#f4f6f8', media: '(prefers-color-scheme: light)' }],
    ['meta', { name: 'theme-color', content: '#0b1018', media: '(prefers-color-scheme: dark)' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:site_name', content: 'Recollect' }],
    ['meta', { property: 'og:image', content: `${origin}${base}visuals/recollect-og.png` }],
    ['meta', { property: 'og:image:width', content: '1200' }],
    ['meta', { property: 'og:image:height', content: '630' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
  ],
  sitemap: { hostname: `${origin}${base}` },
  markdown: { theme: { light: 'github-light', dark: 'github-dark' }, lineNumbers: false },
  themeConfig: {
    logo: { src: '/brand/recollect-symbol.svg', alt: '' },
    siteTitle: 'Recollect',
    nav: [
      { text: 'Product', link: '/' },
      { text: 'Documentation', link: '/docs/', activeMatch: '/docs/' },
      { text: 'Get started', link: '/docs/installation' },
    ],
    socialLinks: [{ icon: 'github', link: github }],
    search: { provider: 'local' },
    darkModeSwitchLabel: 'Theme',
    lightModeSwitchTitle: 'Switch to Light',
    darkModeSwitchTitle: 'Switch to Ink',
    sidebar: { '/docs/': [
      { text: 'Overview', items: [{ text: 'Documentation', link: '/docs/' }] },
      ...groups.map(group => ({ text: group.text, collapsed: false, items: group.items.map(([slug, title]) => ({ text: title, link: `/docs/${slug}` })) })),
    ] },
    outline: { level: [2, 3], label: 'On this page' },
    editLink: {
      pattern: page => {
        return 'https://github.com/MikeK184/Recollect/edit/main/' + (page.frontmatter.source || 'site/pages/' + page.relativePath);
      },
      text: 'Edit this guide on GitHub',
    },
    footer: {
      message: 'Open source under the Apache-2.0 license.',
      copyright: 'Recollect · A place for what you know.',
    },
  },
});
