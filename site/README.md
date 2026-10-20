# Recollect public website

The product page and searchable operator guides at **https://recollecthub.com**
are built with stable VitePress. The site follows the version 2 Light and Ink
design system ([ADR 0025](../docs/adr/0025-two-theme-desktop-redesign.md) and its
2026-10-20 amendment); the [site contract](../docs/contracts/public-product-and-docs-site.md)
governs what it publishes.

```sh
cd site
npm ci
npm run build
npm run check
npm run preview
```

Open the printed preview URL. `npm run dev` rebuilds the source content before
starting a development server. The preview server reads the build output once at
startup, so restart it after rebuilding.

## How it is assembled

- `catalog.mjs` is the public guide allowlist. `scripts/prepare.mjs` copies the
  canonical Markdown into ignored `content/`, preserving code fences and converting
  links: published guides link to each other, other tracked records link to
  GitHub and unpublished local artifacts are named as local files. Edit the
  original guides, not generated content; restart development after changing them.
- `prepare.mjs` also copies the brand marks, Mona Sans, JetBrains Mono and
  Newsreader with their OFL notices from `web/public/`, and the Open Graph image
  from `docs/assets/recollect-og.png`.
- The theme imports `web/src/design/tokens.css`, so colours change only there.
  `.vitepress/theme/style.css` maps VitePress onto those roles and styles the
  guides; `home.css` styles the product page and its diagrams. A head script and
  `theme/index.ts` keep `data-theme` in step with the Light and Ink switch.
- Product copy lives in `pages/` and `theme/ProductHome.vue`. The animated
  diagrams are framework-free (`theme/diagrams/`): `stories.ts` holds each
  diagram's markup and illustrated steps, and `player.ts` plays them only while
  visible, with a pause control and a reduced-motion end state.

## Publication

The workflow builds and checks pull requests; only `main` deploys the static
artifact, and Pages must use **GitHub Actions** as its publishing source. The
custom domain `recollecthub.com` is configured in the repository's Pages
settings, so the build targets the domain root (`SITE_BASE` defaults to `/`;
set it, for example to `/Recollect/`, only to preview under a project path).
There are no custom credentials or application services in the deployment.
Font licenses are included in the artifact. Use Node.js 24 or newer.

The README's animated SVGs are separate from this build. Regenerate them after
changing tokens or their copy with `node scripts/render-readme-art.mjs` from the
repository root. To refresh the Open Graph image, render the product hero at
1200 × 630 from a running preview and save it as `docs/assets/recollect-og.png`.
