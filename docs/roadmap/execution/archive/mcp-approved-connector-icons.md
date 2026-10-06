# Approved connector icons

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Display real cached optional MCP server icons with generic fallback.
- Non-goals: Arbitrary favicon requests, private-target fetches, SVG/HTML rendering.
- Delivery shape: Explicit inspection cache in existing approved definition JSON.

## Governing Sources

- [Catalogue](../../../contracts/mcp-catalogue-and-profiles.md)
- [Runtime](../../../contracts/mcp-runtime-and-credentials.md)
- [Owner](../../epics/mcp-coordination.md)

## Scope

- In scope: Up to three initialized server icon candidates; safe public same-origin
  HTTPS or inline raster validation, normalized PNG summary used in Connections/cards.
- Out of scope: Tool-specific icons and network fetch during reads/viewing.
- Blockers: None; accepted explicit discovery boundary establishes limits.

## Surface and Interface Changes

- Interfaces: Optional normalized PNG data URI in definition manifest/summary.
- Storage: Existing JSON manifest; serde default preserves existing definitions.
- Ownership: Runtime yields optional metadata; server validates/fetches/normalizes;
  browser renders only approved cached bytes and uses generic fallback on error.

## Data and Authority

- Inputs: Untrusted initialized metadata and explicitly inspected MCP target.
- Authority: Owner explicit inspection/approval; icons never affect grants/readiness.
- Blind spots: Many servers omit icons or publish rejected formats/locations.

## States and Edge Cases

- Loading: Generic stable icon until available.
- Empty: Generic icon when no metadata.
- Error: Any icon failure falls back without failing tools/connection metadata.
- Blocked: Non-public, non-same-origin, redirect, SVG/HTML and oversize candidates rejected.
- No-access: Existing owner/Brain reads remain authoritative.
- Duplicate or replay: Explicit inspections bounded; manifest approval preserves existing idempotency.
- Stale data: Approved cached icon changes only through explicit operator definition update.
- Reconciliation divergence: An icon establishes identity decoration only, never connection success.

## Integrations and Runtime Inputs

- Providers: rmcp initialize server_info; separate unauthenticated no-proxy Reqwest client.
- Environment: N/A; no new variables.
- Secrets: No auth headers/cookies/userinfo/query forwarded; no remote icon URL retained in summaries.
- Failure handling: All DNS answers public and connection pinned; no redirects/retries;
  six-second total budget, ≤3 candidates, ≤256KiB and512×512 input, 4MiB decode,
  PNG/JPEG/WebP only, stripped/reencoded PNG.

## Tests and Acceptance

- Automated: Private/reserved/mapped IPv6, query/userinfo/cross-origin, redirect/oversize,
  malformed/SVG and valid PNG decoding; compatibility/build and `./scripts/validate.sh`.
- Manual: Inspect generic and cached icon in actual Connections/card screenshot.
- Acceptance: Bounded raster appears locally with no runtime remote request on view;
  missing/failed icons leave successful metadata inspection usable.

## Closeout

- Planned: Safe optional connector icon caching and generic fallback.
- Shipped: SDK initialized icon metadata, normalized local PNG manifest/summary,
  credential-free public same-origin DNS-pinned bounded fetch, and generic
  fallback across Connections/cards/global library. Viewing reads cached bytes.
- Not shipped: No live external MCP icon publication/fetch claim; tool icons and
  arbitrary favicon requests remain out of scope.
- New blockers: None.
- Docs updated: Catalogue/runtime amendments, pack, owning epic/index and evidence.
- Validation: Four icon rejection/normalization units and ten server library
  units pass; runtime description and isolated MCP fixtures pass. An owned
  normalized PNG passes canonical approval and loads in list and inspector;
  absent icon keeps the generic fallback. Build/type/design and Clippy pass.
- Evidence: [Coordinated delivery and limits](../../../mappings/desktop-browser-management-2026-10-05.md).
- Version: N/A; no release requested.
- Commit: Uncommitted.
