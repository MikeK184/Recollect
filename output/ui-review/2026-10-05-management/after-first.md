# Independent review of first implementation — 5 October 2026

Compared the current localhost implementation with the prior live baseline and all
six approved image concepts. This is an independent visual/interaction review,
not connectivity proof or a claim that automated acceptance has passed.

## Evidence and scope

- Reviewer used a separate Edge tab after the IAB session became unavailable.
  Parent authenticated the shared local profile; reviewer entered no credentials.
  Existing user tabs were not navigated. The temporary viewport override was reset.
- All six main surfaces were inspected at 1440 × 900 and 1920 × 1080. URL/config
  dialogs were also reviewed at both sizes; approved-connector and existing-edit
  forms were exercised at 1440 × 900. The subsequent recheck should include these
  two long forms at 1920 × 1080 after the common footer is corrected.
- Read-only actions: navigation, inspection tabs, opening/cancelling forms, host
  switch, search filter and harmless synthetic config parsing. No connection
  inspections, external tool calls, model checks, credential values, policy saves,
  retries, connection changes or definition approvals were submitted.
- Retained JPEGs are in [after/](after/). High-level browser screenshot capture
  produced scaled upper-left output despite correct DOM viewport dimensions.
  Captures use the documented CDP Page.captureScreenshot API with current viewport
  offsets; image bytes were saved unchanged. The review uses actual DOM geometry
  and these accurate captures, not the broken high-level capture.
- Current AI counts changed during autonomous background activity; the final review
  observed 412 represented and 27 blocked or failed. Those are point observations.

## What now matches the approved direction

- Cream/ink/sage, serif headings, existing Recollect mark and icon family are kept.
- Brains, My agents and Connectors are global links; Connectors is installation
  scoped and displays the one actual approved definition rather than fake tiles.
- Connections uses selected rows and Overview/Tools/Activity inspection tabs.
- One Add entry offers URL, config and approved-connector methods. URL has None,
  Secret and Credential reference choices. Config format selection and inert
  parsing work. Malformed JSON displays an error; replacing it with a valid
  placeholder-based config recovers. No remote server was inspected.
- Privacy uses the same retention rows and labels in view and edit. At 1440 × 900,
  the Raw sessions label stayed at document coordinates x305 / y754.484375 in
  both states. Cancel restored the saved 30-day values. Distinct evidence and
  storage classes remain present.
- AI processing is expressed as readable permitted-purpose rows and content
  chips. Coverage is separate; detailed diagnostics open only through an explicit
  action. Configured credentials and permissions remain separate from call success.
- Agents has a clearer owner roster plus host-setup and real memory-check cards.
  Host switching works, and dates/status refer to observed use and credentials.

## Corrections required before visual acceptance

Severity: P1 materially impairs the intended interaction; P2 meaningful approved
layout/clarity gap; P3 polish. No destructive or external action was attempted.

| ID | Severity | Surface / observed issue | Concrete correction |
| --- | --- | --- | --- |
| UI-01 | P1 | Connections inspector begins at y431 at 1440 × 900, placing Test/Edit/Pause below the first viewport despite a single record. At 1920 × 1080 it still starts around y439, creating a broad empty list region. | Raise/compact the inspector and toolbar arrangement so its main actions are visible at the laptop size; keep the inspector near the upper content, as in the concept. |
| UI-02 | P1 | Parsed-config dialog footer is partially clipped by the modal body boundary at 900px height. Approved-connector/edit forms require long scrolling with Close/Save absent from the visible lower edge. | Keep Cancel/Close and primary review/save actions in a fixed visible modal footer; scroll only the form body. Compact long field groups without dropping governing fields. |
| UI-03 | P2 | Global sidebar navigation consumes flexible vertical space: Brain picker starts at y326 for 900px height and y506 for 1080px height. Larger screens create a growing blank gap and push Manage near the bottom. | Keep global links and Brain picker in a naturally flowing, consistently spaced group; reserve flexible space for the account footer. |
| UI-04 | P2 | Connection row omits the target URL and where-it-runs summary. It substitutes connector name/Brain scope, and the observed date wraps. | Show target and scope/placement in the row; use concise observed-date presentation with exact timestamp available in inspection. Give Inspect a clear textual affordance. |
| UI-05 | P2 | Global Connectors is missing the approved contextual Add/import/review panel. With one real definition, the page has a large unused canvas; search and state filter are widely separated at 1920. | Add the useful side panel and import/review explanation from the vision; keep the actual catalogue count and compact filters. Do not add illustrative connectors. |
| UI-06 | P2 | No-match connector state says to add a configuration, which confuses filtered-empty with an empty catalogue and encourages duplicate setup. | Explain the active filters and offer Clear search / change filters. Reserve Add guidance for a truly empty catalogue. |
| UI-07 | P2 | Privacy Standard retention status stretches across roughly half the content width as a thin bar. Sections are substantially taller than the approved compact layout. | Prevent the badge from flex-growing; preserve a short status pill. Reduce section/row spacing while retaining all separate policy classes and stable edit geometry. |
| UI-08 | P2 | AI primary card pushes allowed-content chips below the 900px fold and below most of 1080px. Coverage emphasizes 0 pending while the blocked/failed count is a small line. Normal model card still displays embedding dimensions. | Compact policy rows/introduction so content classes are visible; emphasize the actual blocked/failed count and attention state. Keep dimensions in diagnostics. |
| UI-09 | P2 | Agent setup and read-check cards display full long prompts and UUIDs before their copy actions, causing long lower-page cards. Connect agent uses a pale secondary treatment. Subtitle says agents are connected, while evidence is observed usage and credential validity. | Use compact setup/read-check presentation with prominent copy actions and optional full-prompt details; make Connect agent a clear primary action; use the approved observed-use subtitle. |
| UI-10 | P2 | Approved-connector selection enables Save connection immediately after choosing a definition, even when name and target are blank. No submission was attempted, so backend/native validation is unverified here. | Verify required-field handling; keep save disabled or provide inline validation until the draft is valid. |
| UI-11 | P3 | Tools inspector and global definition dialog expose an enormous unformatted tool description by default. The first Context7 tool alone extends far beyond the viewport. | Show a concise bounded preview per tool with an explicit full-description expansion; preserve the exact cached metadata. |
| UI-12 | P3 | Config success lacks the short parsed-count/transport confirmation from the vision; error title is generic and contains the technical word bounded. Config import uses an ordinary textarea with no gutter. | Add concise parse-success feedback and a specific parse-error heading. A line gutter is optional visual polish, not a reason to add a new editor dependency. |

The credential-provider destination differs from the illustrative concept because
the implementation explicitly supports the installation-local development file
and approved aliases. Do not claim an OS store or production vault is available.
If file import is part of this approved slice, its visible import action is also
missing: the current global flow only offers URL and Paste config.

## Remaining validation boundaries

No horizontal page overflow was observed; AI and Agents document width checks
equalled the requested viewport width at the measured sizes. Role/no-access,
stale revisions, duplicate creation, provider-file behavior and network failures
remain the parent's isolated fixture/API test responsibility. This browser review
did not modify shared data to synthesize those states. Await corrections and a
second deployed comparison before accepting the visual slice.
