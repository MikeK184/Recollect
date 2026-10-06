# Published Recollect Product and Documentation Website

Observed: 2026-10-06
Confidence: verified

## Sources and Method

- Accepted [public-site contract](../contracts/public-product-and-docs-site.md)
  and [archived execution pack](../roadmap/execution/archive/repository-product-docs-site.md).
- Stable VitePress 1.6.4, pinned in `site/package.json` and its lockfile;
  [deployment](https://vitepress.dev/guide/deploy),
  [theme extension](https://vitepress.dev/guide/extending-default-theme),
  [local search](https://vitepress.dev/reference/default-theme-search) and
  [site configuration](https://vitepress.dev/reference/site-config) documentation.
  Context7 was not available in the current tool inventory; primary official
  documentation was used directly. No successful Context7 lookup is claimed.
- Official GitHub [Pages workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)
  and [Pages API](https://docs.github.com/en/rest/pages/pages?apiVersion=2022-11-28).
  Action tags were resolved through the GitHub API and pinned to exact commits.
- Website revision `696361919660688ebc850d86ea623421032230ca` on `origin/main`.
  [Initial workflow run](https://github.com/MikeK184/Recollect/actions/runs/37532940591)
  completed successfully: build 32 seconds; deploy 9 seconds.
- Local `npm ci --ignore-scripts --no-audit --no-fund`, `npm run build`,
  `npm run check` in `site/`; `./scripts/validate.sh`; `git diff --check`.
- Browser acceptance on the local preview and the public HTTPS site, using
  desktop 1440×1000, mobile 390×844 and default 824×954 viewports.
  Public routes/assets also checked independently with Python `urllib.request`.

## Observations

- [Product page](https://mikek184.github.io/Recollect/) and
  [docs index](https://mikek184.github.io/Recollect/docs/) return HTTP 200.
  Installation and private-runner guide deep links also return 200.
  The original SVG logo, Manrope font and agent/private-runner v3 illustrations
  return 200. An initial check used a nonexistent `recollect-logo-light.svg`
  name; the actual page references `brand/recollect-logo.svg`, verified successfully.
- The static build contains 27 canonical guides plus product/index/404 pages.
  Artifact validation passes 30 HTML routes, 1,484 local links/anchors and 110
  files. All guide edit links point to their canonical repository sources.
- Product and documentation retain the original light cream/ink/sage identity,
  local fonts and approved workflow illustrations. Knowledge grants, tool grants,
  environment scope, central execution and private dispatch remain distinct.
- Desktop/mobile pages show no horizontal overflow. Local acceptance proved
  documentation search, navigation, code copying, setup-command copying and
  the mobile sidebar. Public search for `private runner` returned 16 matches;
  selecting the first opened the correct `#register-a-private-runner` section.
  Public browser warning/error logs were empty.
- Governance validation passed all 32 checker tests. Actions repeated the locked
  build, artifact checks and repository validation before deployment.
  Actions reported informational runner-image and nested upload-action Node
  migration warnings; both jobs succeeded.
- Pages uses workflow publication and enforces HTTPS. The GitHub About homepage
  now points to the public product URL. Repository visibility remains public and
  its default branch remains `main`.
- Canonical content is allowlisted and regenerated from existing Markdown.
  Other tracked documentation links target GitHub; local-only artifact paths are
  presented as local files. Original font licenses are included in the artifact.
  No custom deployment credentials, external font requests or analytics were added.

## Translation and Limits

The product and documentation website is published and live-proven. It is a
static public presentation and does not host the Recollect application or prove
current agent/runner/model connectivity. Runtime records remain dated evidence.
No custom domain or DNS configuration was made. The workflow builds relevant
pull requests and deploys matching `main` changes; branch protection is not
established by this delivery.

Local screenshots are ignored review artifacts in
`output/public-site-2026-10-06/` (`product-live.png`, `product-live-hero.png`,
`docs-live.png`), not public site inputs or tracked documentation assets.

## Follow-up

Edit canonical guides to maintain operator content; update the explicit catalogue
when adding a public guide. For a chosen custom domain, follow the
[website maintenance guide](../../site/README.md) and GitHub Pages DNS guidance.
Recheck Actions and public routes after deployment changes. No further website
acceptance work remains for this assignment.
