# Recollect public website

The product landing page and searchable operator guides are built with stable
VitePress. Original logo, fonts and illustrations come from the existing product.

```sh
cd site
npm ci
npm run build
npm run check
npm run preview
```

Open the printed preview URL with `/Recollect/` appended. `npm run dev` rebuilds
the source content before starting a development server.

`catalog.mjs` is the public guide allowlist. `scripts/prepare.mjs` copies canonical
Markdown into ignored `content/`, preserving code fences and converting links:
published guides link to each other; other tracked records link to GitHub;
unpublished local artifacts are named as local files. Edit the original guides,
not generated content. Product-only copy lives in `pages/` and the theme component.
Changes to canonical guides require regeneration/restart in development.

The workflow builds and checks pull requests; only `main` can deploy the static
artifact. Pages must use **GitHub Actions** as its publishing source. There are
no custom credentials or application services in the deployment. Font licenses
are included in the artifact. Use Node.js 24 or newer for the website toolchain.

Default URLs:

- Product: https://mikek184.github.io/Recollect/
- Documentation: https://mikek184.github.io/Recollect/docs/

For a later custom domain, configure GitHub Pages and DNS, set `SITE_BASE=/` for
the build, and update the canonical/sitemap host in `.vitepress/config.mts`.
No custom domain is configured by this change.
