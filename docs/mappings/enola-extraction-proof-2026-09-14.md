# Enola committed-input adapter proof

Observed: 2026-09-14
Confidence: verified

## Sources and Method

Read Enola's official v0.4.19 [artifact contract](https://github.com/enola-labs/enola/blob/v0.4.19/docs/schema/README.md),
[facts](https://github.com/enola-labs/enola/blob/v0.4.19/docs/schema/facts.md),
[receipt](https://github.com/enola-labs/enola/blob/v0.4.19/docs/schema/receipt.md),
[insights](https://github.com/enola-labs/enola/blob/v0.4.19/docs/schema/insights.md),
[HCL limits](https://github.com/enola-labs/enola/blob/v0.4.19/docs/extraction/hcl.md)
and [CLI](https://github.com/enola-labs/enola/blob/v0.4.19/docs/CLI.md).
The GitHub release API reported v0.4.19, published 2026-09-12. A cached web
`releases/latest` result incorrectly pointed to v0.4.10; the live API and actual
binary established the version used. Several GitHub/raw web opens missed cache;
direct primary HTTPS reads succeeded. Guessed CONFIG/CONFIGURATION documents
returned 404; CLI and ARCHITECTURE.md supplied the configuration interface.

Context7's Enola resolution returned unrelated libraries, so no unrelated result
was used. Context7 `/git/htmldocs` supplied revision, tree-listing and archive
attribute guidance. The installed Git is 2.50.1. Exact input used
`rev-parse`, `ls-tree -rlz --full-tree` and `cat-file blob`; ordinary archives can
omit or substitute content through `export-ignore`/`export-subst` attributes.
The official [Git command reference](https://git-scm.com/docs/git) also verifies
`GIT_NO_LAZY_FETCH=1` and `GIT_NO_REPLACE_OBJECTS=1`: preparation must neither
fetch missing promisor objects nor substitute locally configured replacement refs.
See [Git tree listing](https://git-scm.com/docs/git-ls-tree),
[object reading](https://git-scm.com/docs/git-cat-file) and
[archive attributes](https://git-scm.com/docs/git-archive).

Downloaded the official Darwin ARM64 release into Recollect's ignored cache.
The archive member is `enola-0.4.19-darwin-arm64`, not `enola`; the initial member
assumption failed before execution. Installed that regular member plus LICENSE
and NOTICE. The executable reports `version:0.4.19`, `extractor_version:v265`.
No custom checksum, strict version gate, personal installation or agent hooks
were introduced.

## Observations

Built a temporary committed fixture inside Recollect with Rust functions,
TypeScript imports/Express route, Terraform resources/local and remote modules,
a Kubernetes Deployment YAML, an unknown format and `.gitattributes` marking a
Rust file export-ignore. Exported exact Git blobs into an isolated tree, then
changed the original fixture's Rust working copy to a dirty-only symbol.

Ran the actual binary with an explicit external configuration, empty external
providers/renderers, disabled incremental/history persistence, no update checks,
a repository-owned temp directory and a minimal child environment. The child
did not receive HOME or provider credentials. Git ceiling and configuration
settings prevented the extracted tree from inheriting Recollect's parent Git
metadata or personal Git configuration. The native environment was not modified.

| Behavior | Result |
| --- | --- |
| Output path | An absolute output directory was rejected before extraction. A relative `.enola` inside the isolated tree succeeded. |
| Native run | Exit zero in about 67 ms extraction time; 23 facts and one insight. Consumer artifacts facts.jsonl, insights.json and receipt.json were created. Internal snapshot.meta.json is not a consumer dependency. |
| Rust | Committed `greet`/`welcome` and their call relation were present. The dirty-only marker was absent, and export-ignore did not omit the committed input. |
| TypeScript | Import/dependency facts and the GET `/hello` Express route were present. External/unresolved targets were preserved. |
| Terraform | Resource/variable/output symbols, resource references and the local-module directory dependency were present. The remote module was an external literal, without a fabricated fetched module graph. |
| GitOps and unknown formats | Deployment YAML produced zero facts. Receipt census explicitly listed `.yaml` and `.unknown` as excluded kinds. No Kubernetes/Helm or deployed-state inference is supported by this fixture. |
| File accounting | Eleven files walked, six producing facts, zero parse errors, four excluded by kind and one claimed by the manifest extractor without facts. |
| Global state | Enola logged that its global receipt was skipped because the child had no HOME. Generation still succeeded. This avoids its normal writes to personal `~/.enola/receipt.json` and graph registry. |

Source inspection confirmed that the global receipt failure is nonfatal. The
isolated tree had no Git metadata, and its receipt had no `git` field. Recollect
must carry independently resolved repository UUID/origin/commit provenance and a
stable isolated directory label; Enola's short label and remote normalization
cannot supply Recollect identity.

## Translation and Limits

This is an adapter experiment, not delivery of repository publication. Fixtures
and logs are under ignored
`.cache/repository-extraction-proof-a7d670aa73444d2e894942d023997c14/`.
Official reference downloads are in `.cache/enola-reference/v0.4.19/`; the tested
binary is `.cache/enola-tools/v0.4.19/enola`. No SWEG source contents were used.

The product contract must retain duplicate fact records, absent target IDs,
quality gaps and unsupported formats. Current upstream IDs may be retained as
opaque adapter identities; Recollect does not compute or require their hashes.
Future fields/format labels are recorded or ignored by shape rather than becoming
strict version negotiation. Publication, authorization, bounded secret exclusion,
recovery, manifests and browser workflows still require implementation and proof.
