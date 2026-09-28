# Import and organize source evidence

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Purpose and Prerequisites

Use the locally delivered Brain evidence catalogue with an authenticated account.
Readers may inspect evidence; writers/admins import and organize; admins control
document retention. Start Recollect using [local development](local-development.md).
The API and worker must use the same artifact directory and database.

## Procedure

1. Open or create a Brain, then select **Sources** in its sidebar.
2. Use **Manage views** to create collections, areas and environments. Filters
   intersect; a source can belong to several views without being copied.
3. Choose **Add source**. Supply a title and UTF-8 text, Markdown, JSON, YAML
   or TOML, either as a file or pasted text. The content limit is 1 MiB. Select
   its views and explicitly choose whether to retain content. An observation
   time is optional and is separate from the recorded capture time.
4. For reference-only evidence, supply a source URI and leave content retention
   off. Recollect does not fetch or verify that external URI.
5. Open a source to inspect content, contributor, capture/observation times and
   processing. **Edit source** appends a version. **Version history → Evidence version** shows
   earlier content; **Organize source** changes the shared source's views.
6. **More source actions → Reprocess source** rebuilds support spans from the retained current version.
   Removing a view or an association preserves the source and its history.

In **Settings → Retention & privacy**, admins can turn off **Allow document content retention** to restrict new imports
to references. This preserves already retained evidence; it is not an erasure
operation. Do not import credentials or private keys.

Artifacts are UUID files under `RECOLLECT_ARTIFACT_DIR`, default `.data/artifacts`.
Keep this directory alongside the database when preserving the installation.
It is not a public static directory. Use an absolute path if API and worker may
start from different working directories. Do not edit accepted artifact files.

## Verification

Open an imported source and compare its visible text and provenance. Processing
should move from queued to processed when a capture worker completes. Source
content remains available while processing is queued. A reference-only badge
does not mean external content was retrieved.

Run the focused proof from the repository root:

```sh
./scripts/test-platform.sh
./scripts/test-ui.sh tests/evidence.spec.ts
```

For one manual capture pass with the environment loaded:

```sh
cargo run -p recollect-server -- worker-once capture
```

The [proof mapping](../mappings/evidence-collections-proof-2026-09-14.md) records
authorization, immutable history, real artifacts and missing-source recovery.

## Failure and Recovery

- Import error: correct validation or policy errors and retry. Storage failure
  commits no source metadata, audit or job. Verify the configured artifact volume
  is writable without changing unrelated paths or permissions.
- Stale editor: reload the current version before applying the edit again.
- Missing/unreadable content: verify the configured artifact path and restore
  only the exact registered artifact from a known source, then reprocess. If
  original bytes are unavailable, import a new attributable version.
- Delayed/failed processing: inspect the Brain's Processing panel and
  `.cache/runtime/worker.log`. Restart the normal worker after fixing storage.
  Current role/device revocation intentionally prevents queued publication.
- Lost access: content is hidden after the server rejects current authorization.
  Restoring access requires a Brain admin; re-importing cannot bypass it.

File/database commits are not atomic. Preserve unexplained orphan files for
controlled recovery; do not bulk-delete them or treat them as accepted sources.
Full erasure and backup/restore drills have separate roadmap slices.
