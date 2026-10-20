# Final desktop deployment and host acceptance

Observed: 2026-10-01
Confidence: observed-once
Status: dated baseline, reduced on 2026-10-19 to its results and limits; later
checkpoints control current runtime claims. The full text is at the pinned
revision
[a0aea9d](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/mappings/desktop-final-acceptance-2026-10-01.md).

## Sources and method

The user resumed Ask-primary, plugin/OpenCode work and the six older desktop
acceptance packs and authorized deployment to the existing local installation. The
record follows the [four-slice continuation](https://github.com/MikeK184/Recollect/blob/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs/mappings/desktop-continuation-2026-10-01.md)
and preserves the [September 26 implementation evidence](https://github.com/MikeK184/Recollect/blob/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs/mappings/desktop-experience-implementation-2026-09-26.md)
as dated history. Logs and screenshots were in the ignored, owner-only
`.cache/final-closeout-20261001/` directory; no credentials belong here.

## Deployment and preserved state

The root Compose API and worker (image `64d06daac505`) replaced the prior native
API and older worker; migration and readiness passed and storage, environment and
credentials were preserved. Pre/post inventory is identical: 16 Brains, 26
sources, 27 source versions, 29 claims and 37 claim revisions, plus the recorded
grant, model-policy, retention and capture digests. A private pre-upgrade
database backup was retained and must not be restored over later erasures
without the canonical deletion journal. Disposable answer-proof Brains were
deleted through the product; existing Brains were not used for mutations.

## Desktop acceptance

- The continuation matrix covers all 37 then-current Playwright spec files: 65
  passes and seven explicit opt-in skips (skips are not relabeled as passes).
- Two separately enabled real-provider browser cases pass (model settings,
  transmission and budget; autonomous learning of port 8080, revision to 9090 and
  refreshed handover). A deployed actual-provider question returned a qualified
  answer with two exact citations; an authorized source update and UI correction
  changed the claim to 9090 with new exact support, and Search returned the
  corrected assertion. The final successful journey made one answer request;
  earlier bounded attempts also made requests.
- The journey exposed a real lineage bug (the correction dialog lacked its scope
  catalogue, leaving Save disabled), fixed and covered by a browser regression.
- Five Ask regressions pass; a combined run of ten cases then hit the normal
  ten-logins-per-minute limit on its eleventh (the focused rerun passed alone).
- Activity acceptance covers owner diagnostics versus member denial, private
  output canaries, role-loss clearing, empty/partial/error feeds, late Brain
  switches and exception links, plus actual MCP-runtime cases (permitted output,
  uncertain-call recovery, cancellation, expiry, Use revocation).
- Independent owner visual review approved all twelve destinations and Ask at
  1280x800, 1440x900 and 1920x1080.

The user's [display and interaction priority](../contracts/desktop-experience.md#display-and-interaction-priority)
governs: laptops and larger monitors up to about 32 inches; small-screen polish
is optional. The ADR 0017 Wiring wording discrepancy remains recorded in the
continuation mapping; Team authority was not moved or widened.

## Performance and carried proof

Twenty-sample local measurements: graph reads 64.1 ms p95, pointer-to-detail
134.6 ms, exact/lexical recall 43.8 ms; heading-visible navigation 34.1/34.6/
35.9/35.2 ms for Memory/Sources/Graph/Repositories with first-visit request counts
2/1/4/1 then 0/0/1/0; a synthetic 500-node/2,000-edge graph rendered and selected
an entity in 1,195 ms. These are bounded local observations, not production
capacity. Earlier Rust proof passed 39 workspace tests and the platform suite
(three focused reruns for SFTP/native initialization); four installed-host cases
passed in the owned Linux harness (Codex 0.154.0, Claude Code 2.1.270, synthetic
transport) and Recollect-managed Context7 completed a React lookup.

## OpenCode and Codex native proof (macOS)

OpenCode 2.0.21 discovered all six roles and both documentation skills (root and
nested `web`); an actual session observed root instructions and completed
Context7 and chrome-devtools calls, using a dedicated loopback server and a
process-only credential without changing personal defaults. Live proof exposed
three macOS FFI defects in the Keychain access-list helper (a C-string argument,
`SecAccessCreate` arity, a double release), now matched to the Apple SDK headers
and covered by a native unit test (19 agent tests and all-target Clippy pass).
Installed Codex 0.159.3 completed a model-driven direct-HTTP workflow on a
disposable Brain (empty scope, one contribution, separate retrieval, exact recall
and provenance of value 9191); a second session read the installed skill and
recalled it. The native bridge waited in `SecItemCopyMatching` until the user
approved the OS prompt, then initialized and listed all 22 tools; OpenCode
connected through the rendered stdio configuration and completed `workspace.list`.
The `capture run` collision preflight refused the fixture's direct-HTTP entry with
exit 1 and unchanged config. Cleanup deleted the disposable Brain, revoked its
device and both temporary tokens, and the inventory matched the original 16
Brain IDs. Existing Keychain entries were not migrated; OS approval remains
user-controlled.

## Validation and limits

Web typecheck/design, the production image build, agent tests and Clippy, 34
workspace library tests (three live-Vault cases ignored), documentation
validation with 32 governance tests, `git diff --check` and `cargo fmt` passed.
LongMemEval's paid answer-level benchmark, published marketplace/OAuth and
OpenCode capture hooks retain separate decisions or non-goals. No external
deployment, release, commit or push was performed. Version N/A.
