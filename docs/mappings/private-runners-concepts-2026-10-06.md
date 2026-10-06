# Private runner label and inline concepts

Observed: 2026-10-06
Confidence: verified

## Sources and Method

- User requested Private Runners naming plus read and inline Add/Edit images.
- [Desktop authority](../contracts/desktop-experience.md#private-runner-cards-and-inline-editing--2026-10-06),
  [private registration contract](../contracts/mcp-vault-and-private-runners.md)
  and current `web/src/McpPrivateRunners.tsx`/generated DTOs ground fields/states.
- Built-in imagegen generated two images and a tighter inline revision. Exact
  prompts and all images are retained in `output/imagegen/2026-10-06-private-runners/`.

## Observations

The actual tab label is now **Private Runners**, preserving `tab=runners`.
Connection guidance and its empty-registration hint use the same name; the
existing browser fixture expectation was updated. Actual CUA on SWEG — test
confirmed the selected renamed tab, no old Runners tab and zero registrations.
Read-only test tab closed; original user tab/modal/drafts preserved.

`view.png` proposes light bounded cards with paired-device identity, observed
Connected/Offline state and compact setup/edit actions. `inline-v2.png` proposes
name/Enabled editing in a stable card header, a device picker only for creation,
locked existing device binding and inline Cancel/Save. New registration shows
Not connected. Fictional Office network/Home lab examples are not real runners.
`inline-v1.png` is the initial larger alternative. `prompts.md` retains the
original and tightening prompts; `tab-name-live.png` is the actual label proof.

Web type/design/build, whitespace, CodeGraph and required governance32 passed.
Normal local stack startup installed frontend-only image
`sha256:d85254e2eda5f5b5261f5ee3badcfb1753a64546f55fbd0cd4b88b1a38594f68`;
`/health/ready` returned `{"ready":true}`. Existing backend is unchanged.
Logs: `.cache/private-runners-label-*`.

## Translation and Limits

Only naming/guidance is implemented. Inline registration/editing is a visual
proposal; the existing modal remains. Generated typography/control proportions
and read-view toggle styling are not specifications for a new mutation path.
Registration remains metadata; pairing, explicit runner startup, admin authority,
immutable device binding and independent tool Use checks retain current behavior.
No runner, token or network connection was created or tested by this task.
Version N/A; commit uncommitted. No independent review was requested for this
concept-only deliverable and small naming correction.

## Follow-up

The user subsequently approved implementation. The successor
[inline-delivery evidence](desktop-inline-private-runners-2026-10-06.md) records
the compact cards, guarded forms and local runtime proof; the concept-only
observations above remain historical.
