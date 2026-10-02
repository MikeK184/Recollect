# Publish committed repository evidence

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Purpose and Prerequisites

Publish exact locally available Git commits into an authorized Brain and select
immutable environment revisions. The service retains structural artifacts and
contributor history; raw file text needs explicit selection and admin policy.
This is local extraction and shared evidence, not verification of a remote origin
or deployment. See the [contract](../contracts/evidence-repository-publication.md).

Connect the [packaged plugin](../../plugins/recollect/README.md) with writer/admin
access and install Git. The package includes its native Enola extractor. Select
the installed runtime for the commands below:

```sh
PLUGIN_RUNTIME=/absolute/installed/plugin/bin/recollect-plugin
```

Source-checkout developers can still run `python3 scripts/setup-enola.py` and
`cargo build -p recollect-agent`, then use `target/debug/recollect-agent` with the
legacy paired profile. That setup installs Enola in `.cache/enola-tools/`.
`RECOLLECT_ENOLA_BIN` can select another executable; its declared build and
extractor labels are retained without a strict version handshake. Git and Enola
run locally. No OpenAI credential or model call is needed for static extraction.

## Procedure

Register checkout metadata through the [workspace workflow](workspace-scope.md),
then obtain the Brain and repository UUIDs from Workspace & tasks. An existing
registration may be used with an explicit Brain without placing a new selector
in a customer directory. The plugin uses its connected endpoint and OS credential.

Substitute the uppercase arguments with their actual UUIDs and checkout path:

```sh
"$PLUGIN_RUNTIME" scope start BRAIN "Committed publication" --repository REPOSITORY
"$PLUGIN_RUNTIME" repository publish BRAIN REPOSITORY TASK /path/to/checkout
```

The scope command returns `task.id`. Publication creates its capture operation
before reading source inputs. It resolves HEAD once, reads Git objects directly,
ignores archive substitutions/replacement refs, disables lazy fetching and records
the ordinary checkout's dirty/branch observation separately. It does not reset,
switch, fetch, modify the checkout or execute repository commands/hooks.

Use `--revision REF` to select another locally available commit. Optional source
retention is a separate deliberate choice:

```sh
"$PLUGIN_RUNTIME" repository publish BRAIN REPOSITORY TASK /path/to/checkout --revision REF --retain-file src/lib.rs
```

Repeat `--retain-file` for up to 20 permitted text files. First enable **Allow
explicitly selected repository file text** in **Settings → Retention & privacy**. The default is disabled. An absent/excluded selected file is an error;
turning the policy off later does not erase accepted text.

In the browser, open **Repositories → Published repositories → Snapshots**. Inspect
exact commit, files/availability, fact records, coverage, insights, receipt and
contributors. Reprocessing uses retained artifacts with current authority. The
native inspection commands are:

```sh
"$PLUGIN_RUNTIME" repository snapshots BRAIN REPOSITORY
"$PLUGIN_RUNTIME" repository snapshot BRAIN SNAPSHOT
"$PLUGIN_RUNTIME" repository manifests BRAIN
"$PLUGIN_RUNTIME" repository manifest BRAIN MANIFEST
```

Create an environment in **Sources → Manage views**, then use **Repositories → Environments → New manifest**. Select repositories and published snapshots, or explicitly enter an
exact revision without a snapshot. Config paths are relative references. Choose
committed code, desired configuration or a recorded deployment observation;
observations require a timestamp and supporting reference. Recollect does not
infer deployment from a checkout. Edit the current selection to append a new
immutable revision and inspect earlier selections in Revision history.

## Verification

A successful publication returns snapshot and contribution identities. **Ready**
means the worker materialized the retained static fact records. Snapshot/manifest
inspection after device revocation remains available to current Brain members.
Missing file text, missing artifacts, parse gaps, unsupported formats and
unresolved targets have separate states. A manifest without a snapshot never
selects the newest publication implicitly.

The [local proof](../mappings/repository-publication-proof-2026-09-14.md) covers
native Git/Enola isolation, shared authorization/replay, actual interrupted upload,
browser history/mobile and a bounded real SWEG publication. Focused commands:

```sh
cargo test -p recollect-agent --test publication
./scripts/test-platform.sh
./scripts/test-ui.sh tests/publication.spec.ts
./scripts/validate.sh
```

## Failure and Recovery

- Upload interrupted: the runtime prints a bundle UUID. Resume with
  `"$PLUGIN_RUNTIME" repository resume BRAIN BUNDLE_UUID` using the
  original endpoint and paired device. It uploads the sanitized prepared input,
  keeping its original operation/idempotency identity despite checkout changes.
- The plugin stages inside its private application-data directory (or explicit
  `RECOLLECT_PLUGIN_DATA`). The legacy companion defaults to `.data/publications/`;
  its `RECOLLECT_PUBLICATION_DIR` override must remain inside Recollect.
  Successful uploads remove their own bundle and
  normal preparation removes its exclusively created staging tree. Crash leftovers
  remain private. Do not bulk-delete unknown paths; reconcile an identified owned
  bundle/stage explicitly. The later erasure slice consumes this boundary too.
- Missing Git objects or extractor failure: restore the local dependency/input
  deliberately and start another preparation. There is no automatic fetch,
  working-copy fallback, provider invocation or invented empty graph.
- Conflicting artifacts for the same revision/settings: inspect the contributor
  and adapter evidence. Immutable history is not overwritten; an intentionally
  different setting is a distinct variant.
- Policy or access changed: resolve current Brain/device authority. An old task,
  operation or bundle is not a grant. Revoked-device queued work cannot publish.
- Artifact storage unavailable: restore the exact retained artifact and retry
  processing. File paths in records never authorize arbitrary filesystem reads.
- Manifest edit conflict: close the stale editor, inspect the current selection,
  and reopen it before saving. The prior revision remains immutable.
- Environment deletion conflict: manifest history prevents ordinary view removal;
  preserve it until the explicit erasure lifecycle handles dependent evidence.
