import { defineConfig } from 'vitepress';
import { groups } from '../catalog.mjs';

const base = process.env.SITE_BASE || '/Recollect/';
if (!/^\/[a-zA-Z0-9/_-]*\/$/.test(base) && base !== '/') throw new Error('SITE_BASE must be an absolute directory path');
const github = 'https://github.com/MikeK184/Recollect';

export default defineConfig({
  title: 'Recollect',
  description: 'Shared engineering memory and governed MCP tools for coding agents. Self-hosted, with private execution inside your network.',
  lang: 'en',
  base,
  srcDir: 'content',
  vite: { publicDir: '../public' },
  appearance: false,
  cleanUrls: false,
  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: `${base}brand/recollect-symbol.svg` }],
    ['meta', { name: 'theme-color', content: '#f7f4ec' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:image', content: `https://mikek184.github.io${base}visuals/recollect-agent-workflow-v3.png` }],
  ],
  sitemap: { hostname: `https://mikek184.github.io${base}` },
  markdown: { theme: 'github-light', lineNumbers: false },
  themeConfig: {
    logo: { src: '/brand/recollect-logo.svg', alt: 'Recollect' },
    siteTitle: false,
    nav: [
      { text: 'Product', link: '/' },
      { text: 'Documentation', link: '/docs/', activeMatch: '/docs/' },
      { text: 'Get started', link: '/docs/installation' },
    ],
    socialLinks: [{ icon: 'github', link: github }],
    search: { provider: 'local' },
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
