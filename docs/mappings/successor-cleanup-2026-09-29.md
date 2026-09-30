# Successor cleanup verification, 2026-09-29

Observed: 2026-09-29
Confidence: observed-once

## Sources and Method

- Pack `docs/roadmap/execution/active/mcp-successor-cleanup.md` under epic
  `docs/roadmap/epics/mcp-coordination.md`; implemented in three parallel
  tracks (server dedupe, review-UI removal, Ask/Search simplification).
- Rebuilt API image with `./scripts/stack.sh up --build`; api/worker healthy.
- Live dev stack at `http://127.0.0.1:8787`; owner login via `.env`
  in-process; proof scripts print statuses only, no token values.
- `cargo clippy --locked -p recollect-server --all-targets` clean;
  `cargo test --locked -p recollect-server --test operations_offline` 3 passed;
  live ignored pairing test passed; web `npm run typecheck` passed
  (design checks plus `tsc --noEmit`, exit 0); `./scripts/validate.sh` 32/32.

## Observations

- Device dedupe: two pairings with the same normalized name (second with
  changed casing and surrounding whitespace) returned the same device id
  (`reused=True`); device list showed exactly 1 live row for the name;
  both records revoked afterwards (204), leaving no residue.
- Review-UI removal: served web chunks contain the read-only `Review history`
  title and the autonomous-learning notice; `Confirm review`,
  `Confirm conflict resolution`, `Revise proposal`, and `Save proposal` entry
  strings are absent from rendered UI (kept only inside the unrendered
  `ClaimEditor` export). Server review APIs are unchanged.
- Ask/Search simplification: served `AskPage` chunk contains the
  `Search and Ask your Brain` title and the `no_evidence` next-steps copy
  (add sources, publish snapshot, run learning, search again); served
  `RecallPanel` chunk contains the revised `no_match` and insufficient
  copy. Retrieval semantics and APIs are unchanged.

## Translation and Limits

- Dedupe reuses the most recent unrevoked unexpired row per account and
  normalized name; legacy duplicates remain until individually revoked.
- Historical revoked test rows are unchanged; only new duplicate issuance stops.
- UI proof is bundle-string plus typecheck evidence, not a browser click run.

## Follow-up

- Revoke the SWEG hardcoded test tokens when testing ends
  (`SWEG opencode hardcoded test token`, Codex literal-token device).
- Codex model turn and Claude live run remain deferred host-side items.
