# Dashboard refresh stability

Observed: 2026-10-07
Confidence: verified

## Sources and Method

- Current BrainLayout, BrainPipeline and pipeline endpoint source; CodeGraph status/node/sync and bounded source reads.
- CUA in an agent-owned background browser tab against the DLAG Dashboard at 127.0.0.1:8787. Temporary Fetch fixtures affected only that tab's GET responses; no source, policy, access or model writes.
- [React state identity](https://react.dev/learn/preserving-and-resetting-state) and [TanStack Query visibility refetch](https://tanstack.com/query/latest/docs/framework/react/guides/window-focus-refetching), verified today. Context7 tools unavailable.
- Private local artifacts: `.cache/dashboard-refresh/proof.json` and `dashboard.png`; build, model-tests, governance, CodeGraph and stack logs use the same prefix in `.cache/`.

## Observations

- Before the correction, a visibility event discarded a successful response immediately: graph count zero and fresh-check message visible, despite the response being less than one second old.
- Brain.updated_at was unchanged during the initial live investigation; ordinary learning was not observed changing it. The provider key nevertheless unnecessarily remounted its complete route when metadata changed.
- Corrected build: pending visibility refresh retained the exact graph backend DOM identity and open inspector; same result after a controlled Brain metadata revision.
- Brief hidden-page fixture retained graph identity, set motion-paused state and emitted no pulses; resume kept the same canvas. The native visibility property was restored immediately.
- A controlled effective-role change reset the graph and closed inspection. This was a presentation fixture, not a real access/grant mutation.
- Held replacement request: at the original six-second deadline, graph and inspector cleared. A blocked request also cleared the graph and showed Activity unavailable. All interception/blocking was reset.
- Six normal pipeline polls completed with HTTP 200 over 11.7 seconds; same graph DOM each time, zero loading and fresh-check messages. No unexpected browser console errors.
- Existing stack rebuilt with `./scripts/stack.sh up --build`; readiness returned true. Web build/design/typecheck, two pure pipeline model tests, 32 governance fixtures and CodeGraph sync passed.
- New browser regression is authored and successfully collected in the control-panel suite. Equivalent runtime scenarios were executed through CUA; the full CLI browser suite was not run for this correction.

## Translation and Limits

Keep a successful snapshot only until its unchanged server deadline. Visibility
controls polling and motion; it is not an authorization or retention revision.
Metadata revisions still invalidate assurance reads; Brain/role boundaries retain
their reset behavior. The loading skeleton remains appropriate on first entry.
A long absence, failed request, expired snapshot or access change can still
legitimately replace content. This proves the identified reset paths; it does
not establish that every historical disappearance had that cause.

## Follow-up

Reload an already-open dashboard once to receive the deployed frontend. Work is
local and uncommitted; no push or external deployment was requested.
