# Public Product and Documentation Site

Status: accepted

## Source

The user's 2026-10-06 request for GitHub Pages displaying Recollect's product and
documentation, the [repository governance contract](repository-governance.md),
and the accepted [product vision](../foundation/vision.md).

## Contract

- Publish a static product landing page and searchable operator documentation
  on the existing public repository's GitHub Pages site. This request lifts the
  bootstrap's publishing/CI deferral for this bounded website only.
- Preserve Recollect's original logo, cream/ink/sage visual system and approved
  workflow illustrations. Present shared engineering memory, independently
  governed MCP tools and private execution as one self-hosted workflow.
- The website is documentation, not an application installation. It contains no
  account login, Brain records, private runtime data, model calls or credentials.
- Canonical operator Markdown remains in `docs/runbooks/` and the plugin README.
  An explicit allowlist generates the website's documentation from those sources;
  links to unpublished repository records go to their public GitHub sources.
- A separately locked VitePress build in `site/` produces only static artifacts.
  Generated content, dependencies, caches and build outputs remain ignored.
- GitHub Actions builds/checks the site and deploys the artifact from `main`.
  Pull-request builds may validate but cannot deploy. Deployment permissions are
  limited to the deployment job, with no additional repository secrets.
- Support the `/Recollect/` project base path, deep links, syntax highlighting,
  local documentation search, accessible navigation and narrow viewports.
- Existing local acceptance/production limits remain explicit. Product
  illustrations are examples, not current installation screenshots.
- A future purchased custom domain is optional. No domain purchase, DNS change,
  runtime deployment or application release is included.

## Acceptance

Build from locked dependencies; verify generated routes, internal links and
assets; inspect desktop/mobile product and documentation views; exercise local
search and code copying; run `./scripts/validate.sh`. A successful GitHub Pages
deployment and HTTPS requests to the product and docs URLs establish publication.
Local builds alone do not establish a live site.

## Explicit Deferrals

Custom domains, analytics, hosted accounts, application runtime hosting and
product performance claims remain separate.
