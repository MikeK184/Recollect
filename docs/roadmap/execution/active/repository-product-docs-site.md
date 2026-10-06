# Recollect Product and Documentation Website

Status: in-progress
Owning epic: `docs/roadmap/epics/repository-governance.md`
Work type: governance

## Summary

- Goal: A branded public product page and searchable documentation on GitHub Pages.
- Non-goals: Host the Recollect application, buy a domain or change runtime data.
- Delivery shape: Static website, locked build and Pages workflow with live proof.

## Governing Sources

- [Public site contract](../../../contracts/public-product-and-docs-site.md)
- [Repository governance](../../../contracts/repository-governance.md)
- [Product vision](../../../foundation/vision.md)
- [Owning epic](../../epics/repository-governance.md)

## Scope

- In scope: VitePress landing page, allowlisted canonical guides, search,
  brand assets, responsive styles, build/link checks, Pages configuration and deployment.
- Out of scope: Product runtime, private data, analytics and domain/DNS changes.
- Blockers: None; current repository is public with administrator access and Actions enabled.

## Surface and Interface Changes

- Interfaces: `npm ci`, `npm run build`, `npm run check`, `npm run preview` in `site/`;
  product root and `/docs/` under the GitHub Pages project path.
- Storage: N/A; no application persistence. Generated static files are ignored.
- Ownership: Repository governance owns public website code and publication;
  existing canonical guide owners retain operator-content authority.

## Data and Authority

- Inputs: Explicit canonical Markdown/brand/image allowlists and product copy.
- Authority: Existing accepted product contracts and user-selected presentation.
- Blind spots: A local build does not prove publication or current product runtime health.

## States and Edge Cases

- Loading: Static content renders before client navigation/search hydration.
- Empty: Search reports no matches; documentation starts with a guided index.
- Error: Build rejects missing sources/broken internal routes; unknown routes use 404.
- Blocked: Failed build prevents artifact deployment; Pages errors remain visible.
- No-access: Public reading; repository settings/deployment use existing GitHub authority.
- Duplicate or replay: Deterministic generation replaces only owned ignored directories;
  deployment concurrency preserves the running deployment.
- Stale data: Main-branch guide changes regenerate the site; dated evidence stays dated.
- Reconciliation divergence: Unpublished source links point to GitHub, not missing site pages.

## Integrations and Runtime Inputs

- Providers: Stable VitePress 1.6.4 and official GitHub Pages Actions.
- Environment: `SITE_BASE` for project/custom-domain path; default `/Recollect/`.
- Secrets: No custom secrets; scoped GitHub Actions token/OIDC deployment only.
- Failure handling: Build failures stop publication; operator may rerun the workflow.

## Tests and Acceptance

- Automated: Locked build, generated route/asset/link check, repository validation.
- Manual: Desktop/mobile browser, docs navigation/search/code-copy, live HTTPS checks.
- Acceptance: Branded product and searchable docs published at the Pages URL.

## Closeout

- Planned: Product site, canonical searchable guides, Actions publication and proof.
- Shipped: Local branded product page, 27 canonical guides, search/navigation,
  original assets and locked static build; publication acceptance remains in progress.
- Not shipped: Custom domain, analytics, application hosting and runtime changes.
- New blockers: None at preparation.
- Docs updated: README, documentation entry point, site maintenance guide,
  contract/index, owning epic/index and active execution index.
- Validation: Locked dependency reinstall/build; 30 HTML routes and 1,484 local
  references/anchors; desktop/mobile browser layout, private-runner search/deep
  link, canonical edit links, code/setup copying; governance lint and 32 tests.
- Version: N/A; public documentation website, no application release.
- Commit: Uncommitted during implementation; publication requires the website commit.
