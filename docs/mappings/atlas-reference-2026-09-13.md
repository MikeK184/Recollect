# Agent Memory Atlas Reference Checkout

Observed: 2026-09-13
Confidence: verified

## Sources and Method

The user requested a local reference clone for learning from the pattern
library, Evidence Before Belief, and the comparative report.

- Origin: <https://github.com/neoneye/agent-memory-atlas.git>
- Local directory: `agent-memory-atlas/`, a separate Git repository ignored by
  Recollect's root `.gitignore`.
- Verified HEAD: `7eca7f7abd934c2e44bc096fc1dd0cbf94275b99`.
- Full clone: `git rev-parse --is-shallow-repository` returned `false`.
- Verified the Markdown and generated HTML files exist and are nonempty;
  compared each HTML file byte-for-byte with its published page.
- Inspected `scripts/build_site.sh`: the comparison renders from
  `content/overview.md`, while patterns render from `content/patterns/`.

## Observations

All three requested pages are included, with generated HTML identical to the
published response at verification time:

| Published page | Markdown source inside the clone | Generated HTML inside the clone |
| --- | --- | --- |
| [Pattern library](https://neoneye.github.io/agent-memory-atlas/patterns/) | `content/patterns/index.md` | `docs/patterns/index.html` |
| [Evidence Before Belief](https://neoneye.github.io/agent-memory-atlas/patterns/evidence-before-belief/) | `content/patterns/evidence-before-belief.md` | `docs/patterns/evidence-before-belief/index.html` |
| [Comparative report](https://neoneye.github.io/agent-memory-atlas/compare/) | `content/overview.md` | `docs/compare/index.html` |

HTML SHA256 values, in table order:

```text
defed3ee5db376603ff3b6daa6f5b8f35f0c204acaaf58eb4f7f34ec3307f09f
0f53b1064b11944cfde4657e9a460ddc3ab3a64463de9933b22442f2dad4fbcd
b9ae4b94e734b480addfb6e98c8dee28af667e87b1bd44a2ec64083885bc3103
```

The Atlas checkout is clean. Recollect ignores it as an independent reference
repository, matching the separation used for the existing Cognee checkout.

## Translation and Limits

Use the Markdown sources as research input for Recollect's patterns, design
reports, and acceptance criteria. The comparative source is about 2 MB, so
locate relevant headings and named systems before reading whole sections.
Keep the Atlas revision and each reviewed system's upstream revision attached
to any conclusions. Recheck implementation claims against current source.

Atlas reports and agent workflows are external reference material. Recollect's
accepted ADRs, contracts, and user decisions remain its implementation
authority. This clone does not change the selected Cognee foundation or import
the Atlas's skills/configuration. Its generated HTML duplicates the Markdown
for research ingestion purposes. No build scripts or application services
were executed as part of this reference checkout.

## Follow-up

Read scoped patterns and comparison sections as needed when preparing future
Recollect decisions. Inspect local changes before updating the clone; a later
revision requires refreshing any evidence that depends on changed reports.
